//! The second factor of a login: a code that changes every 30 seconds.
//!
//! The PHP application calls the one kind it has `google`. It is the time-based code of
//! RFC 6238 that authenticator apps show: six digits, SHA-1, a new code every 30
//! seconds. The user and the server share a secret, which is written in Base32.
//!
//! PHP Organizr takes the code of the current 30 seconds and of the five steps before
//! and after it, so a clock that is two minutes off still works. This check does the
//! same.
//!
//! PHP takes a code again and again while it is in that window. Here a login remembers
//! the step of the code it took, and the next login of the account needs the code of a
//! later step. So a code that somebody saw does not work a second time.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use thiserror::Error;
use totp_rs::{Algorithm, Builder, Secret, Totp};

/// The kind of second factor, as the PHP application names it.
pub const KIND: &str = "google";

/// How many steps of 30 seconds before and after the current one are accepted.
pub const STEPS_AROUND: u16 = 5;

/// The seconds that one code lasts.
pub const STEP_SECONDS: u64 = 30;

/// Why a second factor cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TwoFactorError {
    /// The secret is not Base32 text.
    #[error("the secret of the second factor is not Base32 text")]
    InvalidSecret,
    /// The picture for the authenticator app cannot be made.
    #[error("the QR code cannot be made: {0}")]
    Picture(String),
}

/// A new random secret, in Base32.
pub fn new_secret() -> String {
    Secret::generate().to_base32()
}

fn generator(secret: &str, issuer: &str, account: &str) -> Result<Totp, TwoFactorError> {
    // An authenticator app shows the secret in groups, and a person can type it so.
    let compact: String = secret
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    if compact.is_empty() {
        return Err(TwoFactorError::InvalidSecret);
    }
    let secret = Secret::try_from_base32(compact.to_uppercase())
        .map_err(|_| TwoFactorError::InvalidSecret)?;
    // The secrets of the PHP application have 80 bits, which is less than the RFC asks
    // for. They must keep working, so the checks of the builder are not used.
    Ok(Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_digits(6)
        .with_skew(STEPS_AROUND)
        .with_step_duration(STEP_SECONDS)
        .with_secret(secret)
        .with_issuer(Some(issuer.replace(':', " ")))
        .with_account_name(account.replace(':', " "))
        .build_noncompliant())
}

/// Whether a text is a secret that a code can be made from.
pub fn is_secret(secret: &str) -> bool {
    generator(secret, "", "").is_ok()
}

/// Whether `code` is the code of the secret at the time `now`, in seconds since 1970,
/// or of one of the steps around it. Space around the code does not count.
pub fn verify(secret: &str, code: &str, now: u64) -> Result<bool, TwoFactorError> {
    Ok(step_of(secret, code, now)?.is_some())
}

/// The step of 30 seconds whose code `code` is, when it is one of the steps that
/// [`verify`] takes at the time `now`.
pub fn step_of(secret: &str, code: &str, now: u64) -> Result<Option<u64>, TwoFactorError> {
    let generator = generator(secret, "", "")?;
    Ok(generator.check(code.trim(), now))
}

/// The last step that a login took, for each second factor.
static LAST_STEPS: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

/// Takes the code of `step` for the second factor that `factor` names, once. Returns
/// `false` when a login already took the code of this step or of a later one.
///
/// `factor` must name the account and its secret, so a new secret starts again. The
/// steps live in memory, so a restart forgets them. A restart takes longer than the
/// window of a code, so that does not matter much.
pub(crate) fn take_step(factor: &str, step: u64, now: u64) -> bool {
    let mut guard = LAST_STEPS.lock().unwrap_or_else(PoisonError::into_inner);
    let steps = guard.get_or_insert_with(HashMap::new);
    // A step before the window blocks no code that the window takes.
    let oldest = (now / STEP_SECONDS).saturating_sub(u64::from(STEPS_AROUND));
    steps.retain(|_, last| *last >= oldest);
    match steps.get(factor) {
        Some(last) if step <= *last => false,
        _ => {
            steps.insert(factor.to_owned(), step);
            true
        }
    }
}

/// The code of the secret at a time, in seconds since 1970.
pub fn code_at(secret: &str, time: u64) -> Result<String, TwoFactorError> {
    Ok(generator(secret, "", "")?.generate(time).to_string())
}

/// A QR code that an authenticator app reads to learn the secret, as a `data:` address
/// of a PNG picture. The app shows `issuer` and `account` next to the code.
pub fn picture(secret: &str, issuer: &str, account: &str) -> Result<String, TwoFactorError> {
    let png = generator(secret, issuer, account)?
        .to_qr_base64()
        .map_err(|error| TwoFactorError::Picture(error.to_string()))?;
    Ok(format!("data:image/png;base64,{png}"))
}

