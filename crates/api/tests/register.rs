//! `POST /users/register` through the real route table: what the recorded PHP answers
//! cannot show.
//!
//! `crates/testkit/tests/parity.rs` replays registrations that PHP answered. These tests
//! look at the account a registration creates, at two mistakes of PHP that are not
//! copied, and at the limit on wrong registration passwords, which PHP does not have.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::SocketAddr;

use axum::Router;
use axum::extract::ConnectInfo;
use organizr_auth::password::{self, PasswordCheck};
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, request, send};
use organizr_testkit::{app, installed_app, installed_app_with, seed};
use serde_json::{Value, json};

const REGISTRATION_PASSWORD: &str = "let new users in";
const HOME: &str = "198.51.100.7";
const ELSEWHERE: &str = "203.0.113.50";

/// The application of an install whose registration password is
/// [`REGISTRATION_PASSWORD`], with other values for some settings.
fn app_with_registration(database: &Database, changed: &[(&str, Value)]) -> Router {
    let hash = password::hash(REGISTRATION_PASSWORD).expect("the password can be hashed");
    let mut settings = vec![("registrationPassword", json!(hash.expose()))];
    settings.extend_from_slice(changed);
    installed_app_with(database, &settings)
}

fn zoe() -> Value {
    json!({
        "username": "Zoe",
        "password": "the password of zoe",
        "email": "Zoe@Example.com",
        "registrationPassword": REGISTRATION_PASSWORD,
    })
}

fn with(mut body: Value, key: &str, value: Value) -> Value {
    body.as_object_mut()
        .expect("the body is an object")
        .insert(key.to_owned(), value);
    body
}

async fn register_from(
    app: &Router,
    address: &str,
    body: &Value,
    headers: &[(&str, &str)],
) -> Answer {
    let mut all = vec![("content-type", "application/json")];
    all.extend_from_slice(headers);
    let mut request = request("POST", "/api/v2/users/register", &all, &body.to_string());
    let peer: SocketAddr = format!("{address}:50000")
        .parse()
        .expect("the address is valid");
    request.extensions_mut().insert(ConnectInfo(peer));
    send(app, request).await
}

async fn register(app: &Router, body: &Value) -> Answer {
    register_from(app, HOME, body, &[]).await
}

#[tokio::test]
async fn registration_creates_an_account_in_the_default_group_and_logs_it_in() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with_registration(database, &[]);

        let answer = register(&app, &zoe()).await;

        assert_eq!(answer.status, 200, "{}", answer.body);
        assert_eq!(answer.message(), json!("User created"));
        let user = database
            .users()
            .find_by_username("zoe")
            .await
            .expect("the lookup works")
            .expect("the account is stored");
        assert_eq!(user.username, "Zoe");
        assert_eq!(user.email.as_deref(), Some("Zoe@Example.com"));
        assert_eq!(
            user.image.as_deref(),
            Some("https://www.gravatar.com/avatar/76cec4168e20ad317a8afb3a81ae3fce?s=100&d=mm")
        );
        assert_eq!(
            password::verify("the password of zoe", &user.password_hash),
            PasswordCheck::Correct { outdated: false }
        );
        assert!(!user.locked);
        let group = database
            .groups()
            .find(user.group)
            .await
            .expect("the lookup works")
            .expect("the group exists");
        assert_eq!((group.name.as_str(), group.level.get()), ("User", 4));

        // The answer logs the account in: a cookie that scripts cannot read, for the
        // seven days of the `rememberMeDays` setting.
        let cookie_name = format!("organizr_token_{}", seed::INSTALL_ID);
        let cookies = answer.cookies_named(&cookie_name);
        assert_eq!(cookies.len(), 2, "{:?}", answer.cookies);
        for cookie in &cookies {
            assert!(cookie.ends_with("; SameSite=Lax; HttpOnly"), "{cookie}");
        }
        let session = answer.cookie(&cookie_name).expect("the cookie is set");
        let me = send(
            &app,
            request("GET", "/api/v2/token/me", &[("cookie", &session)], ""),
        )
        .await;
        assert_eq!(me.data()["username"], json!("Zoe"));
        assert_eq!(me.data()["groupID"], json!(4));
        let sessions = database
            .sessions()
            .list_valid_of_user(user.id, user.created_at)
            .await
            .expect("the sessions can be read");
        assert_eq!(sessions.len(), 1);
        assert_eq!(
            sessions[0].expires_at.unix_seconds() - sessions[0].created_at.unix_seconds(),
            7 * 86_400
        );
        assert_eq!(sessions[0].ip.to_string(), HOME);
    })
    .await;
}

#[tokio::test]
async fn new_account_stays_logged_in_for_the_days_of_the_setting() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with_registration(database, &[("rememberMeDays", json!("30"))]);

        let answer = register(&app, &zoe()).await;

        assert_eq!(answer.status, 200, "{}", answer.body);
        let user = database
            .users()
            .find_by_username("zoe")
            .await
            .expect("the lookup works")
            .expect("the account is stored");
        let sessions = database
            .sessions()
            .list_valid_of_user(user.id, user.created_at)
            .await
            .expect("the sessions can be read");
        assert_eq!(
            sessions[0].expires_at.unix_seconds() - sessions[0].created_at.unix_seconds(),
            30 * 86_400
        );
    })
    .await;
}

