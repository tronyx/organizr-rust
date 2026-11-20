//! An FTP server that checks the password of a login: the `ftp` back end of the
//! `authBackend` setting.
//!
//! As in the PHP application, the login connects, turns to TLS with `AUTH TLS` when the
//! scheme is `ftps`, sends `USER` and `PASS`, and ends with `QUIT`. The password is
//! right when the server answers `230`. The certificate of the server is checked; PHP
//! checks none.

use std::sync::Arc;
use std::time::Duration;

use rustls::ClientConfig;
use suppaftp::FtpError;
use suppaftp::tokio::{AsyncRustlsConnector, AsyncRustlsFtpStream};

/// The port of FTP.
pub const PORT: u16 = 21;

/// The FTP server, as the settings describe it.
#[derive(Clone)]
pub struct FtpServer<'a> {
    /// The host: a name or an address.
    pub host: &'a str,
    /// The port.
    pub port: u16,
    /// Whether the connection turns to TLS before the login (`ftps`).
    pub secure: bool,
    /// How long the whole login may take.
    pub timeout: Duration,
    /// The TLS settings, which check the certificate of the server.
    pub tls: Arc<ClientConfig>,
}

impl std::fmt::Debug for FtpServer<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FtpServer")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("secure", &self.secure)
            .finish_non_exhaustive()
    }
}

/// Why the server could not be asked. The text names no password.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FtpFailure {
    /// The server cannot be reached, or the connection broke.
    #[error("the FTP server cannot be reached")]
    Unreachable,
    /// The server did not take `AUTH TLS`, or the TLS connection failed.
    #[error("the FTP server refused TLS")]
    NoTls,
    /// The login took longer than the time limit.
    #[error("the FTP server did not answer in time")]
    TimedOut,
}

/// Whether a text can be sent in a command: it has no control character, so it cannot
/// end the command and start another one.
fn is_safe(text: &str) -> bool {
    !text.chars().any(char::is_control)
}

/// Checks a name and a password. `Ok(true)` when the server logs the user in.
///
/// An empty password is refused without asking, so a server that lets in every user
/// without a password lets nobody into Organizr.
pub async fn log_in(
    server: &FtpServer<'_>,
    user: &str,
    password: &str,
) -> Result<bool, FtpFailure> {
    if user.is_empty() || password.is_empty() || !is_safe(user) || !is_safe(password) {
        return Ok(false);
    }
    tokio::time::timeout(server.timeout, attempt(server, user, password))
        .await
        .unwrap_or(Err(FtpFailure::TimedOut))
}

async fn attempt(server: &FtpServer<'_>, user: &str, password: &str) -> Result<bool, FtpFailure> {
    let host = server.host.trim_start_matches('[').trim_end_matches(']');
    let mut stream = AsyncRustlsFtpStream::connect((host, server.port))
        .await
        .map_err(|error| {
            tracing::warn!(host = %server.host, error = %error, "the FTP server could not be reached");
            FtpFailure::Unreachable
        })?;
    if server.secure {
        let connector =
            AsyncRustlsConnector::from(tokio_rustls::TlsConnector::from(Arc::clone(&server.tls)));
        stream = stream.into_secure(connector, host).await.map_err(|error| {
            tracing::warn!(host = %server.host, error = %error, "the FTP server refused TLS");
            FtpFailure::NoTls
        })?;
    }
    let accepted = match stream.login(user, password).await {
        Ok(()) => true,
        Err(FtpError::UnexpectedResponse(response)) => {
            tracing::debug!(status = ?response.status, "the FTP server refused a login");
            false
        }
        Err(error) => {
            tracing::warn!(host = %server.host, error = %error, "the FTP login broke off");
            return Err(FtpFailure::Unreachable);
        }
    };
    // The server may close the connection first; that is no error of the login.
    let _ = stream.quit().await;
    Ok(accepted)
}

#[cfg(test)]
mod tests {
    use super::is_safe;

    #[test]
    fn texts_with_control_characters_cannot_be_sent() {
        assert!(is_safe("robin"));
        assert!(is_safe("pass word!"));
        assert!(!is_safe("robin\r\nDELE x"));
        assert!(!is_safe("a\0"));
    }
}
