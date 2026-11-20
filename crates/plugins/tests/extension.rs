//! The host of extensions, with two real extensions: the template of the examples, and
//! one that tries what an extension must not be able to do.
//!
//! The tests build both extensions for WebAssembly, so they need the target
//! `wasm32-wasip2` of the Rust toolchain.

#![cfg(feature = "wasm-plugins")]
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use organizr_plugins::extension::{
    Call, Caller, ExtensionError, Extensions, Outcome, RESPONSE_LIMIT, Request, STORAGE_LIMIT,
};

/// The folder of this crate.
const CRATE: &str = env!("CARGO_MANIFEST_DIR");

/// Builds an extension and returns the bytes of its component.
fn build(folder: &Path, name: &str) -> Vec<u8> {
    let target = Path::new(CRATE).join("../../target/extension-guests");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-wasip2",
            "--manifest-path",
        ])
        .arg(folder.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        // The flags of the build of the tests are not for WebAssembly.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_BUILD_TARGET")
        .output()
        .expect("cargo can be started");
    assert!(
        output.status.success(),
        "the extension {name} cannot be built: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let file = target.join(format!("wasm32-wasip2/release/{name}.wasm"));
    std::fs::read(&file).unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()))
}

fn template() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let folder = PathBuf::from(CRATE).join("../../examples/extension-template");
        build(&folder, "extension_template")
    })
}

fn hostile() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let folder = PathBuf::from(CRATE).join("tests/guests/hostile");
        build(&folder, "hostile_extension")
    })
}

fn host() -> &'static Extensions {
    static HOST: OnceLock<Extensions> = OnceLock::new();
    HOST.get_or_init(|| Extensions::new().expect("the runtime starts"))
}

fn request(method: &str, path: &str) -> Request {
    Request {
        method: method.to_owned(),
        path: path.to_owned(),
        query: String::new(),
        headers: Vec::new(),
        body: Vec::new(),
        caller: Caller {
            username: Some("robin".to_owned()),
            level: 4,
        },
    }
}

fn call(can_store: bool, values: BTreeMap<String, String>) -> Call {
    Call {
        extension: "test".to_owned(),
        can_store,
        values,
    }
}

fn ask_template(call: Call, request: Request) -> Outcome {
    host()
        .handle(template(), "template", call, request)
        .expect("the template answers")
}

/// Asks the hostile extension, with or without the permission to store values.
fn ask_hostile(path: &str, can_store: bool) -> Result<Outcome, ExtensionError> {
    host().handle(
        hostile(),
        "hostile",
        call(can_store, BTreeMap::new()),
        request("GET", path),
    )
}

fn text(outcome: &Outcome) -> String {
    String::from_utf8_lossy(&outcome.response.body).into_owned()
}

#[test]
fn template_answers_its_routes_with_what_the_request_holds() {
    let outcome = ask_template(call(false, BTreeMap::new()), request("GET", "/hello"));
    assert_eq!(outcome.response.status, 200);
    assert_eq!(text(&outcome), "Hello, robin!");
    assert_eq!(
        outcome.response.headers,
        [(
            "content-type".to_owned(),
            "text/plain; charset=utf-8".to_owned()
        )]
    );
    assert_eq!(outcome.values, None);

    let mut guest = request("GET", "/hello");
    guest.caller = Caller {
        username: None,
        level: 999,
    };
    assert_eq!(text(&ask_template(Call::default(), guest)), "Hello, guest!");

    let mut echo = request("GET", "/echo");
    echo.query = "a=1&b=2".to_owned();
    echo.body = b"some text".to_vec();
    assert_eq!(
        text(&ask_template(Call::default(), echo)),
        "GET /echo?a=1&b=2 level=4 body=some text"
    );

    let missing = ask_template(Call::default(), request("GET", "/nothing"));
    assert_eq!(missing.response.status, 404);
}

#[test]
fn values_are_stored_only_with_the_permission() {
    // Each call starts with the values that the caller hands in, and gives back the
    // values when it changed them.
    let first = ask_template(call(true, BTreeMap::new()), request("POST", "/count"));
    assert_eq!(text(&first), "1");
    let values = first.values.expect("the call changed the values");
    assert_eq!(values.get("count").map(String::as_str), Some("1"));
    let second = ask_template(call(true, values), request("POST", "/count"));
    assert_eq!(text(&second), "2");

    // Without the permission, nothing is stored and nothing is read.
    let stored = BTreeMap::from([("count".to_owned(), "41".to_owned())]);
    let refused = ask_template(call(false, stored), request("POST", "/count"));
    assert_eq!(refused.response.status, 500);
    assert!(
        text(&refused).contains("no permission"),
        "{}",
        text(&refused)
    );
    assert_eq!(refused.values, None);
}

