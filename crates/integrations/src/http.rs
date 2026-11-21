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

use std::net::{IpAddr, SocketAddr};
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
    /// The same two clients without redirects, for the OpenID Connect login.
    checking_direct: Client,
    not_checking_direct: Client,
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
        let direct = || builder().redirect(reqwest::redirect::Policy::none());
        Ok(Self {
            checking: builder().build()?,
            not_checking: builder().danger_accept_invalid_certs(true).build()?,
            checking_direct: direct().build()?,
            not_checking_direct: direct().danger_accept_invalid_certs(true).build()?,
        })
    }

    /// The client for a URL: the one that checks certificates, unless the URL reaches a
    /// service by its address over HTTPS.
    pub fn client_for(&self, url: &str) -> &Client {
        self.client(url, false)
    }

    /// As [`Outbound::client_for`], with a client that follows no redirect. A provider of
    /// the OpenID Connect login answers each request itself; a redirect could send the
    /// request with the code of a login to another place.
    pub fn direct_client_for(&self, url: &str) -> &Client {
        self.direct_client(url, false)
    }

    /// As [`Outbound::client`], with a client that follows no redirect. With
    /// `never_check`, the certificate is not checked.
    pub fn direct_client(&self, url: &str, never_check: bool) -> &Client {
        if never_check || is_address_over_https(url) {
            &self.not_checking_direct
        } else {
            &self.checking_direct
        }
    }

    /// A client for one request to an address that a caller of the API named. It
    /// reaches `host` at `address` and nowhere else, it follows no redirect, and it
    /// checks the certificate. So the request cannot go to another place than the one
    /// that was checked before.
    pub fn pinned(&self, host: &str, address: SocketAddr) -> Result<Client, reqwest::Error> {
        Client::builder()
            .user_agent(user_agent())
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .resolve(host, address)
            .build()
    }

    /// The client for a URL of a service that has a setting "do not check the
    /// certificate". With `never_check`, the certificate is not checked.
    pub fn client(&self, url: &str, never_check: bool) -> &Client {
        if never_check || is_address_over_https(url) {
            &self.not_checking
        } else {
            &self.checking
        }
    }
}

/// The text of an error of a request without the address of the request. The address
/// of some services holds their API key, which must not go into a log or an answer.
pub fn without_address(error: reqwest::Error) -> String {
    error.without_url().to_string()
}

/// A text for a place in an address: every byte that is no letter, digit, `-`, `.`, `_`
/// or `~` is written as `%` and its two hexadecimal digits.
pub fn escaped(text: &str) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(char::from(byte));
            }
            other => {
                // Writing into a text cannot fail.
                let _ = write!(out, "%{other:02X}");
            }
        }
    }
    out
}

/// The time limit of a request from a setting of the PHP application, which is the
/// time between two refreshes of a homepage item: a number of 1000 or more is in
/// milliseconds, a smaller one is in seconds. A text that is no number is no limit.
pub fn time_limit(setting: &str) -> Option<Duration> {
    let number: f64 = setting.trim().parse().ok()?;
    if !number.is_finite() || number <= 0.0 {
        return None;
    }
    let seconds = if number >= 1000.0 {
        number / 1000.0
    } else {
        number
    };
    Duration::try_from_secs_f64(seconds).ok()
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
    use std::time::Duration;

    use super::{Outbound, is_address_over_https, time_limit, user_agent};

    #[test]
    fn time_limit_is_in_milliseconds_from_one_thousand_up() {
        assert_eq!(time_limit("60000"), Some(Duration::from_secs(60)));
        assert_eq!(time_limit("1000"), Some(Duration::from_secs(1)));
        assert_eq!(time_limit("999"), Some(Duration::from_secs(999)));
        assert_eq!(time_limit(" 30 "), Some(Duration::from_secs(30)));
        for none in ["", "soon", "0", "-5", "NaN", "inf"] {
            assert_eq!(time_limit(none), None, "{none}");
        }
    }

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
    fn text_for_an_address_keeps_only_plain_characters() {
        assert_eq!(super::escaped("abc-XYZ_0.9~"), "abc-XYZ_0.9~");
        assert_eq!(super::escaped("a b&c=d/e?"), "a%20b%26c%3Dd%2Fe%3F");
        assert_eq!(super::escaped("\u{e9}"), "%C3%A9");
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
