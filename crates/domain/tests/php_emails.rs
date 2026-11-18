//! A text that PHP takes for an e-mail address must be taken for one here, and no other.
//!
//! The fixture holds what PHP's `FILTER_VALIDATE_EMAIL` and `FILTER_SANITIZE_EMAIL` say
//! for a list of texts. `parity/php-email/generate.php` makes it.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_domain::email;
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    input: String,
    valid: bool,
    sanitized: String,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("fixtures/php_emails.json"))
        .expect("the fixture is valid JSON")
}

#[test]
fn address_check_agrees_with_php() {
    let fixture = fixture();
    assert!(
        fixture.cases.len() >= 80,
        "the fixture holds {} cases",
        fixture.cases.len()
    );

    let wrong: Vec<String> = fixture
        .cases
        .iter()
        .filter(|case| email::is_valid(&case.input) != case.valid)
        .map(|case| format!("{:?}: PHP says {}", case.input, case.valid))
        .collect();

    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn clean_up_agrees_with_php() {
    let wrong: Vec<String> = fixture()
        .cases
        .iter()
        .filter(|case| email::sanitize(&case.input) != case.sanitized)
        .map(|case| {
            format!(
                "{:?}: PHP stores {:?}, not {:?}",
                case.input,
                case.sanitized,
                email::sanitize(&case.input)
            )
        })
        .collect();

    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn fixture_has_addresses_of_both_kinds() {
    let fixture = fixture();
    let valid = fixture.cases.iter().filter(|case| case.valid).count();

    assert!(valid >= 25, "{valid} valid addresses");
    assert!(fixture.cases.len() - valid >= 40, "{valid} valid addresses");
}
