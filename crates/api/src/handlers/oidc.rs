//! The login with an OpenID Connect provider: `GET /oidc/providers`,
//! `GET /oidc/{provider}/authorize`, `GET /oidc/{provider}/callback` and
//! `GET /oidc/{provider}/test`.
//!
//! The browser goes to the provider with a new `state`, `nonce` and PKCE verifier. The
//! three values wait in a cookie of their own until the browser comes back: the login
//! cookie, sealed with the key of the install, so nobody can read or forge it. It lasts
//! ten minutes, as the session of PHP did, and only for the routes of this login. The
//! callback takes it away again, whatever the outcome.
//!
//! No log line or answer holds a token, a code, the client secret or the values of the
//! login cookie.

use std::time::{Duration, SystemTime};

use axum::Extension;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use organizr_auth::login;
use organizr_auth::oidc::{self as accounts, OidcAccount, OidcAccountError};
use organizr_auth::secrets::{PREFIX, SecretBox};
use organizr_domain::GroupLevel;
use organizr_integrations::oidc::{
    self, Application, Callback, Discovered, MappingMode, OidcError, Person,
};
use organizr_settings::Settings;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::caller::{Caller, now_stamp, session_cookie_name};
use crate::client::Client;
use crate::cookies::{self, CookieScope, cookie_value};
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::handlers::login::device_of;
use crate::handlers::mail::request_site;
use crate::handlers::notice_page;
use crate::router::API_BASE;
use crate::state::AppState;

/// The cookie that holds a login until the browser comes back from the provider.
const LOGIN_COOKIE: &str = "organizr_oidc_login";

/// How long a login at the provider may take, in seconds.
const LOGIN_TIME: i64 = 600;

/// The name under which the login cookie is sealed. A value that was sealed for
/// another purpose does not open as a login cookie.
const SEALED_AS: &str = "oidc/login";

/// Where the browser goes after a login: the start page of this site, as in the PHP
/// application.
const START_PAGE: &str = "/";

/// How long a session lasts when the setting names more days.
const MOST_DAYS: u32 = 365;

const ONE_DAY: Duration = Duration::from_hours(24);

/// The providers, in the order of the PHP application.
const PROVIDERS: [&str; 4] = ["authentik", "keycloak", "pocketid", "zitadel"];

/// A provider with its settings.
#[derive(Debug, Clone, Copy)]
struct Provider<'a> {
    key: &'static str,
    enabled: bool,
    name: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
    discovery_url: &'a str,
    scopes: &'a str,
    /// The setting "group claim override" of the provider.
    group_claim: &'a str,
    /// The claim of the groups when no setting names one.
    default_group_claim: &'static str,
    /// Whether the provider sends its roles as the keys of an object (Zitadel).
    roles_as_object: bool,
}

impl<'a> Provider<'a> {
    fn of(settings: &'a Settings, key: &str) -> Option<Self> {
        let provider = match key {
            "authentik" => Self {
                key: "authentik",
                enabled: settings.oidc_authentik_enabled,
                name: &settings.oidc_authentik_name,
                client_id: &settings.oidc_authentik_client_id,
                client_secret: &settings.oidc_authentik_client_secret,
                discovery_url: &settings.oidc_authentik_discovery_url,
                scopes: &settings.oidc_authentik_scopes,
                group_claim: &settings.oidc_authentik_group_claim,
                default_group_claim: "groups",
                roles_as_object: false,
            },
            "keycloak" => Self {
                key: "keycloak",
                enabled: settings.oidc_keycloak_enabled,
                name: &settings.oidc_keycloak_name,
                client_id: &settings.oidc_keycloak_client_id,
                client_secret: &settings.oidc_keycloak_client_secret,
                discovery_url: &settings.oidc_keycloak_discovery_url,
                scopes: &settings.oidc_keycloak_scopes,
                group_claim: &settings.oidc_keycloak_group_claim,
                default_group_claim: "groups",
                roles_as_object: false,
            },
            "pocketid" => Self {
                key: "pocketid",
                enabled: settings.oidc_pocketid_enabled,
                name: &settings.oidc_pocketid_name,
                client_id: &settings.oidc_pocketid_client_id,
                client_secret: &settings.oidc_pocketid_client_secret,
                discovery_url: &settings.oidc_pocketid_discovery_url,
                scopes: &settings.oidc_pocketid_scopes,
                group_claim: &settings.oidc_pocketid_group_claim,
                default_group_claim: "groups",
                roles_as_object: false,
            },
            "zitadel" => Self {
                key: "zitadel",
                enabled: settings.oidc_zitadel_enabled,
                name: &settings.oidc_zitadel_name,
                client_id: &settings.oidc_zitadel_client_id,
                client_secret: &settings.oidc_zitadel_client_secret,
                discovery_url: &settings.oidc_zitadel_discovery_url,
                scopes: &settings.oidc_zitadel_scopes,
                group_claim: &settings.oidc_zitadel_group_claim,
                default_group_claim: "urn:zitadel:iam:org:project:roles",
                roles_as_object: true,
            },
            _ => return None,
        };
        Some(provider)
    }

