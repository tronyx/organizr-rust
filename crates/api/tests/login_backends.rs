//! Logging in through the back ends of `authBackend` other than Plex (LDAP, FTP, Emby,
//! Emby Connect and Jellyfin), against stand-ins for the servers, and the two LDAP
//! tests of the settings page.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use organizr_api::router;
use organizr_auth::password::{self, PasswordCheck};
use organizr_db::Database;
use organizr_domain::User;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::login_servers::{
    self, FTP_USERS, LDAP_BASE, LDAP_PEOPLE, LDAP_PREFIX, LDAP_SERVICE, LDAP_SUFFIX, Listening,
    closed_port, ldap_server,
};
use organizr_testkit::services::StandIn;
use organizr_testkit::{NO_LOGIN_LIMIT, installed_app_with, installed_settings, seed, state_with};
use serde_json::{Value, json};

const ROBIN_LDAP_PASSWORD: &str = LDAP_PEOPLE[0].password;

/// The settings of an install that checks passwords with the LDAP stand-in.
fn ldap_settings(
    port: u16,
    kind: &str,
    more: &[(&'static str, Value)],
) -> Vec<(&'static str, Value)> {
    let mut settings = vec![
        ("authType", json!(kind)),
        ("authBackend", json!("ldap")),
        ("authBackendHost", json!(format!("ldap://127.0.0.1:{port}"))),
        ("authBaseDN", json!(LDAP_BASE)),
        ("authBackendHostPrefix", json!(LDAP_PREFIX)),
        ("authBackendHostSuffix", json!(LDAP_SUFFIX)),
        ("ldapBindUsername", json!(LDAP_SERVICE.0)),
        ("ldapBindPassword", json!(LDAP_SERVICE.1)),
        ("loginAttempts", json!(NO_LOGIN_LIMIT)),
    ];
    settings.extend(more.iter().cloned());
    settings
}

async fn log_in(app: &Router, name: &str, password: &str) -> Answer {
    let body = json!({"username": name, "password": password});
    call(app, As::Guest, "POST", "/login", Some(body)).await
}

async fn user(database: &Database, name: &str) -> Option<User> {
    database
        .users()
        .find_by_username(name)
        .await
        .expect("the lookup works")
}

fn has_password(user: &User, password: &str) -> bool {
    matches!(
        password::verify(password, &user.password_hash),
        PasswordCheck::Correct { .. }
    )
}

fn logged_in(answer: &Answer) -> bool {
    answer.status == StatusCode::OK
        && answer
            .cookies
            .iter()
            .any(|cookie| cookie.starts_with("organizr_token_"))
}

#[tokio::test]
async fn ldap_login_makes_an_account_with_the_name_in_lower_case_and_the_mail_of_the_entry() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = ldap_server(false).await;
        let app = installed_app_with(database, &ldap_settings(server.port(), "external", &[]));

        let answer = log_in(&app, "Robin", ROBIN_LDAP_PASSWORD).await;

        assert!(logged_in(&answer), "{}", answer.body);
        let robin = user(database, "robin").await.expect("the account was made");
        assert_eq!(robin.username, "robin");
        assert_eq!(robin.email.as_deref(), Some("robin@ldap.example.com"));
        // The password of the login is also the password of the account.
        assert!(has_password(&robin, ROBIN_LDAP_PASSWORD));
        // The server got the name in lower case, with the prefix and the suffix.
        assert!(
            server
                .seen()
                .contains(&format!("bind uid=robin{LDAP_SUFFIX}"))
        );

        // A wrong password fails, as a wrong password of Organizr does.
        let wrong = log_in(&app, "robin", "wrong").await;
        assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
        assert_eq!(wrong.message(), json!("User credentials incorrect"));
    })
    .await;
}

#[tokio::test]
async fn ldap_login_finds_the_account_by_the_typed_name_and_updates_its_password() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = ldap_server(false).await;
        // The account "kim" exists, with another password and its own address.
        let app = installed_app_with(database, &ldap_settings(server.port(), "both", &[]));
        let _ = seed::logged_in(database, &app, "kim", 4).await;
        let kim = LDAP_PEOPLE[1];

        let answer = log_in(&app, "KIM", kim.password).await;

        assert!(logged_in(&answer), "{}", answer.body);
        let account = user(database, "kim").await.expect("the account is there");
        assert!(has_password(&account, kim.password));
        assert!(!has_password(&account, seed::USER_PASSWORD));
        // No second account was made.
        assert_eq!(database.users().list().await.expect("it loads").len(), 2);
    })
    .await;
}

