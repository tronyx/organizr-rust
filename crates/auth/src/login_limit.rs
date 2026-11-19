//! A limit on failed logins, kept on the server.
//!
//! The PHP application let the browser count its own failed logins, so anyone who sent
//! requests without a browser was never locked out. Here the server counts, in two ways
//! at once:
//!
//! - by the address of the client, which stops one client from trying many names;
//! - by the name that was tried, which stops many clients from trying one name.
//!
//! When either count reaches the limit, that address or that name is locked for the
//! lock-out time, and every login from it or for it is refused, whatever the password.
//! The limit and the lock-out time are the `loginAttempts` and `loginLockout` settings.
//! A login that succeeds sets both counts back to zero.
//!
//! The lock on a name has a cost: someone who knows a name can keep its owner locked
//! out by failing logins on purpose. Without it, an attacker with many addresses could
//! try passwords for one name without end, which is the worse of the two.
//!
//! The counts live in memory. A restart of the application forgets them.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// How long a failed login counts when no lock follows. A lock-out time longer than
/// this is used instead, so a failure never ends before the lock it can cause.
const FAILURE_MEMORY: Duration = Duration::from_mins(15);

/// How many characters of a name make its key. A longer name shares the count of every
/// name that starts with the same characters.
const NAME_KEY_LENGTH: usize = 256;

/// With more entries than this, entries that no longer matter are removed before a new
/// one is added.
const SWEEP_ABOVE: usize = 10_000;

/// How many failed logins lock a client or a name, and for how long.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimitPolicy {
    /// The number of failed logins that causes a lock. Zero counts as one.
    pub failures: u32,
    /// How long a lock lasts.
    pub lockout: Duration,
}

/// What a login is counted under.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Key {
    /// An IPv4 address, or the network of an IPv6 address.
    Client(IpAddr),
    /// A name in lower case.
    Name(String),
}

/// The failed logins of one key.
#[derive(Debug, Clone, Copy)]
struct Entry {
    failures: u32,
    /// When the count is forgotten, unless a lock follows.
    forget_at: Instant,
    /// When the lock ends, while the key is locked.
    locked_until: Option<Instant>,
}

impl Entry {
    fn is_locked(&self, now: Instant) -> bool {
        self.locked_until.is_some_and(|until| now < until)
    }

    /// Whether the entry still says something at `now`.
    fn matters(&self, now: Instant) -> bool {
        self.is_locked(now) || (self.locked_until.is_none() && now < self.forget_at)
    }
}

/// Counts failed logins and says who is locked out.
#[derive(Debug, Default)]
pub struct LoginLimiter {
    entries: Mutex<HashMap<Key, Entry>>,
}

/// One login attempt, as the limiter sees it: who tries, and which name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    client: Key,
    name: Key,
}

impl Attempt {
    /// An attempt from the given address to log in with the given name or address.
    ///
    /// The letter case of the name does not matter. All addresses of one IPv6 network
    /// (the first 64 bits) count as one client, because one client usually has the
    /// whole network to choose from.
    pub fn new(client: IpAddr, name: &str) -> Self {
        let client = match client {
            IpAddr::V4(_) => client,
            IpAddr::V6(address) => {
                let network = u128::from(address) & (u128::MAX << 64);
                IpAddr::V6(network.into())
            }
        };
        let name = name.to_lowercase().chars().take(NAME_KEY_LENGTH).collect();
        Self {
            client: Key::Client(client),
            name: Key::Name(name),
        }
    }

    fn keys(&self) -> [&Key; 2] {
        [&self.client, &self.name]
    }
}

impl LoginLimiter {
    /// A limiter without counts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Says how long an attempt must wait, when its client or its name is locked.
    /// `None` means the login may go on. Asking does not count as an attempt.
    pub fn locked_for(&self, attempt: &Attempt, now: Instant) -> Option<Duration> {
        let entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        attempt
            .keys()
            .into_iter()
            .filter_map(|key| entries.get(key)?.locked_until)
            .filter(|until| now < *until)
            .max()
            .map(|until| until.duration_since(now))
    }

