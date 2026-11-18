//! Storage of sessions, on both engines.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::IpAddr;

use organizr_db::{Database, DbError};
use organizr_domain::{
    GroupLevel, NewGroup, NewSession, NewUser, PasswordHash, Session, SessionId, SessionTokenHash,
    Timestamp, User, UserId,
};
use organizr_testkit::engines::on_each_engine;

/// The moment the tests call "now".
const NOW: Timestamp = Timestamp::from_unix_seconds(1_800_000_000);

/// A moment the given number of seconds after [`NOW`], or before it when negative.
const fn after(seconds: i64) -> Timestamp {
    Timestamp::from_unix_seconds(NOW.unix_seconds() + seconds)
}

/// A token hash that differs for each number: 64 hexadecimal characters.
fn token_hash(number: u8) -> SessionTokenHash {
    SessionTokenHash::new(format!("{number:02x}").repeat(32))
}

/// A user with the given name in a group of its own.
async fn insert_user(database: &Database, username: &str) -> User {
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
            email: None,
            password_hash: PasswordHash::new("$2y$10$not.a.real.hash".to_owned()),
            group: group.id,
            image: None,
            created_at: NOW,
        })
        .await
        .expect("the user can be stored")
}

/// A session of the given user that is valid for one more hour.
fn new_session(number: u8, user: UserId) -> NewSession {
    NewSession {
        token_hash: token_hash(number),
        user,
        user_agent: Some("Mozilla/5.0 (X11; Linux x86_64)".to_owned()),
        ip: IpAddr::from([192, 0, 2, 7]),
        created_at: NOW,
        expires_at: after(3600),
    }
}

async fn insert_session(database: &Database, number: u8, user: UserId) -> Session {
    database
        .sessions()
        .insert(new_session(number, user))
        .await
        .expect("the session can be stored")
}

async fn valid_ids(database: &Database, user: UserId) -> Vec<SessionId> {
    database
        .sessions()
        .list_valid_of_user(user, NOW)
        .await
        .expect("the sessions can be read")
        .into_iter()
        .map(|session| session.id)
        .collect()
}

#[tokio::test]
async fn session_comes_back_as_it_was_stored() {
    on_each_engine(async |database| {
        let user = insert_user(database, "chris").await;
        // Dates after 2038 do not fit a 32-bit column.
        let created_at = Timestamp::from_unix_seconds(4_102_444_800);
        let expires_at = Timestamp::from_unix_seconds(4_103_049_600);
        let ip: IpAddr = "2001:db8::8a2e:370:7334"
            .parse()
            .expect("the address is valid");

        let stored = database
            .sessions()
            .insert(NewSession {
                user_agent: Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Ünïcödé".to_owned()),
                ip,
                created_at,
                expires_at,
                ..new_session(1, user.id)
            })
            .await
            .expect("the session can be stored");

        assert_eq!(stored.token_hash, token_hash(1));
        assert_eq!(stored.user, user.id);
        assert_eq!(
            stored.user_agent.as_deref(),
            Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Ünïcödé")
        );
        assert_eq!(stored.ip, ip);
        assert_eq!(stored.created_at, created_at);
        assert_eq!(stored.expires_at, expires_at);

        let found = database
            .sessions()
            .find_valid(&token_hash(1), created_at)
            .await
            .expect("the lookup works");
        assert_eq!(found, Some(stored));
    })
    .await;
}

#[tokio::test]
async fn session_without_a_user_agent_can_be_stored() {
    on_each_engine(async |database| {
        let user = insert_user(database, "chris").await;

        let stored = database
            .sessions()
            .insert(NewSession {
                user_agent: None,
                ..new_session(1, user.id)
            })
            .await
            .expect("the session can be stored");

        assert_eq!(stored.user_agent, None);
    })
    .await;
}

#[tokio::test]
async fn unknown_token_finds_no_session() {
    on_each_engine(async |database| {
        let user = insert_user(database, "chris").await;
        insert_session(database, 1, user.id).await;

        let found = database
            .sessions()
            .find_valid(&token_hash(2), NOW)
            .await
            .expect("the lookup works");

        assert_eq!(found, None);
    })
    .await;
}

#[tokio::test]
async fn session_is_valid_until_the_moment_it_expires() {
    on_each_engine(async |database| {
        let user = insert_user(database, "chris").await;
        let session = insert_session(database, 1, user.id).await;
        let expiry = session.expires_at.unix_seconds();

        let one_second_before = database
            .sessions()
            .find_valid(&token_hash(1), Timestamp::from_unix_seconds(expiry - 1))
            .await
            .expect("the lookup works");
        let at_expiry = database
            .sessions()
            .find_valid(&token_hash(1), Timestamp::from_unix_seconds(expiry))
            .await
            .expect("the lookup works");
        let later = database
            .sessions()
            .find_valid(
                &token_hash(1),
                Timestamp::from_unix_seconds(expiry + 86_400),
            )
            .await
            .expect("the lookup works");

        assert_eq!(one_second_before, Some(session));
        assert_eq!(at_expiry, None);
        assert_eq!(later, None);
    })
    .await;
}

#[tokio::test]
async fn two_sessions_cannot_share_a_token() {
    on_each_engine(async |database| {
        let first = insert_user(database, "chris").await;
        let second = insert_user(database, "zoe").await;
        insert_session(database, 1, first.id).await;

        let result = database.sessions().insert(new_session(1, second.id)).await;

        assert!(matches!(result, Err(DbError::Duplicate)), "{result:?}");
    })
    .await;
}

