//! What a login does with other services: Plex checks who a person is, and the
//! applications of single sign-on get a session for the person.

use std::time::{Duration, SystemTime};

use axum::http::{HeaderMap, HeaderValue, header};
use organizr_domain::{GroupLevel, User};
use organizr_integrations::http::Outbound;
use organizr_integrations::plex::PlexAccount;
use organizr_integrations::sso::{self, Fallback, Login};
use organizr_settings::Settings;

use crate::client::Client;
use crate::cookies::CookieScope;
use crate::php_url;
use crate::state::AppState;

/// The backend of the `authBackend` setting that this module knows.
pub const PLEX_BACKEND: &str = "plex";

/// The time that all applications of single sign-on have together, as in PHP.
const HAND_OFF_TIME: Duration = Duration::from_mins(1);

/// Whether an account may log in to this install: it is the account of the `plexAdmin`
/// setting, or the owner of the Plex server shared the server with it.
async fn is_approved(state: &AppState, settings: &Settings, account: &PlexAccount) -> bool {
    let admin = settings.plex_admin.to_lowercase();
    if !admin.is_empty()
        && (admin == account.username.to_lowercase() || admin == account.email.to_lowercase())
    {
        return true;
    }
    let Some(plex) = state.plex() else {
        return false;
    };
    // With the strict check, the server of this install must be shared with the account.
    let server = settings
        .plex_strict_friends
        .then_some(settings.plex_id.as_str());
    match plex
        .is_friend(&settings.plex_token, &account.username, server)
        .await
    {
        Ok(true) => true,
        Ok(false) => {
            tracing::warn!(user = %account.username, "a Plex account that is no friend of the server tried to log in");
            false
        }
        Err(error) => {
            tracing::warn!(%error, "the friend list of Plex could not be read");
            false
        }
    }
}

/// Asks Plex whether a name and a password are those of an account that may log in.
pub async fn by_plex_password(
    state: &AppState,
    settings: &Settings,
    name: &str,
    password: &str,
) -> Option<PlexAccount> {
    let plex = state.plex()?;
    let account = match plex.sign_in(&settings.install_id, name, password).await {
        Ok(account) => account?,
        Err(error) => {
            tracing::warn!(%error, "Plex could not check a login");
            return None;
        }
    };
    is_approved(state, settings, &account)
        .await
        .then_some(account)
}

/// Asks Plex whose token this is, and whether that account may log in.
pub async fn by_plex_token(
    state: &AppState,
    settings: &Settings,
    token: &str,
) -> Option<PlexAccount> {
    let plex = state.plex()?;
    let account = match plex.account(token).await {
        Ok(account) => account?,
        Err(error) => {
            tracing::warn!(%error, "Plex could not check a token");
            return None;
        }
    };
    is_approved(state, settings, &account)
        .await
        .then_some(account)
}

/// A login that the applications of single sign-on get a session for.
#[derive(Clone, Copy)]
pub struct HandOff<'a> {
    /// The account.
    pub user: &'a User,
    /// The level of the group of the account.
    pub level: GroupLevel,
    /// The password of the login. Empty for a login with a Plex token.
    pub password: &'a str,
    /// The Plex token of the login, when Plex checked it.
    pub plex_token: Option<&'a str>,
    /// Until when the cookies last.
    pub expires: SystemTime,
}

/// The path of an address, for the cookie of an application that is not at the root of
/// its host.
fn path_of(url: &str) -> String {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = after_scheme
        .find('/')
        .and_then(|start| after_scheme.get(start..))
        .unwrap_or("/");
    let path = path.split(['?', '#']).next().unwrap_or("/");
    if path.is_empty() {
        "/".to_owned()
    } else {
        path.to_owned()
    }
}

