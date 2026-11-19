//! The address of a tab must be completed as PHP completes it.
//!
//! The fixture holds what the `qualifyURL` function of the PHP application gives for a
//! list of texts, on real PHP. `parity/php-url/generate.php` makes it.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_api::php_url;
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    /// The host of Organizr in every case.
    host: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    input: String,
    https: bool,
    qualified: String,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("fixtures/php_tab_urls.json"))
        .expect("the fixture is valid JSON")
}

#[test]
fn address_is_completed_as_php_completes_it() {
    let fixture = fixture();
    assert!(
        fixture.cases.len() >= 130,
        "the fixture holds {} cases",
        fixture.cases.len()
    );

    let wrong: Vec<String> = fixture
        .cases
        .iter()
        .filter_map(|case| {
            let qualified = php_url::qualify(&case.input, case.https, &fixture.host);
            (qualified != case.qualified).then(|| {
                format!(
                    "{:?}: PHP gives {:?}, not {:?}",
                    case.input, case.qualified, qualified
                )
            })
        })
        .collect();

    assert!(wrong.is_empty(), "{wrong:#?}");
}
