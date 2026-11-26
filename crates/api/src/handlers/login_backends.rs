//! The back ends of the `authBackend` setting that check a password with a server of
//! the Admin: `ldap`, `ftp`, `emby_local`, `emby_connect`, `emby_all` and `jellyfin`.
//! Plex is in [`super::external_login`].
//!
//! Also the two LDAP tests of the settings page: `POST /test/ldap` and
//! `POST /test/ldap/login`.
//!
//! A back end that cannot be reached, or that refuses, counts as a wrong password. What
//! follows is the rule of the `authType` setting, as in PHP: with `both`, the password
//! stored in Organizr is checked next, so the local Admin can still log in; with
//! `external`, the login fails. No log line holds a password or a key.

use std::time::Duration;

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use organizr_domain::GroupLevel;
use organizr_integrations::emby::EmbyError;
use organizr_integrations::ftp::{self, FtpServer};
use organizr_integrations::ldap::{self, Directory, Login};
use organizr_integrations::media_login;
use organizr_integrations::tls::{self, TlsError};
use organizr_settings::Settings;
use serde_json::Value;

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::service_url;
use crate::php_url;
use crate::state::AppState;

/// The back end that checks with an LDAP server.
pub const LDAP: &str = "ldap";
/// The back end that checks with an FTP server.
pub const FTP: &str = "ftp";
/// The back end that checks with the Emby server of the settings.
pub const EMBY_LOCAL: &str = "emby_local";
/// The back end that checks with Emby Connect.
pub const EMBY_CONNECT: &str = "emby_connect";
/// The back end that checks with the Emby server, and then with Emby Connect.
pub const EMBY_ALL: &str = "emby_all";
/// The back end that checks with the Jellyfin server of the settings.
pub const JELLYFIN: &str = "jellyfin";

/// How long an LDAP server has for the connection and for each step, as in PHP.
const LDAP_TIME: Duration = Duration::from_secs(5);

/// How long an FTP server has for the login, as in PHP.
const FTP_TIME: Duration = Duration::from_secs(20);

/// Whether the back end is one of this module.
pub fn is_password_backend(backend: &str) -> bool {
    [LDAP, FTP, EMBY_LOCAL, EMBY_CONNECT, EMBY_ALL, JELLYFIN].contains(&backend)
}

/// The name that goes to the back end and that the account gets. PHP writes it in
/// lower case, but for the back ends of Emby.
pub fn login_name(backend: &str, name: &str) -> String {
    if backend.contains("emby") {
        name.to_owned()
    } else {
        name.to_lowercase()
    }
}

/// An account that a back end vouched for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vouched {
    /// The name that the person typed, as [`login_name`] gives it. The Organizr account
    /// is the one with this user name or this e-mail address, as in PHP.
    pub typed: String,
    /// The user name of a new account.
    pub username: String,
    /// The e-mail address of a new account, or nothing.
    pub email: String,
    /// Whether anyone can make an account at the service, as at Emby Connect. Such a
    /// login never opens an Admin or Co-Admin account.
    pub open: bool,
}

/// The hosts of the `authBackendHost` setting for LDAP, and the port of all of them.
///
/// As in PHP, the setting holds addresses with commas between them. Each is taken apart
/// by `parse_url()`: the host is the host, or else the path. The port is the port of
/// the first address; without one, 389 for the scheme `ldap` (and for no scheme), and
/// 636 for any other scheme. Whether TLS is used is the `ldapSSL` setting, not the
/// scheme. An empty host is left out.
pub fn ldap_hosts(setting: &str) -> (Vec<String>, u16) {
    let mut hosts = Vec::new();
    let mut port = None;
    for address in setting.split(',') {
        let parts = php_url::server_parts(address.trim()).unwrap_or_default();
        let scheme = parts.scheme.as_deref().unwrap_or("ldap").to_lowercase();
        let host = parts.host.or(parts.path).unwrap_or_default();
        let own_port = parts.port.unwrap_or(if scheme == "ldap" {
            ldap::PORT
        } else {
            ldap::TLS_PORT
        });
        port.get_or_insert(own_port);
        if !host.is_empty() {
            hosts.push(host);
        }
    }
    (hosts, port.unwrap_or(ldap::PORT))
}

