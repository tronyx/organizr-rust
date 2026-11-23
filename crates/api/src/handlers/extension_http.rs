//! The requests of extensions to other sites (D-122).
//!
//! An extension names each site that it wants to reach with a permission
//! `http:<host>`, and an Admin approves the list with the extension. A site on a local
//! network also needs the permission `http-local`. Each request is checked here:
//!
//! - the scheme is `http` or `https`, and the host is one of the approved list;
//! - the name is resolved, and when one of its addresses is of this computer or of a
//!   private or link-local network, the request needs `http-local`;
//! - the request goes to the checked address and nowhere else, follows no redirect,
//!   checks the certificate, and has [`TIME_LIMIT`] and [`ANSWER_LIMIT`].

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use organizr_integrations::http::{Outbound, without_address};
use organizr_integrations::scrape;
use organizr_plugins::extension::{HttpGate, HttpRequest, HttpResponse};

/// The prefix of a permission that names a site.
pub const HOST_PREFIX: &str = "http:";

/// The permission to reach sites on a local network.
pub const LOCAL: &str = "http-local";

/// How long one request may take, with its answer.
pub const TIME_LIMIT: Duration = Duration::from_secs(10);

/// How long the name of a site may take to resolve.
const LOOKUP_LIMIT: Duration = Duration::from_secs(5);

/// The largest body of an answer, in bytes.
pub const ANSWER_LIMIT: usize = 4 * 1024 * 1024;

/// Whether a text is a host that a permission may name: a name of small letters,
/// digits, hyphens and dots, or an IPv4 address.
pub fn is_host(text: &str) -> bool {
    let labels_are_valid = text.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|letter| {
                letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '-'
            })
    });
    !text.is_empty() && text.len() <= 253 && labels_are_valid
}

/// Whether this version knows a permission of the requests to other sites.
pub fn is_permission(permission: &str) -> bool {
    permission == LOCAL || permission.strip_prefix(HOST_PREFIX).is_some_and(is_host)
}

/// The gate of one call of an extension.
#[derive(Debug, Clone)]
pub struct Gate {
    outbound: Outbound,
    hosts: Vec<String>,
    local: bool,
    runtime: tokio::runtime::Handle,
    extension: String,
}

impl Gate {
    /// The gate for the approved permissions of an extension, when they name any site.
    /// `permissions` is the text of the database, with commas.
    pub fn for_permissions(
        outbound: &Outbound,
        permissions: &str,
        extension: &str,
        runtime: tokio::runtime::Handle,
    ) -> Option<Self> {
        let hosts: Vec<String> = permissions
            .split(',')
            .filter_map(|one| one.strip_prefix(HOST_PREFIX))
            .filter(|host| is_host(host))
            .map(str::to_owned)
            .collect();
        if hosts.is_empty() {
            return None;
        }
        Some(Self {
            outbound: outbound.clone(),
            hosts,
            local: permissions.split(',').any(|one| one == LOCAL),
            runtime,
            extension: extension.to_owned(),
        })
    }

    /// Where the host of a request is reached, after the checks.
    async fn address_of(&self, url: &str) -> Result<(String, SocketAddr), String> {
        let (host, port) = scrape::target(url)
            .ok_or_else(|| "the address is no http or https address with a host".to_owned())?;
        if !self.hosts.contains(&host) {
            return Err(format!(
                "the site {host} is not approved for this extension"
            ));
        }
        let addresses: Vec<SocketAddr> = match host.parse::<IpAddr>() {
            Ok(address) => vec![SocketAddr::new(address, port)],
            Err(_) => {
                tokio::time::timeout(LOOKUP_LIMIT, tokio::net::lookup_host((host.as_str(), port)))
                    .await
                    .map_err(|_| format!("the name {host} did not resolve in time"))?
                    .map_err(|_| format!("the name {host} does not resolve"))?
                    .collect()
            }
        };
        // A name with one local address among its addresses counts as local: the
        // request could go to that one.
        if !self.local
            && addresses
                .iter()
                .any(|address| !scrape::is_public(address.ip()))
        {
            return Err(format!(
                "the site {host} is on a local network, and this extension has no permission \
                 {LOCAL}"
            ));
        }
        let address = addresses
            .first()
            .copied()
            .ok_or_else(|| format!("the name {host} does not resolve"))?;
        Ok((host, address))
    }

