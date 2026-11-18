//! Runs the real `organizr` binary: `serve` must answer, and `healthcheck` must tell a
//! running server from a missing one.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::fs::File;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
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
        Self::start_with_output(address, data_dir, Stdio::null())
    }

    /// Starts the server and writes its log to a file.
    fn start_logging_to(address: SocketAddr, data_dir: &Path, log: &Path) -> Self {
        let file = File::create(log).expect("the log file can be created");
        Self::start_with_output(address, data_dir, Stdio::from(file))
    }

    fn start_with_output(address: SocketAddr, data_dir: &Path, output: Stdio) -> Self {
        let child = Command::new(ORGANIZR)
            .args(["serve", "--listen", &address.to_string(), "--data-dir"])
            .arg(data_dir)
            // The test must not pick up settings of the machine it runs on.
            .env_remove("ORGANIZR_DATABASE_URL")
            .env_remove("ORGANIZR_TRUSTED_PROXIES")
            .env_remove("ORGANIZR_LOG")
            // Plain text in the log, without colour codes.
            .env("NO_COLOR", "1")
            .stdout(output)
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

/// The set-up code in a server log, when the log has one.
fn set_up_code_in(log: &Path) -> Option<String> {
    let text = std::fs::read_to_string(log).expect("the log can be read");
    let (_, after) = text.split_once("the set-up code ")?;
    Some(after.chars().take(14).collect())
}

/// Sends a JSON body with POST and returns the whole answer as text.
fn post_json(address: SocketAddr, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(address).expect("the server accepts a connection");
    let head = [
        format!("POST {path} HTTP/1.1"),
        format!("Host: {address}"),
        "Content-Type: application/json".to_owned(),
        format!("Content-Length: {}", body.len()),
        "Connection: close".to_owned(),
    ]
    .join("\r\n");
    write!(stream, "{head}\r\n\r\n{body}").expect("the request can be sent");
    let mut answer = String::new();
    stream
        .read_to_string(&mut answer)
        .expect("the answer can be read");
    answer
}

#[test]
fn new_install_is_set_up_with_the_code_from_the_log() {
    let data_dir = tempfile::tempdir().expect("a temporary folder can be created");
    let first_log = data_dir.path().join("first.log");
    let second_log = data_dir.path().join("second.log");
    let third_log = data_dir.path().join("third.log");
    let set_up = |code: &str| {
        format!(
            r#"{{"setupCode":"{code}","license":"personal","api":"abcdefghij0123456789","registrationPassword":"let me in","username":"chris","password":"the password of chris","email":"chris@example.com"}}"#
        )
    };

    // A new install writes a set-up code to its log.
    let address = free_address();
    let mut server = Server::start_logging_to(address, data_dir.path(), &first_log);
    assert!(server.wait_until_healthy(address, Duration::from_secs(30)));
    let first_code = set_up_code_in(&first_log).expect("the log of a new install has a code");
    assert_eq!(first_code.len(), 14, "{first_code}");
    assert_eq!(first_code.matches('-').count(), 2, "{first_code}");
    drop(server);

    // The next start makes another code. The old one no longer works.
    let address = free_address();
    let mut server = Server::start_logging_to(address, data_dir.path(), &second_log);
    assert!(server.wait_until_healthy(address, Duration::from_secs(30)));
    let second_code = set_up_code_in(&second_log).expect("the install is still not set up");
    assert_ne!(second_code, first_code);
    let refused = post_json(address, "/api/v2/wizard", &set_up(&first_code));
    assert!(refused.starts_with("HTTP/1.1 403"), "{refused}");

    // The code of this start sets the install up.
    let accepted = post_json(address, "/api/v2/wizard", &set_up(&second_code));
    assert!(accepted.starts_with("HTTP/1.1 200"), "{accepted}");
    assert!(accepted.contains("\"data\": true"), "{accepted}");
    let again = post_json(address, "/api/v2/wizard", &set_up(&second_code));
    assert!(again.starts_with("HTTP/1.1 401"), "{again}");
    drop(server);

    // An install that is set up writes no code.
    let address = free_address();
    let mut server = Server::start_logging_to(address, data_dir.path(), &third_log);
    assert!(server.wait_until_healthy(address, Duration::from_secs(30)));
    assert_eq!(set_up_code_in(&third_log), None);
    let closed = post_json(address, "/api/v2/wizard", &set_up(&second_code));
    assert!(closed.starts_with("HTTP/1.1 401"), "{closed}");
}