/// The FTP server of the `authBackendHost` setting: the host, the port, and whether TLS
/// is used. As in PHP, the whole setting is one address. Without a scheme it is `ftps`;
/// without a port, 21. `None` for another scheme than `ftp` and `ftps`, or no host.
pub fn ftp_server(setting: &str) -> Option<(String, u16, bool)> {
    let parts = php_url::server_parts(setting).unwrap_or_default();
    let scheme = parts.scheme.as_deref().unwrap_or("ftps").to_lowercase();
    let secure = match scheme.as_str() {
        "ftps" => true,
        "ftp" => false,
        _ => return None,
    };
    let host = parts.host.or(parts.path).filter(|host| !host.is_empty())?;
    Some((host, parts.port.unwrap_or(ftp::PORT), secure))
}

/// The bind name of a person: the user name with the prefix and the suffix of the
/// settings.
fn account_name(settings: &Settings, name: &str) -> String {
    format!(
        "{}{name}{}",
        settings.auth_backend_host_prefix, settings.auth_backend_host_suffix
    )
}

/// Asks the back end of the settings whether the name and the password fit. `None`
/// when they do not, or when the back end cannot be asked; the log says why.
pub async fn check(
    state: &AppState,
    settings: &Settings,
    name: &str,
    password: &str,
) -> Option<Vouched> {
    let backend = settings.auth_backend.as_str();
    let typed = login_name(backend, name);
    if typed.is_empty() || password.is_empty() {
        return None;
    }
    let plain = |email: String| Vouched {
        typed: typed.clone(),
        username: typed.clone(),
        email,
        open: false,
    };
    match backend {
        LDAP => by_ldap(settings, &typed, password).await.map(plain),
        FTP => by_ftp(settings, &typed, password)
            .await
            .then(|| plain(String::new())),
        JELLYFIN => {
            let url = service_url(&settings.jellyfin_url);
            let http = state.outbound()?;
            answer(
                "Jellyfin",
                media_login::jellyfin(http, &url, &typed, password).await,
            )
            .then(|| plain(String::new()))
        }
        EMBY_LOCAL => by_emby(state, settings, &typed, password)
            .await
            .then(|| plain(String::new())),
        EMBY_CONNECT => by_emby_connect(state, settings, &typed, password).await,
        EMBY_ALL => {
            if by_emby(state, settings, &typed, password).await {
                Some(plain(String::new()))
            } else {
                by_emby_connect(state, settings, &typed, password).await
            }
        }
        _ => None,
    }
}

/// Whether a media server took the login. An error is logged and counts as a refusal.
fn answer(service: &str, result: Result<bool, EmbyError>) -> bool {
    result.unwrap_or_else(|error| {
        tracing::warn!(%error, service, "a login could not be checked");
        false
    })
}

async fn by_emby(state: &AppState, settings: &Settings, name: &str, password: &str) -> bool {
    let Some(http) = state.outbound() else {
        return false;
    };
    let url = service_url(&settings.emby_url);
    answer(
        "Emby",
        media_login::emby_local(http, &url, name, password).await,
    )
}

async fn by_emby_connect(
    state: &AppState,
    settings: &Settings,
    name: &str,
    password: &str,
) -> Option<Vouched> {
    let http = state.outbound()?;
    let url = service_url(&settings.emby_url);
    let servers = (
        state.emby_connect(),
        url.as_str(),
        settings.emby_token.as_str(),
    );
    match media_login::emby_connect(http, servers, name, password).await {
        Ok(Some(account)) => Some(Vouched {
            typed: name.to_owned(),
            username: if account.username.is_empty() {
                name.to_owned()
            } else {
                account.username
            },
            email: account.email,
            open: true,
        }),
        Ok(None) => None,
        Err(error) => {
            tracing::warn!(%error, "Emby Connect could not check a login");
            None
        }
    }
}

