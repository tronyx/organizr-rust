//! An LDAP server that checks the password of a login: the `ldap` back end of the
//! `authBackend` setting, and the two LDAP tests of the settings page.
//!
//! The steps are those of the library that the PHP application uses (Adldap2):
//!
//! 1. Connect to the first host that answers, and bind with the account of the settings
//!    (`ldapBindUsername`), or bind without a name when the settings name none.
//! 2. Bind with the name of the person, with the prefix and the suffix of the settings,
//!    and the password of the login. The password is right when this bind succeeds.
//! 3. Bind with the account of the settings again, and read the attribute `mail` of the
//!    entry whose distinguished name is the name of step 2.
//!
//! No error holds a password. The texts of the errors are those that PHP shows, which
//! are the texts of `libldap`, the LDAP library of C.

use std::sync::Arc;
use std::time::Duration;

use ldap3::{Ldap, LdapConnAsync, LdapConnSettings, LdapError as Failure, Scope, SearchEntry};
use rustls::ClientConfig;

/// The port of LDAP without TLS.
pub const PORT: u16 = 389;

/// The port of LDAP over TLS.
pub const TLS_PORT: u16 = 636;

/// The result code of a bind with a wrong name or a wrong password.
const INVALID_CREDENTIALS: u32 = 49;

/// The LDAP server, as the settings describe it.
#[derive(Clone)]
pub struct Directory<'a> {
    /// The hosts, tried in this order until one answers.
    pub hosts: &'a [String],
    /// The port of every host: the port of the first host of the settings, as in PHP.
    pub port: u16,
    /// Whether the connection uses TLS from the start (`ldaps`).
    pub ssl: bool,
    /// Whether the connection turns to TLS after the connection is made (`StartTLS`).
    pub start_tls: bool,
    /// The name and the password of the account that the application binds with. No
    /// name is a bind without a name.
    pub bind: (&'a str, &'a str),
    /// How long the connection and each step may take.
    pub timeout: Duration,
    /// The TLS settings, which check the certificate of the server.
    pub tls: Arc<ClientConfig>,
}

/// The bind password is a secret, so `Debug` does not show it.
impl std::fmt::Debug for Directory<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Directory")
            .field("hosts", &self.hosts)
            .field("port", &self.port)
            .field("ssl", &self.ssl)
            .field("start_tls", &self.start_tls)
            .field("bind_name", &self.bind.0)
            .finish_non_exhaustive()
    }
}

/// Why the server could not be asked. The text is the one that PHP shows.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LdapError {
    /// No host could be reached, or the TLS connection failed.
    #[error("Can't contact LDAP server")]
    Unreachable,
    /// A step took longer than the time limit.
    #[error("Timed out")]
    TimedOut,
    /// The settings name a bind account without a password. Such a bind is a bind
    /// without a name on many servers, so it is not sent.
    #[error("The bind password is empty")]
    NoBindPassword,
    /// The server refused the bind of the account of the settings.
    #[error("{}", result_text(*.0))]
    Refused(u32),
}

/// What the server said about a login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Login {
    /// The password is right. The e-mail address is the first value of `mail`, when
    /// the entry can be read and has one.
    Accepted {
        /// The e-mail address of the entry.
        email: Option<String>,
    },
    /// The name or the password is wrong.
    Refused,
}

/// The text of an LDAP result code, as `libldap` writes it.
pub fn result_text(code: u32) -> String {
    let text = match code {
        0 => "Success",
        1 => "Operations error",
        2 => "Protocol error",
        3 => "Time limit exceeded",
        7 => "Auth method not supported",
        8 => "Strong(er) authentication required",
        13 => "Confidentiality required",
        32 => "No such object",
        34 => "Invalid DN syntax",
        48 => "Inappropriate authentication",
        49 => "Invalid credentials",
        50 => "Insufficient access",
        51 => "Server is busy",
        52 => "Server is unavailable",
        53 => "Server is unwilling to perform",
        80 => "Other (e.g., implementation specific) error",
        other => return format!("Unknown error ({other})"),
    };
    text.to_owned()
}

