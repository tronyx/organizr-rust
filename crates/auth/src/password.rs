//! Checking and hashing passwords.
//!
//! The PHP application stored bcrypt hashes (`$2y$`). An imported user must be able to
//! log in with such a hash, so they are still checked here. Every new hash is made with
//! Argon2id. When a correct password was checked against an older hash, the caller
//! stores a new hash, so old hashes go away as users log in.
//!
//! Both functions take tens of milliseconds on purpose. Call them from
//! `tokio::task::spawn_blocking`, not directly from a request handler.

use argon2::password_hash::phc;
use argon2::{ARGON2ID_IDENT, Argon2, Params, PasswordHasher, PasswordVerifier, Version};
use organizr_domain::PasswordHash;
use thiserror::Error;

/// The result of checking a password against a stored hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordCheck {
    /// The password is the one that was hashed.
    Correct {
        /// Whether the hash was made with an older method or with weaker settings than
        /// [`hash`] uses now. When it is, store a new hash of the password.
        outdated: bool,
    },
    /// The password is not the one that was hashed.
    Wrong,
    /// The stored value is not a hash this application can check. No password matches it.
    Unusable,
}

/// A password could not be hashed.
#[derive(Debug, Error)]
#[error("the password could not be hashed: {0}")]
pub struct HashError(#[source] argon2::password_hash::Error);

/// Checks a password against a stored hash.
///
/// A bcrypt hash is checked as PHP's `password_verify()` checks it: only the first 72
/// bytes of the password count, and the password ends at its first zero byte.
pub fn verify(password: &str, hash: &PasswordHash) -> PasswordCheck {
    let stored = hash.expose();
    if stored.starts_with("$2") {
        verify_bcrypt(password, stored)
    } else if stored.starts_with("$argon2") {
        verify_argon2(password, stored)
    } else {
        PasswordCheck::Unusable
    }
}

/// Hashes a password for storing, with Argon2id and a new random salt.
pub fn hash(password: &str) -> Result<PasswordHash, HashError> {
    let hash = Argon2::default()
        .hash_password(password.as_bytes())
        .map_err(HashError)?;
    Ok(PasswordHash::new(hash.to_string()))
}

fn verify_bcrypt(password: &str, stored: &str) -> PasswordCheck {
    // PHP hands the password to a C function, which reads it up to the first zero byte.
    let password = password
        .split_once('\0')
        .map_or(password, |(before_zero, _)| before_zero);

    match bcrypt::verify(password, stored) {
        Ok(true) => PasswordCheck::Correct { outdated: true },
        Ok(false) => PasswordCheck::Wrong,
        Err(_) => PasswordCheck::Unusable,
    }
}

fn verify_argon2(password: &str, stored: &str) -> PasswordCheck {
    let Ok(parsed) = phc::PasswordHash::new(stored) else {
        return PasswordCheck::Unusable;
    };
    // Without both parts there is nothing to compare a password with.
    if parsed.salt.is_none() || parsed.hash.is_none() {
        return PasswordCheck::Unusable;
    }

    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(()) => PasswordCheck::Correct {
            outdated: !has_current_settings(&parsed),
        },
        Err(argon2::password_hash::Error::PasswordInvalid) => PasswordCheck::Wrong,
        Err(_) => PasswordCheck::Unusable,
    }
}

/// Whether an Argon2 hash was made as [`hash`] makes one now.
fn has_current_settings(parsed: &phc::PasswordHash) -> bool {
    let current = Params::DEFAULT;
    let Ok(used) = Params::try_from(parsed) else {
        return false;
    };

    parsed.algorithm == ARGON2ID_IDENT
        && parsed.version == Some(u32::from(Version::default()))
        && used.m_cost() == current.m_cost()
        && used.t_cost() == current.t_cost()
        && used.p_cost() == current.p_cost()
}

#[cfg(test)]
mod tests {
    use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
    use organizr_domain::PasswordHash;

    use super::{PasswordCheck, hash, verify};

    /// A hash of `password` with Argon2 settings that the caller chooses.
    fn argon2_hash(algorithm: Algorithm, version: Version, params: Params) -> PasswordHash {
        let hash = Argon2::new(algorithm, version, params)
            .hash_password(b"password")
            .expect("the password can be hashed");
        PasswordHash::new(hash.to_string())
    }

    fn params(m_cost: u32, t_cost: u32, p_cost: u32) -> Params {
        Params::new(m_cost, t_cost, p_cost, None).expect("the settings are valid")
    }

    #[test]
    fn new_hash_is_argon2id_and_never_the_password() {
        let hashed = hash("correct horse").expect("the password can be hashed");

        assert!(
            hashed
                .expose()
                .starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
            "{}",
            hashed.expose()
        );
        assert!(!hashed.expose().contains("correct horse"));
    }

