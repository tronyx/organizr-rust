//! Checking and hashing passwords.
//!
//! The PHP application stored bcrypt hashes (`$2y$`). An imported user must be able to
//! log in with such a hash, so they are still checked here. Every new hash is made with
//! Argon2id. When a correct password was checked against an older hash, the caller
//! stores a new hash, so old hashes go away as users log in.
//!
//! Both functions take tens of milliseconds and 19 MiB of memory on purpose. Call them
//! through [`verify_in_turn`] and [`hash_in_turn`], which run them away from the tasks
//! that serve requests and only a few at a time (D-140).

use argon2::password_hash::phc;
use argon2::{ARGON2ID_IDENT, Argon2, Params, PasswordHasher, PasswordVerifier, Version};
use organizr_domain::PasswordHash;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use thiserror::Error;
use tokio::sync::Semaphore;

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

/// How long a password check waits for its turn before the answer is "busy".
const LONGEST_WAIT: Duration = Duration::from_secs(10);

/// The fewest and the most password checks that run at the same time.
const FEWEST_AT_ONCE: usize = 2;
const MOST_AT_ONCE: usize = 16;

/// A password check or hash that did not run.
#[derive(Debug, Error)]
pub enum InTurnError {
    /// Every place for a check stayed taken for [`LONGEST_WAIT`].
    #[error("the server is busy with other password checks")]
    Busy,
    /// The work stopped before it ended.
    #[error("a password check stopped: {0}")]
    Task(#[source] tokio::task::JoinError),
}

/// The places for password checks: as many as the server has processors, between
/// [`FEWEST_AT_ONCE`] and [`MOST_AT_ONCE`]. With all taken, a check waits for its turn,
/// so many logins at once use at most that many times 19 MiB (D-140).
struct Turns {
    places: Arc<Semaphore>,
    longest_wait: Duration,
}

impl Turns {
    fn new(places: usize, longest_wait: Duration) -> Self {
        Self {
            places: Arc::new(Semaphore::new(places)),
            longest_wait,
        }
    }

    async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, InTurnError> {
        let place = tokio::time::timeout(self.longest_wait, self.places.clone().acquire_owned())
            .await
            .map_err(|_| InTurnError::Busy)?
            .map_err(|_| InTurnError::Busy)?;
        // The place goes with the work: a caller that gives up (a client that hangs up)
        // does not free it while the check still runs on its thread.
        tokio::task::spawn_blocking(move || {
            let result = work();
            drop(place);
            result
        })
        .await
        .map_err(InTurnError::Task)
    }
}

static TURNS: LazyLock<Turns> = LazyLock::new(|| {
    let processors = std::thread::available_parallelism().map_or(FEWEST_AT_ONCE, usize::from);
    Turns::new(processors.clamp(FEWEST_AT_ONCE, MOST_AT_ONCE), LONGEST_WAIT)
});

/// [`verify`], in its turn among the other password checks.
///
/// # Errors
///
/// [`InTurnError::Busy`] when no place was free for [`LONGEST_WAIT`].
pub async fn verify_in_turn(
    password: String,
    hash: PasswordHash,
) -> Result<PasswordCheck, InTurnError> {
    TURNS.run(move || verify(&password, &hash)).await
}

/// [`hash`], in its turn among the password checks.
///
/// # Errors
///
/// [`InTurnError::Busy`] when no place was free for [`LONGEST_WAIT`].
pub async fn hash_in_turn(
    password: String,
) -> Result<Result<PasswordHash, HashError>, InTurnError> {
    TURNS.run(move || hash(&password)).await
}

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

    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::{InTurnError, PasswordCheck, Turns, hash, verify};

    #[tokio::test]
    async fn checks_run_no_more_than_the_places_at_once() {
        // Many logins at once run their checks a few at a time (D-140).
        let turns = Arc::new(Turns::new(2, Duration::from_secs(10)));
        let (running, most) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let checks = (0..8).map(|_| {
            let (turns, running, most) = (turns.clone(), running.clone(), most.clone());
            tokio::spawn(async move {
                turns
                    .run(move || {
                        let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                        most.fetch_max(now, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(30));
                        running.fetch_sub(1, Ordering::SeqCst);
                    })
                    .await
            })
        });
        for check in checks.collect::<Vec<_>>() {
            check
                .await
                .expect("the task ends")
                .expect("each check gets its turn");
        }
        assert_eq!(most.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn caller_that_gives_up_keeps_its_place_until_the_check_ends() {
        // A login whose client hangs up drops its future, but the check still runs on
        // its thread, so its place stays taken until the check ends (D-140).
        let turns = Arc::new(Turns::new(2, Duration::from_secs(10)));
        let (running, most) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        for _ in 0..6 {
            let (turns, running, most) = (turns.clone(), running.clone(), most.clone());
            let caller = tokio::spawn(async move {
                turns
                    .run(move || {
                        let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                        most.fetch_max(now, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(200));
                        running.fetch_sub(1, Ordering::SeqCst);
                    })
                    .await
            });
            tokio::time::sleep(Duration::from_millis(10)).await;
            caller.abort();
        }
        tokio::time::sleep(Duration::from_millis(900)).await;
        assert!(
            most.load(Ordering::SeqCst) <= 2,
            "{}",
            most.load(Ordering::SeqCst)
        );
        assert_eq!(running.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn check_that_waits_too_long_is_busy() {
        let turns = Arc::new(Turns::new(1, Duration::from_millis(50)));
        let slow = {
            let turns = turns.clone();
            tokio::spawn(async move {
                turns
                    .run(|| std::thread::sleep(Duration::from_millis(500)))
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        let waited = turns.run(|| ()).await;
        assert!(matches!(waited, Err(InTurnError::Busy)), "{waited:?}");
        slow.await
            .expect("the task ends")
            .expect("the first check ran");
    }

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