/// Whether a user name can go into the name of a bind as it is. A name with a character
/// that has a meaning in a distinguished name (RFC 4514) or in a search filter
/// (RFC 4515) is refused, so a user name cannot change the entry that is asked for.
pub fn is_safe_name(name: &str) -> bool {
    const SPECIAL: &[char] = &[',', '+', '"', '\\', '<', '>', ';', '=', '*', '(', ')'];
    !name.is_empty()
        && name.len() <= 256
        && !name.starts_with(['#', ' '])
        && !name.ends_with(' ')
        && !name
            .chars()
            .any(|character| character.is_control() || SPECIAL.contains(&character))
}

/// The address of a host, for the LDAP library.
fn address(host: &str, port: u16, ssl: bool) -> String {
    let scheme = if ssl { "ldaps" } else { "ldap" };
    if host.contains(':') && !host.starts_with('[') {
        format!("{scheme}://[{host}]:{port}")
    } else {
        format!("{scheme}://{host}:{port}")
    }
}

fn failed(error: &Failure) -> LdapError {
    match error {
        Failure::Timeout { .. } => LdapError::TimedOut,
        _ => LdapError::Unreachable,
    }
}

/// An open connection, bound with the account of the settings.
struct Session {
    ldap: Ldap,
    timeout: Duration,
}

impl Session {
    /// A simple bind. `Ok(code)` holds the result code of the server.
    async fn bind(&mut self, name: &str, password: &str) -> Result<u32, LdapError> {
        let result = self
            .ldap
            .with_timeout(self.timeout)
            .simple_bind(name, password)
            .await
            .map_err(|error| failed(&error))?;
        Ok(result.rc)
    }

    /// Binds with the account of the settings, or without a name.
    async fn bind_as_application(
        &mut self,
        (name, password): (&str, &str),
    ) -> Result<(), LdapError> {
        let code = if name.is_empty() {
            self.bind("", "").await?
        } else if password.is_empty() {
            return Err(LdapError::NoBindPassword);
        } else {
            self.bind(name, password).await?
        };
        if code == 0 {
            Ok(())
        } else {
            Err(LdapError::Refused(code))
        }
    }

    /// The first value of `mail` of the entry with the distinguished name, or nothing.
    async fn mail_of(&mut self, name: &str) -> Option<String> {
        let found = self
            .ldap
            .with_timeout(self.timeout)
            .search(name, Scope::Base, "(objectClass=*)", vec!["mail"])
            .await
            .ok()?
            .success()
            .ok()?;
        let entry = found.0.into_iter().next()?;
        SearchEntry::construct(entry)
            .attrs
            .into_iter()
            .find(|(attribute, _)| attribute.eq_ignore_ascii_case("mail"))
            .and_then(|(_, values)| values.into_iter().next())
            .filter(|mail| !mail.is_empty())
    }

    async fn close(mut self) {
        // The server may close first; that is no error of the login.
        let _ = self.ldap.unbind().await;
    }
}

/// Connects to the first host that answers.
async fn connect(directory: &Directory<'_>) -> Result<Session, LdapError> {
    let mut last = LdapError::Unreachable;
    for host in directory.hosts.iter().filter(|host| !host.is_empty()) {
        let settings = LdapConnSettings::new()
            .set_conn_timeout(directory.timeout)
            .set_starttls(directory.start_tls && !directory.ssl)
            .set_config(Arc::clone(&directory.tls));
        let url = address(host, directory.port, directory.ssl);
        let opened = tokio::time::timeout(
            directory.timeout,
            LdapConnAsync::with_settings(settings, &url),
        )
        .await;
        match opened {
            Ok(Ok((connection, ldap))) => {
                ldap3::drive!(connection);
                return Ok(Session {
                    ldap,
                    timeout: directory.timeout,
                });
            }
            Ok(Err(error)) => {
                tracing::warn!(host = %host, error = %error, "an LDAP host could not be reached");
                last = failed(&error);
            }
            Err(_) => {
                tracing::warn!(host = %host, "an LDAP host did not answer in time");
                last = LdapError::TimedOut;
            }
        }
    }
    Err(last)
}

