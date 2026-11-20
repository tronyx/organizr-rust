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
            .env_remove("ORGANIZR_SECRET_KEY")
            .env_remove("ORGANIZR_SECRET_KEY_FILE")
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
    // The log also goes to the file of the day in the folder `logs`, as JSON lines.
    let deadline = Instant::now() + Duration::from_secs(10);
    let logged = loop {
        let found = log_files_in(data_dir.path())
            .iter()
            .any(|file| file_holds(file, r#""message":"Organizr is listening""#));
        if found || Instant::now() > deadline {
            break found;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(logged, "the log file has the line of the start");
}

/// The log files in the folder `logs` of a data folder.
fn log_files_in(data_dir: &Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(data_dir.join("logs")).map_or_else(
        |_| Vec::new(),
        |entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.extension().is_some_and(|extension| extension == "log")
                        && path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.starts_with("organizr-"))
                })
                .collect()
        },
    )
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

/// Sends a GET request with the API key and returns the whole answer as text.
fn get_with_key(address: SocketAddr, path: &str, key: &str) -> String {
    let mut stream = TcpStream::connect(address).expect("the server accepts a connection");
    let head = [
        format!("GET {path} HTTP/1.1"),
        format!("Host: {address}"),
        format!("Token: {key}"),
        "Connection: close".to_owned(),
    ]
    .join("\r\n");
    write!(stream, "{head}\r\n\r\n").expect("the request can be sent");
    let mut answer = String::new();
    stream
        .read_to_string(&mut answer)
        .expect("the answer can be read");
    answer
}

/// Whether a file of the data folder holds the text. A missing file holds nothing.
fn file_holds(file: &Path, text: &str) -> bool {
    std::fs::read(file).is_ok_and(|bytes| {
        bytes
            .windows(text.len())
            .any(|window| window == text.as_bytes())
    })
}

#[test]
fn stored_secrets_need_the_key_file_of_the_install() {
    const API_KEY: &str = "abcdefghij0123456789";
    let data_dir = tempfile::tempdir().expect("a temporary folder can be created");
    let key_file = data_dir.path().join("secret.key");
    let log = data_dir.path().join("first.log");

    // The first start makes the key file. The set-up stores the API key, which is a secret.
    let address = free_address();
    let mut server = Server::start_logging_to(address, data_dir.path(), &log);
    assert!(server.wait_until_healthy(address, Duration::from_secs(30)));
    let key = std::fs::read_to_string(&key_file).expect("the first start writes the key file");
    assert_eq!(key.trim().len(), 64, "the key file holds 64 hex digits");
    let code = set_up_code_in(&log).expect("the log of a new install has a code");
    let body = format!(
        r#"{{"setupCode":"{code}","license":"personal","api":"{API_KEY}","registrationPassword":"let me in","username":"robin","password":"the password of robin","email":"robin@example.com"}}"#
    );
    let accepted = post_json(address, "/api/v2/wizard", &body);
    assert!(accepted.starts_with("HTTP/1.1 200"), "{accepted}");
    let with_key = get_with_key(address, "/api/v2/tabs", API_KEY);
    assert!(with_key.starts_with("HTTP/1.1 200"), "{with_key}");
    drop(server);

    // Neither the database nor its journal holds the API key as it was entered.
    for name in ["organizr.db", "organizr.db-wal"] {
        assert!(
            !file_holds(&data_dir.path().join(name), API_KEY),
            "{name} holds the API key in plain text"
        );
    }

    // The next start reads the same key file, so the stored API key still works.
    let address = free_address();
    let mut server = Server::start(address, data_dir.path());
    assert!(server.wait_until_healthy(address, Duration::from_secs(30)));
    assert_eq!(
        std::fs::read_to_string(&key_file).expect("the key file is still there"),
        key
    );
    let with_key = get_with_key(address, "/api/v2/tabs", API_KEY);
    assert!(with_key.starts_with("HTTP/1.1 200"), "{with_key}");
    drop(server);

    // Without the key file, the server makes a new key. The stored secrets cannot be
    // opened with it: the server says so in its log, and the API key does not work.
    std::fs::remove_file(&key_file).expect("the key file can be removed");
    let log = data_dir.path().join("third.log");
    let address = free_address();
    let mut server = Server::start_logging_to(address, data_dir.path(), &log);
    assert!(server.wait_until_healthy(address, Duration::from_secs(30)));
    let with_key = get_with_key(address, "/api/v2/tabs", API_KEY);
    assert!(with_key.starts_with("HTTP/1.1 401"), "{with_key}");
    let text = std::fs::read_to_string(&log).expect("the log can be read");
    assert!(
        text.contains("the stored secret `organizrAPI` cannot be opened"),
        "{text}"
    );
    assert!(!text.contains(API_KEY), "the log holds the API key");
    let files = log_files_in(data_dir.path());
    assert!(!files.is_empty(), "the log files are written");
    for file in files {
        assert!(
            !file_holds(&file, API_KEY),
            "{} holds the API key",
            file.display()
        );
    }
}

