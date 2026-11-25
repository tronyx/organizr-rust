//! The second factor of a login through the real route table: what the recorded PHP
//! answers cannot show (D-57).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::extract::ConnectInfo;
use axum::http::StatusCode;
use organizr_auth::secrets::two_factor_name;
use organizr_auth::two_factor::code_at;
use organizr_db::{Database, SealedTwoFactor};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call, request, send};
use organizr_testkit::{installed_app, installed_app_with, seed};
use serde_json::{Value, json};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is after 1970")
        .as_secs()
}

/// The code of a secret at this moment.
fn code(secret: &str) -> String {
    code_at(secret, now()).expect("the secret is valid")
}

/// Six digits that are no code of the secret near this moment.
fn wrong_code(secret: &str) -> String {
    let near: Vec<String> = (0..17_u64)
        .map(|step| code_at(secret, now() + step * 30 - 8 * 30).expect("the secret is valid"))
        .collect();
    (0..1_000_000)
        .map(|number| format!("{number:06}"))
        .find(|code| !near.contains(code))
        .expect("some code is wrong")
}

/// Gives the user of a session a second factor, as the settings page does, and returns
/// the secret.
async fn add_second_factor(app: &Router, cookie: &str) -> String {
    let created = call(app, As::User(cookie), "POST", "/2fa/google", None).await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.body);
    let secret = created.data()["secret"]
        .as_str()
        .expect("the answer holds a secret")
        .to_owned();
    let body = json!({"secret": secret, "type": "google"});
    let saved = call(app, As::User(cookie), "PUT", "/2fa", Some(body)).await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    secret
}

async fn log_in(app: &Router, name: &str, code: Option<&str>) -> Answer {
    let mut body = json!({"username": name, "password": seed::USER_PASSWORD});
    if let Some(code) = code {
        body["tfaCode"] = json!(code);
    }
    call(app, As::Guest, "POST", "/login", Some(body)).await
}

#[tokio::test]
async fn second_factor_is_shown_by_its_kind_and_its_secret_is_sealed() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        let other = seed::logged_in(database, &app, "other", 4).await;
        let service = |answer: &Answer| answer.data()["authService"].clone();

        let me = call(&app, As::User(&robin), "GET", "/token/me", None).await;
        assert_eq!(service(&me), json!("internal"));

        let secret = add_second_factor(&app, &robin).await;

        // The PHP application shows the kind of the factor where the service was.
        let me = call(&app, As::User(&robin), "GET", "/token/me", None).await;
        assert_eq!(service(&me), json!("google"));
        let me = call(&app, As::User(&other), "GET", "/token/me", None).await;
        assert_eq!(service(&me), json!("internal"));
        // An Admin sees the kind, and never the secret. PHP shows `google::<secret>`.
        let users = call(&app, As::Admin, "GET", "/users", None).await;
        assert!(!users.body.contains(&secret), "{}", users.body);
        let services: Vec<(Value, Value)> = users
            .data()
            .as_array()
            .expect("the data is a list")
            .iter()
            .map(|user| (user["username"].clone(), user["auth_service"].clone()))
            .collect();
        assert_eq!(
            services,
            [
                (json!("admin"), json!("internal")),
                (json!("robin"), json!("google")),
                (json!("other"), json!("internal")),
            ]
        );

        // The database holds the secret sealed, under the name of this user.
        let user = database
            .users()
            .find_by_username("robin")
            .await
            .expect("the lookup works")
            .expect("the user is there");
        let stored = stored_factor(database, user.id).await.expect("a factor");
        assert_eq!(stored.kind, "google");
        assert!(!stored.sealed_secret.contains(&secret));
        assert_eq!(
            seed::secrets()
                .open(&two_factor_name(user.id), &stored.sealed_secret)
                .expect("the secret opens"),
            secret
        );

        // Only the account with the factor needs a code.
        assert_eq!(log_in(&app, "other", None).await.status, StatusCode::OK);
        let needed = log_in(&app, "robin", None).await;
        assert_eq!(needed.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(needed.json()["response"]["result"], json!("warning"));
        // A person can type the code with a space around it.
        let padded = format!(" {} ", code(&secret));
        assert_eq!(
            log_in(&app, "robin", Some(&padded)).await.status,
            StatusCode::OK
        );

        let removed = call(&app, As::User(&robin), "DELETE", "/2fa", None).await;
        assert_eq!(removed.status, StatusCode::NO_CONTENT);
        assert_eq!(stored_factor(database, user.id).await, None);
        assert_eq!(log_in(&app, "robin", None).await.status, StatusCode::OK);
    })
    .await;
}

