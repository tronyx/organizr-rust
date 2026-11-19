//! Logging in with a stored password, on both engines.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::IpAddr;
use std::time::Duration;

use organizr_auth::login::{self, Device, LoggedIn, LoginError};
use organizr_auth::password::{self, PasswordCheck};
use organizr_db::Database;
use organizr_domain::{GroupLevel, NewGroup, NewUser, PasswordHash, Timestamp, User, UserId};
use organizr_testkit::engines::on_each_engine;

const PASSWORD: &str = "correct horse battery staple";
const NOW: Timestamp = Timestamp::from_unix_seconds(1_800_000_000);
const ONE_DAY: Duration = Duration::from_hours(24);

/// A bcrypt hash of [`PASSWORD`], as an imported user has one. The cost is the lowest
/// there is, to keep the tests fast.
fn bcrypt_hash() -> PasswordHash {
    PasswordHash::new(bcrypt::hash(PASSWORD, 4).expect("the password can be hashed"))
}

fn argon2_hash() -> PasswordHash {
    password::hash(PASSWORD).expect("the password can be hashed")
}

async fn insert_user(database: &Database, username: &str, password_hash: PasswordHash) -> User {
    let level = u16::try_from(database.users().count().await.expect("the count works"))
        .expect("the tests make few users");
    let group = database
        .groups()
        .insert(NewGroup {
            level: GroupLevel::new(level).expect("the level is valid"),
            name: format!("Group of {username}"),
            image: None,
            is_default: false,
        })
        .await
        .expect("the group can be stored");
    database
        .users()
        .insert(NewUser {
            username: username.to_owned(),
            email: Some(format!("{username}@example.com")),
            password_hash,
            group: group.id,
            image: None,
            created_at: NOW,
        })
        .await
        .expect("the user can be stored")
}

fn device() -> Device {
    Device {
        ip: IpAddr::from([192, 0, 2, 7]),
        user_agent: Some("Mozilla/5.0 (X11; Linux x86_64)".to_owned()),
    }
}

async fn log_in(database: &Database, name: &str, password: &str) -> Result<LoggedIn, LoginError> {
    login::with_password(database, name, password, device(), NOW, ONE_DAY).await
}

async fn stored_hash(database: &Database, username: &str) -> PasswordHash {
    database
        .users()
        .find_by_username(username)
        .await
        .expect("the lookup works")
        .expect("the user exists")
        .password_hash
}

async fn session_count(database: &Database, user: UserId) -> usize {
    database
        .sessions()
        .list_valid_of_user(user, NOW)
        .await
        .expect("the sessions can be read")
        .len()
}

#[tokio::test]
async fn correct_password_gives_a_stored_session_and_its_token() {
    on_each_engine(async |database| {
        let user = insert_user(database, "robin", argon2_hash()).await;

        let logged_in = log_in(database, "robin", PASSWORD)
            .await
            .expect("the login succeeds");

        assert_eq!(logged_in.user, user);
        let session = &logged_in.session;
        assert_eq!(session.user, user.id);
        assert_eq!(session.token_hash, logged_in.token.hash());
        assert_eq!(session.ip, device().ip);
        assert_eq!(session.user_agent, device().user_agent);
        assert_eq!(session.created_at, NOW);
        assert_eq!(
            session.expires_at.unix_seconds() - NOW.unix_seconds(),
            86_400
        );
        let found = database
            .sessions()
            .find_valid(&logged_in.token.hash(), NOW)
            .await
            .expect("the lookup works");
        assert_eq!(found.as_ref(), Some(session));
    })
    .await;
}

#[tokio::test]
async fn name_and_address_work_in_any_letter_case() {
    on_each_engine(async |database| {
        let user = insert_user(database, "Robin", argon2_hash()).await;

        for name in [
            "Robin",
            "robin",
            "ROBIN",
            "robin@example.com",
            "Robin@Example.COM",
        ] {
            let logged_in = log_in(database, name, PASSWORD)
                .await
                .unwrap_or_else(|error| panic!("the login as `{name}` fails: {error}"));
            assert_eq!(logged_in.user.id, user.id, "logged in as `{name}`");
        }
    })
    .await;
}

