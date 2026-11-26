//! The host names that a service address which is only a path, such as `/sonarr`, may
//! be completed with (D-154).
//!
//! PHP puts the scheme and the host of the incoming request in front of such an address.
//! The caller chooses that host, and the request to the address carries the key of the
//! service, so on its own the host of a request is never trusted (D-137). The operator
//! names the hosts of Organizr in `ORGANIZR_ALLOWED_HOSTS`. A request that names one of
//! them completes a path with that host, for as long as the request runs; any other
//! request completes no path.

use std::future::Future;
use std::str::FromStr;

use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::Response;

use crate::client::Client;
use crate::state::AppState;

/// One host of the list: the name in small letters, and the port when the entry names
/// one.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Allowed {
    name: String,
    port: Option<u16>,
}

/// The hosts of `ORGANIZR_ALLOWED_HOSTS`. Empty means that no host is allowed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AllowedHosts(Vec<Allowed>);

/// An entry of `ORGANIZR_ALLOWED_HOSTS` that is no host.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is no host name, or host name and port")]
pub struct NoHost(String);

/// A host name and its port as a `Host` header or an entry writes them, such as
/// `organizr.example.com`, `192.168.1.10:8080` or `[::1]:8080`.
fn split(text: &str) -> Option<(String, Option<u16>)> {
    let text = text.trim();
    let (name, port) = if let Some(rest) = text.strip_prefix('[') {
        let (name, after) = rest.split_once(']')?;
        let port = match after {
            "" => None,
            after => Some(after.strip_prefix(':')?.parse().ok()?),
        };
        (format!("[{name}]"), port)
    } else {
        match text.rsplit_once(':') {
            Some((name, port)) => (name.to_owned(), Some(port.parse().ok()?)),
            None => (text.to_owned(), None),
        }
    };
    let is_name = !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-[]:".contains(character));
    is_name.then(|| (name.to_ascii_lowercase(), port))
}

impl FromStr for AllowedHosts {
    type Err = NoHost;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| {
                split(entry)
                    .map(|(name, port)| Allowed { name, port })
                    .ok_or_else(|| NoHost(entry.to_owned()))
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }
}

impl AllowedHosts {
    /// Whether the host that a `Host` header names is on the list. An entry without a
    /// port allows the host with any port; an entry with a port allows that port only.
    pub fn allows(&self, host: &str) -> bool {
        let Some((name, port)) = split(host) else {
            return false;
        };
        self.0.iter().any(|allowed| {
            allowed.name == name && allowed.port.is_none_or(|wanted| Some(wanted) == port)
        })
    }
}

tokio::task_local! {
    /// The address of the site of the request that runs, such as
    /// `https://organizr.example.com`, when its host is allowed.
    static SITE: String;
}

/// The address of the site that a path of a service address is completed with: the
/// scheme and the host of the request that runs, when the host is allowed. Nothing
/// outside a request, or for a host that is not on the list.
pub fn current_site() -> Option<String> {
    SITE.try_with(Clone::clone).ok()
}

/// Runs `work` as the work of a request to `site`, for a test or a task that a request
/// started.
pub async fn within<F: Future>(site: String, work: F) -> F::Output {
    SITE.scope(site, work).await
}

/// The layer that remembers the site of a request whose host is allowed (D-154).
pub async fn remember_site(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .filter(|host| state.allowed_hosts().allows(host))
        .map(str::to_owned);
    let Some(host) = host else {
        return next.run(request).await;
    };
    let https = request
        .extensions()
        .get::<Client>()
        .is_some_and(|client| client.https);
    let scheme = if https { "https" } else { "http" };
    within(format!("{scheme}://{host}"), next.run(request)).await
}

#[cfg(test)]
mod tests {
    use super::{AllowedHosts, current_site, within};

    fn hosts(text: &str) -> AllowedHosts {
        text.parse().expect("the list is valid")
    }

    #[test]
    fn list_allows_its_hosts_and_no_other() {
        let list = hosts(" Organizr.Example.com , 192.168.1.10:8080,[::1]:8080,,");
        for host in [
            "organizr.example.com",
            "ORGANIZR.example.com:443",
            "192.168.1.10:8080",
            "[::1]:8080",
        ] {
            assert!(list.allows(host), "{host}");
        }
        for host in [
            "evil.example.com",
            "organizr.example.com.evil.example",
            "192.168.1.10",
            "192.168.1.10:8081",
            "[::1]",
            "",
            "organizr.example.com/x",
        ] {
            assert!(!list.allows(host), "{host}");
        }
        // Nothing is allowed without a list.
        assert!(!AllowedHosts::default().allows("organizr.example.com"));
    }

    #[test]
    fn entry_that_is_no_host_is_refused() {
        for text in ["https://organizr.example.com", "a b", "host:port", "[::1"] {
            assert!(text.parse::<AllowedHosts>().is_err(), "{text}");
        }
    }

    #[tokio::test]
    async fn site_holds_only_inside_its_request() {
        assert_eq!(current_site(), None);
        let inside = within("https://organizr.example.com".to_owned(), async {
            current_site()
        })
        .await;
        assert_eq!(inside.as_deref(), Some("https://organizr.example.com"));
        assert_eq!(current_site(), None);
    }
}