#[test]
fn new_install_is_set_up_with_the_code_from_the_log() {
    let data_dir = tempfile::tempdir().expect("a temporary folder can be created");
    let first_log = data_dir.path().join("first.log");
    let second_log = data_dir.path().join("second.log");
    let third_log = data_dir.path().join("third.log");
    let set_up = |code: &str| {
        format!(
            r#"{{"setupCode":"{code}","license":"personal","api":"abcdefghij0123456789","registrationPassword":"let me in","username":"robin","password":"the password of robin","email":"robin@example.com"}}"#
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

#[test]
fn serve_with_a_key_from_the_environment_writes_no_key_file() {
    let data_dir = tempfile::tempdir().expect("a temporary folder can be created");
    let address = free_address();
    let child = Command::new(ORGANIZR)
        .args(["serve", "--listen", &address.to_string(), "--data-dir"])
        .arg(data_dir.path())
        .env_remove("ORGANIZR_DATABASE_URL")
        .env_remove("ORGANIZR_SECRET_KEY_FILE")
        .env("ORGANIZR_SECRET_KEY", "ab".repeat(32))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the server can be started");
    let mut server = Server(child);

    assert!(server.wait_until_healthy(address, Duration::from_secs(30)));
    assert!(data_dir.path().join("organizr.db").is_file());
    assert!(!data_dir.path().join("secret.key").exists());
}

#[test]
fn serve_refuses_a_key_that_is_no_key() {
    let data_dir = tempfile::tempdir().expect("a temporary folder can be created");
    let errors = data_dir.path().join("errors.log");
    let file = File::create(&errors).expect("the file can be created");
    let child = Command::new(ORGANIZR)
        .args(["serve", "--listen", "127.0.0.1:0", "--data-dir"])
        .arg(data_dir.path())
        .env_remove("ORGANIZR_DATABASE_URL")
        .env("ORGANIZR_SECRET_KEY", "not a key")
        .stdout(Stdio::null())
        .stderr(Stdio::from(file))
        .spawn()
        .expect("the binary can be run");
    // The server is stopped at the end, also when it did not stop by itself.
    let mut server = Server(child);

    let start = Instant::now();
    let status = loop {
        if let Some(status) = server.0.try_wait().expect("the server can be checked") {
            break status;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "the server did not stop"
        );
        std::thread::sleep(Duration::from_millis(100));
    };

    assert!(!status.success());
    let message = std::fs::read_to_string(&errors).expect("the errors can be read");
    assert!(
        message.contains("ORGANIZR_SECRET_KEY is not a key"),
        "{message}"
    );
}

/// The made-up PHP install of the importer tests: its configuration file and its
/// database as SQL text.
const LEGACY_CONFIG: &str = include_str!("../../import/tests/fixtures/legacy/config.php");
const LEGACY_DATABASE: &str = include_str!("../../import/tests/fixtures/legacy/organizr.sql");

/// Writes the PHP install into a folder, as the PHP application lays it out.
fn write_legacy_install(folder: &Path) {
    use sea_orm::ConnectionTrait;

    let config = folder.join("data/config/config.php");
    let database = folder.join("data/parity-db/parity.db");
    for file in [&config, &database] {
        std::fs::create_dir_all(file.parent().expect("the file is in a folder"))
            .expect("the folder can be made");
    }
    std::fs::write(&config, LEGACY_CONFIG).expect("the configuration file can be written");
    let url = format!(
        "sqlite:{}?mode=rwc",
        database.display().to_string().replace('\\', "/")
    );
    tokio::runtime::Runtime::new()
        .expect("a runtime can be made")
        .block_on(async {
            let connection = sea_orm::Database::connect(url)
                .await
                .expect("the file can be made");
            connection
                .execute_unprepared(LEGACY_DATABASE)
                .await
                .expect("the statements run");
            connection.close().await.expect("the file can be closed");
        });
}

/// Runs `organizr import` and returns whether it succeeded, its output and its errors.
fn import(arguments: &[&str], from: &Path, data_dir: &Path) -> (bool, String, String) {
    let output = Command::new(ORGANIZR)
        .arg("import")
        .args(arguments)
        .arg("--from")
        .arg(from)
        .arg("--data-dir")
        .arg(data_dir)
        .env_remove("ORGANIZR_DATABASE_URL")
        .env_remove("ORGANIZR_SECRET_KEY")
        .env_remove("ORGANIZR_SECRET_KEY_FILE")
        .env("NO_COLOR", "1")
        .output()
        .expect("the command can be run");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn import_moves_a_php_install_into_a_new_data_folder() {
    let legacy = tempfile::tempdir().expect("a temporary folder can be made");
    write_legacy_install(legacy.path());
    let home = tempfile::tempdir().expect("a temporary folder can be made");
    let data_dir = home.path().join("data");

    // A dry run prints the report and creates nothing.
    let (succeeded, output, errors) = import(&["--dry-run"], legacy.path(), &data_dir);
    assert!(succeeded, "{errors}");
    assert!(
        output.starts_with("Dry run: nothing was written."),
        "{output}"
    );
    assert!(output.contains("     4  users"), "{output}");
    assert!(!data_dir.exists(), "a dry run made the data folder");

    let (succeeded, output, errors) = import(&[], legacy.path(), &data_dir);
    assert!(succeeded, "{errors}");
    assert!(output.starts_with("The import is complete."), "{output}");
    assert!(data_dir.join("organizr.db").is_file());
    assert!(data_dir.join("secret.key").is_file());
    // The secrets of the PHP install are sealed in the new database.
    for file in ["organizr.db", "organizr.db-wal"] {
        assert!(!file_holds(
            &data_dir.join(file),
            "made-up torrent password"
        ));
    }

    // The server starts on the imported install: it is set up, the API key of the PHP
    // install works, and the users are there.
    let address = free_address();
    let log = home.path().join("server.log");
    let mut server = Server::start_logging_to(address, &data_dir, &log);
    assert!(server.wait_until_healthy(address, Duration::from_secs(30)));
    let users = get_with_key(address, "/api/v2/users", "parityapikey01234567");
    assert!(users.starts_with("HTTP/1.1 200"), "{users}");
    for name in ["admin", "robin", "pat", "Sam"] {
        assert!(
            users.contains(&format!("\"username\": \"{name}\"")),
            "{users}"
        );
    }
    assert_eq!(set_up_code_in(&log), None, "an imported install is set up");
    drop(server);

    // A second import into the same data folder is refused.
    let (succeeded, _, errors) = import(&[], legacy.path(), &data_dir);
    assert!(!succeeded);
    assert!(errors.contains("the new database is not empty"), "{errors}");
}

#[test]
fn import_from_a_folder_without_an_install_creates_nothing() {
    let empty = tempfile::tempdir().expect("a temporary folder can be made");
    let home = tempfile::tempdir().expect("a temporary folder can be made");
    let data_dir = home.path().join("data");

    let (succeeded, output, errors) = import(&[], empty.path(), &data_dir);

    assert!(!succeeded);
    assert_eq!(output, "");
    assert!(errors.contains("no Organizr install was found"), "{errors}");
    assert!(!data_dir.exists(), "the failed import made the data folder");
}

#[test]
fn check_of_an_install_without_a_plex_token_asks_nothing_and_succeeds() {
    let data_dir = tempfile::tempdir().expect("a temporary folder can be made");

    let output = Command::new(ORGANIZR)
        .args(["check", "--data-dir"])
        .arg(data_dir.path())
        .env_remove("ORGANIZR_DATABASE_URL")
        .env_remove("ORGANIZR_LOG")
        .env_remove("ORGANIZR_SECRET_KEY")
        .env_remove("ORGANIZR_SECRET_KEY_FILE")
        .output()
        .expect("the binary can be run");

    let report = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{report}");
    assert_eq!(
        report.trim_end(),
        "skipped  plex.tv knows the stored Plex token: the settings hold no Plex token"
    );
}

/// Sends a GET request with more header lines, and returns the whole answer as text.
fn get_with_headers(address: SocketAddr, path: &str, headers: &[&str]) -> String {
    let mut stream = TcpStream::connect(address).expect("the server accepts a connection");
    let mut head = vec![
        format!("GET {path} HTTP/1.1"),
        format!("Host: {address}"),
        "Connection: close".to_owned(),
    ];
    head.extend(headers.iter().map(|line| (*line).to_owned()));
    write!(stream, "{}\r\n\r\n", head.join("\r\n")).expect("the request can be sent");
    let mut answer = String::new();
    stream
        .read_to_string(&mut answer)
        .expect("the answer can be read");
    answer
}

/// The value of the header `X-Request-Id` of an answer.
fn request_id_of(answer: &str) -> String {
    answer
        .lines()
        .take_while(|line| !line.is_empty())
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("x-request-id")
                .then(|| value.trim().to_owned())
        })
        .unwrap_or_else(|| panic!("the answer has no X-Request-Id: {answer}"))
}

#[test]
fn each_answer_has_a_request_id_that_its_log_lines_carry() {
    let data_dir = tempfile::tempdir().expect("a temporary folder can be created");
    let output = data_dir.path().join("output.log");
    let address = free_address();
    let mut server = Server::start_logging_to(address, data_dir.path(), &output);
    assert!(server.wait_until_healthy(address, Duration::from_secs(30)));

    // A file of the web interface, an answer of the API, a refusal and "not found".
    let page = get_with_headers(address, "/", &[]);
    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    let refused = get_with_headers(address, "/api/v2/tabs", &[]);
    assert!(refused.starts_with("HTTP/1.1 401"), "{refused}");
    let missing = get_with_headers(address, "/api/v2/no-such-route", &[]);
    assert!(missing.starts_with("HTTP/1.1 404"), "{missing}");
    let ids = [&page, &refused, &missing].map(|answer| request_id_of(answer));
    for id in &ids {
        assert_eq!(id.len(), 36, "{id}");
    }
    assert!(ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2]);

    // The id of a client is not used. This request writes a warning to the log: the
    // address in X-Forwarded-For comes from no trusted proxy.
    let answer = get_with_headers(
        address,
        "/api/v2/status",
        &[
            "X-Request-Id: injected-by-the-client",
            "X-Forwarded-For: 203.0.113.9",
        ],
    );
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    let id = request_id_of(&answer);
    assert_ne!(id, "injected-by-the-client");

    let in_the_file = format!(r#""request_id":"{id}""#);
    let deadline = Instant::now() + Duration::from_secs(10);
    let found = loop {
        let line = log_files_in(data_dir.path()).iter().find_map(|file| {
            std::fs::read_to_string(file)
                .ok()?
                .lines()
                .find(|line| line.contains(&in_the_file))
                .map(str::to_owned)
        });
        if line.is_some() || Instant::now() > deadline {
            break line;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let line = found.expect("the log file has the warning of the request");
    assert!(line.contains("X-Forwarded-For"), "{line}");
    drop(server);
    let text = std::fs::read_to_string(&output).expect("the output can be read");
    assert!(text.contains(&format!("request_id={id}")), "{text}");
    assert!(!text.contains("injected-by-the-client"), "{text}");
    for file in log_files_in(data_dir.path()) {
        assert!(!file_holds(&file, "injected-by-the-client"));
    }
}