#[tokio::test]
async fn ldap_that_cannot_be_reached_leaves_the_local_admin_in_with_both_but_not_with_external() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let closed = closed_port().await;

        let both = installed_app_with(database, &ldap_settings(closed, "both", &[]));
        let answer = log_in(&both, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        assert!(logged_in(&answer), "{}", answer.body);

        // With "the service only", PHP checks no stored password, so nobody gets in.
        let external = installed_app_with(database, &ldap_settings(closed, "external", &[]));
        let answer = log_in(&external, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(answer.message(), json!("User credentials incorrect"));
    })
    .await;
}

#[tokio::test]
async fn ldap_logins_that_fail_count_toward_the_lock_out() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = ldap_server(false).await;
        let settings = ldap_settings(server.port(), "both", &[("loginAttempts", json!("2"))]);
        let app = installed_app_with(database, &settings);

        assert_eq!(
            log_in(&app, "robin", "wrong").await.status,
            StatusCode::UNAUTHORIZED
        );
        let locked = log_in(&app, "robin", "wrong").await;
        assert_eq!(locked.status, StatusCode::FORBIDDEN);
        assert_eq!(locked.message(), json!("User is locked out"));
        // The right password does not help during the lock-out.
        let answer = log_in(&app, "robin", ROBIN_LDAP_PASSWORD).await;
        assert_eq!(answer.status, StatusCode::FORBIDDEN);
    })
    .await;
}

#[tokio::test]
async fn ldap_refuses_names_that_could_change_the_entry_and_empty_passwords() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = ldap_server(true).await;
        let app = installed_app_with(database, &ldap_settings(server.port(), "external", &[]));

        for name in ["robin,ou=admins", "*", "robin)(uid=*", "cn=organizr"] {
            let answer = log_in(&app, name, ROBIN_LDAP_PASSWORD).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{name}");
        }
        // An empty password would bind without a name, which this server takes.
        let answer = log_in(&app, "robin", "").await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert!(server.seen().is_empty(), "{:?}", server.seen());
        assert!(user(database, "robin").await.is_none());
    })
    .await;
}

fn ftp_settings(server: &Listening) -> Vec<(&'static str, Value)> {
    vec![
        ("authType", json!("external")),
        ("authBackend", json!("ftp")),
        (
            "authBackendHost",
            json!(format!("ftp://127.0.0.1:{}", server.port())),
        ),
        ("loginAttempts", json!(NO_LOGIN_LIMIT)),
    ]
}

#[tokio::test]
async fn ftp_login_makes_an_account_without_an_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = login_servers::ftp_server().await;
        let app = installed_app_with(database, &ftp_settings(&server));
        let (name, password) = FTP_USERS[0];

        let answer = log_in(&app, "ROBIN", password).await;

        assert!(logged_in(&answer), "{}", answer.body);
        let robin = user(database, name).await.expect("the account was made");
        // FTP names no address, so the account gets one of PHP's placeholder domain.
        assert!(
            robin
                .email
                .as_deref()
                .is_some_and(|email| email.ends_with("@placeholder.eml")),
            "{:?}",
            robin.email
        );
        assert_eq!(server.seen()[0], format!("USER {name}"));

        let wrong = log_in(&app, name, "wrong").await;
        assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn ftp_login_refuses_anonymous_although_the_server_would_take_it() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = login_servers::ftp_server().await;
        let app = installed_app_with(database, &ftp_settings(&server));

        for name in ["anonymous", "ANONYMOUS", " ftp "] {
            let answer = log_in(&app, name, "any-password").await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{name:?}");
            assert_eq!(answer.message(), json!("User credentials incorrect"));
        }
        // The server was not asked, and no account was made.
        assert_eq!(server.seen(), Vec::<String>::new());
        assert!(user(database, "anonymous").await.is_none());
        assert!(user(database, "ftp").await.is_none());
    })
    .await;
}

#[tokio::test]
async fn ftp_without_tls_on_the_server_fails_with_the_default_scheme() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = login_servers::ftp_server().await;
        // Without a scheme, PHP uses FTPS; this server offers no TLS.
        let mut settings = ftp_settings(&server);
        settings.push((
            "authBackendHost",
            json!(format!("127.0.0.1:{}", server.port())),
        ));
        let app = installed_app_with(database, &settings);
        let (name, password) = FTP_USERS[0];

        let answer = log_in(&app, name, password).await;

        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(server.seen(), ["AUTH TLS"]);
    })
    .await;
}