    /// The provider, when the login and the provider are switched on.
    fn enabled(settings: &'a Settings, key: &str) -> Option<Self> {
        Self::of(settings, key).filter(|provider| settings.oidc_enabled && provider.enabled)
    }

    /// The claim that holds the groups: the setting of the provider, or else the
    /// global setting, or else the usual claim of the provider.
    fn group_claim(&self, settings: &'a Settings) -> &'a str {
        [self.group_claim, settings.oidc_group_claim_name.as_str()]
            .into_iter()
            .map(str::trim)
            .find(|claim| !claim.is_empty())
            .unwrap_or(self.default_group_claim)
    }

    fn application(&self, redirect: &'a str) -> Application<'a> {
        Application {
            client_id: self.client_id,
            client_secret: self.client_secret,
            redirect,
        }
    }
}

/// The address of the callback of a provider, as the PHP application builds it: the
/// address of this site that the browser used, then `api/v2/oidc/<provider>/callback`.
/// The address must be listed at the provider, which refuses any other.
fn redirect_address(headers: &HeaderMap, client: &Client, provider: &str) -> Option<String> {
    let site = request_site(headers, client);
    (!site.is_empty()).then(|| {
        format!(
            "{site}{}/oidc/{provider}/callback",
            API_BASE.trim_start_matches('/')
        )
    })
}

/// `GET /oidc/providers`: the providers that are switched on, with the name that the
/// login page shows. Anybody may ask.
pub async fn providers(State(state): State<AppState>) -> Envelope<Value> {
    let settings = state.settings();
    let listed: Map<String, Value> = PROVIDERS
        .into_iter()
        .filter_map(|key| Provider::enabled(&settings, key))
        .map(|provider| {
            (
                provider.key.to_owned(),
                json!({"name": provider.name, "enabled": true}),
            )
        })
        .collect();
    // PHP sends an empty list for no provider, and an object for some.
    Envelope::success(if listed.is_empty() {
        json!([])
    } else {
        Value::Object(listed)
    })
}

/// What the login page and the logout of the interface need, in the `launch` answer:
/// whether the login is on, the provider of the automatic redirect, or an empty text,
/// and the custom logout address, or an empty text. The PHP application writes the
/// redirect into its login page on the server.
pub fn login_page(settings: &Settings) -> Value {
    let redirect = if settings.oidc_auto_redirect {
        Provider::enabled(settings, &settings.oidc_auto_redirect_provider)
            .map_or("", |provider| provider.key)
    } else {
        ""
    };
    let logout = if settings.oidc_enabled {
        settings.oidc_auto_redirect_logout_url.trim()
    } else {
        ""
    };
    json!({"enabled": settings.oidc_enabled, "autoRedirect": redirect, "logoutURL": logout})
}

/// `GET /oidc/{provider}/authorize`: sends the browser to the login page of the
/// provider, with the login cookie.
pub async fn authorize(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Response, Refusal> {
    let settings = state.settings();
    if !settings.oidc_enabled {
        return Err(Refusal::Request(
            StatusCode::FORBIDDEN,
            "OIDC is not enabled".to_owned(),
        ));
    }
    let Some(provider) = Provider::enabled(&settings, &key) else {
        return Err(Refusal::not_found(&format!("Provider not enabled: {key}")));
    };
    let failed = || {
        Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to generate authorization URL".to_owned(),
        )
    };
    let redirect = redirect_address(&headers, &client, provider.key).ok_or_else(failed)?;
    let discovered = discover(&state, &provider).await.map_err(|_| failed())?;
    let started = oidc::start(&discovered, provider.application(&redirect), provider.scopes)
        .map_err(|error| {
            tracing::warn!(provider = provider.key, %error, "the login at the provider could not start");
            failed()
        })?;
    let login = LoginCookie {
        provider: provider.key.to_owned(),
        state: started.state,
        nonce: started.nonce,
        verifier: started.verifier,
        back: START_PAGE.to_owned(),
        expires: now_seconds().saturating_add(LOGIN_TIME),
    };
    let cookie = login.seal(state.secrets()).ok_or_else(failed)?;
    let mut response = (StatusCode::FOUND, [(header::LOCATION, started.address)]).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        login_cookie(&cookie, LOGIN_TIME, client.https).ok_or_else(failed)?,
    );
    Ok(response)
}

/// `GET /oidc/{provider}/callback`: where the provider sends the browser back. After a
/// good login, the browser gets the session cookie and goes to the start page. Any
/// failure gets a page that says what failed, and no session. The answer is HTML, not
/// the envelope of the API, as in the PHP application.
pub async fn callback(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path(key): Path<String>,
    RawQuery(query): RawQuery,
) -> Response {
    let kept = cookie_value(&headers, LOGIN_COOKIE)
        .and_then(|cookie| LoginCookie::open(state.secrets(), cookie, now_seconds()));
    let mut response = match finish(&state, &client, &headers, &key, query.as_deref(), kept).await {
        Ok(response) => response,
        Err(message) => notice_page::login_failed(&message),
    };
    // The login cookie is used once, whatever the outcome.
    if let Some(gone) = login_cookie("", 0, client.https) {
        response.headers_mut().append(header::SET_COOKIE, gone);
    }
    response
}

/// The steps of the callback, in the order of the PHP application. The error is the
/// text of the page.
async fn finish(
    state: &AppState,
    client: &Client,
    headers: &HeaderMap,
    key: &str,
    query: Option<&str>,
    kept: Option<LoginCookie>,
) -> Result<Response, String> {
    if let Some(error) = query_field(query, "error").filter(|error| !error.is_empty()) {
        tracing::warn!(
            provider = key,
            "the provider answered the login with an error"
        );
        // The PHP application shows the description of the provider. Anybody can write
        // a link with any text there, so the page names only the error code, when it
        // has the form of one.
        let is_code = error.len() <= 64
            && error
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_');
        return Err(if is_code {
            format!("The provider refused the login: {error}")
        } else {
            "The provider refused the login".to_owned()
        });
    }
    let code = query_field(query, "code").filter(|code| !code.is_empty());
    let sent_state = query_field(query, "state").filter(|state| !state.is_empty());
    let (Some(code), Some(sent_state)) = (code, sent_state) else {
        return Err("Missing code or state parameter".to_owned());
    };
    let invalid_state = || "Invalid or expired state parameter".to_owned();
    let Some(kept) = kept else {
        tracing::warn!(
            provider = key,
            "the login cookie is missing, expired or not valid"
        );
        return Err(invalid_state());
    };
    if !same_text(&kept.state, &sent_state) || kept.provider != key {
        tracing::warn!(provider = key, "the state of the login does not match");
        return Err(invalid_state());
    }
    let settings = state.settings();
    let Some(provider) = Provider::enabled(&settings, key) else {
        tracing::warn!(
            provider = key,
            "a login came back for a provider that is switched off"
        );
        return Err(format!("Provider not enabled: {key}"));
    };

    let exchange_failed = || "Failed to exchange authorization code".to_owned();
    let redirect = redirect_address(headers, client, provider.key).ok_or_else(exchange_failed)?;
    let discovered = discover(state, &provider)
        .await
        .map_err(|_| exchange_failed())?;
    let outbound = state.outbound().ok_or_else(exchange_failed)?;
    let callback = Callback {
        code: &code,
        nonce: &kept.nonce,
        verifier: &kept.verifier,
    };
    let person = oidc::finish(
        outbound,
        &discovered,
        provider.application(&redirect),
        callback,
    )
    .await
    .map_err(|error| {
        tracing::warn!(provider = provider.key, %error, "the login at the provider failed");
        match error {
            OidcError::WrongNonce => "Invalid nonce".to_owned(),
            OidcError::InvalidIdToken(_) | OidcError::NoIdToken => "Invalid ID token".to_owned(),
            _ => exchange_failed(),
        }
    })?;

    let user = account_of(state, &settings, &provider, &person).await?;
    let now = SystemTime::now();
    let lifetime = ONE_DAY * settings.remember_me_days.min(MOST_DAYS);
    let authentication_failed = || "Authentication failed".to_owned();
    let logged_in = login::start_session(
        state.database(),
        user,
        device_of(client, headers),
        now_stamp(now),
        lifetime,
    )
    .await
    .map_err(|error| {
        tracing::error!(%error, "the session of a login at a provider could not be started");
        authentication_failed()
    })?;
    let session_cookie = CookieScope::of(headers, client.https)
        .set_hidden(
            &session_cookie_name(&settings),
            logged_in.token.expose(),
            now + lifetime,
        )
        .map_err(|_| authentication_failed())?;
    tracing::info!(user = %logged_in.user.username, provider = provider.key, client = %client.address, "a user logged in at a provider");

    let mut response = (
        StatusCode::FOUND,
        [(header::LOCATION, safe_place(&kept.back))],
    )
        .into_response();
    cookies::append(&mut response, session_cookie);
    Ok(response)
}

/// The account of the person, found, linked or made as the settings say.
async fn account_of(
    state: &AppState,
    settings: &Settings,
    provider: &Provider<'_>,
    person: &Person,
) -> Result<organizr_domain::User, String> {
    let failed = || "Failed to create or link user".to_owned();
    let Some(username) = person.username() else {
        tracing::warn!(provider = provider.key, "the provider named no user name");
        return Err(failed());
    };
    let groups = oidc::groups_of(
        &person.other,
        provider.group_claim(settings),
        provider.roles_as_object,
    );
    let mode = MappingMode::from_setting(&settings.oidc_group_mapping_mode);
    let level = oidc::mapped_level(&groups, &settings.oidc_group_mappings, mode)
        .or_else(|| u16::try_from(settings.oidc_default_group_id).ok())
        .and_then(|level| GroupLevel::new(level).ok())
        // The level that the PHP application takes when the setting is no level.
        .unwrap_or_else(|| GroupLevel::new(3).unwrap_or(GroupLevel::GUEST));
    let account = OidcAccount {
        provider: provider.key,
        username,
        email: person.email.as_deref().unwrap_or_default(),
        email_verified: person.email_verified,
        picture: person.picture.as_deref(),
        level,
        link_existing: settings.oidc_link_existing_users,
        auto_create: settings.oidc_auto_create_users,
        update_group: settings.oidc_update_groups_on_login,
    };
    let now = now_stamp(SystemTime::now());
    match accounts::find_or_create(state.database(), account, now).await {
        Ok(found) => Ok(found.user),
        Err(
            error @ (OidcAccountError::NotLinked(_)
            | OidcAccountError::NoAccount(_)
            | OidcAccountError::Protected(_)),
        ) => {
            tracing::warn!(provider = provider.key, %error, "a login at the provider has no account");
            Err(failed())
        }
        Err(error) => {
            tracing::error!(provider = provider.key, %error, "the account of a login at the provider could not be used");
            Err(failed())
        }
    }
}

/// `GET /oidc/{provider}/test`: reads the discovery document of a provider, for the
/// settings page. Admins and Co-Admins only.
pub async fn test(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(key): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let settings = state.settings();
    let Some(provider) = Provider::of(&settings, &key) else {
        return Err(Refusal::not_found("Provider not found"));
    };
    if provider.discovery_url.is_empty() {
        return Err(Refusal::unprocessable("Discovery URL not configured"));
    }
    if provider.client_id.is_empty() {
        return Err(Refusal::unprocessable("Client ID not configured"));
    }
    let failed = |message: String| Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, message);
    match discover(&state, &provider).await {
        Ok(discovered) => Ok(Envelope::success(Value::Null).with_message(format!(
            "OIDC connection successful - Issuer: {}",
            discovered.issuer()
        ))),
        Err(OidcError::Missing(field)) => {
            Err(failed(format!("Discovery document missing: {field}")))
        }
        Err(OidcError::InvalidDiscovery(_)) => {
            Err(failed("Discovery document is not valid".to_owned()))
        }
        Err(OidcError::Keys(_)) => Err(failed("Failed to fetch the signing keys".to_owned())),
        Err(_) => Err(failed("Failed to fetch discovery document".to_owned())),
    }
}