#[test]
fn storage_has_a_limit_and_values_can_be_deleted() {
    let filled = ask_hostile("/fill", true).expect("the extension answers");
    // Ten values of 100,000 bytes fit into the limit, with their keys. The next one
    // does not.
    assert!(
        text(&filled).starts_with("10 stored; "),
        "{}",
        text(&filled)
    );
    assert!(text(&filled).contains("at most"), "{}", text(&filled));
    let values = filled.values.expect("the call changed the values");
    let size: usize = values
        .iter()
        .map(|(key, value)| key.len() + value.len())
        .sum();
    assert!(size <= STORAGE_LIMIT, "{size}");

    let read = host()
        .handle(
            hostile(),
            "hostile",
            call(true, values.clone()),
            request("GET", "/read"),
        )
        .expect("the extension answers");
    assert_eq!(text(&read), "Some(100000)");
    assert_eq!(read.values, None);
    let hidden = host()
        .handle(
            hostile(),
            "hostile",
            call(false, values.clone()),
            request("GET", "/read"),
        )
        .expect("the extension answers");
    assert_eq!(text(&hidden), "None");

    let forgot = host()
        .handle(
            hostile(),
            "hostile",
            call(true, values),
            request("GET", "/forget"),
        )
        .expect("the extension answers");
    let left = forgot.values.expect("the call changed the values");
    assert_eq!(left.len(), 9);
    assert!(!left.contains_key("key0"));
}

#[test]
fn call_that_works_without_an_end_is_stopped() {
    let stopped = ask_hostile("/spin", false);
    assert!(
        matches!(stopped, Err(ExtensionError::Stopped(_))),
        "{stopped:?}"
    );
    // The next call works: each call has a sandbox of its own.
    let after = ask_hostile("/environment", false).expect("the extension answers");
    assert_eq!(after.response.status, 200);
}

#[test]
fn call_that_takes_too_much_memory_or_gives_up_is_stopped() {
    for path in ["/grow", "/panic"] {
        let stopped = ask_hostile(path, false);
        assert!(
            matches!(stopped, Err(ExtensionError::Stopped(_))),
            "{path}: {stopped:?}"
        );
    }
    let big = ask_hostile("/big", false);
    assert!(
        matches!(big, Err(ExtensionError::AnswerTooLarge)),
        "{big:?}"
    );
    const { assert!(RESPONSE_LIMIT < 9 * 1024 * 1024) };
}

#[test]
fn extension_has_no_files_no_network_and_no_environment() {
    let files = ask_hostile("/files", false).expect("the extension answers");
    assert!(text(&files).starts_with("no folder: "), "{}", text(&files));
    let write = ask_hostile("/write", false).expect("the extension answers");
    assert!(
        text(&write).starts_with("not written: "),
        "{}",
        text(&write)
    );
    assert!(!Path::new("/tmp/from-the-extension.txt").exists());
    let network = ask_hostile("/network", false).expect("the extension answers");
    assert!(
        text(&network).starts_with("no connection: "),
        "{}",
        text(&network)
    );
    let environment = ask_hostile("/environment", false).expect("the extension answers");
    assert_eq!(text(&environment), "0 variables, 0 arguments");
    // A flood of log lines does not stop the call.
    let flood = ask_hostile("/flood", false).expect("the extension answers");
    assert_eq!(text(&flood), "flooded");
}

#[test]
fn file_that_is_no_extension_is_refused() {
    for (name, bytes) in [
        ("text", b"no WebAssembly".as_slice()),
        // A module of WebAssembly, which is no component.
        ("module", b"\0asm\x01\0\0\0".as_slice()),
        ("empty", b"".as_slice()),
    ] {
        let refused = host().check(bytes, name);
        assert!(
            matches!(refused, Err(ExtensionError::NotAnExtension(_))),
            "{name}: {refused:?}"
        );
        let refused = host().handle(bytes, name, Call::default(), request("GET", "/"));
        assert!(
            matches!(refused, Err(ExtensionError::NotAnExtension(_))),
            "{name}: {refused:?}"
        );
    }
    host()
        .check(template(), "template")
        .expect("the template is an extension");
    host()
        .check(hostile(), "hostile")
        .expect("the hostile one is an extension too");
}
