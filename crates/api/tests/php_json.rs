//! The PHP-compatible JSON writer must print exactly what PHP's `json_encode` prints.
//!
//! The fixture holds the output of real PHP. `parity/php-json/generate.php` creates it.

use serde::Deserialize;

const FIXTURE: &str = include_str!("fixtures/php_json_cases.json");

#[derive(Deserialize)]
struct Fixture {
    php_version: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    input: String,
    expected: String,
}

#[test]
fn output_matches_php_for_every_recorded_case() {
    let fixture: Fixture = serde_json::from_str(FIXTURE).expect("the fixture is valid JSON");
    assert!(!fixture.cases.is_empty(), "the fixture holds no cases");

    let mut failures = Vec::new();
    for case in &fixture.cases {
        let value: serde_json::Value =
            serde_json::from_str(&case.input).expect("the case input is valid JSON");
        let bytes = organizr_api::php_json::to_vec(&value).expect("the value can be encoded");
        let actual = String::from_utf8(bytes).expect("the writer produces UTF-8");
        if actual != case.expected {
            failures.push(format!(
                "case `{}`\n  expected: {:?}\n  actual:   {:?}",
                case.name, case.expected, actual
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} cases differ from PHP {}:\n{}",
        failures.len(),
        fixture.cases.len(),
        fixture.php_version,
        failures.join("\n")
    );
}