/// Connects, and binds with the account of the settings: the test "LDAP connection".
pub async fn test_connection(directory: &Directory<'_>) -> Result<(), LdapError> {
    let mut session = connect(directory).await?;
    let bound = session.bind_as_application(directory.bind).await;
    session.close().await;
    bound
}

/// Connects, binds with the account of the settings, and then with the name and the
/// password of a person: the test "Test Login". `Ok(true)` when the second bind
/// succeeds.
pub async fn test_login(
    directory: &Directory<'_>,
    name: &str,
    password: &str,
) -> Result<bool, LdapError> {
    let mut session = connect(directory).await?;
    if let Err(error) = session.bind_as_application(directory.bind).await {
        session.close().await;
        return Err(error);
    }
    let accepted = !password.is_empty() && session.bind(name, password).await? == 0;
    session.close().await;
    Ok(accepted)
}

/// Checks the password of a login. `name` is the distinguished name, or the name that
/// the server takes, of the person: the user name with the prefix and the suffix of the
/// settings. An empty password is refused without asking, because a bind with a name
/// and no password succeeds on many servers.
pub async fn log_in(
    directory: &Directory<'_>,
    name: &str,
    password: &str,
) -> Result<Login, LdapError> {
    if name.is_empty() || password.is_empty() {
        return Ok(Login::Refused);
    }
    let mut session = connect(directory).await?;
    if let Err(error) = session.bind_as_application(directory.bind).await {
        session.close().await;
        return Err(error);
    }
    let code = match session.bind(name, password).await {
        Ok(code) => code,
        Err(error) => {
            session.close().await;
            return Err(error);
        }
    };
    if code != 0 {
        if code != INVALID_CREDENTIALS {
            tracing::warn!(code, text = %result_text(code), "the LDAP server refused a login");
        }
        session.close().await;
        return Ok(Login::Refused);
    }
    // As in PHP, the address is read with the account of the settings, and the login
    // fails when that account can no longer bind.
    if let Err(error) = session.bind_as_application(directory.bind).await {
        session.close().await;
        return Err(error);
    }
    let email = session.mail_of(name).await;
    session.close().await;
    Ok(Login::Accepted { email })
}

#[cfg(test)]
mod tests {
    use super::{address, is_safe_name, result_text};

    #[test]
    fn names_with_characters_of_a_distinguished_name_or_a_filter_are_refused() {
        for safe in [
            "robin",
            "robin.hood",
            "r_hood-1",
            "robin@example.com",
            "Zoë",
        ] {
            assert!(is_safe_name(safe), "{safe}");
        }
        for unsafe_name in [
            "",
            "robin,ou=admins",
            "robin+cn=x",
            "a\"b",
            "a\\b",
            "<a>",
            "a;b",
            "cn=admin",
            "*",
            "a)(uid=*",
            "#robin",
            " robin",
            "robin ",
            "robin\0",
            "robin\n",
        ] {
            assert!(!is_safe_name(unsafe_name), "{unsafe_name:?}");
        }
        assert!(!is_safe_name(&"a".repeat(257)));
    }

    #[test]
    fn address_names_the_scheme_host_and_port() {
        assert_eq!(
            address("ldap.example.com", 389, false),
            "ldap://ldap.example.com:389"
        );
        assert_eq!(address("10.0.0.5", 636, true), "ldaps://10.0.0.5:636");
        assert_eq!(address("[fd00::5]", 389, false), "ldap://[fd00::5]:389");
        assert_eq!(address("fd00::5", 389, false), "ldap://[fd00::5]:389");
    }

    #[test]
    fn result_codes_have_the_texts_of_openldap() {
        assert_eq!(result_text(49), "Invalid credentials");
        assert_eq!(result_text(32), "No such object");
        assert_eq!(result_text(4711), "Unknown error (4711)");
    }
}