/// Reads the discovery document of a provider, and logs why it failed.
async fn discover(state: &AppState, provider: &Provider<'_>) -> Result<Discovered, OidcError> {
    let Some(outbound) = state.outbound() else {
        return Err(OidcError::Discovery(
            "requests to other services cannot be sent".to_owned(),
        ));
    };
    if provider.discovery_url.is_empty() {
        tracing::warn!(
            provider = provider.key,
            "the discovery URL of the provider is not set"
        );
        return Err(OidcError::Discovery("the address is not set".to_owned()));
    }
    oidc::discover(outbound, provider.discovery_url)
        .await
        .inspect_err(|error| tracing::warn!(provider = provider.key, %error, "the discovery of the provider failed"))
}

/// What the login cookie holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LoginCookie {
    provider: String,
    state: String,
    nonce: String,
    verifier: String,
    /// The place of this site to go to after the login.
    back: String,
    /// Until when the login may come back, in seconds since 1970.
    expires: i64,
}

impl LoginCookie {
    /// The value of the cookie: the sealed JSON, as hexadecimal digits.
    fn seal(&self, secrets: &SecretBox) -> Option<String> {
        let text = serde_json::to_string(self).ok()?;
        let sealed = secrets
            .seal(SEALED_AS, &text)
            .inspect_err(|error| tracing::error!(%error, "the login cookie could not be sealed"))
            .ok()?;
        sealed.strip_prefix(PREFIX).map(str::to_owned)
    }

