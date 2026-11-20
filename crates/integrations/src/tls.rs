//! The TLS settings of the connections that are no HTTP requests: to an LDAP server and
//! to an FTP server, for a login.
//!
//! The certificate of the server is always checked, against the certificate authorities
//! that the operating system trusts. The PHP application accepts every certificate on
//! these connections.

use std::sync::{Arc, OnceLock};

use rustls::ClientConfig;
use rustls::RootCertStore;

/// Why the TLS settings could not be made.
pub use rustls::Error as TlsError;

/// The TLS settings: the ciphers of `ring`, the safe versions of TLS, and the
/// certificate authorities of the operating system. They are made once.
pub fn client_config() -> Result<Arc<ClientConfig>, TlsError> {
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    if let Some(config) = CONFIG.get() {
        return Ok(Arc::clone(config));
    }
    let mut roots = RootCertStore::empty();
    let found = rustls_native_certs::load_native_certs();
    if !found.errors.is_empty() {
        tracing::warn!(
            errors = found.errors.len(),
            "some certificate authorities of the system could not be read"
        );
    }
    let (_added, ignored) = roots.add_parsable_certificates(found.certs);
    if ignored > 0 {
        tracing::debug!(
            ignored,
            "some certificate authorities of the system were not used"
        );
    }
    let config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth();
    Ok(Arc::clone(CONFIG.get_or_init(|| Arc::new(config))))
}
