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
//!
//! An Admin can upload the certificate of an authority of their own (see
//! [`crate::custom_certificate`]). The requests to a service whose setting "use the
//! custom certificate" is on then also trust the certificates that this authority
//! signed: [`Outbound::with_custom_certificate`] gives the client for them.

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Once, PoisonError, RwLock};
use std::time::Duration;

use reqwest::{Certificate, Client};

use crate::custom_certificate::CustomCertificate;

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
    /// The same two clients for requests that carry a key in a header: they follow a
    /// redirect only on the same host (see [`stays_on_the_host`]).
    checking_keyed: Client,
    not_checking_keyed: Client,
    /// The same two clients without redirects, for the OpenID Connect login.
    checking_direct: Client,
    not_checking_direct: Client,
    /// The clients that check certificates and also trust the authority that the Admin
    /// uploaded, when there is one. Every copy of this value shares them.
    custom: Arc<RwLock<Option<CustomClients>>>,
}

/// The clients that also trust an uploaded authority: one for any request, and one for
/// requests with a key.
#[derive(Debug, Clone)]
struct CustomClients {
    any: Client,
    keyed: Client,
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
        let keyed = || builder().redirect(same_host_redirects());
        Ok(Self {
            checking: builder().build()?,
            not_checking: builder().danger_accept_invalid_certs(true).build()?,
            checking_keyed: keyed().build()?,
            not_checking_keyed: keyed().danger_accept_invalid_certs(true).build()?,
            checking_direct: direct().build()?,
            not_checking_direct: direct().danger_accept_invalid_certs(true).build()?,
            custom: Arc::default(),
        })
    }

    /// Trusts the authority of an uploaded certificate from now on, in the requests of
    /// [`Outbound::with_custom_certificate`], or no such authority with `None`. Every
    /// copy of this value sees the change.
    pub fn set_custom_certificate(
        &self,
        certificate: Option<&CustomCertificate>,
    ) -> Result<(), reqwest::Error> {
        let clients = certificate.map(custom_clients).transpose()?;
        *self.custom.write().unwrap_or_else(PoisonError::into_inner) = clients;
        Ok(())
    }

    /// Whether a client can be made that trusts the authority of a certificate, without
    /// trusting it.
    pub fn check_custom_certificate(certificate: &CustomCertificate) -> Result<(), reqwest::Error> {
        custom_clients(certificate).map(|_| ())
    }

    /// Whether an uploaded authority is trusted.
    pub fn has_custom_certificate(&self) -> bool {
        self.custom
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
    }

    /// This client for the requests to one service. With `wanted`, the setting "use the
    /// custom certificate" of the service, and an uploaded authority, the requests that
    /// check certificates also trust that authority. The name in the certificate is
    /// still checked.
    #[must_use]
    pub fn with_custom_certificate(&self, wanted: bool) -> Self {
        let mut copy = self.clone();
        if wanted
            && let Some(clients) = self
                .custom
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        {
            copy.checking = clients.any;
            copy.checking_keyed = clients.keyed;
        }
        copy
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
    /// checks the certificate. It uses no proxy of the environment either, which would
    /// reach the host by its own lookup. So the request cannot go to another place
    /// than the one that was checked before.
    pub fn pinned(&self, host: &str, address: SocketAddr) -> Result<Client, reqwest::Error> {
        Client::builder()
            .user_agent(user_agent())
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
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

    /// As [`Outbound::client_for`], for a request that carries a key or a token in a
    /// header of its own, such as `X-Api-Key`. The client does not take the request
    /// to another host or port by a redirect, so the key goes to no other place.
    pub fn keyed_client_for(&self, url: &str) -> &Client {
        self.keyed_client(url, false)
    }

    /// As [`Outbound::client`], for a request that carries a key or a token in a header
    /// of its own (see [`Outbound::keyed_client_for`]).
    pub fn keyed_client(&self, url: &str, never_check: bool) -> &Client {
        if never_check || is_address_over_https(url) {
            &self.not_checking_keyed
        } else {
            &self.checking_keyed
        }
    }
}

/// The largest number of redirects that a request follows, as `reqwest` has it.
const MOST_REDIRECTS: usize = 10;

/// The redirects of a request with a key: followed on the same host, and stopped at
/// another host or port. The caller then sees the redirect as the answer.
///
/// `reqwest` drops the headers `Authorization` and `Cookie` when a redirect leads to
/// another host, but it sends every other header again, also one with a key.
fn same_host_redirects() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= MOST_REDIRECTS {
            return attempt.error("too many redirects");
        }
        let stays = attempt
            .previous()
            .last()
            .is_some_and(|from| stays_on_the_host(from, attempt.url()));
        if stays {
            attempt.follow()
        } else {
            attempt.stop()
        }
    })
}

