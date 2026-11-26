//! `POST /login` through the real route table: what the recorded PHP answers cannot show.
//!
//! The answers themselves are compared with PHP in `crates/testkit/tests/parity.rs`. These
//! tests look at what a login stores, at the cookie itself, and at requests that the PHP
//! application answers with a crash.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::SocketAddr;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use organizr_api::{AppState, router};
use organizr_auth::SessionToken;
use organizr_auth::password::{self, PasswordCheck};
use organizr_db::Database;
use organizr_domain::{NewUser, Session, Timestamp, User};
use organizr_settings::Settings;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::{NO_LOGIN_LIMIT, app_with, installed_app, installed_app_with, seed};
use serde_json::{Value, json};
use tower::ServiceExt;

/// The name of the session cookie of the seeded install.
const SESSION_COOKIE: &str = "organizr_token_5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21";

/// What a refused login answers.
const WRONG_CREDENTIALS: &str = "{\n    \"response\": {\n        \"result\": \"error\",\n        \"message\": \"User credentials incorrect\",\n        \"data\": null\n    }\n}";

/// The parts of an answer the tests look at.
struct Answer {
    status: StatusCode,
    cookies: Vec<String>,
    body: String,
}

impl Answer {
    /// The `Set-Cookie` values of the session cookie.
    /// The session cookies that carry a token. The answer also drops cookies of the same
    /// name at the paths of the API (D-145).
    fn session_cookies(&self) -> Vec<&str> {
        self.cookies
            .iter()
            .map(String::as_str)
            .filter(|cookie| cookie.starts_with("organizr_token_") && !cookie.contains("=;"))
            .collect()
    }

    /// The token of the session cookie.
    fn token(&self) -> SessionToken {
        let cookie = self
            .session_cookies()
            .first()
            .copied()
            .expect("the answer sets the session cookie");
        let value = cookie
            .strip_prefix(SESSION_COOKIE)
            .and_then(|rest| rest.strip_prefix('='))
            .and_then(|rest| rest.split(';').next())
            .expect("the cookie has the name of the install and a value");
        SessionToken::parse(value).expect("the value of the cookie is a session token")
    }
}

/// Sends a login request with a JSON body from the given address.
async fn post_login(app: &Router, body: &Value, headers: &[(&str, &str)], peer: &str) -> Answer {
    let peer: SocketAddr = peer.parse().expect("the address is valid");
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/v2/login")
        .header(header::HOST, "organizr.example.com")
        .header(header::CONTENT_TYPE, "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut request = request
        .body(Body::from(body.to_string()))
        .expect("the request is valid");
    request.extensions_mut().insert(ConnectInfo(peer));

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers");
    let status = response.status();
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().expect("a cookie is text").to_owned())
        .collect();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("the body can be read")
        .to_bytes();
    Answer {
        status,
        cookies,
        body: String::from_utf8(body.to_vec()).expect("the body is UTF-8"),
    }
}

fn admin() -> Value {
    json!({"username": seed::ADMIN_NAME, "password": seed::ADMIN_PASSWORD})
}

async fn admin_user(database: &Database) -> User {
    database
        .users()
        .find_by_username(seed::ADMIN_NAME)
        .await
        .expect("the lookup works")
        .expect("the seed has the admin account")
}

/// Every session of the admin account, also the ones that have expired by now.
async fn admin_sessions(database: &Database) -> Vec<Session> {
    let admin = admin_user(database).await;
    database
        .sessions()
        .list_valid_of_user(admin.id, Timestamp::from_unix_seconds(0))
        .await
        .expect("the sessions can be read")
}

#[tokio::test]
async fn login_stores_a_session_for_the_token_of_the_cookie() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        let answer = post_login(
            &app,
            &admin(),
            &[("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")],
            "198.51.100.7:50000",
        )
        .await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let session_cookies = answer.session_cookies();
        assert_eq!(session_cookies.len(), 2, "{:?}", answer.cookies);
        // A cookie of the same name that another host planted for the API is dropped.
        for path in ["path=/api;", "path=/api/v2;"] {
            let dropped = answer
                .cookies
                .iter()
                .filter(|cookie| cookie.starts_with("organizr_token_") && cookie.contains("=;"))
                .filter(|cookie| cookie.contains(path))
                .count();
            assert_eq!(dropped, 2, "{path}: {:?}", answer.cookies);
        }
        let value = |cookie: &str| cookie.split(';').next().map(str::to_owned);
        assert_eq!(
            session_cookies.first().copied().and_then(value),
            session_cookies.get(1).copied().and_then(value),
            "both cookies carry the same token"
        );

        let sessions = admin_sessions(database).await;
        let [session] = sessions.as_slice() else {
            panic!("one session is stored, not {}", sessions.len());
        };
        let token = answer.token();
        assert_eq!(session.token_hash, token.hash());
        assert_ne!(
            session.token_hash.as_str(),
            token.expose(),
            "the token itself is not stored"
        );
        assert_eq!(session.ip.to_string(), "198.51.100.7");
        assert_eq!(
            session.user_agent.as_deref(),
            Some("Mozilla/5.0 (X11; Linux x86_64)")
        );
        assert_eq!(
            session.expires_at.unix_seconds() - session.created_at.unix_seconds(),
            86_400
        );
    })
    .await;
}

