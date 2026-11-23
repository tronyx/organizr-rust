//! `POST /login`: logging in with a name and a password.

use std::time::{Duration, Instant, SystemTime};

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};
use organizr_auth::external::{self, ExternalAccount};
use organizr_auth::login::{self, Device, LoggedIn, LoginError, SecondFactor};
use organizr_auth::login_limit::{Attempt, LimitPolicy};
use organizr_domain::{GroupLevel, Timestamp};
use organizr_integrations::plex::PlexAccount;
use organizr_settings::{AuthType, Settings};

use crate::api_data::ApiData;
use crate::caller::{now_stamp, session_cookie_name};
use crate::client::{Client, LocalNetworks};
use crate::cookies::{self, CookieScope};
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::external_login::{self, HandOff};
use crate::handlers::login_backends;
use crate::handlers::mail;
use crate::state::AppState;

/// How long a login lasts when the user did not ask to be remembered.
const ONE_DAY: Duration = Duration::from_hours(24);

/// The longest a login lasts, in days, whatever the setting says.
const MOST_DAYS: u32 = 365;

/// `POST /login`: checks a name, or an e-mail address, and a password. When they fit,
/// the answer sets the session cookie.
///
/// Fields of the body: `username`, `password` and, to stay logged in for the number of
/// days of the `rememberMeDays` setting instead of one day, `remember` with any value.
///
/// Failed logins are counted by the address of the client and by the name. When either
/// count reaches the `loginAttempts` setting, the answer is `403 "User is locked out"`,
/// for that login and for every login in the next `loginLockout` milliseconds, whatever
/// the password. The field `loginAttempts` of the body, in which the browser of the PHP
/// application sent its own count, is not read.
pub async fn login(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Response, ApiError> {
    let settings = state.settings();
    // A missing field counts as an empty text, as in the PHP application.
    let name = data.text("username").unwrap_or_default();
    let password = data.text("password").unwrap_or_default();
    let lifetime = if data.is_set("remember") {
        ONE_DAY * settings.remember_me_days.min(MOST_DAYS)
    } else {
        ONE_DAY
    };

    let limiter = state.login_limiter();
    let attempt = Attempt::new(client.address, &name);
    if let Some(retry_after) = limiter.locked_for(&attempt, Instant::now()) {
        tracing::warn!(name = %name, client = %client.address, "a login was refused during a lock-out");
        return Err(ApiError::LockedOut { retry_after });
    }

    let now = SystemTime::now();
    let device = device_of(&client, &headers);

    // The code of the second factor. With the `ignoreTFALocal` setting, a login from
    // the local network needs none.
    let code = data.text("tfaCode").unwrap_or_default();
    let second_factor = SecondFactor {
        code: &code,
        secrets: state.secrets(),
        skip: settings.ignore_tfa_local
            && LocalNetworks::from_setting(&settings.local_ip_list).contains(&client),
    };
    let request = Attempted {
        headers: &headers,
        client: &client,
        name: &name,
        password: &password,
        plex_token: data.text("oAuth").filter(|token| !token.is_empty()),
        kind_of_token: data.text("oAuthType"),
        second_factor,
        device,
        now: now_stamp(now),
        lifetime,
    };
    let result = check_login(&state, &settings, request).await;
    let outcome = match result {
        Ok(outcome) => outcome,
        // A wrong code counts as a failed login, as a wrong password does.
        Err(error @ (LoginError::WrongCredentials | LoginError::WrongSecondFactor)) => {
            tracing::warn!(name = %name, client = %client.address, %error, "a login failed");
            let policy = LimitPolicy {
                failures: settings.login_attempts,
                lockout: Duration::from_millis(settings.login_lockout),
            };
            // The failure that reaches the limit already gets the lock-out answer.
            return Err(
                match limiter.record_failure(&attempt, policy, Instant::now()) {
                    Some(retry_after) => {
                        tracing::warn!(
                            name = %name,
                            client = %client.address,
                            seconds = retry_after.as_secs(),
                            "too many logins failed; the client and the name are locked out"
                        );
                        ApiError::LockedOut { retry_after }
                    }
                    None => error.into(),
                },
            );
        }
        Err(error) => return Err(error.into()),
    };
    limiter.record_success(&attempt);
    let logged_in = &outcome.logged_in;
    tracing::info!(user = %logged_in.user.username, client = %client.address, "a user logged in");
    if outcome.created {
        let site = mail::site_address(&settings, &headers, &client);
        let user = &logged_in.user;
        mail::welcome(&state, &site, &user.username, user.email.as_deref());
        crate::handlers::homepage::ombi::import_users_later(&state, &settings, &headers, &client);
    }

    tell_about_a_new_device(&state, &headers, &client, logged_in, now_stamp(now)).await?;

    let session_cookie = CookieScope::of(&headers, client.https).set_hidden(
        &session_cookie_name(&settings),
        logged_in.token.expose(),
        now + lifetime,
    )?;
    // The PHP application leaves the text of the code check in the answer.
    let answer = if logged_in.second_factor_checked {
        Envelope::success(NoData).with_message("Verification code verified")
    } else {
        Envelope::success(NoData)
    };
    let mut response = answer.into_response();
    cookies::append(&mut response, session_cookie);
    let scope = CookieScope::of(&headers, client.https);
    if outcome.with_plex_token {
        // The PHP application marks a login with a Plex token for its interface.
        cookies::append(&mut response, scope.set("oAuth", "true", now + lifetime)?);
    }

    let expires = now + ONE_DAY * settings.remember_me_days.min(MOST_DAYS);
    let sessions = single_sign_on(&state, &headers, &client, &outcome, &password, expires).await?;
    cookies::append(&mut response, sessions);
    Ok(response)
}

/// Gets the user a session of each application of single sign-on, and returns the
/// cookies for the browser. They last as long as a login with "remember me" does.
async fn single_sign_on(
    state: &AppState,
    headers: &HeaderMap,
    client: &Client,
    outcome: &Outcome,
    password: &str,
    expires: SystemTime,
) -> Result<Vec<HeaderValue>, ApiError> {
    let user = &outcome.logged_in.user;
    let level = state
        .database()
        .groups()
        .find(user.group)
        .await?
        .map_or(GroupLevel::GUEST, |group| group.level);
    let hand_off = HandOff {
        user,
        level,
        password,
        plex_token: outcome.plex_token.as_deref(),
        expires,
    };
    Ok(external_login::hand_off(state, headers, client, hand_off).await)
}

/// What a login brings.
struct Attempted<'a> {
    /// The request, for the addresses of services that are paths on this host.
    headers: &'a HeaderMap,
    client: &'a Client,
    name: &'a str,
    password: &'a str,
    /// The token of the field `oAuth`.
    plex_token: Option<String>,
    /// The field `oAuthType`.
    kind_of_token: Option<String>,
    second_factor: SecondFactor<'a>,
    device: Device,
    now: Timestamp,
    lifetime: Duration,
}