/// Whether a redirect from one address to another stays on the same host and port.
/// From `http` to `https` on the same host is also fine, when neither address names a
/// port.
fn stays_on_the_host(from: &reqwest::Url, to: &reqwest::Url) -> bool {
    let to_https = from.scheme() == "http"
        && to.scheme() == "https"
        && from.port().is_none()
        && to.port().is_none();
    from.host() == to.host()
        && (from.port_or_known_default() == to.port_or_known_default() || to_https)
}

/// The clients that check certificates and also trust the authority of a certificate.
fn custom_clients(certificate: &CustomCertificate) -> Result<CustomClients, reqwest::Error> {
    let roots = Certificate::from_pem_bundle(certificate.pem().as_bytes())?;
    let builder = || {
        Client::builder()
            .user_agent(user_agent())
            .connect_timeout(CONNECT_TIMEOUT)
            .tls_certs_merge(roots.clone())
    };
    Ok(CustomClients {
        any: builder().build()?,
        keyed: builder().redirect(same_host_redirects()).build()?,
    })
}

/// The text of an error of a request without the address of the request. The address
/// of some services holds their API key, which must not go into a log or an answer.
///
/// The text names the cause in plain words, with the deepest technical cause after it,
/// for example "the name of the server cannot be found (failed to lookup address
/// information: Name or service not known)". The top message of `reqwest`, "error
/// sending request", names no cause.
pub fn without_address(error: reqwest::Error) -> String {
    let error = error.without_url();
    let mut causes = Vec::new();
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        causes.push(cause.to_string());
        source = cause.source();
    }
    let all = causes.join(" ").to_lowercase();
    let has = |words: &[&str]| words.iter().any(|word| all.contains(word));
    let reason = if error.is_timeout() || has(&["timed out", "deadline has elapsed"]) {
        "the server did not answer in time"
    } else if has(&[
        "dns error",
        "failed to lookup address",
        "no such host",
        "name or service not known",
    ]) {
        "the name of the server cannot be found: check the address, and that Organizr is on a network where the name resolves"
    } else if has(&["connection refused", "actively refused"]) {
        "the server refused the connection: nothing answers at that port"
    } else if has(&["certificate", "unknownissuer", "unknown issuer"]) {
        "the certificate of the server is not trusted: switch on the setting that skips the check, or give Organizr the certificate of its authority"
    } else if has(&["unreachable", "no route to host"]) {
        "the server cannot be reached from Organizr"
    } else if error.is_connect() {
        "Organizr cannot connect to the server"
    } else {
        return match causes.last() {
            Some(detail) => format!("{error}: {detail}"),
            None => error.to_string(),
        };
    };
    match causes.last() {
        Some(detail) => format!("{reason} ({detail})"),
        None => reason.to_owned(),
    }
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

