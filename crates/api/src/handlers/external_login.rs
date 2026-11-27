//! What a login does with other services: Plex checks who a person is, and the
//! applications of single sign-on get a session for the person.

use std::time::{Duration, SystemTime};

use axum::http::{HeaderMap, HeaderValue, header};
use organizr_domain::{GroupLevel, Tab, User};
use organizr_integrations::http::Outbound;
use organizr_integrations::plex::PlexAccount;
use organizr_integrations::sso::{self, Fallback, Login};
use organizr_settings::Settings;

use crate::client::Client;
use crate::cookies::CookieScope;
use crate::handlers::homepage::{service_url, tab_address};
use crate::state::AppState;

/// The backend of the `authBackend` setting that this module knows.
pub const PLEX_BACKEND: &str = "plex";

/// The time that all applications of single sign-on have together, as in PHP.
const HAND_OFF_TIME: Duration = Duration::from_mins(1);

/// Whether the Plex account is the one of the `plexAdmin` setting: the owner of the
/// server, whom the Admin named.
pub fn is_plex_owner(settings: &Settings, account: &PlexAccount) -> bool {
    let admin = settings.plex_admin.to_lowercase();
    !admin.is_empty()
        && (admin == account.username.to_lowercase() || admin == account.email.to_lowercase())
}

/// Whether an account may log in to this install: it is the account of the `plexAdmin`
/// setting, or the owner of the Plex server shared the server with it.
async fn is_approved(state: &AppState, settings: &Settings, account: &PlexAccount) -> bool {
    if is_plex_owner(settings, account) {
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
    /// The e-mail address of the Plex account, when Plex checked the login (D-150).
    pub plex_email: Option<&'a str>,
    /// Until when the cookies last.
    pub expires: SystemTime,
}

/// Who may use the shared fallback account of an application: a user who sees its
/// homepage item or one of its tabs (D-195). PHP hands the account to every user.
struct FallbackAccess {
    level: GroupLevel,
    tabs: Vec<Tab>,
}

impl FallbackAccess {
    /// What decides for a user of `level`. The tabs are read only when the single sign-on
    /// of one of the three applications is on.
    async fn of(state: &AppState, settings: &Settings, level: GroupLevel) -> Self {
        let tabs = if settings.sso_ombi || settings.sso_overseerr || settings.sso_petio {
            state.database().tabs().list().await.unwrap_or_else(|error| {
                tracing::warn!(%error, "the tabs could not be read; no fallback account is used");
                Vec::new()
            })
        } else {
            Vec::new()
        };
        Self { level, tabs }
    }

    /// The fallback account of Ombi that the user may use.
    fn ombi<'a>(&self, settings: &'a Settings) -> Option<Fallback<'a>> {
        let item_level = Some(settings.homepage_ombi_auth);
        self.allows("ombi", &settings.ombi_url, item_level)
            .then(|| {
                fallback(
                    &settings.ombi_fallback_user,
                    &settings.ombi_fallback_password,
                )
            })
            .flatten()
    }

    /// The fallback account of Overseerr that the user may use.
    fn overseerr<'a>(&self, settings: &'a Settings) -> Option<Fallback<'a>> {
        let item_level = Some(settings.homepage_overseerr_auth);
        self.allows("overseerr", &settings.overseerr_url, item_level)
            .then(|| {
                fallback(
                    &settings.overseerr_fallback_user,
                    &settings.overseerr_fallback_password,
                )
            })
            .flatten()
    }

    /// The fallback account of Petio that the user may use. Petio has no homepage item.
    fn petio<'a>(&self, settings: &'a Settings) -> Option<Fallback<'a>> {
        self.allows("petio", &settings.petio_url, None)
            .then(|| {
                fallback(
                    &settings.petio_fallback_user,
                    &settings.petio_fallback_password,
                )
            })
            .flatten()
    }

    /// Whether the user may use the fallback account of the application `name`, at
    /// `address`. `item_level` is the level with the least access that sees the homepage
    /// item of the application, when it has one.
    fn allows(&self, name: &str, address: &str, item_level: Option<u32>) -> bool {
        item_level.is_some_and(|least_access| u32::from(self.level.get()) <= least_access)
            || self.tabs.iter().any(|tab| {
                crate::handlers::launch::is_shown(tab, self.level) && is_tab_of(tab, name, address)
            })
    }
}

