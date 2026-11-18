//! The `healthcheck` command: asks a running server for `/api/v2/ping`.
//!
//! The container image has no shell and no `curl`, so the image's health check runs this
//! command. It speaks plain HTTP over a socket and needs no HTTP client library.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use anyhow::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// How long the server has to accept the connection and to answer.
const TIME_LIMIT: Duration = Duration::from_secs(5);

const REQUEST: &[u8] = b"GET /api/v2/ping HTTP/1.0\r\nHost: localhost\r\n\r\n";

/// The options of `organizr healthcheck`.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// The address and port the server listens on. The same option and environment
    /// variable as for `serve`, so the check needs no configuration of its own.
    #[arg(long, env = "ORGANIZR_LISTEN", default_value = "0.0.0.0:8080")]
    listen: SocketAddr,
}

/// Succeeds when the server answers the ping with status 200.
pub async fn run(args: &Args) -> anyhow::Result<()> {
    let target = reachable(args.listen);

    let mut stream = timeout(TIME_LIMIT, TcpStream::connect(target))
        .await
        .with_context(|| format!("no answer from {target} in time"))?
        .with_context(|| format!("cannot connect to {target}"))?;
    stream
        .write_all(REQUEST)
        .await
        .context("cannot send the request")?;

    let mut answer = Vec::new();
    timeout(TIME_LIMIT, stream.read_to_end(&mut answer))
        .await
        .context("the answer did not arrive in time")?
        .context("cannot read the answer")?;

    let answer = String::from_utf8_lossy(&answer);
    let status_line = answer.lines().next().unwrap_or_default();
    anyhow::ensure!(
        is_success(status_line),
        "the server answered `{status_line}`"
    );
    Ok(())
}

/// The address to connect to for a server that listens on `listen`.
///
/// A server that listens on every address (`0.0.0.0`) is reached on this machine.
fn reachable(listen: SocketAddr) -> SocketAddr {
    if listen.ip().is_unspecified() {
        let local: IpAddr = match listen {
            SocketAddr::V4(_) => Ipv4Addr::LOCALHOST.into(),
            SocketAddr::V6(_) => Ipv6Addr::LOCALHOST.into(),
        };
        SocketAddr::new(local, listen.port())
    } else {
        listen
    }
}

/// Whether an HTTP status line reports status 200.
fn is_success(status_line: &str) -> bool {
    let mut parts = status_line.split(' ');
    let protocol = parts.next().unwrap_or_default();
    let status = parts.next().unwrap_or_default();
    protocol.starts_with("HTTP/1.") && status == "200"
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::{is_success, reachable};

    fn address(text: &str) -> SocketAddr {
        text.parse().expect("the address is valid")
    }

    #[test]
    fn server_on_every_address_is_reached_on_this_machine() {
        assert_eq!(
            reachable(address("0.0.0.0:8080")),
            address("127.0.0.1:8080")
        );
        assert_eq!(reachable(address("[::]:8080")), address("[::1]:8080"));
    }

    #[test]
    fn server_on_one_address_is_reached_there() {
        assert_eq!(
            reachable(address("192.0.2.10:9000")),
            address("192.0.2.10:9000")
        );
    }

    #[test]
    fn only_status_200_is_healthy() {
        assert!(is_success("HTTP/1.1 200 OK"));
        assert!(is_success("HTTP/1.0 200 OK"));
        assert!(!is_success("HTTP/1.1 404 Not Found"));
        assert!(!is_success("HTTP/1.1 500 Internal Server Error"));
        assert!(!is_success("HTTP/1.1 2000 Odd"));
        assert!(!is_success("SSH-2.0-OpenSSH"));
        assert!(!is_success(""));
    }
}