#[tokio::test]
async fn name_of_spaces_creates_no_account() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with_registration(database, &[]);

        // PHP creates an account without a name for this request.
        let answer = register(&app, &with(zoe(), "username", json!("   "))).await;

        assert_eq!(answer.status, 409, "{}", answer.body);
        assert_eq!(answer.message(), json!("Username was set but empty"));
        assert_eq!(database.users().count().await.expect("the count works"), 1);
    })
    .await;
}

#[tokio::test]
async fn name_is_stored_as_it_was_entered() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with_registration(database, &[]);

        // PHP stores `&lt;b&gt;Zoe&lt;/b&gt; &amp; Co` (D-24).
        let answer = register(&app, &with(zoe(), "username", json!("<b>Zoe</b> & Co"))).await;

        assert_eq!(answer.status, 200, "{}", answer.body);
        let user = database
            .users()
            .find_by_login("zoe@example.com")
            .await
            .expect("the lookup works")
            .expect("the account is stored");
        assert_eq!(user.username, "<b>Zoe</b> & Co");
    })
    .await;
}

#[tokio::test]
async fn limits_count_bytes_as_php_counts_them() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with_registration(database, &[]);

        // 26 letters of two bytes each: 52 bytes.
        let too_long = register(&app, &with(zoe(), "username", json!("é".repeat(26)))).await;
        let fits = register(&app, &with(zoe(), "username", json!("é".repeat(25)))).await;

        assert_eq!(too_long.status, 409, "{}", too_long.body);
        assert_eq!(too_long.message(), json!("String is over limit of: 50"));
        assert_eq!(fits.status, 200, "{}", fits.body);
    })
    .await;
}

#[tokio::test]
async fn install_without_a_registration_password_registers_nobody() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // No registration password is stored: the setting is empty.
        let app = installed_app(database);

        for sent in [REGISTRATION_PASSWORD, "anything", " "] {
            let body = with(zoe(), "registrationPassword", json!(sent));
            let answer = register(&app, &body).await;

            assert_eq!(answer.status, 401, "{sent:?}: {}", answer.body);
            assert_eq!(
                answer.message(),
                json!("Registration Password was incorrect")
            );
        }
        assert_eq!(database.users().count().await.expect("the count works"), 1);
    })
    .await;
}

#[tokio::test]
async fn registration_password_of_the_set_up_works_at_once() {
    on_each_engine(async |database| {
        let app = app(database);
        let set_up = json!({
            "setupCode": seed::SETUP_CODE,
            "license": "personal",
            "api": "abcdefghij0123456789",
            "registrationPassword": REGISTRATION_PASSWORD,
            "username": "admin",
            "password": "the password of the admin",
            "email": "admin@example.com",
        });
        let headers = [("content-type", "application/json")];
        let done = send(
            &app,
            request("POST", "/api/v2/wizard", &headers, &set_up.to_string()),
        )
        .await;
        assert_eq!(done.status, 200, "{}", done.body);

        // In PHP this fails until an admin saves the registration password again.
        let answer = register(&app, &zoe()).await;

        assert_eq!(answer.status, 200, "{}", answer.body);
        assert_eq!(database.users().count().await.expect("the count works"), 2);
    })
    .await;
}

#[tokio::test]
async fn wrong_registration_passwords_lock_the_registration() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with_registration(database, &[]);
        let wrong = with(zoe(), "registrationPassword", json!("a guess"));

        for attempt in 1..=4 {
            let answer = register_from(&app, HOME, &wrong, &[]).await;
            assert_eq!(answer.status, 401, "attempt {attempt}: {}", answer.body);
        }
        let fifth = register_from(&app, HOME, &wrong, &[]).await;
        assert_eq!(fifth.status, 403, "{}", fifth.body);
        assert_eq!(fifth.message(), json!("User is locked out"));
        assert!(fifth.headers.contains_key("retry-after"));

        // The right password is refused now, from this address and from any other:
        // many addresses that guess one password are stopped like one address.
        for address in [HOME, ELSEWHERE] {
            let answer = register_from(&app, address, &zoe(), &[]).await;
            assert_eq!(answer.status, 403, "{address}: {}", answer.body);
        }
        assert_eq!(database.users().count().await.expect("the count works"), 1);

        // A request that lacks a field is answered before the lock is looked at.
        let incomplete = register_from(&app, ELSEWHERE, &json!({}), &[]).await;
        assert_eq!(incomplete.status, 422, "{}", incomplete.body);

        // Logging in from another address is not locked.
        let login = json!({"username": seed::ADMIN_NAME, "password": seed::ADMIN_PASSWORD});
        let mut request = request(
            "POST",
            "/api/v2/login",
            &[("content-type", "application/json")],
            &login.to_string(),
        );
        let peer: SocketAddr = format!("{ELSEWHERE}:50000")
            .parse()
            .expect("the address is valid");
        request.extensions_mut().insert(ConnectInfo(peer));
        let logged_in = send(&app, request).await;
        assert_eq!(logged_in.status, 200, "{}", logged_in.body);
    })
    .await;
}

#[tokio::test]
async fn another_site_cannot_register_an_account() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with_registration(database, &[]);

        let answer = register_from(&app, HOME, &zoe(), &[("sec-fetch-site", "cross-site")]).await;

        assert_eq!(answer.status, 401, "{}", answer.body);
        assert_eq!(
            answer.message(),
            json!("Not authorized for current Route: /api/v2/users/register")
        );
        assert_eq!(database.users().count().await.expect("the count works"), 1);
    })
    .await;
}
