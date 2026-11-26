//! The `/token` routes and the session of a request, through the real route table: what
//! the recorded PHP answers cannot show.
//!
//! The answers themselves are compared with PHP in `crates/testkit/tests/parity.rs`. The
//! recorder replaces tokens, addresses and times by placeholders, so these tests look at
//! the real values, and at rules that the recording does not reach.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use organizr_auth::{SessionToken, password};
use organizr_db::Database;
use organizr_domain::{
    GroupLevel, NewSession, NewUser, Session, SessionId, Timestamp, User, UserId,
};
use organizr_settings::Settings;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, request, send};
use organizr_testkit::{app_with, installed_app, seed};
use serde_json::{Value, json};

/// The name of the session cookie of the seeded install.
const SESSION_COOKIE: &str = "organizr_token_5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21";

const USER_PASSWORD: &str = "the password of zoe";

fn now() -> i64 {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is past 1970");
    i64::try_from(since_epoch.as_secs()).expect("the time fits")
}

/// Logs in and returns the session cookie as a browser sends it back.
async fn log_in(app: &Router, name: &str, password: &str, user_agent: &str) -> String {
    let body = json!({"username": name, "password": password}).to_string();
    let headers = [
        ("content-type", "application/json"),
        ("user-agent", user_agent),
    ];
    let answer = send(app, request("POST", "/api/v2/login", &headers, &body)).await;
    answer
        .cookie(SESSION_COOKIE)
        .unwrap_or_else(|| panic!("the login sets the session cookie: {}", answer.body))
}

async fn log_in_as_admin(app: &Router, user_agent: &str) -> String {
    log_in(app, seed::ADMIN_NAME, seed::ADMIN_PASSWORD, user_agent).await
}

async fn get(app: &Router, path: &str, cookie: &str, user_agent: &str) -> Answer {
    let headers = [("cookie", cookie), ("user-agent", user_agent)];
    send(app, request("GET", path, &headers, "")).await
}

async fn delete(app: &Router, id: &str, cookie: &str, headers: &[(&str, &str)]) -> Answer {
    let mut all = vec![("cookie", cookie)];
    all.extend_from_slice(headers);
    send(
        app,
        request("DELETE", &format!("/api/v2/token/{id}"), &all, ""),
    )
    .await
}

async fn validate(app: &Router, target: &str, headers: &[(&str, &str)], body: &str) -> Answer {
    send(app, request("POST", target, headers, body)).await
}

/// The token in a cookie of the form `name=value`.
fn token_of(cookie: &str) -> &str {
    cookie
        .split_once('=')
        .map(|(_, value)| value)
        .expect("the cookie has a value")
}

async fn admin(database: &Database) -> User {
    database
        .users()
        .find_by_username(seed::ADMIN_NAME)
        .await
        .expect("the lookup works")
        .expect("the seed has the admin account")
}

/// A second account, in the group "User" (level 4).
async fn insert_zoe(database: &Database) -> User {
    let group = database
        .groups()
        .find_by_level(GroupLevel::new(4).expect("the level is valid"))
        .await
        .expect("the lookup works")
        .expect("the seed has the group User");
    database
        .users()
        .insert(NewUser {
            username: "zoe".to_owned(),
            email: None,
            password_hash: password::hash(USER_PASSWORD).expect("the password can be hashed"),
            group: group.id,
            image: None,
            created_at: Timestamp::from_unix_seconds(1_800_000_000),
        })
        .await
        .expect("the user can be stored")
}

/// Every stored session of a user, also the ones that have expired by now.
async fn sessions_of(database: &Database, user: UserId) -> Vec<Session> {
    database
        .sessions()
        .list_valid_of_user(user, Timestamp::from_unix_seconds(0))
        .await
        .expect("the sessions can be read")
}

fn installed_app_with(database: &Database, key: &str, value: &Value) -> Router {
    let mut settings = Settings::defaults();
    settings
        .set("uuid", &json!(seed::INSTALL_ID))
        .expect("the value is valid");
    settings.set(key, value).expect("the value is valid");
    app_with(database, settings)
}