    /// Counts a failed login. Returns how long the client or the name is locked, when
    /// this failure caused a lock.
    pub fn record_failure(
        &self,
        attempt: &Attempt,
        policy: LimitPolicy,
        now: Instant,
    ) -> Option<Duration> {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        if entries.len() > SWEEP_ABOVE {
            entries.retain(|_, entry| entry.matters(now));
        }

        let mut locked = None;
        for key in attempt.keys() {
            let entry = entries.entry(key.clone()).or_insert(Entry {
                failures: 0,
                forget_at: now,
                locked_until: None,
            });
            // A count that is forgotten, or that belongs to a lock that has ended,
            // starts again.
            if !entry.matters(now) {
                entry.failures = 0;
                entry.locked_until = None;
            }
            entry.failures = entry.failures.saturating_add(1);
            entry.forget_at = now + FAILURE_MEMORY.max(policy.lockout);
            if entry.failures >= policy.failures.max(1) {
                entry.locked_until = Some(now + policy.lockout);
                locked = Some(policy.lockout);
            }
        }
        locked
    }

    /// Forgets the failed logins of the client and for the name of an attempt, after a
    /// login that succeeded. A user who mistyped a password a few times starts again at
    /// zero.
    ///
    /// A client with an account of its own can use this to try other names for longer.
    /// The count for each of those names still stops it.
    pub fn record_success(&self, attempt: &Attempt) {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        for key in attempt.keys() {
            entries.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::time::{Duration, Instant};

    use super::{Attempt, FAILURE_MEMORY, LimitPolicy, LoginLimiter, SWEEP_ABOVE};

    const POLICY: LimitPolicy = LimitPolicy {
        failures: 3,
        lockout: Duration::from_mins(1),
    };

    fn attempt(client: &str, name: &str) -> Attempt {
        let client: IpAddr = client.parse().expect("the address is valid");
        Attempt::new(client, name)
    }

    /// Fails `count` logins and returns what the last one caused.
    fn fail(
        limiter: &LoginLimiter,
        attempt: &Attempt,
        count: u32,
        now: Instant,
    ) -> Option<Duration> {
        let mut last = None;
        for _ in 0..count {
            last = limiter.record_failure(attempt, POLICY, now);
        }
        last
    }

    #[test]
    fn failures_below_the_limit_lock_nobody() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        let attempt = attempt("192.0.2.7", "robin");

        assert_eq!(fail(&limiter, &attempt, 2, now), None);
        assert_eq!(limiter.locked_for(&attempt, now), None);
    }

    #[test]
    fn failure_that_reaches_the_limit_locks_for_the_lockout_time() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        let attempt = attempt("192.0.2.7", "robin");

        assert_eq!(fail(&limiter, &attempt, 3, now), Some(POLICY.lockout));

        assert_eq!(limiter.locked_for(&attempt, now), Some(POLICY.lockout));
        let later = now + Duration::from_secs(45);
        assert_eq!(
            limiter.locked_for(&attempt, later),
            Some(Duration::from_secs(15))
        );
    }

    #[test]
    fn asking_does_not_count_as_an_attempt() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        let attempt = attempt("192.0.2.7", "robin");
        fail(&limiter, &attempt, 2, now);