/// A login that passed.
struct Outcome {
    logged_in: LoggedIn,
    /// Whether the login made the account.
    created: bool,
    /// The Plex token of the user, when Plex checked the login.
    plex_token: Option<String>,
    /// Whether the login came with a Plex token and no password.
    with_plex_token: bool,
}

/// Who vouched for a login.
enum Vouched {
    /// Plex, with the account that it describes.
    Plex(PlexAccount),
    /// One of the other back ends of `authBackend`.
    Server(login_backends::Vouched),
}

/// Checks a login: with Plex or the other back end of `authBackend` when the request or
/// the settings ask for that, and with the stored password when the settings allow it.
async fn check_login(
    state: &AppState,
    settings: &Settings,
    request: Attempted<'_>,
) -> Result<Outcome, LoginError> {
    let database = state.database();
    let with_plex_token = request.plex_token.is_some();
    let vouched = if let Some(token) = &request.plex_token {
        if request.kind_of_token.as_deref() != Some(external_login::PLEX_BACKEND) {
            return Err(LoginError::UnknownTokenKind);
        }
        if !settings.plex_oauth {
            return Err(LoginError::TokenLoginIsOff);
        }
        match external_login::by_plex_token(state, settings, token).await {
            Some(account) => Some(Vouched::Plex(account)),
            None => return Err(LoginError::WrongCredentials),
        }
    } else if settings.auth_type == AuthType::Internal {
        None
    } else if settings.auth_backend == external_login::PLEX_BACKEND {
        external_login::by_plex_password(state, settings, request.name, request.password)
            .await
            .map(Vouched::Plex)
    } else if login_backends::is_password_backend(&settings.auth_backend) {
        let site = (request.headers, request.client);
        login_backends::check(state, settings, site, request.name, request.password)
            .await
            .map(Vouched::Server)
    } else {
        None
    };

    let Some(vouched) = vouched else {
        // Only the service counts with `external`. Every other setting takes the stored
        // password, also after the service said no or could not be reached.
        if settings.auth_type == AuthType::External {
            return Err(LoginError::WrongCredentials);
        }
        let logged_in = login::with_password(
            database,
            request.name,
            request.password,
            request.second_factor,
            request.device,
            request.now,
            request.lifetime,
        )
        .await?;
        return Ok(Outcome {
            logged_in,
            created: false,
            plex_token: None,
            with_plex_token: false,
        });
    };

    let second_factor = SecondFactor {
        // With the `ignoreTFAIfPlexOAuth` setting, Plex is the second factor.
        skip: request.second_factor.skip || (with_plex_token && settings.ignore_tfa_if_plex_o_auth),
        ..request.second_factor
    };
    let account = match &vouched {
        Vouched::Plex(account) => ExternalAccount {
            username: &account.username,
            email: &account.email,
            password: (!with_plex_token).then_some(request.password),
            plex_token: Some(&account.token),
            found_by: None,
        },
        Vouched::Server(account) => ExternalAccount {
            username: &account.username,
            email: &account.email,
            password: Some(request.password),
            plex_token: None,
            found_by: Some(&account.typed),
        },
    };
    let done = external::log_in(
        database,
        account,
        second_factor,
        request.device,
        request.now,
        request.lifetime,
    )
    .await?;
    Ok(Outcome {
        logged_in: done.logged_in,
        created: done.created,
        plex_token: match vouched {
            Vouched::Plex(account) => Some(account.token),
            Vouched::Server(_) => None,
        },
        with_plex_token,
    })
}