#[tokio::test]
async fn me_shows_the_account_and_the_sessions_but_no_token() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        // A login takes long on a slow machine, so the test notes the time around both.
        let start = now();
        let this_browser = log_in_as_admin(&app, "Browser/1").await;
        let other_browser = log_in_as_admin(&app, "Browser/2").await;
        let before = now();

        let answer = get(&app, "/api/v2/token/me", &this_browser, "Browser/1").await;

        let me = answer.data();
        assert_eq!(me["token"], Value::Null);
        assert_eq!(me["username"], json!("admin"));
        assert_eq!(me["group"], json!("Admin"));
        assert_eq!(me["groupID"], json!(0));
        assert_eq!(me["email"], json!("admin@example.com"));
        assert_eq!(me["userID"], json!(admin(database).await.id.get()));
        assert_eq!(me["loggedin"], json!(true));
        assert_eq!(me["locked"], json!(0));
        assert_eq!(me["authService"], json!("internal"));
        // A test request has no connection, so the address is 0.0.0.0. The value is
        // from Python: hashlib.md5(b"0.0.0.0").hexdigest()[:5].
        assert_eq!(me["uid"], json!("f1f17"));

        let list = me["tokenList"].as_array().expect("the list is there");
        let tokens: Vec<&Value> = list.iter().map(|session| &session["token"]).collect();
        assert_eq!(tokens, [&Value::Null, &Value::Null]);
        let current: Vec<&Value> = list.iter().map(|session| &session["current"]).collect();
        assert_eq!(
            current,
            [&json!(true), &json!(false)],
            "the first session is the one of this browser"
        );
        for cookie in [&this_browser, &other_browser] {
            assert!(
                !answer.body.contains(token_of(cookie)),
                "no token is anywhere in the answer"
            );
        }
        assert_eq!(list[0]["ip"], json!("0.0.0.0"));

        // The times are those of the session, in the two forms of the PHP application.
        let stored = sessions_of(database, admin(database).await.id).await;
        let created = stored[0].created_at.unix_seconds();
        assert!((start..=before).contains(&created));
        let text = |seconds: i64| {
            jiff::Timestamp::from_second(seconds)
                .expect("the time is valid")
                .strftime("%Y-%m-%d %H:%M:%S")
                .to_string()
        };
        assert_eq!(list[0]["created"], json!(text(created)));
        assert_eq!(list[0]["expires"], json!(text(created + 86_400)));
        assert_eq!(
            me["tokenDate"],
            json!({"date": format!("{}.000000", text(created)), "timezone_type": 1, "timezone": "+00:00"})
        );
        assert_eq!(
            me["tokenExpire"]["date"],
            json!(format!("{}.000000", text(created + 86_400)))
        );
    })
    .await;
}

#[tokio::test]
async fn expired_session_makes_the_caller_a_guest_and_drops_the_cookie() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let token = SessionToken::generate().expect("the system gives random bytes");
        database
            .sessions()
            .insert(NewSession {
                token_hash: token.hash(),
                user: admin(database).await.id,
                user_agent: None,
                ip: IpAddr::from([192, 0, 2, 7]),
                created_at: Timestamp::from_unix_seconds(now() - 86_400),
                expires_at: Timestamp::from_unix_seconds(now() - 1),
            })
            .await
            .expect("the session can be stored");
        let cookie = format!("{SESSION_COOKIE}={}", token.expose());

        let me = get(&app, "/api/v2/token/me", &cookie, "Browser/1").await;
        let revoke = delete(&app, "1", &cookie, &[]).await;

        assert_eq!(me.data(), Value::Null);
        // Dropped at `/`, and at the two paths of the API that a planted cookie would use
        // (D-145), each for the wide and the own domain.
        let dropped = me.cookies_named(SESSION_COOKIE);
        assert_eq!(dropped.len(), 6, "{:?}", me.cookies);
        for cookie in dropped {
            assert!(
                cookie.starts_with(&format!("{SESSION_COOKIE}=;")),
                "{cookie}"
            );
        }
        // The caller has the rights of a guest.
        assert_eq!(revoke.status, 401, "{}", revoke.body);
        assert_eq!(revoke.message(), json!("Not Authorized"));
    })
    .await;
}

#[tokio::test]
async fn valid_session_keeps_its_cookie() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let cookie = log_in_as_admin(&app, "Browser/1").await;

        for path in ["/api/v2/token/me", "/api/v2/ping", "/api/v2/nothing/here"] {
            let answer = get(&app, path, &cookie, "Browser/2").await;

            assert_eq!(
                answer.cookies_named(SESSION_COOKIE),
                Vec::<&str>::new(),
                "{path}"
            );
        }
    })
    .await;
}