/// Asks each application of single sign-on that is switched on for a session of the
/// user, and returns the cookies that hand the sessions to the browser.
///
/// An application that gives no session is left out. The login itself does not fail.
pub async fn hand_off(
    state: &AppState,
    headers: &HeaderMap,
    client: &Client,
    login: HandOff<'_>,
) -> Vec<HeaderValue> {
    let settings = state.settings();
    let Some(http) = state.outbound() else {
        return Vec::new();
    };
    let timeout = time_for_each(&settings);

    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let qualified = |address: &str| php_url::qualify(address.trim(), client.https, host);
    let client_ip = client.address.to_string();
    let who = Login {
        username: &login.user.username,
        email: login.user.email.as_deref().unwrap_or_default(),
        password: login.password,
        plex_token: login.plex_token,
        client_ip: &client_ip,
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok()),
    };
    let fallback = |user: &'_ str, password: &'_ str| {
        (!user.is_empty() && !password.is_empty()).then_some((user.to_owned(), password.to_owned()))
    };

    let ombi = async {
        if !settings.sso_ombi {
            return None;
        }
        let account = fallback(
            &settings.ombi_fallback_user,
            &settings.ombi_fallback_password,
        );
        let account = account
            .as_ref()
            .map(|(user, password)| Fallback { user, password });
        sso::ombi_token(http, &qualified(&settings.ombi_url), who, account, timeout).await
    };
    let overseerr = async {
        if !settings.sso_overseerr {
            return None;
        }
        let account = fallback(
            &settings.overseerr_fallback_user,
            &settings.overseerr_fallback_password,
        );
        let account = account
            .as_ref()
            .map(|(user, password)| Fallback { user, password });
        sso::overseerr_session(
            http,
            &qualified(&settings.overseerr_url),
            who,
            account,
            timeout,
        )
        .await
    };
    let tautulli = tautulli_sessions(http, &settings, &qualified, who, login.level, timeout);
    let jellyfin = jellyfin_session(http, &settings, &qualified, who, timeout);
    let (ombi, overseerr, tautulli, jellyfin) = tokio::join!(ombi, overseerr, tautulli, jellyfin);

    let scope = CookieScope::of(headers, client.https);
    let mut cookies = Vec::new();
    let mut add = |name: &str, value: &str, hidden: bool, path: &str| match scope
        .set_for_application(name, value, login.expires, hidden, path)
    {
        Ok(values) => cookies.extend(values),
        Err(error) => tracing::warn!(%error, name, "a single sign-on cookie could not be written"),
    };
    if let Some(token) = login.plex_token.filter(|_| settings.sso_plex) {
        add("mpt", token, false, "/");
    }
    if let Some(token) = ombi {
        add("Auth", &token, false, "/");
    }
    for session in tautulli {
        let name = format!("tautulli_token_{}", session.uuid);
        add(&name, &session.token, true, &session.path);
    }
    if let Some(session) = overseerr {
        add("connect.sid", &session, false, "/");
    }
    // The web client of Jellyfin keeps its login in the storage of the browser. The
    // interface copies these cookies there, so a script must read them (D-114).
    if let Some(session) = jellyfin {
        add(&session.user_key, &session.user, false, "/");
        add("jellyfin_credentials", &session.credentials, false, "/");
    }
    cookies
}

/// Asks each Tautulli install for a session, for users of the level of the
/// `ssoTautulliAuth` setting, or with more access.
async fn tautulli_sessions(
    http: &Outbound,
    settings: &Settings,
    qualified: &impl Fn(&str) -> String,
    who: Login<'_>,
    level: GroupLevel,
    timeout: Duration,
) -> Vec<sso::TautulliSession> {
    if !settings.sso_tautulli || u32::from(level.get()) > settings.sso_tautulli_auth {
        return Vec::new();
    }
    let installs: Vec<(String, String)> = settings
        .tautulli_url
        .split(',')
        .filter(|address| !address.trim().is_empty())
        .map(|address| {
            let url = qualified(address);
            let path = path_of(&url);
            (url, path)
        })
        .collect();
    sso::tautulli_sessions(http, &installs, who, timeout).await
}

/// Asks Jellyfin for the login of its web client, when its single sign-on is on (D-114).
/// Jellyfin takes the name and the password; a login with a Plex token has none.
async fn jellyfin_session(
    http: &Outbound,
    settings: &Settings,
    qualified: &impl Fn(&str) -> String,
    who: Login<'_>,
    timeout: Duration,
) -> Option<sso::JellyfinSession> {
    if !settings.sso_jellyfin || who.password.is_empty() || settings.jellyfin_url.is_empty() {
        return None;
    }
    sso::jellyfin_session(
        http,
        &qualified(&settings.jellyfin_url),
        &qualified(&settings.jellyfin_ssourl),
        who,
        timeout,
    )
    .await
}

/// The time that each application of single sign-on has to answer: one minute, shared
/// by the applications that are switched on.
fn time_for_each(settings: &Settings) -> Duration {
    let enabled = [
        settings.sso_plex,
        settings.sso_ombi,
        settings.sso_tautulli,
        settings.sso_jellyfin,
        settings.sso_overseerr,
        settings.sso_petio,
        settings.sso_komga,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count();
    HAND_OFF_TIME / u32::try_from(enabled.max(1)).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::path_of;

    #[test]
    fn cookie_path_is_the_path_of_the_address() {
        for (url, path) in [
            ("http://tautulli.example.com", "/"),
            ("http://tautulli.example.com/", "/"),
            ("https://example.com:8181/tautulli", "/tautulli"),
            (
                "https://example.com/apps/tautulli/?x=1#top",
                "/apps/tautulli/",
            ),
            ("http://10.0.0.5:8181?x=1", "/"),
        ] {
            assert_eq!(path_of(url), path, "{url}");
        }
    }
}