async fn stored_factor(
    database: &Database,
    user: organizr_domain::UserId,
) -> Option<SealedTwoFactor> {
    database
        .users()
        .login_of(user)
        .await
        .expect("the login loads")
        .expect("the user is there")
        .two_factor
}

#[tokio::test]
async fn values_that_would_lock_the_account_are_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        let secret = "JBSWY3DPEHPK3PXP";

        for (body, status, message) in [
            (
                json!({"secret": secret, "type": "other"}),
                StatusCode::NOT_FOUND,
                "other is not an available to be setup",
            ),
            (
                json!({"secret": "not base32!", "type": "google"}),
                StatusCode::UNPROCESSABLE_ENTITY,
                "Secret is not valid",
            ),
            (
                json!({"secret": "0", "type": "google"}),
                StatusCode::UNPROCESSABLE_ENTITY,
                "Secret was not supplied or left blank",
            ),
        ] {
            let answer = call(&app, As::User(&robin), "PUT", "/2fa", Some(body.clone())).await;
            assert_eq!(answer.status, status, "{body}: {}", answer.body);
            assert_eq!(answer.message(), json!(message), "{body}");
        }
        // Nothing was stored, so the login needs no code.
        assert_eq!(log_in(&app, "robin", None).await.status, StatusCode::OK);

        let body = json!({"secret": "not base32!", "code": "123456", "type": "google"});
        let checked = call(&app, As::User(&robin), "POST", "/2fa", Some(body)).await;
        assert_eq!(checked.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(checked.message(), json!("Secret is not valid"));
        assert_eq!(checked.data(), json!(false));

        // A secret of 16 characters, as the PHP application makes them, is taken.
        let body = json!({"secret": secret, "type": "google"});
        let saved = call(&app, As::User(&robin), "PUT", "/2fa", Some(body)).await;
        assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        assert_eq!(
            log_in(&app, "robin", Some(&code(secret))).await.status,
            StatusCode::OK
        );
    })
    .await;
}

#[tokio::test]
async fn script_with_the_api_key_has_no_account_for_a_second_factor() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        let created = call(&app, As::Admin, "POST", "/2fa/google", None).await;
        assert_eq!(created.status, StatusCode::OK, "{}", created.body);
        assert_eq!(created.data()["type"], json!("google"));
        let picture = created.data()["url"].as_str().expect("a text").to_owned();
        assert!(picture.starts_with("data:image/png;base64,"), "{picture}");

        let body = json!({"secret": created.data()["secret"], "type": "google"});
        let saved = call(&app, As::Admin, "PUT", "/2fa", Some(body)).await;
        assert_eq!(saved.status, StatusCode::UNAUTHORIZED, "{}", saved.body);
        assert_eq!(saved.message(), json!("Not Authorized"));
        let removed = call(&app, As::Admin, "DELETE", "/2fa", None).await;
        assert_eq!(removed.status, StatusCode::NO_CONTENT);

        // The Admin account of the install has no second factor from any of it.
        let body = json!({"username": seed::ADMIN_NAME, "password": seed::ADMIN_PASSWORD});
        let login = call(&app, As::Guest, "POST", "/login", Some(body)).await;
        assert_eq!(login.status, StatusCode::OK, "{}", login.body);
    })
    .await;
}

#[tokio::test]
async fn wrong_codes_count_for_the_login_limit() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[("loginAttempts", json!("3"))]);
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        let secret = add_second_factor(&app, &robin).await;
        let wrong = wrong_code(&secret);

        // A login without a code is no failed login.
        for _ in 0..5 {
            let needed = log_in(&app, "robin", None).await;
            assert_eq!(needed.message(), json!("2FA Code Needed"));
        }
        for _ in 0..2 {
            let refused = log_in(&app, "robin", Some(&wrong)).await;
            assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(refused.message(), json!("Wrong 2FA"));
        }
        // The third wrong code reaches the limit. After it, the right code is refused too.
        let locked = log_in(&app, "robin", Some(&wrong)).await;
        assert_eq!(locked.status, StatusCode::FORBIDDEN, "{}", locked.body);
        assert_eq!(locked.message(), json!("User is locked out"));
        let locked = log_in(&app, "robin", Some(&code(&secret))).await;
        assert_eq!(locked.status, StatusCode::FORBIDDEN, "{}", locked.body);
    })
    .await;
}

/// Logs in from an address. The server sees that address as the one of the connection.
async fn log_in_from(app: &Router, address: [u8; 4]) -> Answer {
    let body = json!({"username": "robin", "password": seed::USER_PASSWORD}).to_string();
    let mut request = request(
        "POST",
        "/api/v2/login",
        &[("content-type", "application/json")],
        &body,
    );
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((address, 40_000))));
    send(app, request).await
}

