//! Runs the real `organizr` binary: `serve` must answer, and `healthcheck` must tell a
//! running server from a missing one.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const ORGANIZR: &str = env!("CARGO_BIN_EXE_organizr");

/// A port that was free a moment ago.
fn free_address() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a local port can be opened");
    listener.local_addr().expect("the port has an address")
}

/// Whether `organizr healthcheck` reports a healthy server at `address`.
fn is_healthy(address: SocketAddr) -> bool {
    Command::new(ORGANIZR)
        .args(["healthcheck", "--listen", &address.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("the binary can be run")
        .success()
}

/// A running `organizr serve`, stopped when the value is dropped.
struct Server(Child);

impl Server {
    fn start(address: SocketAddr, data_dir: &Path) -> Self {
        let child = Command::new(ORGANIZR)
            .args(["serve", "--listen", &address.to_string(), "--data-dir"])
            .arg(data_dir)
            // The test must not pick up settings of the machine it runs on.
            .env_remove("ORGANIZR_DATABASE_URL")
            .env_remove("ORGANIZR_TRUSTED_PROXIES")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("the server can be started");
        Self(child)
    }

    /// Waits until the server answers, for at most the given time.
    fn wait_until_healthy(&mut self, address: SocketAddr, limit: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < limit {
            if let Some(status) = self.0.try_wait().expect("the server can be checked") {
                panic!("the server stopped early with {status}");
            }
            if is_healthy(address) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn serve_answers_and_creates_its_database_in_the_data_folder() {
    let data_dir = tempfile::tempdir().expect("a temporary folder can be created");
    let address = free_address();

    let mut server = Server::start(address, data_dir.path());

    assert!(
        server.wait_until_healthy(address, Duration::from_secs(30)),
        "the server did not answer in time"
    );
    assert!(
        data_dir.path().join("organizr.db").is_file(),
        "the default database is an SQLite file in the data folder"
    );
}

#[test]
fn serve_refuses_a_list_of_trusted_proxies_that_is_not_valid() {
    let data_dir = tempfile::tempdir().expect("a temporary folder can be created");

    let output = Command::new(ORGANIZR)
        .args([
            "serve",
            "--listen",
            &free_address().to_string(),
            "--data-dir",
        ])
        .arg(data_dir.path())
        .args(["--trusted-proxies", "172.18.0.0/16, my-proxy"])
        .env_remove("ORGANIZR_DATABASE_URL")
        .output()
        .expect("the binary can be run");

    assert!(!output.status.success(), "the server must not start");
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(
        message.contains("the list of trusted proxies is not valid")
            && message.contains("`my-proxy` is not an IP address or a network"),
        "the error names the entry: {message}"
    );
}

#[test]
fn healthcheck_fails_when_no_server_listens() {
    let address = free_address();

    assert!(!is_healthy(address));
}

#[test]
fn version_is_the_version_of_the_workspace() {
    let output = Command::new(ORGANIZR)
        .arg("--version")
        .output()
        .expect("the binary can be run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        concat!("organizr ", env!("CARGO_PKG_VERSION"))
    );
}
