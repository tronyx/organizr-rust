//! A limit on failed logins, kept on the server.
//!
//! The PHP application let the browser count its own failed logins, so anyone who sent
//! requests without a browser was never locked out. Here the server counts, in two ways
//! at once:
//!
//! - by the address of the client, which stops one client from trying many names;
//! - by the name that was tried, which stops many clients from trying one name. A name
//!   or an e-mail address that belongs to an account counts for that account, so the
//!   name and the address of one account share one count.
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

use organizr_domain::UserId;
use unicode_normalization::UnicodeNormalization;

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
    /// A name in lower case, which belongs to no account.
    Name(String),
    /// An account, whichever name or address it was tried with.
    Account(UserId),
}

/// The key of a client: an IPv4 address, or the network of an IPv6 address.
fn client_key(client: IpAddr) -> Key {
    match client {
        IpAddr::V4(_) => Key::Client(client),
        IpAddr::V6(address) => {
            let network = u128::from(address) & (u128::MAX << 64);
            Key::Client(IpAddr::V6(network.into()))
        }
    }
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

/// The name of a login as the limiter counts it: in the compatibility form of Unicode
/// (NFKC, so a name in wide letters is the same name in plain ones), in lower case, and
/// without white space, control characters and characters that show nothing. A login
/// server such as LDAP, Jellyfin or Emby may take `robin `, `ro bin` or `robin` with a
/// soft hyphen for `robin`; counted apart, each of them would get its own failures after
/// `robin` is locked out. Names that only differ in this way share one count, which can
/// only lock out sooner.
pub fn name_key(name: &str) -> String {
    name.nfkc()
        .filter(|character| {
            !character.is_whitespace() && !character.is_control() && !is_invisible(*character)
        })
        .flat_map(char::to_lowercase)
        .take(NAME_KEY_LENGTH)
        .collect()
}

/// Whether a name holds a control character or a character that shows nothing, such as
/// a soft hyphen or a zero-width space. No real name of a login needs one, and a login
/// server may drop them, so such a name is refused before any server is asked.
pub fn has_hidden_characters(name: &str) -> bool {
    name.chars()
        .any(|character| character.is_control() || is_invisible(character))
}

/// The characters that Unicode lets a program show as nothing (the property
/// `Default_Ignorable_Code_Point`).
fn is_invisible(character: char) -> bool {
    matches!(
        character,
        '\u{ad}'
            | '\u{34f}'
            | '\u{61c}'
            | '\u{115f}'..='\u{1160}'
            | '\u{17b4}'..='\u{17b5}'
            | '\u{180b}'..='\u{180f}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{206f}'
            | '\u{3164}'
            | '\u{fe00}'..='\u{fe0f}'
            | '\u{feff}'
            | '\u{ffa0}'
            | '\u{fff0}'..='\u{fff8}'
            | '\u{1bca0}'..='\u{1bca3}'
            | '\u{1d173}'..='\u{1d17a}'
            | '\u{e0000}'..='\u{e0fff}'
    )
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
    /// The name counts as [`name_key`] has it, so names that a login server may take
    /// for the same one share a count. All addresses of one IPv6 network (the first 64
    /// bits) count as one client, because one client usually has the whole network to
    /// choose from.
    pub fn new(client: IpAddr, name: &str) -> Self {
        Self {
            client: client_key(client),
            name: Key::Name(name_key(name)),
        }
    }

    /// An attempt from the given address to log in to the given account. Use it when
    /// the name or the address belongs to an account, so that every way to name the
    /// account shares one count.
    pub fn for_account(client: IpAddr, account: UserId) -> Self {
        Self {
            client: client_key(client),
            name: Key::Account(account),
        }
    }

    fn keys(&self) -> [&Key; 2] {
        [&self.client, &self.name]
    }
}

/// An attempt that the limiter counted before its check, until the check says how it
/// went (D-139).
///
/// The attempt counts as a failed one from the start, so attempts that run at the same
/// time cannot all be checked before the first failure is counted. [`Pending::failed`]
/// keeps the count, [`Pending::succeeded`] clears the counts as after a login, and an
/// attempt that ends without either, such as a form with a missing field, gives its
/// count back.
#[derive(Debug)]
#[must_use = "an attempt that is dropped gives its count back"]
pub struct Pending<'a> {
    limiter: &'a LoginLimiter,
    attempt: Attempt,
    policy: LimitPolicy,
    /// The lock that this attempt caused, when it reached the limit.
    locked: Option<Duration>,
    settled: bool,
}

impl Pending<'_> {
    /// The check failed: the count stays. Returns how long the client or the name is
    /// locked, when this attempt reached the limit.
    pub fn failed(mut self) -> Option<Duration> {
        self.settled = true;
        self.locked
    }

    /// The check succeeded: the counts of the client and of the name start again.
    pub fn succeeded(mut self) {
        self.settled = true;
        self.limiter.record_success(&self.attempt);
    }
}