/// The LDAP server of the settings. `None` when the base DN or the host is not set.
fn directory<'a>(
    settings: &'a Settings,
    hosts: &'a [String],
    port: u16,
) -> Result<Option<Directory<'a>>, TlsError> {
    if settings.auth_base_dn.is_empty() || settings.auth_backend_host.is_empty() {
        return Ok(None);
    }
    Ok(Some(Directory {
        hosts,
        port,
        ssl: settings.ldap_ssl,
        start_tls: settings.ldap_tls,
        bind: (
            settings.ldap_bind_username.as_str(),
            settings.ldap_bind_password.as_str(),
        ),
        timeout: LDAP_TIME,
        tls: tls::client_config()?,
    }))
}

/// Checks a login with the LDAP server. The e-mail address of the entry, or an empty
/// text, when the password is right.
async fn by_ldap(settings: &Settings, name: &str, password: &str) -> Option<String> {
    if !ldap::is_safe_name(name) {
        tracing::warn!(user = %name, "a user name with characters that LDAP does not allow was refused");
        return None;
    }
    let (hosts, port) = ldap_hosts(&settings.auth_backend_host);
    let directory = match directory(settings, &hosts, port) {
        Ok(Some(directory)) => directory,
        Ok(None) => return None,
        Err(error) => {
            tracing::error!(%error, "the TLS settings for LDAP could not be made");
            return None;
        }
    };
    match ldap::log_in(&directory, &account_name(settings, name), password).await {
        Ok(Login::Accepted { email }) => Some(email.unwrap_or_default()),
        Ok(Login::Refused) => None,
        Err(error) => {
            tracing::warn!(%error, "the LDAP server could not check a login");
            None
        }
    }
}

/// Checks a login with the FTP server.
async fn by_ftp(settings: &Settings, name: &str, password: &str) -> bool {
    let Some((host, port, secure)) = ftp_server(&settings.auth_backend_host) else {
        tracing::warn!("the FTP address of the settings is not an FTP address");
        return false;
    };
    let tls = match tls::client_config() {
        Ok(tls) => tls,
        Err(error) => {
            tracing::error!(%error, "the TLS settings for FTP could not be made");
            return false;
        }
    };
    let server = FtpServer {
        host: &host,
        port,
        secure,
        timeout: FTP_TIME,
        tls,
    };
    ftp::log_in(&server, name, password)
        .await
        .unwrap_or_else(|error| {
            tracing::warn!(%error, "the FTP server could not check a login");
            false
        })
}

/// What a test answers.
type Outcome = Result<Envelope<Value>, Refusal>;

const NOT_SET: &str = "authBaseDN and/or BackendHost not supplied";
const CONNECTED: &str = "LDAP connection successful";
const WRONG: &str = "Username/Password Failed to authenticate";

fn passed() -> Envelope<Value> {
    Envelope::success(Value::Null).with_message(CONNECTED)
}

