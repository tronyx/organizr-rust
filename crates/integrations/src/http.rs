//! The HTTP client for requests to other services.
//!
//! Every request names the application in its `User-Agent` and gives up when no
//! connection is made within five seconds, as in the PHP application.
//!
//! A service that is reached by its address over HTTPS, such as
//! `https://192.168.1.10:8181`, cannot have a certificate that a browser accepts: no
//! authority signs a certificate for a private address. The PHP application does not
//! check the certificate of such a service, and neither does this client. The
//! certificate of every service that is reached by a name is checked.

use std::net::IpAddr;
use std::sync::Once;
use std::time::Duration;

use reqwest::Client;

/// How long a connection may take to open.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// The `User-Agent` of every request.
pub fn user_agent() -> String {
    format!("organizr/{}", env!("CARGO_PKG_VERSION"))
}

/// The client for requests to other services.
#[derive(Debug, Clone)]
pub struct Outbound {
    checking: Client,
    not_checking: Client,
}

impl Outbound {
    /// Builds the client.
    pub fn new() -> Result<Self, reqwest::Error> {
        // The TLS library needs to know which implementation of its ciphers to use.
        // Another part of the process may have said so already, which is fine.
        static CIPHERS: Once = Once::new();
        CIPHERS.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });

        let builder = || {
            Client::builder()
                .user_agent(user_agent())
                .connect_timeout(CONNECT_TIMEOUT)
        };
        Ok(Self {
            checking: builder().build()?,
            not_checking: builder().danger_accept_invalid_certs(true).build()?,
        })
    }

    /// The client for a URL: the one that checks certificates, unless the URL reaches a
    /// service by its address over HTTPS.
    pub fn client_for(&self, url: &str) -> &Client {
        if is_address_over_https(url) {
            &self.not_checking
        } else {
            &self.checking
        }
    }
}

/// Whether a URL names its host by an IP address and uses HTTPS.
pub fn is_address_over_https(url: &str) -> bool {
    let Some(rest) = url
        .get(..8)
        .filter(|scheme| scheme.eq_ignore_ascii_case("https://"))
        .and_then(|_| url.get(8..))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // The part after the last `@` is the host, with or without a port.
    let host = authority.rsplit('@').next().unwrap_or_default();
    let host = match host.strip_prefix('[') {
        // An IPv6 address stands in brackets.
        Some(bracketed) => bracketed.split(']').next().unwrap_or_default(),
        None => host.split(':').next().unwrap_or_default(),
    };
    host.parse::<IpAddr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::{Outbound, is_address_over_https, user_agent};

    #[test]
    fn certificate_is_not_checked_only_for_an_address_over_https() {
        for url in [
            "https://192.168.1.10",
            "https://192.168.1.10:8181/tautulli",
            "HTTPS://10.0.0.5/",
            "https://user:password@10.0.0.5:5055/api",
            "https://[fd00::10]:8181/",
            "https://[::1]",
        ] {
            assert!(is_address_over_https(url), "{url}");
        }
        for url in [
            "https://plex.tv/users/sign_in.json",
            "https://ombi.example.com",
            // A name that holds numbers is a name.
            "https://10.0.0.5.example.com/",
            "https://plex.example.com/192.168.1.10",
            "https://example.com/?to=https://10.0.0.5",
            // Without HTTPS there is no certificate.
            "http://192.168.1.10:8181",
            "192.168.1.10",
            "https://",
            "",
        ] {
            assert!(!is_address_over_https(url), "{url}");
        }
    }

    #[test]
    fn client_names_the_application() {
        assert_eq!(
            user_agent(),
            concat!("organizr/", env!("CARGO_PKG_VERSION"))
        );
        let outbound = Outbound::new().expect("the client can be built");
        // Two clients: the choice is by the URL.
        let named = std::ptr::from_ref(outbound.client_for("https://plex.tv/"));
        let by_address = std::ptr::from_ref(outbound.client_for("https://10.0.0.5/"));
        assert_ne!(named, by_address);
    }
}