impl Drop for Pending<'_> {
    fn drop(&mut self) {
        if !self.settled {
            self.limiter
                .give_back(&self.attempt, self.policy, self.locked.is_some());
        }
    }
}

impl LoginLimiter {
    /// A limiter without counts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts an attempt: refuses it with the time to wait while its client or its name
    /// is locked, and otherwise counts it as a failed one at once, in the same step
    /// (D-139). The answer says how the check went.
    pub fn begin(
        &self,
        attempt: &Attempt,
        policy: LimitPolicy,
        now: Instant,
    ) -> Result<Pending<'_>, Duration> {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let wait = attempt
            .keys()
            .into_iter()
            .filter_map(|key| entries.get(key)?.locked_until)
            .filter(|until| now < *until)
            .max();
        if let Some(until) = wait {
            return Err(until.duration_since(now));
        }
        let locked = count_failure(&mut entries, attempt, policy, now);
        Ok(Pending {
            limiter: self,
            attempt: attempt.clone(),
            policy,
            locked,
            settled: false,
        })
    }

    /// Takes back the count of an attempt that ended without a verdict, and the lock it
    /// caused.
    fn give_back(&self, attempt: &Attempt, policy: LimitPolicy, caused_lock: bool) {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        for key in attempt.keys() {
            if let Some(entry) = entries.get_mut(key) {
                entry.failures = entry.failures.saturating_sub(1);
                if caused_lock && entry.failures < policy.failures.max(1) {
                    entry.locked_until = None;
                }
            }
        }
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
        count_failure(&mut entries, attempt, policy, now)
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

/// Counts a failed attempt for the client and for the name. Returns how long they are
/// locked, when this failure caused a lock.
fn count_failure(
    entries: &mut HashMap<Key, Entry>,
    attempt: &Attempt,
    policy: LimitPolicy,
    now: Instant,
) -> Option<Duration> {
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

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::time::{Duration, Instant};

    use super::{
        Attempt, FAILURE_MEMORY, LimitPolicy, LoginLimiter, Pending, SWEEP_ABOVE,
        has_hidden_characters, name_key,
    };

    #[test]
    fn names_that_a_login_server_may_take_for_one_share_a_key() {
        for name in [
            "robin",
            "ROBIN",
            " robin",
            "robin ",
            "ro bin",
            "ro\tbin",
            "rob\u{ad}in",
            "r\u{200b}obin",
            "\u{ff52}\u{ff4f}\u{ff42}\u{ff49}\u{ff4e}",
        ] {
            assert_eq!(name_key(name), "robin", "{name:?}");
        }
        assert_ne!(name_key("robyn"), "robin");
        assert!(has_hidden_characters("rob\u{ad}in"));
        assert!(has_hidden_characters("robin\u{feff}"));
        assert!(has_hidden_characters("rob\u{7}in"));
        assert!(!has_hidden_characters("ro bin"));
        assert!(!has_hidden_characters("rob\u{e9}rt"));
    }

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

    #[test]
    fn attempts_under_way_count_before_their_check_ends() {
        // A burst of logins for one name gets as many checks as the limit, however many
        // run at once (D-139).
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        let attempt = attempt("192.0.2.7", "robin");
        let under_way: Vec<_> = (0..3)
            .map(|_| {
                limiter
                    .begin(&attempt, POLICY, now)
                    .expect("not locked yet")
            })
            .collect();
        assert!(limiter.begin(&attempt, POLICY, now).is_err());
        // The attempt that reached the limit says so when it fails.
        let mut causes = under_way.into_iter().map(Pending::failed);
        assert_eq!(causes.next(), Some(None));
        assert_eq!(causes.next(), Some(None));
        assert_eq!(causes.next(), Some(Some(POLICY.lockout)));
    }

    #[test]
    fn attempt_that_ends_without_a_verdict_counts_for_nothing() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        let attempt = attempt("192.0.2.7", "robin");
        for _ in 0..10 {
            drop(limiter.begin(&attempt, POLICY, now).expect("not locked"));
        }
        // Two failures and a third attempt under way: the third may still go on.
        limiter
            .begin(&attempt, POLICY, now)
            .expect("not locked")
            .failed();
        limiter
            .begin(&attempt, POLICY, now)
            .expect("not locked")
            .failed();
        let last = limiter.begin(&attempt, POLICY, now).expect("not locked");
        drop(last);
        assert_eq!(limiter.locked_for(&attempt, now), None);
        assert!(limiter.begin(&attempt, POLICY, now).is_ok());
    }

    #[test]
    fn attempt_that_succeeds_clears_the_counts() {
        let limiter = LoginLimiter::new();
        let now = Instant::now();
        let attempt = attempt("192.0.2.7", "robin");
        limiter
            .begin(&attempt, POLICY, now)
            .expect("not locked")
            .failed();
        limiter
            .begin(&attempt, POLICY, now)
            .expect("not locked")
            .failed();
        limiter
            .begin(&attempt, POLICY, now)
            .expect("not locked")
            .succeeded();
        assert_eq!(fail(&limiter, &attempt, 2, now), None);
    }
}