    #[test]
    fn new_hash_accepts_its_password_and_is_up_to_date() {
        let hashed = hash("correct horse").expect("the password can be hashed");

        assert_eq!(
            verify("correct horse", &hashed),
            PasswordCheck::Correct { outdated: false }
        );
    }

    #[test]
    fn new_hash_refuses_another_password() {
        let hashed = hash("correct horse").expect("the password can be hashed");

        for wrong in ["", "correct hors", "correct horse ", "Correct horse"] {
            assert_eq!(verify(wrong, &hashed), PasswordCheck::Wrong, "{wrong:?}");
        }
    }

    #[test]
    fn same_password_gives_another_hash_each_time() {
        let first = hash("correct horse").expect("the password can be hashed");
        let second = hash("correct horse").expect("the password can be hashed");

        assert_ne!(first.expose(), second.expose());
    }

    #[test]
    fn password_is_not_cut_short_by_argon2() {
        let long = "a".repeat(80);
        let hashed = hash(&long).expect("the password can be hashed");

        assert_eq!(
            verify(&long, &hashed),
            PasswordCheck::Correct { outdated: false }
        );
        // bcrypt reads only 72 bytes. Argon2 reads all of them.
        assert_eq!(verify(&"a".repeat(72), &hashed), PasswordCheck::Wrong);
        // bcrypt stops at a zero byte. Argon2 does not.
        let with_zero = hash("abc\0def").expect("the password can be hashed");
        assert_eq!(verify("abc", &with_zero), PasswordCheck::Wrong);
        assert_eq!(
            verify("abc\0def", &with_zero),
            PasswordCheck::Correct { outdated: false }
        );
    }

    #[test]
    fn argon2_hash_with_other_settings_is_outdated() {
        let current = Params::DEFAULT;
        let (memory, time, lanes) = (current.m_cost(), current.t_cost(), current.p_cost());
        let outdated = [
            (
                Algorithm::Argon2id,
                Version::V0x13,
                params(memory / 2, time, lanes),
            ),
            (
                Algorithm::Argon2id,
                Version::V0x13,
                params(memory, time - 1, lanes),
            ),
            (
                Algorithm::Argon2id,
                Version::V0x13,
                params(memory, time, lanes + 1),
            ),
            (
                Algorithm::Argon2id,
                Version::V0x10,
                params(memory, time, lanes),
            ),
            (
                Algorithm::Argon2i,
                Version::V0x13,
                params(memory, time, lanes),
            ),
            (
                Algorithm::Argon2d,
                Version::V0x13,
                params(memory, time, lanes),
            ),
        ];

        for (algorithm, version, params) in outdated {
            let stored = argon2_hash(algorithm, version, params);

            assert_eq!(
                verify("password", &stored),
                PasswordCheck::Correct { outdated: true },
                "{}",
                stored.expose()
            );
            assert_eq!(
                verify("Password", &stored),
                PasswordCheck::Wrong,
                "{}",
                stored.expose()
            );
        }
    }

    #[test]
    fn argon2_hash_with_the_current_settings_is_up_to_date() {
        let stored = argon2_hash(Algorithm::Argon2id, Version::V0x13, Params::DEFAULT);

        assert_eq!(
            verify("password", &stored),
            PasswordCheck::Correct { outdated: false }
        );
    }

    #[test]
    fn value_that_is_no_hash_matches_no_password() {
        let unusable = [
            "",
            "password",
            "$",
            "$2y$",
            // bcrypt, cut short.
            "$2y$10$abcdefghijklmnopqrstuv",
            // bcrypt with a cost that does not exist.
            "$2y$99$abcdefghijklmnopqrstuuabcdefghijklmnopqrstuvwxyz01234",
            // Argon2 without a salt and a hash.
            "$argon2id$v=19$m=19456,t=2,p=1",
            "$argon2id$v=19$m=19456,t=2,p=1$not base64!$also not",
            // Methods of PHP's crypt() that Organizr never used.
            "$1$rasmusle$rISCgZzpwk3UhDidwXvin0",
            "$6$rounds=5000$usesomesillystri$D4IrlXatmP7rx3P3InaxBeoomnAihCKRVQP22JkLQ4hBE/SzVlTtNvXDp7OAjBeQgTV4lIdQ5oAZINs1ygnp1/",
            // An algorithm that is not Argon2.
            "$scrypt$ln=16,r=8,p=1$aM15713r3Xsvxbi31lqr1Q$nFNh2CVHVjNldFVKDHDlm4CbdRSCdEBsjjJxD+iCs5E",
        ];

        for stored in unusable {
            let stored = PasswordHash::new(stored.to_owned());

            for candidate in ["", "password"] {
                assert_eq!(
                    verify(candidate, &stored),
                    PasswordCheck::Unusable,
                    "{candidate:?} against {:?}",
                    stored.expose()
                );
            }
        }
    }
}