    /// The login of a cookie value, when it opens with the key of the install and has
    /// not expired.
    fn open(secrets: &SecretBox, value: &str, now: i64) -> Option<Self> {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let text = secrets.open(SEALED_AS, &format!("{PREFIX}{value}")).ok()?;
        let login: Self = serde_json::from_str(&text).ok()?;
        (login.expires > now).then_some(login)
    }
}

/// The `Set-Cookie` value of the login cookie. An empty value takes the cookie away.
fn login_cookie(value: &str, age: i64, secure: bool) -> Option<HeaderValue> {
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{LOGIN_COOKIE}={value}; Path={API_BASE}/oidc; Max-Age={age}; HttpOnly; SameSite=Lax{secure}"
    ))
    .ok()
}

/// A place inside this site: a path that starts with one `/`. Anything else, which
/// could lead to another site, is the start page.
fn safe_place(place: &str) -> &str {
    let mut characters = place.chars();
    let inside = characters.next() == Some('/')
        && !matches!(characters.next(), Some('/' | '\\'))
        && !place
            .chars()
            .any(|character| character.is_control() || character == '\\');
    if inside { place } else { START_PAGE }
}

/// Whether two texts are equal, in a time that does not tell how much of them matched.
fn same_text(left: &str, right: &str) -> bool {
    use subtle::ConstantTimeEq as _;
    left.len() == right.len() && bool::from(left.as_bytes().ct_eq(right.as_bytes()))
}