#[tokio::test]
async fn login_with_a_stale_cookie_leaves_the_browser_with_the_new_session() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let stale = format!("{SESSION_COOKIE}={}", "ab".repeat(32));
        let body = json!({"username": seed::ADMIN_NAME, "password": seed::ADMIN_PASSWORD});
        let headers = [("content-type", "application/json"), ("cookie", &stale)];

        let answer = send(
            &app,
            request("POST", "/api/v2/login", &headers, &body.to_string()),
        )
        .await;

        // A browser applies the cookies in order. The deletion of the stale cookie must
        // come first, so the new cookie stays.
        let cookies = answer.cookies_named(SESSION_COOKIE);
        let is_dropped_at_root = |cookie: &&str| {
            cookie.starts_with(&format!("{SESSION_COOKIE}=;")) && cookie.contains("path=/;")
        };
        let with_token: Vec<usize> = (0..cookies.len())
            .filter(|place| !cookies[*place].starts_with(&format!("{SESSION_COOKIE}=;")))
            .collect();
        let dropped_at_root: Vec<usize> = (0..cookies.len())
            .filter(|place| is_dropped_at_root(&cookies[*place]))
            .collect();
        assert_eq!(with_token.len(), 2, "{:?}", answer.cookies);
        assert_eq!(dropped_at_root.len(), 2, "{:?}", answer.cookies);
        assert!(
            dropped_at_root
                .iter()
                .all(|dropped| with_token.iter().all(|kept| dropped < kept))
        );
        let last = cookies[with_token[1]]
            .split(';')
            .next()
            .expect("the cookie has a value");
        assert!(SessionToken::parse(token_of(last)).is_some(), "{last}");
    })
    .await;
}

#[tokio::test]
async fn browser_rule_rejects_a_session_from_another_browser() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, "matchUserAgents", &json!(true));
        let cookie = log_in_as_admin(&app, "Browser/1").await;

        let same = get(&app, "/api/v2/token/me", &cookie, "Browser/1").await;
        let other = get(&app, "/api/v2/token/me", &cookie, "Browser/2").await;

        assert_eq!(same.data()["username"], json!("admin"));
        assert_eq!(same.cookies_named(SESSION_COOKIE), Vec::<&str>::new());
        assert_eq!(other.data(), Value::Null);
        assert_eq!(other.cookies_named(SESSION_COOKIE).len(), 6);

        // Asking about the token from another browser: PHP answers "success" with
        // `false` and no message.
        let target = format!("/api/v2/token/validate?Token={}", token_of(&cookie));
        let asked_from_other = validate(&app, &target, &[("user-agent", "Browser/2")], "").await;
        let asked_from_same = validate(&app, &target, &[("user-agent", "Browser/1")], "").await;
        assert_eq!(asked_from_other.status, 200);
        assert_eq!(asked_from_other.data(), json!(false));
        assert_eq!(asked_from_other.message(), Value::Null);
        assert_eq!(asked_from_same.message(), json!("Token is valid"));
    })
    .await;
}

#[tokio::test]
async fn validate_reads_the_token_from_the_query_or_a_form_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let cookie = log_in_as_admin(&app, "Browser/1").await;
        let token = token_of(&cookie);
        let form = [("content-type", "application/x-www-form-urlencoded")];
        let json_type = [("content-type", "application/json")];

        let in_query = validate(
            &app,
            &format!("/api/v2/token/validate?Token={token}"),
            &[],
            "",
        )
        .await;
        let in_form = validate(
            &app,
            "/api/v2/token/validate",
            &form,
            &format!("Token={token}"),
        )
        .await;
        // The PHP application answers these three with a crash (status 500 and an
        // HTML page). Here they are a text that is no token.
        let missing = validate(&app, "/api/v2/token/validate", &[], "").await;
        let other_name = validate(
            &app,
            &format!("/api/v2/token/validate?token={token}"),
            &[],
            "",
        )
        .await;
        let in_json = validate(
            &app,
            "/api/v2/token/validate",
            &json_type,
            &json!({"Token": token}).to_string(),
        )
        .await;

        for valid in [&in_query, &in_form] {
            assert_eq!(valid.status, 200, "{}", valid.body);
            assert_eq!(valid.data()["username"], json!("admin"));
            assert_eq!(valid.data()["token"], json!(token));
            // The full form of the session list, which has no `current` field.
            assert_eq!(valid.data()["tokenList"][0]["token"], json!(token));
            assert_eq!(valid.data()["tokenList"][0].get("current"), None);
            assert_eq!(valid.data()["tokenList"][0]["browser"], json!("Browser/1"));
            assert_eq!(
                valid.data()["tokenList"][0]["user_id"],
                valid.data()["userID"]
            );
        }
        for refused in [&missing, &other_name, &in_json] {
            assert_eq!(refused.status, 403, "{}", refused.body);
            assert_eq!(refused.message(), json!("Token was invalid"));
            assert_eq!(refused.data(), json!(false));
        }
    })
    .await;
}

