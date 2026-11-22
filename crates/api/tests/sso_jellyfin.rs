//! Single sign-on to Jellyfin: a login with a password hands the login of the web client
//! of Jellyfin to the browser as the cookies of PHP Organizr (D-114).

#![allow(
    clippy::expect_used,
    reason = "a test reads the answers it knows, and its helpers may panic"
)]

use axum::http::StatusCode;
use organizr_db::Database;
use organizr_domain::{NewUser, Timestamp};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::services::{self, JELLYFIN_SERVER_ID};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

/// The password that the stand-in for Jellyfin knows for robin.
const PASSWORD: &str = "the password of robin in organizr";

async fn store_robin(database: &Database) {
    let group = database
        .groups()
        .find_default()
        .await
        .expect("the lookup works")
        .expect("the install has a default group");
    database
        .users()
        .insert(NewUser {
            username: "robin".to_owned(),
            email: Some("robin@example.com".to_owned()),
            password_hash: organizr_auth::password::hash(PASSWORD).expect("hashed"),
            group: group.id,
            image: None,
            created_at: Timestamp::from_unix_seconds(1_800_000_000),
        })
        .await
        .expect("the user can be stored");
}

/// The whole `Set-Cookie` text of a cookie that an answer sets.
fn cookie<'a>(answer: &'a Answer, name: &str) -> Option<&'a str> {
    answer
        .cookies
        .iter()
        .map(String::as_str)
        .find(|cookie| cookie.starts_with(&format!("{name}=")))
}

/// The value of a cookie, decoded as a browser reads it.
fn value_of(cookie: &str) -> String {
    let raw = cookie
        .split(';')
        .next()
        .and_then(|pair| pair.split_once('='))
        .map_or("", |(_, value)| value);
    let bytes = raw.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        if let (b'%', Some(value)) = (byte, hex) {
            decoded.push(value);
            index += 3;
        } else {
            decoded.push(byte);
            index += 1;
        }
    }
    String::from_utf8(decoded).expect("UTF-8")
}

#[tokio::test]
async fn login_with_a_password_hands_the_jellyfin_login_to_the_browser() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_robin(database).await;
        let jellyfin = services::jellyfin().await;
        let app = installed_app_with(
            database,
            &[
                ("ssoJellyfin", json!(true)),
                ("jellyfinURL", json!(jellyfin.url())),
                (
                    "jellyfinSSOURL",
                    json!("https://organizr.example.com/jellyfin"),
                ),
            ],
        );

        let body = json!({"username": "robin", "password": PASSWORD});
        let answer = call(&app, As::Guest, "POST", "/login", Some(body)).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let credentials = cookie(&answer, "jellyfin_credentials").expect("the credentials");
        // A script of Organizr copies it into the storage of the browser, so it is not
        // HttpOnly.
        assert!(!credentials.contains("HttpOnly"), "{credentials}");
        let credentials: Value = serde_json::from_str(&value_of(credentials)).expect("JSON");
        assert_eq!(
            credentials["Servers"][0]["ManualAddress"],
            json!("https://organizr.example.com/jellyfin")
        );
        assert_eq!(
            credentials["Servers"][0]["AccessToken"],
            json!("jellyfin-token-of-robin")
        );
        let user_key = format!("user-jellyfin-user-robin-{JELLYFIN_SERVER_ID}");
        let user = cookie(&answer, &user_key).expect("the user of the web client");
        let user: Value = serde_json::from_str(&value_of(user)).expect("JSON");
        assert_eq!(user["Name"], json!("Robin"));
    })
    .await;
}

#[tokio::test]
async fn no_jellyfin_cookies_when_the_switch_is_off_or_jellyfin_refuses() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_robin(database).await;
        let jellyfin = services::jellyfin().await;
        let body = json!({"username": "robin", "password": PASSWORD});

        let off = installed_app_with(database, &[("jellyfinURL", json!(jellyfin.url()))]);
        let answer = call(&off, As::Guest, "POST", "/login", Some(body.clone())).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert!(cookie(&answer, "jellyfin_credentials").is_none());
        assert!(jellyfin.seen().is_empty(), "Jellyfin is not asked");

        // A user whom Jellyfin does not know still logs in to Organizr.
        let wrong = installed_app_with(
            database,
            &[
                ("ssoJellyfin", json!(true)),
                ("jellyfinURL", json!("http://127.0.0.1:9")),
            ],
        );
        let answer = call(&wrong, As::Guest, "POST", "/login", Some(body)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert!(cookie(&answer, "jellyfin_credentials").is_none());
    })
    .await;
}