#[tokio::test]
async fn session_cannot_belong_to_a_user_that_does_not_exist() {
    on_each_engine(async |database| {
        let result = database
            .sessions()
            .insert(new_session(1, UserId::new(12_345)))
            .await;

        assert!(
            matches!(result, Err(DbError::ReferenceViolation)),
            "{result:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn valid_sessions_of_a_user_are_listed_oldest_first() {
    on_each_engine(async |database| {
        let chris = insert_user(database, "chris").await;
        let zoe = insert_user(database, "zoe").await;
        let first = insert_session(database, 1, chris.id).await;
        insert_session(database, 2, zoe.id).await;
        let expired = database
            .sessions()
            .insert(NewSession {
                expires_at: NOW,
                ..new_session(3, chris.id)
            })
            .await
            .expect("the session can be stored");
        let last = insert_session(database, 4, chris.id).await;

        let listed = database
            .sessions()
            .list_valid_of_user(chris.id, NOW)
            .await
            .expect("the sessions can be read");

        assert_eq!(listed, [first, last]);
        assert!(!listed.contains(&expired));
    })
    .await;
}

#[tokio::test]
async fn session_is_deleted_by_its_token_once() {
    on_each_engine(async |database| {
        let user = insert_user(database, "chris").await;
        insert_session(database, 1, user.id).await;
        let other = insert_session(database, 2, user.id).await;

        let first = database
            .sessions()
            .delete_by_token(&token_hash(1))
            .await
            .expect("the delete works");
        let second = database
            .sessions()
            .delete_by_token(&token_hash(1))
            .await
            .expect("the delete works");

        assert!(first, "the session existed");
        assert!(!second, "the session is gone");
        assert_eq!(valid_ids(database, user.id).await, [other.id]);
    })
    .await;
}

#[tokio::test]
async fn user_can_delete_only_a_session_of_their_own() {
    on_each_engine(async |database| {
        let chris = insert_user(database, "chris").await;
        let zoe = insert_user(database, "zoe").await;
        let of_chris = insert_session(database, 1, chris.id).await;
        let of_zoe = insert_session(database, 2, zoe.id).await;

        let foreign = database
            .sessions()
            .delete_one_of_user(chris.id, of_zoe.id)
            .await
            .expect("the delete works");
        let own = database
            .sessions()
            .delete_one_of_user(chris.id, of_chris.id)
            .await
            .expect("the delete works");
        let again = database
            .sessions()
            .delete_one_of_user(chris.id, of_chris.id)
            .await
            .expect("the delete works");

        assert!(!foreign, "the session of another user stays");
        assert!(own, "the own session existed");
        assert!(!again, "the own session is gone");
        assert_eq!(valid_ids(database, chris.id).await, []);
        assert_eq!(valid_ids(database, zoe.id).await, [of_zoe.id]);
    })
    .await;
}

#[tokio::test]
async fn all_sessions_of_a_user_are_deleted_at_once() {
    on_each_engine(async |database| {
        let chris = insert_user(database, "chris").await;
        let zoe = insert_user(database, "zoe").await;
        insert_session(database, 1, chris.id).await;
        insert_session(database, 2, chris.id).await;
        let of_zoe = insert_session(database, 3, zoe.id).await;

        let deleted = database
            .sessions()
            .delete_all_of_user(chris.id)
            .await
            .expect("the delete works");
        let again = database
            .sessions()
            .delete_all_of_user(chris.id)
            .await
            .expect("the delete works");

        assert_eq!((deleted, again), (2, 0));
        assert_eq!(valid_ids(database, chris.id).await, []);
        assert_eq!(valid_ids(database, zoe.id).await, [of_zoe.id]);
    })
    .await;
}

#[tokio::test]
async fn expired_sessions_are_purged_and_valid_ones_stay() {
    on_each_engine(async |database| {
        let user = insert_user(database, "chris").await;
        for (number, expires_at) in [(1, after(-86_400)), (2, after(-1)), (3, NOW)] {
            database
                .sessions()
                .insert(NewSession {
                    expires_at,
                    ..new_session(number, user.id)
                })
                .await
                .expect("the session can be stored");
        }
        let valid = database
            .sessions()
            .insert(NewSession {
                expires_at: after(1),
                ..new_session(4, user.id)
            })
            .await
            .expect("the session can be stored");

        let purged = database
            .sessions()
            .delete_expired(NOW)
            .await
            .expect("the purge works");
        let again = database
            .sessions()
            .delete_expired(NOW)
            .await
            .expect("the purge works");

        assert_eq!((purged, again), (3, 0));
        // Reading at an early time shows every row that is left.
        let left = database
            .sessions()
            .list_valid_of_user(user.id, Timestamp::from_unix_seconds(0))
            .await
            .expect("the sessions can be read");
        assert_eq!(left, [valid]);
    })
    .await;
}

#[tokio::test]
async fn deleting_a_user_deletes_their_sessions() {
    on_each_engine(async |database| {
        let chris = insert_user(database, "chris").await;
        let zoe = insert_user(database, "zoe").await;
        insert_session(database, 1, chris.id).await;
        insert_session(database, 2, chris.id).await;
        let of_zoe = insert_session(database, 3, zoe.id).await;

        let first = database
            .users()
            .delete(chris.id)
            .await
            .expect("the delete works");
        let second = database
            .users()
            .delete(chris.id)
            .await
            .expect("the delete works");

        assert!(first, "the user existed");
        assert!(!second, "the user is gone");
        for number in [1, 2] {
            let found = database
                .sessions()
                .find_valid(&token_hash(number), NOW)
                .await
                .expect("the lookup works");
            assert_eq!(found, None, "session {number}");
        }
        assert_eq!(valid_ids(database, zoe.id).await, [of_zoe.id]);
        let users = database.users().count().await.expect("the count works");
        assert_eq!(users, 1);
    })
    .await;
}
