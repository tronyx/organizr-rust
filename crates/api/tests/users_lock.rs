//! The routes of the lock screen.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{NO_LOGIN_LIMIT, installed_app_with, seed};
use serde_json::{Value, json};

async fn post(app: &Router, who: As<'_>, path: &str, body: Option<Value>) -> Answer {
    call(app, who, "POST", path, body).await
}

/// Whether the account with the name is locked, as the database has it.
async fn is_locked(database: &Database, name: &str) -> bool {
    database
        .users()
        .find_by_username(name)
        .await
        .expect("the lookup works")
        .expect("the user is there")
        .locked
}

#[tokio::test]
async fn user_locks_the_own_account_and_opens_it_with_the_password() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[("loginAttempts", json!(NO_LOGIN_LIMIT))]);
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        seed::logged_in(database, &app, "zoe", 4).await;

        let locked = post(&app, As::User(&robin), "/users/lock", None).await;
        assert_eq!(locked.status, StatusCode::OK, "{}", locked.body);
        assert_eq!(locked.message(), json!("User account locked"));
        assert!(is_locked(database, "robin").await);
        assert!(!is_locked(database, "zoe").await);
        // The launch tells the interface, which then shows the lock screen.
        let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
        assert_eq!(launch.data()["user"]["locked"], json!(1));

        let none = post(&app, As::User(&robin), "/users/unlock", Some(json!({}))).await;
        assert_eq!(none.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(none.message(), json!("Password Not Set"));
        let body = json!({"password": "wrong"});
        let wrong = post(&app, As::User(&robin), "/users/unlock", Some(body)).await;
        assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
        assert_eq!(wrong.message(), json!("Password Incorrect"));
        assert!(is_locked(database, "robin").await);

        let body = json!({"password": seed::USER_PASSWORD});
        let opened = post(&app, As::User(&robin), "/users/unlock", Some(body)).await;
        assert_eq!(opened.status, StatusCode::OK, "{}", opened.body);
        assert_eq!(opened.message(), json!("User account unlocked"));
        assert!(!is_locked(database, "robin").await);

        // A guest has no account to lock, and a script with the API key has none either.
        for path in ["/users/lock", "/users/unlock"] {
            let guest = post(&app, As::Guest, path, Some(json!({"password": "x"}))).await;
            assert_eq!(guest.status, StatusCode::UNAUTHORIZED, "{path}");
            let script = post(&app, As::Admin, path, Some(json!({"password": "x"}))).await;
            assert_eq!(script.status, StatusCode::CONFLICT, "{path}");
            assert_eq!(
                script.message(),
                json!("Locking not allowed on Guest users")
            );
        }
    })
    .await;
}

#[tokio::test]
async fn wrong_passwords_for_the_lock_screen_lock_the_client_out() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let limit = [
            ("loginAttempts", json!("2")),
            ("loginLockout", json!("60000")),
        ];
        let app = installed_app_with(database, &limit);
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        post(&app, As::User(&robin), "/users/lock", None).await;

        let wrong = || {
            post(
                &app,
                As::User(&robin),
                "/users/unlock",
                Some(json!({"password": "no"})),
            )
        };
        assert_eq!(wrong().await.status, StatusCode::UNAUTHORIZED);
        let second = wrong().await;
        assert_eq!(second.status, StatusCode::FORBIDDEN);
        assert_eq!(second.message(), json!("User is locked out"));
        // The right password does not open the account while the limit holds.
        let body = json!({"password": seed::USER_PASSWORD});
        let right = post(&app, As::User(&robin), "/users/unlock", Some(body)).await;
        assert_eq!(right.status, StatusCode::FORBIDDEN);
        assert!(is_locked(database, "robin").await);
    })
    .await;
}

#[tokio::test]
async fn manager_locks_and_opens_the_account_of_another_user() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        let boss = seed::logged_in(database, &app, "pat", 1).await;
        let admin = seed::logged_in(database, &app, "zed", 0).await;
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        let id_of = async |name: &str| {
            database
                .users()
                .find_by_username(name)
                .await
                .expect("the lookup works")
                .expect("the user is there")
                .id
        };
        let robin_id = id_of("robin").await;
        let admin_id = id_of("zed").await;

        let lock = format!("/users/lock/{robin_id}");
        let unlock = format!("/users/unlock/{robin_id}");
        for who in [As::Guest, As::User(&robin)] {
            assert_eq!(
                post(&app, who, &lock, None).await.status,
                StatusCode::UNAUTHORIZED
            );
            assert_eq!(
                post(&app, who, &unlock, None).await.status,
                StatusCode::UNAUTHORIZED
            );
        }
        let locked = post(&app, As::User(&boss), &lock, None).await;
        assert_eq!(locked.status, StatusCode::OK, "{}", locked.body);
        assert_eq!(locked.message(), json!("User account locked"));
        assert!(is_locked(database, "robin").await);
        let opened = post(&app, As::User(&boss), &unlock, None).await;
        assert_eq!(opened.message(), json!("User account unlocked"));
        assert!(!is_locked(database, "robin").await);

        for path in [
            "/users/lock/987654",
            "/users/unlock/987654",
            "/users/lock/abc",
        ] {
            let missing = post(&app, As::User(&boss), path, None).await;
            assert_eq!(missing.status, StatusCode::NOT_FOUND, "{path}");
            assert_eq!(missing.message(), json!("User not found"));
        }

        // Only an Admin locks the account of an Admin.
        let admin_lock = format!("/users/lock/{admin_id}");
        let refused = post(&app, As::User(&boss), &admin_lock, None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            refused.message(),
            json!("Only admins can lock or unlock admins")
        );
        assert!(!is_locked(database, "zed").await);
        let done = post(&app, As::User(&admin), &admin_lock, None).await;
        assert_eq!(done.status, StatusCode::OK);
        assert!(is_locked(database, "zed").await);
    })
    .await;
}