#[tokio::test]
async fn wrong_password_and_unknown_name_fail_in_the_same_way_and_store_nothing() {
    on_each_engine(async |database| {
        let user = insert_user(database, "robin", argon2_hash()).await;
        let attempts = [
            ("robin", "not the password"),
            ("robin", ""),
            ("robin", "correct horse battery stapl"),
            ("nobody", PASSWORD),
            ("", PASSWORD),
            ("", ""),
            (" robin", PASSWORD),
        ];

        for (name, password) in attempts {
            let result = log_in(database, name, password).await;

            assert!(
                matches!(result, Err(LoginError::WrongCredentials)),
                "`{name}` with `{password}`: {result:?}"
            );
        }
        assert_eq!(session_count(database, user.id).await, 0);
    })
    .await;
}

#[tokio::test]
async fn every_login_gets_a_session_and_a_token_of_its_own() {
    on_each_engine(async |database| {
        let user = insert_user(database, "robin", argon2_hash()).await;

        let first = log_in(database, "robin", PASSWORD)
            .await
            .expect("the login succeeds");
        let second = log_in(database, "robin", PASSWORD)
            .await
            .expect("the login succeeds");

        assert_ne!(first.token.expose(), second.token.expose());
        assert_ne!(first.session.id, second.session.id);
        assert_eq!(session_count(database, user.id).await, 2);
    })
    .await;
}

#[tokio::test]
async fn session_lasts_as_long_as_the_caller_says() {
    on_each_engine(async |database| {
        insert_user(database, "robin", argon2_hash()).await;
        let week = Duration::from_hours(7 * 24);

        let logged_in = login::with_password(database, "robin", PASSWORD, device(), NOW, week)
            .await
            .expect("the login succeeds");

        assert_eq!(
            logged_in.session.expires_at.unix_seconds() - NOW.unix_seconds(),
            604_800
        );
    })
    .await;
}

#[tokio::test]
async fn login_with_a_bcrypt_hash_replaces_it_by_an_argon2_hash() {
    on_each_engine(async |database| {
        let old_hash = bcrypt_hash();
        insert_user(database, "robin", old_hash.clone()).await;

        log_in(database, "robin", PASSWORD)
            .await
            .expect("the login with the bcrypt hash succeeds");

        let renewed = stored_hash(database, "robin").await;
        assert!(
            renewed.expose().starts_with("$argon2id$"),
            "the stored hash was not renewed"
        );
        assert_eq!(
            password::verify(PASSWORD, &renewed),
            PasswordCheck::Correct { outdated: false }
        );

        // The next login finds a current hash and leaves it as it is.
        log_in(database, "robin", PASSWORD)
            .await
            .expect("the login with the new hash succeeds");
        assert_eq!(stored_hash(database, "robin").await, renewed);
    })
    .await;
}

#[tokio::test]
async fn wrong_password_does_not_touch_a_bcrypt_hash() {
    on_each_engine(async |database| {
        let old_hash = bcrypt_hash();
        insert_user(database, "robin", old_hash.clone()).await;

        let result = log_in(database, "robin", "not the password").await;

        assert!(
            matches!(result, Err(LoginError::WrongCredentials)),
            "{result:?}"
        );
        assert_eq!(stored_hash(database, "robin").await, old_hash);
    })
    .await;
}

#[tokio::test]
async fn user_with_a_stored_value_that_is_no_hash_cannot_log_in() {
    on_each_engine(async |database| {
        // The value is the password itself, as a broken import could leave it.
        let user = insert_user(database, "robin", PasswordHash::new(PASSWORD.to_owned())).await;

        for password in [PASSWORD, ""] {
            let result = log_in(database, "robin", password).await;

            assert!(
                matches!(result, Err(LoginError::WrongCredentials)),
                "`{password}`: {result:?}"
            );
        }
        assert_eq!(session_count(database, user.id).await, 0);
    })
    .await;
}