#[tokio::test]
async fn user_can_delete_a_session_of_their_own_but_not_of_another_user() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let zoe = insert_zoe(database).await;
        let app = installed_app(database);
        let admin_cookie = log_in_as_admin(&app, "Browser/1").await;
        let zoe_cookie = log_in(&app, "zoe", USER_PASSWORD, "Browser/2").await;
        let zoe_other = log_in(&app, "zoe", USER_PASSWORD, "Browser/3").await;
        let admin_id = admin(database).await.id;
        let ids = async |user| -> Vec<SessionId> {
            sessions_of(database, user)
                .await
                .iter()
                .map(|session| session.id)
                .collect()
        };
        let admin_session = ids(admin_id).await[0];
        let zoe_sessions = ids(zoe.id).await;

        // Zoe names the session of the admin: the answer is the same, nothing happens.
        let foreign = delete(&app, &admin_session.to_string(), &zoe_cookie, &[]).await;
        assert_eq!(foreign.status, 204);
        assert_eq!(foreign.body, "");
        assert_eq!(ids(admin_id).await, [admin_session]);

        // The admin cannot delete a session of Zoe through this route either.
        let by_admin = delete(&app, &zoe_sessions[1].to_string(), &admin_cookie, &[]).await;
        assert_eq!(by_admin.status, 204);
        assert_eq!(ids(zoe.id).await, zoe_sessions);

        // Zoe deletes the session of her other browser.
        let own = delete(&app, &zoe_sessions[1].to_string(), &zoe_cookie, &[]).await;
        assert_eq!(own.status, 204);
        assert_eq!(ids(zoe.id).await, [zoe_sessions[0]]);
        let other_browser = get(&app, "/api/v2/token/me", &zoe_other, "Browser/3").await;
        assert_eq!(
            other_browser.data(),
            Value::Null,
            "that browser is logged out"
        );
        let this_browser = get(&app, "/api/v2/token/me", &zoe_cookie, "Browser/2").await;
        assert_eq!(this_browser.data()["username"], json!("zoe"));
        assert_eq!(this_browser.data()["groupID"], json!(4));
        assert_eq!(this_browser.data()["email"], Value::Null);
    })
    .await;
}

#[tokio::test]
async fn another_site_cannot_delete_a_session() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let cookie = log_in_as_admin(&app, "Browser/1").await;
        let admin_id = admin(database).await.id;
        let session = sessions_of(database, admin_id).await[0].id;

        for headers in [
            [("sec-fetch-site", "cross-site")],
            [("sec-fetch-site", "same-site")],
            [("origin", "https://evil.example.net")],
        ] {
            let answer = delete(&app, &session.to_string(), &cookie, &headers).await;

            assert_eq!(answer.status, 401, "{headers:?}");
            assert_eq!(
                answer.message(),
                json!(format!(
                    "Not authorized for current Route: /api/v2/token/{session}"
                )),
                "{headers:?}"
            );
        }
        assert_eq!(sessions_of(database, admin_id).await.len(), 1);

        let from_here = delete(
            &app,
            &session.to_string(),
            &cookie,
            &[("sec-fetch-site", "same-origin")],
        )
        .await;
        assert_eq!(from_here.status, 204);
        assert_eq!(sessions_of(database, admin_id).await, []);
    })
    .await;
}