/// A stand-in for an Emby or a Jellyfin server and for Emby Connect: "robin" logs in
/// with "robin-media-password" at the server, and with "robin-connect-password" at
/// Emby Connect, whose account is linked to the Emby user "robin".
async fn media_stand_in(login_path: &'static str) -> StandIn {
    let session = json!({
        "SessionInfo": {"Id": "s1"},
        "User": {"Name": "robin", "HasPassword": true},
        "AccessToken": "session-token",
    });
    StandIn::start(
        Router::new()
            .route(
                login_path,
                post(move |body: String| {
                    let sent: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                    let password = sent.get("pw").or_else(|| sent.get("Pw")).cloned();
                    let fits = sent["Username"] == "robin"
                        && password == Some(json!("robin-media-password"));
                    let session = session.clone();
                    async move {
                        if fits {
                            (StatusCode::OK, axum::Json(session))
                        } else {
                            (StatusCode::UNAUTHORIZED, axum::Json(Value::Null))
                        }
                    }
                }),
            )
            .route("/Sessions/Logout", post(|| async { StatusCode::NO_CONTENT }))
            .route(
                "/Users",
                get(|headers: HeaderMap| async move {
                    if headers.get("x-emby-token").is_none_or(|key| key != "the-admin-key") {
                        return (StatusCode::UNAUTHORIZED, axum::Json(Value::Null));
                    }
                    (
                        StatusCode::OK,
                        axum::Json(json!([
                            {"Name": "robin", "ConnectUserName": "Robin", "ConnectLinkType": "LinkedUser"},
                        ])),
                    )
                }),
            )
            .route(
                "/service/user/authenticate",
                post(|body: String| async move {
                    if body == "nameOrEmail=robin&rawpw=robin-connect-password" {
                        (
                            StatusCode::OK,
                            axum::Json(json!({
                                "AccessToken": "connect-token",
                                "User": {"Name": "Robin", "Email": "robin@connect.example.com"},
                            })),
                        )
                    } else {
                        (StatusCode::UNAUTHORIZED, axum::Json(Value::Null))
                    }
                }),
            ),
    )
    .await
}

fn media_app(database: &Database, stand_in: &StandIn, backend: &str) -> Router {
    let settings = installed_settings(&[
        ("authType", json!("external")),
        ("authBackend", json!(backend)),
        ("embyURL", json!(stand_in.url())),
        ("embyToken", json!("the-admin-key")),
        ("jellyfinURL", json!(stand_in.url())),
        ("loginAttempts", json!(NO_LOGIN_LIMIT)),
    ]);
    let state = state_with(database, settings).with_emby_connect(&stand_in.url());
    router(state).expect("the route table is valid")
}

#[tokio::test]
async fn emby_and_jellyfin_logins_make_an_account_with_the_typed_name() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        for (backend, path, name) in [
            ("emby_local", "/Users/AuthenticateByName", "robin"),
            ("jellyfin", "/Users/authenticatebyname", "ROBIN"),
        ] {
            let stand_in = media_stand_in(path).await;
            let app = media_app(database, &stand_in, backend);

            let answer = log_in(&app, name, "robin-media-password").await;
            assert!(logged_in(&answer), "{backend}: {}", answer.body);
            let wrong = log_in(&app, name, "wrong").await;
            assert_eq!(wrong.status, StatusCode::UNAUTHORIZED, "{backend}");
        }
        // Jellyfin got the name in lower case, so both logins are one account.
        let robin = user(database, "robin").await.expect("the account was made");
        assert!(has_password(&robin, "robin-media-password"));
        assert_eq!(database.users().list().await.expect("it loads").len(), 2);
    })
    .await;
}