/// `POST /test/ldap`: connects to the LDAP server of the settings, and binds with the
/// account of the settings. Only an Admin or a Co-Admin can run it.
///
/// `422 "authBaseDN and/or BackendHost not supplied"` without those settings, and `409`
/// with the text of the LDAP library when the bind fails.
pub async fn test_ldap(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Outcome {
    caller.require(GroupLevel::CO_ADMIN)?;
    let settings = state.settings();
    let (hosts, port) = ldap_hosts(&settings.auth_backend_host);
    let directory = directory(&settings, &hosts, port)
        .map_err(|_| {
            Refusal::Request(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not connect".to_owned(),
            )
        })?
        .ok_or_else(|| Refusal::unprocessable(NOT_SET))?;
    match ldap::test_connection(&directory).await {
        Ok(()) => Ok(passed()),
        Err(error) => {
            tracing::warn!(%error, "the LDAP test failed");
            Err(Refusal::conflict(error.to_string()))
        }
    }
}

/// `POST /test/ldap/login`: as `POST /test/ldap`, and then binds with the `username`
/// of the body, with the prefix and the suffix of the settings, and the `password` of
/// the body. Only an Admin or a Co-Admin can run it.
///
/// `422 "Username and/or Password not supplied"` without both, `401 "Username/Password
/// Failed to authenticate"` when the second bind fails, and `500` with the text of the
/// LDAP library when the first one fails.
pub async fn test_ldap_login(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Outcome {
    caller.require(GroupLevel::CO_ADMIN)?;
    let username = data.text("username").unwrap_or_default();
    let password = data.text("password").unwrap_or_default();
    if username.is_empty() || password.is_empty() {
        return Err(Refusal::unprocessable(
            "Username and/or Password not supplied",
        ));
    }
    let settings = state.settings();
    let (hosts, port) = ldap_hosts(&settings.auth_backend_host);
    let directory = directory(&settings, &hosts, port)
        .map_err(|_| {
            Refusal::Request(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not connect".to_owned(),
            )
        })?
        .ok_or_else(|| Refusal::unprocessable(NOT_SET))?;
    let wrong = || Refusal::Request(StatusCode::UNAUTHORIZED, WRONG.to_owned());
    if !ldap::is_safe_name(&username) {
        return Err(wrong());
    }
    match ldap::test_login(&directory, &account_name(&settings, &username), &password).await {
        Ok(true) => Ok(passed()),
        Ok(false) => Err(wrong()),
        Err(error) => {
            tracing::warn!(%error, "the LDAP login test failed");
            Err(Refusal::Request(
                StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ftp_server, ldap_hosts, login_name};

    #[test]
    fn ldap_hosts_are_read_as_php_reads_them() {
        let hosts = |list: &[&str]| {
            list.iter()
                .map(|host| (*host).to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(ldap_hosts("ldap.lan"), (hosts(&["ldap.lan"]), 389));
        assert_eq!(ldap_hosts("ldap://ldap.lan"), (hosts(&["ldap.lan"]), 389));
        assert_eq!(ldap_hosts("LDAPS://ldap.lan"), (hosts(&["ldap.lan"]), 636));
        assert_eq!(ldap_hosts("ldap.lan:3890"), (hosts(&["ldap.lan"]), 3890));
        assert_eq!(
            ldap_hosts(" ldap://a.lan:1389 , ldaps://b.lan:2636,c.lan"),
            (hosts(&["a.lan", "b.lan", "c.lan"]), 1389)
        );
        // The port is that of the first address; a later one is ignored.
        assert_eq!(
            ldap_hosts("a.lan,ldaps://b.lan:2636"),
            (hosts(&["a.lan", "b.lan"]), 389)
        );
        assert_eq!(
            ldap_hosts("ldap://[fd00::5]:389"),
            (hosts(&["[fd00::5]"]), 389)
        );
        assert_eq!(ldap_hosts("10.0.0.5"), (hosts(&["10.0.0.5"]), 389));
        // An empty address names no host, but it still gives the port.
        assert_eq!(ldap_hosts(""), (hosts(&[]), 389));
        assert_eq!(ldap_hosts(",ldap.lan:3890"), (hosts(&["ldap.lan"]), 389));
    }

    #[test]
    fn ftp_server_is_read_as_php_reads_it() {
        let server = |host: &str, port, secure| Some((host.to_owned(), port, secure));
        assert_eq!(ftp_server("ftp.lan"), server("ftp.lan", 21, true));
        assert_eq!(ftp_server("ftp://ftp.lan"), server("ftp.lan", 21, false));
        assert_eq!(
            ftp_server("FTPS://ftp.lan:990"),
            server("ftp.lan", 990, true)
        );
        assert_eq!(ftp_server("ftp.lan:2121"), server("ftp.lan", 2121, true));
        assert_eq!(
            ftp_server("ftp://10.0.0.5:2121"),
            server("10.0.0.5", 2121, false)
        );
        for none in ["", "http://ftp.lan", "ldap://ftp.lan", "sftp://ftp.lan"] {
            assert_eq!(ftp_server(none), None, "{none}");
        }
    }

    #[test]
    fn names_are_in_lower_case_but_for_emby() {
        assert_eq!(login_name("ldap", "Robin"), "robin");
        assert_eq!(login_name("jellyfin", "Robin"), "robin");
        assert_eq!(login_name("emby_local", "Robin"), "Robin");
        assert_eq!(login_name("emby_all", "Robin"), "Robin");
    }
}