#[tokio::test]
async fn first_login_replaces_the_hash_that_php_made() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        assert!(
            admin_user(database)
                .await
                .password_hash
                .expose()
                .starts_with("$2y$"),
            "the seed has a hash from PHP"
        );

        let answer = post_login(&app, &admin(), &[], "198.51.100.7:50000").await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let renewed = admin_user(database).await.password_hash;
        assert!(renewed.expose().starts_with("$argon2id$"));
        assert_eq!(
            password::verify(seed::ADMIN_PASSWORD, &renewed),
            PasswordCheck::Correct { outdated: false }
        );
        // The password still works.
        let again = post_login(&app, &admin(), &[], "198.51.100.7:50000").await;
        assert_eq!(again.status, StatusCode::OK, "{}", again.body);
    })
    .await;
}

#[tokio::test]
async fn refused_login_stores_nothing_and_sets_no_session_cookie() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // This test is not about the login limit, and it fails more logins than the limit.
        let app = installed_app_with(database, &[("loginAttempts", json!(NO_LOGIN_LIMIT))]);
        let attempts = [
            json!({"username": seed::ADMIN_NAME, "password": "not-it"}),
            json!({"username": "nobody", "password": seed::ADMIN_PASSWORD}),
            json!({}),
            // The PHP application answers these with a crash (status 500 and an HTML
            // page). Here they are a name or a password that fits no user.
            json!({"username": [seed::ADMIN_NAME], "password": seed::ADMIN_PASSWORD}),
            json!({"username": seed::ADMIN_NAME, "password": [seed::ADMIN_PASSWORD]}),
            json!({"username": {"a": 1}, "password": {"b": 2}}),
        ];

        for attempt in attempts {
            let answer = post_login(&app, &attempt, &[], "198.51.100.7:50000").await;

            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{attempt}");
            assert_eq!(answer.body, WRONG_CREDENTIALS, "{attempt}");
            assert_eq!(answer.session_cookies(), Vec::<&str>::new(), "{attempt}");
        }
        assert_eq!(admin_sessions(database).await, []);
        assert!(
            admin_user(database)
                .await
                .password_hash
                .expose()
                .starts_with("$2y$"),
            "a refused login does not renew the hash"
        );
    })
    .await;
}

#[tokio::test]
async fn password_of_digits_can_be_sent_as_a_number() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let admin = admin_user(database).await;
        database
            .users()
            .insert(NewUser {
                username: "digits".to_owned(),
                email: None,
                password_hash: password::hash("123456").expect("the password can be hashed"),
                group: admin.group,
                image: None,
                created_at: Timestamp::from_unix_seconds(1_800_000_000),
            })
            .await
            .expect("the user can be stored");
        let app = installed_app(database);

        let as_number = post_login(
            &app,
            &json!({"username": "digits", "password": 123_456}),
            &[],
            "198.51.100.7:50000",
        )
        .await;
        let other_number = post_login(
            &app,
            &json!({"username": "digits", "password": 123_457}),
            &[],
            "198.51.100.7:50000",
        )
        .await;

        assert_eq!(as_number.status, StatusCode::OK, "{}", as_number.body);
        assert_eq!(other_number.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn remembered_login_lasts_the_days_of_the_setting_but_at_most_a_year() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let body = json!({
            "username": seed::ADMIN_NAME,
            "password": seed::ADMIN_PASSWORD,
            "remember": true,
        });

        for (setting, days) in [("30", 30), ("1000", 365)] {
            let mut settings = Settings::defaults();
            settings
                .set("uuid", &json!(seed::INSTALL_ID))
                .expect("the value is valid");
            settings
                .set("rememberMeDays", &json!(setting))
                .expect("the value is valid");
            let app = app_with(database, settings);

            let answer = post_login(&app, &body, &[], "198.51.100.7:50000").await;

            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            let token = answer.token();
            let session = admin_sessions(database)
                .await
                .into_iter()
                .find(|session| session.token_hash == token.hash())
                .expect("the session of this login is stored");
            assert_eq!(
                session.expires_at.unix_seconds() - session.created_at.unix_seconds(),
                days * 86_400,
                "rememberMeDays = {setting}"
            );
        }
    })
    .await;
}