#[tokio::test]
async fn emby_name_keeps_its_letter_case() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let stand_in = media_stand_in("/Users/AuthenticateByName").await;
        let app = media_app(database, &stand_in, "emby_local");
        // The stand-in knows "robin" only: the name goes to Emby as it was typed.
        let answer = log_in(&app, "Robin", "robin-media-password").await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn emby_connect_login_makes_an_account_with_the_name_and_address_of_connect() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let stand_in = media_stand_in("/Users/AuthenticateByName").await;
        let app = media_app(database, &stand_in, "emby_connect");

        let answer = log_in(&app, "robin", "robin-connect-password").await;

        assert!(logged_in(&answer), "{}", answer.body);
        let robin = user(database, "Robin").await.expect("the account was made");
        assert_eq!(robin.username, "Robin");
        assert_eq!(robin.email.as_deref(), Some("robin@connect.example.com"));
        // The password of Emby itself is not the one of Emby Connect.
        let local = log_in(&app, "robin", "robin-media-password").await;
        assert_eq!(local.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn emby_all_tries_the_server_first_and_then_emby_connect() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let stand_in = media_stand_in("/Users/AuthenticateByName").await;
        let app = media_app(database, &stand_in, "emby_all");

        assert!(logged_in(
            &log_in(&app, "robin", "robin-media-password").await
        ));
        assert!(logged_in(
            &log_in(&app, "robin", "robin-connect-password").await
        ));
        let wrong = log_in(&app, "robin", "wrong").await;
        assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
        // Both logins found the account of the typed name.
        assert_eq!(database.users().list().await.expect("it loads").len(), 2);
    })
    .await;
}

#[tokio::test]
async fn ldap_tests_are_for_an_admin_or_a_co_admin() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = ldap_server(false).await;
        let app = installed_app_with(database, &ldap_settings(server.port(), "internal", &[]));
        let user = seed::logged_in(database, &app, "pat", 4).await;
        for path in ["/test/ldap", "/test/ldap/login"] {
            for who in [As::Guest, As::User(&user)] {
                let answer = call(&app, who, "POST", path, None).await;
                assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{path}");
            }
        }
        let boss = seed::logged_in(database, &app, "boss", 1).await;
        let answer = call(&app, As::User(&boss), "POST", "/test/ldap", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    })
    .await;
}

#[tokio::test]
async fn ldap_test_binds_with_the_account_of_the_settings() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = ldap_server(false).await;
        let port = server.port();

        let app = installed_app_with(database, &ldap_settings(port, "internal", &[]));
        let answer = call(&app, As::Admin, "POST", "/test/ldap", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("LDAP connection successful"));
        assert_eq!(server.seen()[0], format!("bind {}", LDAP_SERVICE.0));

        let wrong = ldap_settings(port, "internal", &[("ldapBindPassword", json!("wrong"))]);
        let app = installed_app_with(database, &wrong);
        let answer = call(&app, As::Admin, "POST", "/test/ldap", None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), json!("Invalid credentials"));
        assert!(!answer.body.contains("wrong"), "{}", answer.body);

        let closed = closed_port().await;
        let app = installed_app_with(database, &ldap_settings(closed, "internal", &[]));
        let answer = call(&app, As::Admin, "POST", "/test/ldap", None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), json!("Can't contact LDAP server"));

        let app = installed_app_with(database, &[("authBackendHost", json!("ldap.lan"))]);
        let answer = call(&app, As::Admin, "POST", "/test/ldap", None).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            answer.message(),
            json!("authBaseDN and/or BackendHost not supplied")
        );
    })
    .await;
}

#[tokio::test]
async fn ldap_login_test_binds_with_the_name_and_password_of_the_body() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = ldap_server(false).await;
        let app = installed_app_with(database, &ldap_settings(server.port(), "internal", &[]));
        let test = |body: Value| call(&app, As::Admin, "POST", "/test/ldap/login", Some(body));

        let answer = test(json!({"username": "robin", "password": ROBIN_LDAP_PASSWORD})).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("LDAP connection successful"));

        let answer = test(json!({"username": "robin", "password": "wrong"})).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            answer.message(),
            json!("Username/Password Failed to authenticate")
        );

        for body in [
            json!({"username": "robin"}),
            json!({"password": "x"}),
            json!({}),
        ] {
            let answer = test(body).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                answer.message(),
                json!("Username and/or Password not supplied")
            );
        }

        let closed = closed_port().await;
        let app = installed_app_with(database, &ldap_settings(closed, "internal", &[]));
        let body = json!({"username": "robin", "password": ROBIN_LDAP_PASSWORD});
        let answer = call(&app, As::Admin, "POST", "/test/ldap/login", Some(body)).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.message(), json!("Can't contact LDAP server"));
    })
    .await;
}