#[tokio::test]
async fn setting_lets_the_local_network_log_in_without_a_code() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let strict = installed_app(database);
        let robin = seed::logged_in(database, &strict, "robin", 4).await;
        add_second_factor(&strict, &robin).await;
        let lenient = installed_app_with(database, &[("ignoreTFALocal", json!(true))]);
        let local = [192, 168, 1, 20];
        let remote = [203, 0, 113, 9];

        // Without the setting, the local network needs the code too.
        let answer = log_in_from(&strict, local).await;
        assert_eq!(answer.message(), json!("2FA Code Needed"));

        let answer = log_in_from(&lenient, local).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), Value::Null);
        let answer = log_in_from(&lenient, remote).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(answer.message(), json!("2FA Code Needed"));
    })
    .await;
}

#[tokio::test]
async fn stored_factor_that_cannot_be_used_refuses_the_login() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        seed::logged_in(database, &app, "robin", 4).await;
        let user = database
            .users()
            .find_by_username("robin")
            .await
            .expect("the lookup works")
            .expect("the user is there");
        let secret = "JBSWY3DPEHPK3PXP";
        let seal = |name: &str| {
            seed::secrets()
                .seal(name, secret)
                .expect("the secret can be sealed")
        };

        // A kind that the application does not have, as an old install can hold it, and
        // a secret that was sealed for another user.
        for (kind, sealed_secret) in [
            ("other", seal(&two_factor_name(user.id))),
            ("google", seal("users/999/two_factor_secret")),
            ("google", "not sealed and no Base32!".to_owned()),
        ] {
            let factor = SealedTwoFactor {
                kind: kind.to_owned(),
                sealed_secret,
            };
            database
                .users()
                .set_two_factor(user.id, Some(factor))
                .await
                .expect("the factor can be stored");

            let needed = log_in(&app, "robin", None).await;
            assert_eq!(needed.message(), json!("2FA Code Needed"), "{kind}");
            let refused = log_in(&app, "robin", Some(&code(secret))).await;
            assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY, "{kind}");
            assert_eq!(refused.message(), json!("Wrong 2FA"), "{kind}");
        }
    })
    .await;
}

/// Removes the second factor of a user through the route of the managers.
async fn delete(app: &Router, who: As<'_>, path: &str) -> Answer {
    call(app, who, "DELETE", path, None).await
}

#[tokio::test]
async fn manager_removes_the_second_factor_of_another_user() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let boss = seed::logged_in(database, &app, "pat", 1).await;
        let admin = seed::logged_in(database, &app, "zed", 0).await;
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        let user_of = async |name: &str| {
            database
                .users()
                .find_by_username(name)
                .await
                .expect("the lookup works")
                .expect("the user is there")
                .id
        };
        let robin_id = user_of("robin").await;
        let admin_id = user_of("zed").await;
        add_second_factor(&app, &robin).await;
        add_second_factor(&app, &admin).await;

        // A guest and a user without the level of a Co-Admin cannot.
        let path = format!("/users/2fa/{robin_id}");
        for who in [As::Guest, As::User(&robin)] {
            let refused = delete(&app, who, &path).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{}", refused.body);
        }
        assert!(stored_factor(database, robin_id).await.is_some());

        let removed = delete(&app, As::User(&boss), &path).await;
        assert_eq!(removed.status, StatusCode::OK, "{}", removed.body);
        assert_eq!(removed.message(), json!("Second factor removed"));
        assert_eq!(stored_factor(database, robin_id).await, None);
        assert_eq!(log_in(&app, "robin", None).await.status, StatusCode::OK);
        // A user without a second factor is no error.
        let again = delete(&app, As::User(&boss), &path).await;
        assert_eq!(again.status, StatusCode::OK, "{}", again.body);

        for path in ["/users/2fa/987654", "/users/2fa/abc"] {
            let missing = delete(&app, As::User(&boss), path).await;
            assert_eq!(missing.status, StatusCode::NOT_FOUND, "{path}");
            assert_eq!(missing.message(), json!("User not found"));
        }

        // Only an Admin removes the second factor of an Admin.
        let path = format!("/users/2fa/{admin_id}");
        let refused = delete(&app, As::User(&boss), &path).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            refused.message(),
            json!("Only admins can remove the second factor of admins")
        );
        assert!(stored_factor(database, admin_id).await.is_some());
        let done = delete(&app, As::User(&admin), &path).await;
        assert_eq!(done.status, StatusCode::OK, "{}", done.body);
        assert_eq!(stored_factor(database, admin_id).await, None);
        // A script with the API key acts as an Admin.
        let script = delete(&app, As::Admin, &path).await;
        assert_eq!(script.status, StatusCode::OK, "{}", script.body);
    })
    .await;
}
