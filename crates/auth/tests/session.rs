//! Finding the user of a session token, on both engines.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::IpAddr;
use std::time::Duration;

use organizr_auth::SessionToken;
use organizr_auth::login::{self, Device, LoggedIn};
use organizr_auth::password;
use organizr_auth::session::{self, Authenticated, BrowserRule, SessionCheck};
use organizr_db::Database;
use organizr_domain::{GroupLevel, NewGroup, NewUser, Timestamp};
use organizr_testkit::engines::on_each_engine;

const PASSWORD: &str = "correct horse battery staple";
const NOW: Timestamp = Timestamp::from_unix_seconds(1_800_000_000);
const ONE_DAY: Duration = Duration::from_hours(24);
const BROWSER: &str = "Mozilla/5.0 (X11; Linux x86_64)";

const fn after(seconds: i64) -> Timestamp {
    Timestamp::from_unix_seconds(NOW.unix_seconds() + seconds)
}

/// Stores a user in a group of level 4 and logs the user in at [`NOW`] for one day.
async fn logged_in_user(database: &Database, user_agent: Option<&str>) -> LoggedIn {
    let group = database
        .groups()
        .insert(NewGroup {
            level: GroupLevel::new(4).expect("the level is valid"),
            name: "User".to_owned(),
            image: Some("plugins/images/groups/user.png".to_owned()),
            is_default: true,
        })
        .await
        .expect("the group can be stored");
    database
        .users()
        .insert(NewUser {
            username: "robin".to_owned(),
            email: Some("robin@example.com".to_owned()),
            password_hash: password::hash(PASSWORD).expect("the password can be hashed"),
            group: group.id,
            image: None,
            created_at: NOW,
        })
        .await
        .expect("the user can be stored");
    let device = Device {
        ip: IpAddr::from([192, 0, 2, 7]),
        user_agent: user_agent.map(str::to_owned),
    };
    login::with_password(
        database,
        "robin",
        PASSWORD,
        organizr_auth::login::SecondFactor::without_code(&organizr_testkit::seed::secrets()),
        device,
        NOW,
        ONE_DAY,
    )
    .await
    .expect("the login succeeds")
}

fn valid(check: SessionCheck) -> Authenticated {
    match check {
        SessionCheck::Valid(authenticated) => *authenticated,
        other => panic!("the session is not valid: {other:?}"),
    }
}

#[tokio::test]
async fn token_of_a_login_gives_the_user_the_group_and_the_session() {
    on_each_engine(async |database| {
        let logged_in = logged_in_user(database, Some(BROWSER)).await;

        let check = session::check(database, &logged_in.token, NOW, BrowserRule::Any)
            .await
            .expect("the lookup works");

        let authenticated = valid(check);
        assert_eq!(authenticated.user, logged_in.user);
        assert_eq!(authenticated.session, logged_in.session);
        assert_eq!(authenticated.group.id, logged_in.user.group);
        assert_eq!(authenticated.group.name, "User");
        assert_eq!(authenticated.group.level.get(), 4);
    })
    .await;
}

#[tokio::test]
async fn token_that_no_login_made_is_unknown() {
    on_each_engine(async |database| {
        logged_in_user(database, Some(BROWSER)).await;
        let other = SessionToken::generate().expect("the system gives random bytes");

        let check = session::check(database, &other, NOW, BrowserRule::Any)
            .await
            .expect("the lookup works");

        assert_eq!(check, SessionCheck::Unknown);
    })
    .await;
}

#[tokio::test]
async fn session_is_valid_until_it_expires() {
    on_each_engine(async |database| {
        let logged_in = logged_in_user(database, Some(BROWSER)).await;
        let at = async |now| {
            session::check(database, &logged_in.token, now, BrowserRule::Any)
                .await
                .expect("the lookup works")
        };

        assert!(matches!(at(after(86_399)).await, SessionCheck::Valid(_)));
        assert_eq!(at(after(86_400)).await, SessionCheck::Unknown);
        assert_eq!(at(after(30 * 86_400)).await, SessionCheck::Unknown);
    })
    .await;
}

#[tokio::test]
async fn deleted_session_is_unknown() {
    on_each_engine(async |database| {
        let logged_in = logged_in_user(database, Some(BROWSER)).await;
        database
            .sessions()
            .delete_by_token(&logged_in.token.hash())
            .await
            .expect("the delete works");

        let check = session::check(database, &logged_in.token, NOW, BrowserRule::Any)
            .await
            .expect("the lookup works");

        assert_eq!(check, SessionCheck::Unknown);
    })
    .await;
}

#[tokio::test]
async fn session_of_a_deleted_user_is_unknown() {
    on_each_engine(async |database| {
        let logged_in = logged_in_user(database, Some(BROWSER)).await;
        database
            .users()
            .delete(logged_in.user.id)
            .await
            .expect("the delete works");

        let check = session::check(database, &logged_in.token, NOW, BrowserRule::Any)
            .await
            .expect("the lookup works");

        assert_eq!(check, SessionCheck::Unknown);
    })
    .await;
}

#[tokio::test]
async fn browser_rule_accepts_only_the_browser_of_the_login() {
    on_each_engine(async |database| {
        let logged_in = logged_in_user(database, Some(BROWSER)).await;
        let with = async |rule| {
            session::check(database, &logged_in.token, NOW, rule)
                .await
                .expect("the lookup works")
        };

        assert!(matches!(
            with(BrowserRule::Same(Some(BROWSER))).await,
            SessionCheck::Valid(_)
        ));
        assert_eq!(
            with(BrowserRule::Same(Some("curl/8.0"))).await,
            SessionCheck::OtherBrowser
        );
        assert_eq!(
            with(BrowserRule::Same(None)).await,
            SessionCheck::OtherBrowser
        );
        // Without the rule, any browser may use the session.
        assert!(matches!(
            with(BrowserRule::Any).await,
            SessionCheck::Valid(_)
        ));
    })
    .await;
}

#[tokio::test]
async fn login_without_a_user_agent_matches_a_request_without_one() {
    on_each_engine(async |database| {
        let logged_in = logged_in_user(database, None).await;
        let with = async |rule| {
            session::check(database, &logged_in.token, NOW, rule)
                .await
                .expect("the lookup works")
        };

        assert!(matches!(
            with(BrowserRule::Same(None)).await,
            SessionCheck::Valid(_)
        ));
        assert_eq!(
            with(BrowserRule::Same(Some(BROWSER))).await,
            SessionCheck::OtherBrowser
        );
    })
    .await;
}

#[tokio::test]
async fn expired_session_is_unknown_also_from_another_browser() {
    on_each_engine(async |database| {
        let logged_in = logged_in_user(database, Some(BROWSER)).await;

        let check = session::check(
            database,
            &logged_in.token,
            after(86_400),
            BrowserRule::Same(Some("curl/8.0")),
        )
        .await
        .expect("the lookup works");

        assert_eq!(check, SessionCheck::Unknown);
    })
    .await;
}