/// The fallback account of the settings, when it has a name and a password.
fn fallback<'a>(user: &'a str, password: &'a str) -> Option<Fallback<'a>> {
    (!user.is_empty() && !password.is_empty()).then_some(Fallback { user, password })
}

/// Whether a tab is one of the application `name` at `address`: its name or one of its
/// addresses holds the name, or one of its addresses is on the host of `address`.
fn is_tab_of(tab: &Tab, name: &str, address: &str) -> bool {
    let name = name.to_lowercase();
    let host = host_of(address);
    let addresses = [tab.url.as_deref(), tab.url_local.as_deref()];
    tab.name.to_lowercase().contains(&name)
        || addresses.into_iter().flatten().any(|url| {
            url.to_lowercase().contains(&name)
                || host
                    .as_ref()
                    .is_some_and(|host| host_of(url).as_ref() == Some(host))
        })
}

/// The host and the port of an address with a scheme, in lower case.
fn host_of(address: &str) -> Option<String> {
    let (_, rest) = address.trim().split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    (!host.is_empty()).then(|| host.to_lowercase())
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

    // Organizr sends the password of the login to these addresses, so an address that is
    // only a path names no application, whatever host the request names (D-137).
    let qualified = |address: &str| service_url(address);
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
    let access = FallbackAccess::of(state, &settings, login.level).await;

    let ombi = async {
        if !settings.sso_ombi {
            return None;
        }
        let account = access.ombi(&settings);
        sso::ombi_token(http, &qualified(&settings.ombi_url), who, account, timeout).await
    };
    let overseerr = async {
        if !settings.sso_overseerr {
            return None;
        }
        let account = access.overseerr(&settings);
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
    // The address of Jellyfin for the browser goes into a cookie of this answer.
    let for_browser = tab_address(&settings.jellyfin_ssourl, headers, client);
    let jellyfin = jellyfin_session(http, &settings, &qualified, &for_browser, who, timeout);
    let petio_fallback = access.petio(&settings);
    let petio = petio_token(http, &settings, &qualified, (who, petio_fallback), timeout);
    let komga_who = (who, login.plex_email);
    let komga = komga_token(http, &settings, &qualified, komga_who, login.level, timeout);
    let (ombi, overseerr, tautulli, jellyfin, petio, komga) =
        tokio::join!(ombi, overseerr, tautulli, jellyfin, petio, komga);

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
    if let Some(token) = petio {
        add("petio_jwt", &token, false, "/");
    }
    if let Some(token) = komga {
        add("komga_token", &token, false, "/");
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

/// Asks Petio for a token, when its single sign-on is on, with the fallback account of
/// the settings when Petio does not know the user and the user may use it.
async fn petio_token(
    http: &Outbound,
    settings: &Settings,
    qualified: &impl Fn(&str) -> String,
    (who, fallback): (Login<'_>, Option<Fallback<'_>>),
    timeout: Duration,
) -> Option<String> {
    if !settings.sso_petio || settings.petio_url.is_empty() {
        return None;
    }
    sso::petio_token(
        http,
        &qualified(&settings.petio_url),
        who,
        fallback,
        timeout,
    )
    .await
}

/// Asks Komga for a session, when its single sign-on is on and the user has the level of
/// `ssoKomgaAuth`. Komga knows its users by their e-mail address.
///
/// A login with a password asks for the account of the e-mail of the user, with that
/// password. A login through Plex, which brings no password, uses the master password of
/// the settings with the e-mail of the Plex account, which plex.tv vouches for: the
/// e-mail of the Organizr account is the user's to change (D-150).
async fn komga_token(
    http: &Outbound,
    settings: &Settings,
    qualified: &impl Fn(&str) -> String,
    (who, plex_email): (Login<'_>, Option<&str>),
    level: GroupLevel,
    timeout: Duration,
) -> Option<String> {
    if !settings.sso_komga
        || settings.komga_url.is_empty()
        || u32::from(level.get()) > settings.sso_komga_auth
    {
        return None;
    }
    let (user, password) = if who.password.is_empty() {
        (plex_email?, settings.komga_sso_master_password.as_str())
    } else {
        (who.email, who.password)
    };
    let account = Fallback { user, password };
    let fallback = (!settings.komga_fallback_user.is_empty()
        && !settings.komga_fallback_password.is_empty())
    .then(|| Fallback {
        user: &settings.komga_fallback_user,
        password: &settings.komga_fallback_password,
    });
    sso::komga_token(
        http,
        &qualified(&settings.komga_url),
        account,
        fallback,
        timeout,
    )
    .await
}

/// Asks Jellyfin for the login of its web client, when its single sign-on is on (D-114).
/// Jellyfin takes the name and the password; a login with a Plex token has none.
async fn jellyfin_session(
    http: &Outbound,
    settings: &Settings,
    qualified: &impl Fn(&str) -> String,
    for_browser: &str,
    who: Login<'_>,
    timeout: Duration,
) -> Option<sso::JellyfinSession> {
    if !settings.sso_jellyfin || who.password.is_empty() || settings.jellyfin_url.is_empty() {
        return None;
    }
    sso::jellyfin_session(
        http,
        &qualified(&settings.jellyfin_url),
        for_browser,
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
    use organizr_domain::{CategoryId, GroupLevel, LevelRange, Tab, TabId, TabKind};

    use super::{FallbackAccess, host_of, is_tab_of, path_of};

    fn tab(name: &str, url: Option<&str>, url_local: Option<&str>) -> Tab {
        Tab {
            id: TabId::new(1),
            position: 1,
            category: CategoryId::new(1),
            name: name.to_owned(),
            url: url.map(str::to_owned),
            url_local: url_local.map(str::to_owned),
            is_default: false,
            enabled: true,
            access: LevelRange::up_to(GroupLevel::new(4).expect("the level is valid")),
            add_to_admin: false,
            image: None,
            kind: TabKind::Iframe,
            splash: false,
            ping: false,
            ping_url: None,
            timeout: false,
            timeout_ms: None,
            preload: false,
        }
    }

    #[test]
    fn host_of_an_address_is_its_host_and_port() {
        assert_eq!(host_of("http://Ombi:5000/x").as_deref(), Some("ombi:5000"));
        assert_eq!(
            host_of("https://u:p@example.com?a").as_deref(),
            Some("example.com")
        );
        assert_eq!(host_of("/ombi"), None);
        assert_eq!(host_of("Ombi"), None);
    }

    #[test]
    fn tab_of_an_application_has_its_name_or_its_host() {
        let ombi = "http://10.0.0.5:5000";
        assert!(is_tab_of(&tab("Ombi", None, None), "ombi", ombi));
        assert!(is_tab_of(
            &tab("Requests", Some("https://example.com/ombi/"), None),
            "ombi",
            ombi
        ));
        assert!(is_tab_of(
            &tab("Requests", None, Some("http://10.0.0.5:5000/")),
            "ombi",
            ombi
        ));
        assert!(!is_tab_of(
            &tab("Requests", Some("http://10.0.0.5:8080"), None),
            "ombi",
            ombi
        ));
        assert!(!is_tab_of(
            &tab("Plex", Some("https://plex.example.com"), None),
            "ombi",
            ombi
        ));
    }

    #[test]
    fn fallback_account_needs_the_homepage_item_or_a_tab_that_the_user_sees() {
        let level = |value| GroupLevel::new(value).expect("the level is valid");
        let user = FallbackAccess {
            level: level(4),
            tabs: Vec::new(),
        };
        assert!(!user.allows("petio", "http://petio:7777", None));
        assert!(!user.allows("ombi", "http://ombi:5000", Some(1)));
        assert!(user.allows("ombi", "http://ombi:5000", Some(4)));
        let with_tab = FallbackAccess {
            tabs: vec![tab("Petio", None, None)],
            ..user
        };
        assert!(with_tab.allows("petio", "http://petio:7777", None));
        let guest = FallbackAccess {
            level: level(999),
            ..with_tab
        };
        assert!(!guest.allows("petio", "http://petio:7777", None));
    }

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
