//! The recovery password over time: how long it works, and how often an account gets
//! a new one.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::IpAddr;
use std::time::Duration;

use organizr_auth::login::{self, Device, LoginError, SecondFactor};
use organizr_auth::password;
use organizr_auth::recovery::{self, LIFETIME, PAUSE};
use organizr_db::Database;
use organizr_domain::{NewUser, Timestamp};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::seed;

/// The moment of the first request, in seconds since 1970.
const START: i64 = 1_800_000_000;

fn at(seconds_after_start: u64) -> Timestamp {
    let seconds = i64::try_from(seconds_after_start).expect("the time fits");
    Timestamp::from_unix_seconds(START + seconds)
}

async fn add_robin(database: &Database) {
    seed::installed(database).await;
    let group = database
        .groups()
        .find_default()
        .await
        .expect("the lookup works")
        .expect("the install has a group for new users");
    database
        .users()
        .insert(NewUser {
            username: "robin".to_owned(),
            email: Some("Robin@Example.com".to_owned()),
            password_hash: password::hash("the old password").expect("it can be hashed"),
            group: group.id,
            image: None,
            created_at: at(0),
        })
        .await
        .expect("the user can be stored");
}

/// Whether a login with the password succeeds at the given moment.
async fn logs_in(database: &Database, password: &str, now: Timestamp) -> bool {
    let secrets = seed::secrets();
    let device = Device {
        ip: IpAddr::from([192, 0, 2, 7]),
        user_agent: None,
    };
    let result = login::with_password(
        database,
        "robin",
        password,
        SecondFactor::without_code(&secrets),
        device,
        now,
        Duration::from_hours(24),
    )
    .await;
    match result {
        Ok(_) => true,
        Err(LoginError::WrongCredentials) => false,
        Err(error) => panic!("the login failed in another way: {error}"),
    }
}

async fn new_password(database: &Database, now: Timestamp) -> Option<String> {
    recovery::start(database, "robin@example.com", now)
        .await
        .expect("the recovery can start")
        .map(|started| started.password)
}

#[tokio::test]
async fn recovery_password_works_for_one_day() {
    on_each_engine(async |database| {
        add_robin(database).await;
        let password = new_password(database, at(0)).await.expect("a password");
        let last_second = LIFETIME.as_secs() - 1;

        // After the day, the password from the e-mail is no password.
        assert!(!logs_in(database, &password, at(LIFETIME.as_secs())).await);
        assert!(!logs_in(database, &password, at(LIFETIME.as_secs() + 3600)).await);
        // In its last second, it still is.
        assert!(logs_in(database, &password, at(last_second)).await);
        // From then on it is the password of the account, with no end.
        assert!(logs_in(database, &password, at(LIFETIME.as_secs() * 30)).await);
        assert!(!logs_in(database, "the old password", at(last_second)).await);
    })
    .await;
}

#[tokio::test]
async fn old_password_works_after_the_day() {
    on_each_engine(async |database| {
        add_robin(database).await;
        let password = new_password(database, at(0)).await.expect("a password");

        assert!(!logs_in(database, &password, at(LIFETIME.as_secs())).await);
        assert!(logs_in(database, "the old password", at(LIFETIME.as_secs())).await);
    })
    .await;
}

#[tokio::test]
async fn account_gets_one_recovery_password_in_five_minutes() {
    on_each_engine(async |database| {
        add_robin(database).await;
        let first = new_password(database, at(0)).await.expect("a password");

        assert_eq!(new_password(database, at(1)).await, None);
        assert_eq!(new_password(database, at(PAUSE.as_secs() - 1)).await, None);
        // The first one still works after the requests that came too soon.
        let second = new_password(database, at(PAUSE.as_secs()))
            .await
            .expect("a password");
        assert_ne!(first, second);

        // The newer password takes the place of the older one.
        assert!(!logs_in(database, &first, at(PAUSE.as_secs() + 1)).await);
        assert!(logs_in(database, &second, at(PAUSE.as_secs() + 1)).await);
        // Once it is used, there is no recovery password, so a new one can be made.
        assert!(
            new_password(database, at(PAUSE.as_secs() + 2))
                .await
                .is_some()
        );
    })
    .await;
}

#[tokio::test]
async fn unknown_address_starts_nothing() {
    on_each_engine(async |database| {
        add_robin(database).await;

        let started = recovery::start(database, "nobody@example.com", at(0))
            .await
            .expect("the recovery can be asked for");

        assert!(started.is_none());
        let started = recovery::start(database, "ROBIN@EXAMPLE.COM", at(0))
            .await
            .expect("the recovery can start")
            .expect("the address is known in any letter case");
        assert_eq!(started.user.username, "robin");
        assert!(!format!("{started:?}").contains(&started.password));
    })
    .await;
}

#[tokio::test]
async fn account_without_a_recovery_password_keeps_its_password() {
    on_each_engine(async |database| {
        add_robin(database).await;
        let robin = database
            .users()
            .find_by_username("robin")
            .await
            .expect("the lookup works")
            .expect("the user is there");

        // Two logins with the same recovery password can come at the same moment. The
        // second one finds no recovery password, and must change nothing.
        let used = database
            .users()
            .use_recovery(robin.id)
            .await
            .expect("the change can be asked for");

        assert!(!used);
        assert!(logs_in(database, "the old password", at(0)).await);
    })
    .await;
}

#[tokio::test]
async fn login_with_the_old_password_ends_the_recovery_password() {
    on_each_engine(async |database| {
        add_robin(database).await;
        let password = new_password(database, at(0)).await.expect("a password");

        // The owner of the account still knew the password, so the e-mail is not needed.
        assert!(logs_in(database, "the old password", at(60)).await);

        assert!(!logs_in(database, &password, at(120)).await);
        assert!(logs_in(database, "the old password", at(180)).await);
    })
    .await;
}

#[tokio::test]
async fn new_password_of_the_account_ends_the_recovery_password() {
    on_each_engine(async |database| {
        add_robin(database).await;
        let robin = database
            .users()
            .find_by_username("robin")
            .await
            .expect("the lookup works")
            .expect("the user is there");

        let first = new_password(database, at(0)).await.expect("a password");
        let changes = organizr_domain::UserChanges {
            password_hash: organizr_domain::Change::Set(
                password::hash("a new password").expect("it can be hashed"),
            ),
            ..organizr_domain::UserChanges::none()
        };
        database
            .users()
            .update(robin.id, changes)
            .await
            .expect("the change works");
        assert!(!logs_in(database, &first, at(60)).await);

        let second = new_password(database, at(PAUSE.as_secs()))
            .await
            .expect("a password");
        let hash = password::hash("a third password").expect("it can be hashed");
        database
            .users()
            .set_password_hash(robin.id, &hash)
            .await
            .expect("the change works");
        assert!(!logs_in(database, &second, at(PAUSE.as_secs() + 60)).await);
        assert!(logs_in(database, "a third password", at(PAUSE.as_secs() + 60)).await);
    })
    .await;
}