        for _ in 0..10 {
            assert_eq!(limiter.locked_for(&attempt, now), None);
        }
        assert_eq!(fail(&limiter, &attempt, 1, now), Some(POLICY.lockout));
    }

    #[test]
    fn lock_ends_and_the_count_starts_again() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        let attempt = attempt("192.0.2.7", "robin");
        fail(&limiter, &attempt, 3, now);

        let after = now + POLICY.lockout;
        assert_eq!(limiter.locked_for(&attempt, after), None);
        assert_eq!(fail(&limiter, &attempt, 2, after), None, "a new count");
        assert_eq!(fail(&limiter, &attempt, 1, after), Some(POLICY.lockout));
    }

    #[test]
    fn one_client_cannot_try_many_names() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        for name in ["anna", "ben", "robin"] {
            fail(&limiter, &attempt("192.0.2.7", name), 1, now);
        }

        // The client is locked, also for a name it did not try.
        assert!(
            limiter
                .locked_for(&attempt("192.0.2.7", "zoe"), now)
                .is_some()
        );
        // Another client is not, and neither is one of the names.
        assert_eq!(limiter.locked_for(&attempt("192.0.2.8", "anna"), now), None);
    }

    #[test]
    fn many_clients_cannot_try_one_name() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        for client in ["192.0.2.1", "192.0.2.2", "192.0.2.3"] {
            fail(&limiter, &attempt(client, "Robin"), 1, now);
        }

        // The name is locked from every client, in any letter case.
        assert!(
            limiter
                .locked_for(&attempt("198.51.100.9", "robin"), now)
                .is_some()
        );
        assert!(
            limiter
                .locked_for(&attempt("198.51.100.9", "ROBIN"), now)
                .is_some()
        );
        // The clients themselves are not: each failed once.
        assert_eq!(limiter.locked_for(&attempt("192.0.2.1", "anna"), now), None);
    }

    #[test]
    fn addresses_of_one_ipv6_network_are_one_client() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        for (client, name) in [
            ("2001:db8:1:2::1", "anna"),
            ("2001:db8:1:2::2", "ben"),
            ("2001:db8:1:2:ffff:ffff:ffff:ffff", "robin"),
        ] {
            fail(&limiter, &attempt(client, name), 1, now);
        }

        assert!(
            limiter
                .locked_for(&attempt("2001:db8:1:2:abcd::9", "zoe"), now)
                .is_some()
        );
        assert_eq!(
            limiter.locked_for(&attempt("2001:db8:1:3::1", "zoe"), now),
            None,
            "another network"
        );
    }

    #[test]
    fn failure_is_forgotten_after_a_while() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        let attempt = attempt("192.0.2.7", "robin");
        fail(&limiter, &attempt, 2, now);

        let still_counted = now + FAILURE_MEMORY.saturating_sub(Duration::from_secs(1));
        assert_eq!(
            limiter.record_failure(&attempt, POLICY, still_counted),
            Some(POLICY.lockout),
            "the third failure within the memory locks"
        );

        let other = self::attempt("192.0.2.8", "zoe");
        fail(&limiter, &other, 2, now);
        let forgotten = now + FAILURE_MEMORY;
        assert_eq!(limiter.record_failure(&other, POLICY, forgotten), None);
    }

    #[test]
    fn failure_counts_at_least_as_long_as_a_lock_lasts() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        let long = LimitPolicy {
            failures: 2,
            lockout: Duration::from_hours(2),
        };
        let attempt = attempt("192.0.2.7", "robin");
        limiter.record_failure(&attempt, long, now);

        let much_later = now + Duration::from_mins(90);
        assert_eq!(
            limiter.record_failure(&attempt, long, much_later),
            Some(long.lockout)
        );
    }

    #[test]
    fn success_forgets_the_client_and_the_name() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        let own = attempt("192.0.2.7", "robin");
        fail(&limiter, &own, 2, now);
        // A failure of the same client for another name, which the success leaves alone.
        fail(&limiter, &attempt("192.0.2.9", "anna"), 2, now);

        limiter.record_success(&own);

        // The name starts again: two more failures from another client lock nobody.
        assert_eq!(fail(&limiter, &attempt("192.0.2.8", "robin"), 2, now), None);
        // The client starts again too.
        assert_eq!(fail(&limiter, &attempt("192.0.2.7", "ben"), 2, now), None);
        // The count for the other name is still there: its next failure is the third.
        assert_eq!(
            fail(&limiter, &attempt("192.0.2.10", "anna"), 1, now),
            Some(POLICY.lockout)
        );
    }

    #[test]
    fn limit_of_zero_counts_as_one() {
        let limiter = LoginLimiter::new();
        let strict = LimitPolicy {
            failures: 0,
            lockout: Duration::from_secs(30),
        };

        let locked = limiter.record_failure(&attempt("192.0.2.7", "robin"), strict, Instant::now());

        assert_eq!(locked, Some(Duration::from_secs(30)));
    }

    #[test]
    fn very_long_names_share_a_count() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        let long = "a".repeat(300);
        fail(&limiter, &attempt("192.0.2.1", &long), 3, now);

        let same_start = format!("{}b", "a".repeat(299));
        assert!(
            limiter
                .locked_for(&attempt("192.0.2.2", &same_start), now)
                .is_some()
        );
    }

    #[test]
    fn entries_that_no_longer_matter_are_removed() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        // Each attempt adds a client and a name.
        for number in 0..=SWEEP_ABOVE {
            let client = IpAddr::from(
                u32::try_from(number)
                    .expect("the number fits")
                    .to_be_bytes(),
            );
            let attempt = Attempt::new(client, &format!("name-{number}"));
            limiter.record_failure(&attempt, POLICY, now);
        }
        let before = limiter.entries.lock().expect("the lock is free").len();

        let later = now + FAILURE_MEMORY;
        limiter.record_failure(&attempt("192.0.2.7", "robin"), POLICY, later);

        let after = limiter.entries.lock().expect("the lock is free").len();
        assert!(before > SWEEP_ABOVE, "{before}");
        assert_eq!(after, 2, "only the new client and the new name are left");
    }
}
