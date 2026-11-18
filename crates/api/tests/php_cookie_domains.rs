//! The cookie domains must be the ones the PHP application works out.
//!
//! The fixture holds the output of the PHP method `parseDomain`, run in the reference
//! container. `parity/php-cookie-domain/generate.php` creates it.

use organizr_api::cookie_domain::cookie_domains;
use serde::Deserialize;

const FIXTURE: &str = include_str!("fixtures/php_cookie_domains.json");

#[derive(Deserialize)]
struct Fixture {
    php_version: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    host: String,
    wide_domain: String,
    host_domain: String,
}

#[test]
fn domains_match_php_for_every_recorded_host() {
    let fixture: Fixture = serde_json::from_str(FIXTURE).expect("the fixture is valid JSON");
    assert!(!fixture.cases.is_empty(), "the fixture holds no cases");

    let mut failures = Vec::new();
    for case in &fixture.cases {
        let domains = cookie_domains(&case.host);
        if domains.wide != case.wide_domain || domains.host != case.host_domain {
            failures.push(format!(
                "host `{}`\n  expected: wide `{}`, host `{}`\n  actual:   wide `{}`, host `{}`",
                case.host, case.wide_domain, case.host_domain, domains.wide, domains.host
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} hosts differ from PHP {}:\n{}",
        failures.len(),
        fixture.cases.len(),
        fixture.php_version,
        failures.join("\n")
    );
}