/// The device that a request comes from: the address of the client and the name that
/// the browser gives itself.
pub fn device_of(client: &Client, headers: &HeaderMap) -> Device {
    Device {
        ip: client.address,
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    }
}

/// Sends the user an e-mail when this login is the first session of the account from
/// this browser. The mail plugin must be on, and the account must have an address.
async fn tell_about_a_new_device(
    state: &AppState,
    headers: &HeaderMap,
    client: &Client,
    logged_in: &LoggedIn,
    now: Timestamp,
) -> Result<(), ApiError> {
    let settings = state.settings();
    if !settings.mailer_enabled {
        return Ok(());
    }
    let (Some(address), Some(browser)) = (
        logged_in.user.email.as_deref(),
        logged_in.session.user_agent.as_deref(),
    ) else {
        return Ok(());
    };
    let sessions = state
        .database()
        .sessions()
        .list_valid_of_user(logged_in.user.id, now)
        .await?;
    let from_this_browser = sessions
        .iter()
        .filter(|session| session.user_agent.as_deref() == Some(browser))
        .count();
    if from_this_browser <= 1 {
        let site = mail::site_address(&settings, headers, client);
        let device = mail::NewDevice {
            username: &logged_in.user.username,
            address,
            ip: client.address,
            browser,
        };
        mail::new_device(state, &site, device);
    }
    Ok(())
}