    async fn send_checked(&self, request: HttpRequest) -> Result<HttpResponse, String> {
        let method = axum::http::Method::from_bytes(request.method.to_ascii_uppercase().as_bytes())
            .map_err(|_| format!("{} is no method", request.method))?;
        let (host, address) = self.address_of(&request.url).await?;
        let client = self
            .outbound
            .pinned(&host, address)
            .map_err(without_address)?;
        let mut outgoing = client.request(method, &request.url).timeout(TIME_LIMIT);
        for (name, value) in &request.headers {
            // The host of the address names the site; another one is not sent.
            if !name.eq_ignore_ascii_case("host") {
                outgoing = outgoing.header(name.as_str(), value.as_str());
            }
        }
        let mut response = outgoing
            .body(request.body)
            .send()
            .await
            .map_err(without_address)?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect();
        let mut body = Vec::new();
        while let Some(piece) = response.chunk().await.map_err(without_address)? {
            if body.len() + piece.len() > ANSWER_LIMIT {
                return Err(format!(
                    "the answer of {host} is larger than {ANSWER_LIMIT} bytes"
                ));
            }
            body.extend_from_slice(&piece);
        }
        tracing::debug!(
            extension = self.extension,
            host,
            status,
            "an extension sent a request"
        );
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

impl HttpGate for Gate {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, String> {
        // The call of an extension runs on a thread for blocking work, which may wait for
        // the runtime.
        let answer = self.runtime.block_on(self.send_checked(request));
        if let Err(error) = &answer {
            tracing::info!(extension = self.extension, %error, "a request of an extension was refused or failed");
        }
        answer
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::routing::{get, post};
    use organizr_testkit::services::StandIn;

    use super::*;

    #[test]
    fn hosts_and_permissions_are_checked() {
        for host in ["api.github.com", "a", "my-site.example", "192.168.1.5"] {
            assert!(is_host(host), "{host}");
            assert!(is_permission(&format!("http:{host}")), "{host}");
        }
        for host in [
            "",
            "API.github.com",
            "a..b",
            ".a",
            "-a.b",
            "a b",
            "a/b",
            "a:80",
            "*.example",
            &"a".repeat(64),
        ] {
            assert!(!is_host(host), "{host}");
        }
        assert!(is_permission("http-local"));
        for permission in ["http:", "http", "https:a", "network"] {
            assert!(!is_permission(permission), "{permission}");
        }
    }

    #[test]
    fn gate_exists_only_for_permissions_that_name_a_site() {
        let outbound = Outbound::new().expect("the client builds");
        let runtime = tokio::runtime::Runtime::new().expect("a runtime starts");
        let gate = |permissions: &str| {
            Gate::for_permissions(&outbound, permissions, "test", runtime.handle().clone())
        };
        assert!(gate("").is_none());
        assert!(gate("storage").is_none());
        assert!(gate("http-local,storage").is_none());
        let one = gate("http-local,http:a.example,storage").expect("a site is named");
        assert_eq!(one.hosts, ["a.example"]);
        assert!(one.local);
        assert!(!gate("http:a.example").expect("a site is named").local);
    }

    fn get_request(url: &str) -> HttpRequest {
        HttpRequest {
            method: "GET".to_owned(),
            url: url.to_owned(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Sends a request through a gate on a thread for blocking work, as a call does.
    async fn send(gate: Gate, request: HttpRequest) -> Result<HttpResponse, String> {
        tokio::task::spawn_blocking(move || gate.send(request))
            .await
            .expect("the thread ends")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn requests_reach_an_approved_local_site_only_with_http_local() {
        let routes = Router::new()
            .route(
                "/echo",
                post(|body: String| async move { format!("got {body}") }),
            )
            .route("/big", get(|| async { "a".repeat(ANSWER_LIMIT + 1) }))
            .route(
                "/away",
                get(|| async { axum::response::Redirect::temporary("https://elsewhere.example/") }),
            );
        let site = StandIn::start(routes).await;
        let base = site.url();
        let outbound = Outbound::new().expect("the client builds");
        let gate = |permissions: &str| {
            Gate::for_permissions(
                &outbound,
                permissions,
                "test",
                tokio::runtime::Handle::current(),
            )
            .expect("a site is named")
        };

        // Without http-local, the address of this computer is refused.
        let refused = send(gate("http:127.0.0.1"), get_request(&format!("{base}/echo"))).await;
        let error = refused.expect_err("a local site needs http-local");
        assert!(error.contains("local network"), "{error}");
        assert_eq!(site.seen().len(), 0);

        let local = gate("http:127.0.0.1,http-local");
        let answered = send(
            local.clone(),
            HttpRequest {
                method: "post".to_owned(),
                url: format!("{base}/echo"),
                headers: vec![
                    ("x-test".to_owned(), "1".to_owned()),
                    ("host".to_owned(), "elsewhere.example".to_owned()),
                ],
                body: b"ping".to_vec(),
            },
        )
        .await
        .expect("the site answers");
        assert_eq!(answered.status, 200);
        assert_eq!(answered.body, b"got ping");
        let seen = site.seen();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen.first().and_then(|one| one.header("x-test")), Some("1"));
        assert_ne!(
            seen.first().and_then(|one| one.header("host")),
            Some("elsewhere.example")
        );

        // A redirect is handed to the extension, not followed.
        let away = send(local.clone(), get_request(&format!("{base}/away")))
            .await
            .expect("the site answers");
        assert_eq!(away.status, 307);

        let big = send(local.clone(), get_request(&format!("{base}/big"))).await;
        let error = big.expect_err("the answer is too large");
        assert!(error.contains("larger than"), "{error}");

        // Another site, another scheme, or the same site by another name is refused.
        for url in [
            "https://elsewhere.example/".to_owned(),
            format!(
                "ftp://127.0.0.1:{}/",
                site.url().rsplit(':').next().unwrap_or_default()
            ),
            format!(
                "http://localhost:{}/echo",
                site.url().rsplit(':').next().unwrap_or_default()
            ),
            "no address".to_owned(),
        ] {
            let refused = send(local.clone(), get_request(&url)).await;
            assert!(refused.is_err(), "{url}");
        }
    }
}