/// A field of a query, with `%xx` and `+` read as their characters.
fn query_field(query: Option<&str>, name: &str) -> Option<String> {
    form_urlencoded::parse(query?.as_bytes())
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use organizr_settings::Settings;

    use super::{LoginCookie, Provider, login_cookie, safe_place};

    fn login(expires: i64) -> LoginCookie {
        LoginCookie {
            provider: "keycloak".to_owned(),
            state: "s".to_owned(),
            nonce: "n".to_owned(),
            verifier: "v".to_owned(),
            back: "/".to_owned(),
            expires,
        }
    }

    #[test]
    fn login_cookie_opens_only_sealed_and_in_time() {
        let secrets = organizr_testkit::seed::secrets();
        let sealed = login(1_000).seal(&secrets).expect("it seals");
        assert!(sealed.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(!sealed.contains("keycloak"));
        assert_eq!(
            LoginCookie::open(&secrets, &sealed, 999),
            Some(login(1_000))
        );
        assert_eq!(LoginCookie::open(&secrets, &sealed, 1_000), None);

        // A plain value, a changed value and a value of another key do not open.
        let plain = hex::encode(serde_json::to_string(&login(1_000)).expect("json"));
        assert_eq!(LoginCookie::open(&secrets, &plain, 0), None);
        let mut changed = sealed.clone().into_bytes();
        let last = changed.last_mut().expect("not empty");
        *last = if *last == b'0' { b'1' } else { b'0' };
        let changed = String::from_utf8(changed).expect("hex");
        assert_eq!(LoginCookie::open(&secrets, &changed, 0), None);
        let other = organizr_auth::secrets::SecretBox::with_random_key().expect("a box");
        assert_eq!(LoginCookie::open(&other, &sealed, 0), None);
        assert_eq!(LoginCookie::open(&secrets, "", 0), None);
        assert_eq!(LoginCookie::open(&secrets, "zz", 0), None);
    }

    #[test]
    fn login_cookie_is_short_lived_and_only_for_the_routes_of_the_login() {
        assert_eq!(
            login_cookie("ab", 600, true).expect("a header"),
            "organizr_oidc_login=ab; Path=/api/v2/oidc; Max-Age=600; HttpOnly; SameSite=Lax; Secure"
        );
        assert_eq!(
            login_cookie("", 0, false).expect("a header"),
            "organizr_oidc_login=; Path=/api/v2/oidc; Max-Age=0; HttpOnly; SameSite=Lax"
        );
    }

    #[test]
    fn only_places_inside_this_site_are_followed() {
        assert_eq!(safe_place("/"), "/");
        assert_eq!(safe_place("/#/settings"), "/#/settings");
        for outside in [
            "https://evil.example.com/",
            "//evil.example.com/",
            "/\\evil.example.com",
            "evil",
            "",
            "/\nSet-Cookie: x",
        ] {
            assert_eq!(safe_place(outside), "/", "{outside:?}");
        }
    }

    #[test]
    fn group_claim_is_the_override_then_the_global_setting_then_the_usual_claim() {
        let mut settings = Settings::defaults();
        let zitadel = Provider::of(&settings, "zitadel").expect("a provider");
        assert_eq!(
            zitadel.group_claim(&settings),
            "urn:zitadel:iam:org:project:roles"
        );
        settings.oidc_zitadel_group_claim = String::new();
        let zitadel = Provider::of(&settings, "zitadel").expect("a provider");
        assert_eq!(zitadel.group_claim(&settings), "groups");
        settings.oidc_group_claim_name = String::new();
        let zitadel = Provider::of(&settings, "zitadel").expect("a provider");
        assert_eq!(
            zitadel.group_claim(&settings),
            "urn:zitadel:iam:org:project:roles"
        );
        settings.oidc_keycloak_group_claim = "roles".to_owned();
        let keycloak = Provider::of(&settings, "keycloak").expect("a provider");
        assert_eq!(keycloak.group_claim(&settings), "roles");
        assert!(Provider::of(&settings, "other").is_none());
    }
}