#[tokio::test]
async fn login_behind_a_trusted_https_proxy_gets_a_protected_cookie_and_the_real_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let mut settings = Settings::defaults();
        settings
            .set("uuid", &json!(seed::INSTALL_ID))
            .expect("the value is valid");
        let state = AppState::new(database.clone(), settings, seed::secrets())
            .with_trusted_proxies("10.0.0.0/8".parse().expect("the list is valid"));
        let app = router(state).expect("the route table is valid");
        let forwarded = [
            ("x-forwarded-for", "203.0.113.9"),
            ("x-forwarded-proto", "https"),
        ];

        let over_https = post_login(&app, &admin(), &forwarded, "10.1.2.3:40000").await;
        let over_http = post_login(&app, &admin(), &[], "198.51.100.7:50000").await;

        for cookie in over_https.session_cookies() {
            assert!(
                cookie.ends_with("; SameSite=Lax; Secure; HttpOnly"),
                "{cookie}"
            );
        }
        // Over plain HTTP the cookie cannot be `Secure`, but it is still hidden from
        // scripts.
        for cookie in over_http.session_cookies() {
            assert!(cookie.ends_with("; SameSite=Lax; HttpOnly"), "{cookie}");
        }
        assert_eq!(over_https.session_cookies().len(), 2);
        assert_eq!(over_http.session_cookies().len(), 2);
        let addresses: Vec<String> = admin_sessions(database)
            .await
            .iter()
            .map(|session| session.ip.to_string())
            .collect();
        assert_eq!(addresses, ["203.0.113.9", "198.51.100.7"]);
    })
    .await;
}

#[tokio::test]
async fn login_from_another_site_is_refused_before_the_password_is_checked() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let from_elsewhere: [&[(&str, &str)]; 3] = [
            &[("sec-fetch-site", "cross-site")],
            &[("sec-fetch-site", "same-site")],
            &[("origin", "https://evil.example.net")],
        ];

        for headers in from_elsewhere {
            let answer = post_login(&app, &admin(), headers, "198.51.100.7:50000").await;

            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{headers:?}");
            assert!(
                answer
                    .body
                    .contains("Not authorized for current Route: /api/v2/login"),
                "{headers:?}: {}",
                answer.body
            );
            assert_eq!(answer.session_cookies(), Vec::<&str>::new(), "{headers:?}");
        }
        assert_eq!(admin_sessions(database).await, []);

        // The same request from a page of the application itself goes through.
        let from_here = post_login(
            &app,
            &admin(),
            &[("sec-fetch-site", "same-origin")],
            "198.51.100.7:50000",
        )
        .await;
        assert_eq!(from_here.status, StatusCode::OK, "{}", from_here.body);
    })
    .await;
}

#[tokio::test]
async fn login_ends_the_session_that_the_browser_had() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let peer = "198.51.100.7:50000";
        let first = post_login(&app, &admin(), &[], peer).await.token();
        let other_browser = post_login(&app, &admin(), &[], peer).await.token();
        assert_eq!(admin_sessions(database).await.len(), 2);

        // The browser logs in again and sends the cookie of its session.
        let cookie = format!("{SESSION_COOKIE}={}", first.expose());
        let again = post_login(&app, &admin(), &[("cookie", &cookie)], peer).await;

        assert_eq!(again.status, StatusCode::OK, "{}", again.body);
        let hashes: Vec<String> = admin_sessions(database)
            .await
            .into_iter()
            .map(|session| session.token_hash.as_str().to_owned())
            .collect();
        assert!(
            !hashes.contains(&first.hash().as_str().to_owned()),
            "{hashes:?}"
        );
        assert!(hashes.contains(&other_browser.hash().as_str().to_owned()));
        assert!(hashes.contains(&again.token().hash().as_str().to_owned()));
        assert_eq!(hashes.len(), 2);

        // A login that fails ends nothing.
        let cookie = format!("{SESSION_COOKIE}={}", other_browser.expose());
        let wrong = json!({"username": seed::ADMIN_NAME, "password": "not-it"});
        let refused = post_login(&app, &wrong, &[("cookie", &cookie)], peer).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert_eq!(admin_sessions(database).await.len(), 2);
    })
    .await;
}
