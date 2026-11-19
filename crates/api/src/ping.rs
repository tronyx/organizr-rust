//! Probing whether the application behind a tab answers.
//!
//! A tab can name an address to check: a host, with or without a port. With a port, the
//! check opens a TCP connection to it. Without a port, it sends one ICMP echo request,
//! which is what the `ping` command does. The result is the time that the answer took.

use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::TcpStream;

use crate::php_url;

/// How long a probe waits for an answer. The PHP application waits as long.
pub const TIMEOUT: Duration = Duration::from_secs(2);

/// What a probe goes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingTarget {
    /// A host name or an IP address.
    pub host: String,
    /// The port for a TCP connection. Without it, the probe is an ICMP echo request.
    pub port: Option<u16>,
}

impl PingTarget {
    /// Reads the target from the text that a tab stores, as `formatPingHost()` of the PHP
    /// application reads it: `plex.example.com:32400`, `192.168.1.5`, or a whole address
    /// such as `https://plex.example.com:32400/web`. Text without a host is no target.
    pub fn of(entry: &str) -> Option<Self> {
        let (host, port) = php_url::host_and_port(entry.trim())?;
        let Some(bracketed) = host.strip_prefix('[') else {
            return Some(Self { host, port });
        };
        // An IPv6 address stands in brackets, so that its colons are not read as the
        // start of a port. PHP reads such text as a host name, which no host has.
        let (address, rest) = bracketed.split_once(']')?;
        let port = match rest.strip_prefix(':') {
            Some(digits) => Some(digits.parse().ok()?),
            None if rest.is_empty() => port,
            None => return None,
        };
        (!address.is_empty()).then(|| Self {
            host: address.to_owned(),
            port,
        })
    }
}

/// The result of a probe: the time that the answer took, in milliseconds, or `None` when
/// no answer came.
pub type Latency = Option<f64>;

/// The boxed future of one probe.
type Probing = Pin<Box<dyn Future<Output = Latency> + Send>>;

/// Something that probes a target. The application uses the network. A test uses a
/// stand-in, so that it needs no network.
#[derive(Clone)]
pub struct Prober(Arc<dyn Fn(PingTarget) -> Probing + Send + Sync>);

impl Prober {
    /// A prober that runs the given function for each target.
    pub fn new<F, Fut>(probe: F) -> Self
    where
        F: Fn(PingTarget) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Latency> + Send + 'static,
    {
        Self(Arc::new(move |target| Box::pin(probe(target))))
    }

    /// The prober of the application: it asks the network.
    pub fn network() -> Self {
        Self::new(probe)
    }

    /// Probes a target once.
    pub async fn probe(&self, target: PingTarget) -> Latency {
        (self.0)(target).await
    }
}

impl std::fmt::Debug for Prober {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Prober")
    }
}

/// A time in milliseconds with four decimal places, as the PHP application reports it.
fn milliseconds(time: Duration) -> f64 {
    (time.as_secs_f64() * 10_000_000.0).round() / 10_000.0
}

/// Probes a target once, and gives up after [`TIMEOUT`].
async fn probe(target: PingTarget) -> Latency {
    let start = Instant::now();
    let answered = match target.port {
        Some(port) => {
            tokio::time::timeout(TIMEOUT, TcpStream::connect((target.host.as_str(), port)))
                .await
                .is_ok_and(|connection| connection.is_ok())
        }
        None => tokio::time::timeout(TIMEOUT, echo(&target.host))
            .await
            .unwrap_or(false),
    };
    answered.then(|| milliseconds(start.elapsed()))
}

/// Sends one ICMP echo request to a host and waits for the answer.
///
/// The system can refuse to open the socket for it, for example when the application
/// runs without the right to send such requests. Then the host counts as not answering,
/// and the log says why.
async fn echo(host: &str) -> bool {
    let Some(address) = resolve(host).await else {
        return false;
    };
    match surge_ping::ping(address, &[0; 8]).await {
        Ok(_) => true,
        Err(surge_ping::SurgeError::IOError(error)) => {
            tracing::warn!(%error, host, "an ICMP echo request could not be sent");
            false
        }
        Err(_) => false,
    }
}

/// The first address of a host.
async fn resolve(host: &str) -> Option<IpAddr> {
    if let Ok(address) = host.parse() {
        return Some(address);
    }
    tokio::net::lookup_host((host, 0))
        .await
        .ok()?
        .next()
        .map(|address| address.ip())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::net::TcpListener;

    use super::{PingTarget, milliseconds, probe};

    fn target(host: &str, port: Option<u16>) -> PingTarget {
        PingTarget {
            host: host.to_owned(),
            port,
        }
    }

    #[test]
    fn target_is_read_from_the_text_of_a_tab() {
        let cases = [
            (
                "plex.example.com:32400",
                Some(("plex.example.com", Some(32400))),
            ),
            ("192.168.1.5", Some(("192.168.1.5", None))),
            ("  plex.example.com  ", Some(("plex.example.com", None))),
            (
                "https://plex.example.com:32400/web",
                Some(("plex.example.com", Some(32400))),
            ),
            (
                "http://plex.example.com/web/index.html",
                Some(("plex.example.com", None)),
            ),
            ("plex.example.com/web", Some(("plex.example.com", None))),
            ("[2001:db8::1]:8080", Some(("2001:db8::1", Some(8080)))),
            ("[2001:db8::1]", Some(("2001:db8::1", None))),
            (
                "http://[2001:db8::1]:8080/web",
                Some(("2001:db8::1", Some(8080))),
            ),
            ("[2001:db8::1]:port", None),
            ("[2001:db8::1]x", None),
            ("[]:80", None),
            ("", None),
            ("   ", None),
        ];
        for (entry, expected) in cases {
            let expected = expected.map(|(host, port)| target(host, port));
            assert_eq!(PingTarget::of(entry), expected, "{entry:?}");
        }
    }

    #[test]
    fn time_has_four_decimal_places_in_milliseconds() {
        assert!((milliseconds(Duration::from_micros(1_234)) - 1.234).abs() < 1e-9);
        assert!((milliseconds(Duration::from_nanos(1_234_567)) - 1.2346).abs() < 1e-9);
        assert!(milliseconds(Duration::ZERO).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn open_port_answers_and_closed_port_does_not() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a local port can be opened");
        let port = listener
            .local_addr()
            .expect("the port has an address")
            .port();

        let open = probe(target("127.0.0.1", Some(port))).await;
        assert!(open.is_some_and(|time| time >= 0.0), "{open:?}");

        drop(listener);
        assert_eq!(probe(target("127.0.0.1", Some(port))).await, None);
    }

    #[tokio::test]
    async fn host_that_does_not_exist_does_not_answer() {
        // The ending `.invalid` never names a host.
        assert_eq!(probe(target("no-such-host.invalid", Some(80))).await, None);
        assert_eq!(probe(target("no-such-host.invalid", None)).await, None);
    }
}
