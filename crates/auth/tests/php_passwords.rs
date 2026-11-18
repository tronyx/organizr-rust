//! A password that PHP hashed must check here as it checks in PHP.
//!
//! The fixture holds hashes from PHP's `password_hash()` and, for a list of candidate
//! passwords, what PHP's `password_verify()` answers. `parity/php-password/generate.php`
//! makes it.

use organizr_auth::password::{self, PasswordCheck};
use organizr_domain::PasswordHash;
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    hash: String,
    candidate: String,
    valid: bool,
}

#[test]
fn password_check_agrees_with_php() {
    let fixture: Fixture = serde_json::from_str(include_str!("fixtures/php_password_hashes.json"))
        .expect("the fixture is valid JSON");
    assert!(
        fixture.cases.len() >= 27,
        "the fixture holds {} cases",
        fixture.cases.len()
    );

    for case in &fixture.cases {
        let check = password::verify(&case.candidate, &PasswordHash::new(case.hash.clone()));

        // PHP made every hash of the fixture with bcrypt, which is the old method here.
        let expected = if case.valid {
            PasswordCheck::Correct { outdated: true }
        } else {
            PasswordCheck::Wrong
        };
        assert_eq!(
            check, expected,
            "candidate {:?} against {}",
            case.candidate, case.hash
        );
    }
}

#[test]
fn fixture_covers_the_special_cases() {
    let fixture: Fixture = serde_json::from_str(include_str!("fixtures/php_password_hashes.json"))
        .expect("the fixture is valid JSON");
    let accepted = |wanted: &dyn Fn(&str) -> bool| {
        fixture
            .cases
            .iter()
            .any(|case| case.valid && wanted(&case.candidate))
    };

    assert!(
        accepted(&|candidate| candidate.len() > 72),
        "a password longer than bcrypt reads"
    );
    assert!(
        accepted(&|candidate| !candidate.is_ascii()),
        "a password with characters outside ASCII"
    );
    assert!(accepted(&str::is_empty), "an empty password");
    assert!(
        accepted(&|candidate| candidate.contains('\0')),
        "a candidate with a zero byte"
    );
    assert!(
        fixture.cases.iter().any(|case| !case.valid),
        "a wrong password"
    );
}