/// The body of an answer, when it has at most `limit` bytes. `None` when it is larger.
/// The size is counted while the body comes, because a server can name no size, or a
/// wrong one.
pub async fn body_within(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Option<Vec<u8>>, reqwest::Error> {
    if response
        .content_length()
        .is_some_and(|announced| usize::try_from(announced).map_or(true, |size| size > limit))
    {
        return Ok(None);
    }
    let mut body = Vec::new();
    while let Some(piece) = response.chunk().await? {
        if body.len().saturating_add(piece.len()) > limit {
            return Ok(None);
        }
        body.extend_from_slice(&piece);
    }
    Ok(Some(body))
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

/// Whether a URL names its host by an IP address and uses HTTPS. The URL is read as
/// the request reads it, so the host that is checked is the one that is reached.
pub fn is_address_over_https(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|parsed| {
        // An IPv6 address stands in brackets.
        parsed.scheme() == "https"
            && parsed
                .host_str()
                .is_some_and(|host| host.trim_matches(['[', ']']).parse::<IpAddr>().is_ok())
    })
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
            // A backslash ends the host, as for the browser: the request goes to the name.
            "https://name.example\\@10.0.0.5/",
            "https://name.example\\x@[::1]/",
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
        // The clients for requests with a key are others again.
        let keyed = std::ptr::from_ref(outbound.keyed_client_for("https://plex.tv/"));
        let keyed_by_address = std::ptr::from_ref(outbound.keyed_client_for("https://10.0.0.5/"));
        assert_ne!(named, keyed);
        assert_ne!(by_address, keyed_by_address);
        assert_ne!(keyed, keyed_by_address);
    }

    #[test]
    fn requests_with_a_key_follow_a_redirect_only_on_the_same_host() {
        let place = |url: &str| reqwest::Url::parse(url).expect("an address");
        let stays = |from: &str, to: &str| super::stays_on_the_host(&place(from), &place(to));
        assert!(stays("http://sonarr:8989/a", "http://sonarr:8989/b"));
        assert!(stays("http://sonarr:8989/a", "https://sonarr:8989/a"));
        assert!(stays("http://sonarr.example/a", "https://sonarr.example/a"));
        assert!(stays(
            "https://sonarr.example/a",
            "https://SONARR.example:443/b"
        ));
        assert!(!stays("http://sonarr:8989/a", "http://elsewhere:8989/a"));
        assert!(!stays("http://sonarr:8989/a", "http://sonarr:8990/a"));
        assert!(!stays(
            "http://sonarr.example/a",
            "https://sonarr.example:8443/a"
        ));
        assert!(!stays("http://10.0.0.5/a", "http://10.0.0.6/a"));
    }

    #[tokio::test]
    async fn requests_with_a_key_stop_at_a_redirect_to_another_place() {
        use axum::Router;
        use axum::response::Redirect;
        use axum::routing::get;
        use organizr_testkit::services::StandIn;

        let elsewhere =
            StandIn::start(Router::new().route("/target", get(|| async { "away" }))).await;
        let to = format!("{}/target", elsewhere.url());
        let site = StandIn::start(
            Router::new()
                .route("/target", get(|| async { "here" }))
                .route("/same", get(|| async { Redirect::temporary("/target") }))
                .route(
                    "/away",
                    get(move || {
                        let to = to.clone();
                        async move { Redirect::temporary(&to) }
                    }),
                ),
        )
        .await;
        let base = site.url();
        let outbound = Outbound::new().expect("the client can be built");

        let same = format!("{base}/same");
        let answer = outbound
            .keyed_client_for(&same)
            .get(&same)
            .header("X-Api-Key", "secret")
            .send()
            .await
            .expect("the site answers");
        assert_eq!(answer.status(), 200);
        assert_eq!(answer.text().await.expect("a text"), "here");

        let away = format!("{base}/away");
        let answer = outbound
            .keyed_client_for(&away)
            .get(&away)
            .header("X-Api-Key", "secret")
            .send()
            .await
            .expect("the site answers");
        assert_eq!(answer.status(), 307);
        assert!(elsewhere.seen().is_empty(), "the key went to another place");

        // A request without a key still follows the redirect.
        let answer = outbound
            .client_for(&away)
            .get(&away)
            .send()
            .await
            .expect("the site answers");
        assert_eq!(answer.text().await.expect("a text"), "away");
    }

    #[tokio::test]
    async fn body_is_read_up_to_a_limit() {
        use axum::Router;
        use axum::routing::get;
        use organizr_testkit::services::StandIn;

        let site = StandIn::start(Router::new().route("/", get(|| async { "0123456789" }))).await;
        let url = format!("{}/", site.url());
        let outbound = Outbound::new().expect("the client can be built");
        let body = async |limit| {
            let response = outbound
                .client_for(&url)
                .get(&url)
                .send()
                .await
                .expect("the site answers");
            super::body_within(response, limit)
                .await
                .expect("the body comes")
        };
        assert_eq!(body(10).await, Some(b"0123456789".to_vec()));
        assert_eq!(body(9).await, None);
    }

    /// The name of the variable that makes the test below the child process.
    const PROXY_CHILD: &str = "ORGANIZR_TEST_PINNED_PROXY_CHILD";

    #[tokio::test]
    async fn pinned_client_uses_no_proxy_of_the_environment() {
        use axum::Router;
        use axum::routing::get;
        use organizr_testkit::services::StandIn;

        if std::env::var_os(PROXY_CHILD).is_some() {
            // The child process: HTTP_PROXY names a place that never answers.
            let site = StandIn::start(Router::new().route("/", get(|| async { "pinned" }))).await;
            let address: std::net::SocketAddr = site
                .url()
                .trim_start_matches("http://")
                .parse()
                .expect("the address of the site");
            let outbound = Outbound::new().expect("the client can be built");
            let client = outbound
                .pinned("pinned.example", address)
                .expect("the client can be built");
            let url = format!("http://pinned.example:{}/", address.port());
            let answer = client
                .get(&url)
                .timeout(Duration::from_secs(5))
                .send()
                .await
                .expect("the pinned site answers, not the proxy");
            assert_eq!(answer.text().await.expect("a text"), "pinned");
            return;
        }
        // A proxy that takes connections and never answers.
        let proxy = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let proxy_url = format!("http://{}", proxy.local_addr().expect("an address"));
        let status = std::process::Command::new(std::env::current_exe().expect("the test"))
            .args([
                "--exact",
                "http::tests::pinned_client_uses_no_proxy_of_the_environment",
                "--nocapture",
            ])
            .env(PROXY_CHILD, "1")
            .env("HTTP_PROXY", &proxy_url)
            .env("http_proxy", &proxy_url)
            .env("ALL_PROXY", &proxy_url)
            .env_remove("NO_PROXY")
            .env_remove("no_proxy")
            .status()
            .expect("the child runs");
        drop(proxy);
        assert!(status.success(), "the pinned client went through the proxy");
    }
}