#[cfg(test)]
mod tests {
    use super::{
        STEP_SECONDS, TwoFactorError, code_at, is_secret, new_secret, picture, step_of, take_step,
        verify,
    };

    /// The secret of the test vectors of RFC 6238, "12345678901234567890", in Base32.
    const RFC_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn codes_are_those_of_the_rfc() {
        // The RFC lists codes of eight digits. The code of six digits is their end.
        for (time, code) in [
            (59, "287082"),
            (1_111_111_109, "081804"),
            (1_111_111_111, "050471"),
            (1_234_567_890, "005924"),
            (2_000_000_000, "279037"),
            (20_000_000_000, "353130"),
        ] {
            assert_eq!(code_at(RFC_SECRET, time).as_deref(), Ok(code), "{time}");
        }
    }

    #[test]
    fn code_counts_for_five_steps_before_and_after_its_own() {
        let now = 1_234_567_890;
        let code_of_step = |steps: i64| {
            let time = u64::try_from(now + steps * 30).expect("the time is positive");
            code_at(RFC_SECRET, time).expect("the secret is valid")
        };
        let now = u64::try_from(now).expect("the time is positive");

        for steps in [-5, -1, 0, 1, 5] {
            assert_eq!(
                verify(RFC_SECRET, &code_of_step(steps), now),
                Ok(true),
                "{steps}"
            );
        }
        for steps in [-7, -6, 6, 7] {
            assert_eq!(
                verify(RFC_SECRET, &code_of_step(steps), now),
                Ok(false),
                "{steps}"
            );
        }
    }

    #[test]
    fn step_of_a_code_is_the_step_it_was_made_for() {
        let now = 1_234_567_890;
        let step = now / STEP_SECONDS;
        let code = code_at(RFC_SECRET, now - 2 * STEP_SECONDS).expect("the secret is valid");

        assert_eq!(step_of(RFC_SECRET, &code, now), Ok(Some(step - 2)));
        assert_eq!(step_of(RFC_SECRET, "abcdef", now), Ok(None));
    }

    #[test]
    fn step_is_taken_once_and_only_after_the_last_one() {
        let now = 1_234_567_890;
        let step = now / STEP_SECONDS;
        let factor = "step_is_taken_once 1";

        assert!(take_step(factor, step, now));
        assert!(!take_step(factor, step, now));
        assert!(!take_step(factor, step - 1, now));
        assert!(take_step(factor, step + 1, now));
        // Another factor has steps of its own.
        assert!(take_step("step_is_taken_once 2", step, now));
    }

    #[test]
    fn code_is_checked_as_text_of_six_digits() {
        let now = 1_234_567_890;

        assert_eq!(verify(RFC_SECRET, " 005924 ", now), Ok(true));
        for wrong in ["", "5924", "0059240", "00592a", "005 924", "005925"] {
            assert_eq!(verify(RFC_SECRET, wrong, now), Ok(false), "{wrong:?}");
        }
    }

    #[test]
    fn secret_is_base32_in_any_letter_case_and_grouping() {
        let now = 1_234_567_890;
        let grouped = "gezd gnbv gy3t qojq gezd gnbv gy3t qojq";
        assert_eq!(verify(grouped, "005924", now), Ok(true));
        // A secret of the PHP application has 16 characters.
        assert!(is_secret("JBSWY3DPEHPK3PXP"));

        for wrong in ["", "   ", "not base32!", "1890"] {
            assert!(!is_secret(wrong), "{wrong:?}");
            assert_eq!(
                verify(wrong, "005924", now),
                Err(TwoFactorError::InvalidSecret),
                "{wrong:?}"
            );
        }
    }

    #[test]
    fn new_secret_is_random_base32_that_makes_codes() {
        let first = new_secret();
        let second = new_secret();

        assert_ne!(first, second);
        assert!(first.len() >= 32, "{first}");
        assert!(
            first
                .chars()
                .all(|character| character.is_ascii_uppercase() || ('2'..='7').contains(&character)),
            "{first}"
        );
        let code = code_at(&first, 1_234_567_890).expect("the secret is valid");
        assert_eq!(verify(&first, &code, 1_234_567_890), Ok(true));
        assert_eq!(verify(&second, &code, 1_234_567_890), Ok(false));
    }

    #[test]
    fn picture_is_a_png_inside_the_address() {
        let address =
            picture(RFC_SECRET, "My: Organizr", "robin:the user").expect("the picture can be made");

        let png = address
            .strip_prefix("data:image/png;base64,")
            .expect("the address holds a PNG picture");
        // The first bytes of every PNG file, in Base64.
        assert!(png.starts_with("iVBORw0KGgo"), "{png}");
        assert!(
            !address.contains(RFC_SECRET),
            "the secret is only inside the picture"
        );
    }
}
