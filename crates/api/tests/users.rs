//! The `/users` routes through the real route table: what the recorded PHP answers
//! cannot show.
//!
//! The recording covers what both applications answer alike. These tests cover the
//! requests where this application is stricter than PHP (D-15, D-52).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_db::Database;
use organizr_domain::UserId;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_app, seed};
use serde_json::{Value, json};

/// The ids of users of the seed: the Admin, and "zoe" of the registered users.
const ADMIN: i64 = 1;
const ZOE: i64 = 3;

/// The user with the given id as `GET /users/{id}` shows it.
async fn user(app: &Router, id: i64) -> Value {
    let answer = call(app, As::Admin, "GET", &format!("/users/{id}"), None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer.data()
}

async fn users(app: &Router) -> Vec<Value> {
    let answer = call(app, As::Admin, "GET", "/users", None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer
        .data()
        .as_array()
        .expect("the data is a list")
        .clone()
}

/// Changes a user with the API key, and expects that the change is taken.
async fn change(app: &Router, id: i64, body: Value) {
    let answer = call(app, As::Admin, "PUT", &format!("/users/{id}"), Some(body)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.message(), json!("User info updated"));
}

fn assert_refused(answer: &Answer, status: StatusCode, message: &str) {
    assert_eq!(answer.status, status, "{}", answer.body);
    assert_eq!(answer.message(), json!(message));
}

/// The install of the recorded cases: the Admin and four users in the group "User".
async fn install(database: &Database) -> Router {
    seed::installed(database).await;
    seed::registered_users(database).await;
    installed_app(database)
}

/// Logs in with a name and a password, and returns the session cookie.
async fn log_in(app: &Router, name: &str, password: &str) -> String {
    let body = json!({"username": name, "password": password});
    let answer = call(app, As::Guest, "POST", "/login", Some(body)).await;
    answer
        .cookies
        .iter()
        .find(|cookie| cookie.starts_with("organizr_token_") && !cookie.contains("=;"))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap_or_else(|| panic!("the login sets the session cookie: {}", answer.body))
        .to_owned()
}

#[tokio::test]
async fn fixed_path_of_another_route_is_an_id_for_the_other_methods() {
    on_each_engine(async |database| {
        let app = install(database).await;

        // `POST /users/register` is a route of its own. PHP reads the path as
        // `/users/{id}` for every other method.
        let read = call(&app, As::Admin, "GET", "/users/register", None).await;
        assert_eq!(read.status, StatusCode::OK, "{}", read.body);
        assert_eq!(read.data(), Value::Null);
        let read = call(&app, As::Guest, "GET", "/users/register", None).await;
        assert_refused(&read, StatusCode::UNAUTHORIZED, "Not Authorized");

        let body = Some(json!({"image": "x.png"}));
        let changed = call(&app, As::Admin, "PUT", "/users/register", body.clone()).await;
        assert_refused(&changed, StatusCode::NOT_FOUND, "User was not found");
        let changed = call(&app, As::Guest, "PUT", "/users/register", body).await;
        assert_refused(&changed, StatusCode::UNAUTHORIZED, "Not Authorized");

        let deleted = call(&app, As::Admin, "DELETE", "/users/register", None).await;
        assert_refused(&deleted, StatusCode::NOT_FOUND, "id not found");
        assert_eq!(
            deleted
                .headers
                .get("content-type")
                .map(axum::http::HeaderValue::as_bytes),
            Some(b"application/json".as_slice())
        );
    })
    .await;
}

#[tokio::test]
async fn value_must_be_of_the_kind_of_its_field() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let before = users(&app).await;

        let wrong = [
            (json!({"username": ["zoe"]}), "username"),
            (json!({"email": {"address": "zoe@example.com"}}), "email"),
            (json!({"password": ["secret"]}), "password"),
            (json!({"image": ["zoe.png"]}), "image"),
            (json!({"group_id": "many"}), "group_id"),
            (json!({"group_id": [1]}), "group_id"),
            // PHP reads an empty level as 0 and makes the user an Admin.
            (json!({"group_id": ""}), "group_id"),
            (json!({"group_id": null}), "group_id"),
            (json!({"locked": "yes"}), "locked"),
            (json!({"locked": 2}), "locked"),
        ];
        for (body, field) in wrong {
            let answer = call(&app, As::Admin, "PUT", &format!("/users/{ZOE}"), Some(body)).await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                &format!("User {field} is not valid"),
            );
        }

        assert_eq!(users(&app).await, before);
    })
    .await;
}

#[tokio::test]
async fn name_of_spaces_is_refused() {
    on_each_engine(async |database| {
        let app = install(database).await;

        for name in ["   ", "\t\n"] {
            let path = format!("/users/{ZOE}");
            let changed = call(
                &app,
                As::Admin,
                "PUT",
                &path,
                Some(json!({"username": name})),
            )
            .await;
            assert_refused(&changed, StatusCode::CONFLICT, "Username was set but empty");
            let body = json!({"username": name, "password": "pw", "email": "new@example.com"});
            let added = call(&app, As::Admin, "POST", "/users", Some(body)).await;
            assert_refused(&added, StatusCode::CONFLICT, "Username was set but empty");
        }
        assert_eq!(user(&app, ZOE).await["username"], json!("zoe"));
        assert_eq!(users(&app).await.len(), 5);
    })
    .await;
}

#[tokio::test]
async fn value_that_php_takes_for_nothing_is_not_supplied() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let full = json!({"username": "New", "password": "pw", "email": "new@example.com"});

        for (field, message) in [
            ("username", "Username was not supplied"),
            ("password", "Password was not supplied"),
            ("email", "Email was set not supplied"),
        ] {
            for nothing in [
                json!(""),
                json!("0"),
                json!(0),
                json!(false),
                Value::Null,
                json!(["x"]),
            ] {
                let mut body = full.clone();
                body[field] = nothing;
                let answer = call(&app, As::Admin, "POST", "/users", Some(body)).await;
                assert_refused(&answer, StatusCode::CONFLICT, message);
            }
        }
        assert_eq!(users(&app).await.len(), 5);
    })
    .await;
}

#[tokio::test]
async fn fields_that_are_no_part_of_a_user_are_ignored() {
    on_each_engine(async |database| {
        let app = install(database).await;

        change(
            &app,
            ZOE,
            json!({
                "id": 70,
                "group": "Admin",
                "plex_token": "a-token",
                "auth_service": "plex",
                "image": "new.png",
            }),
        )
        .await;

        let zoe = user(&app, ZOE).await;
        assert_eq!(zoe["id"], json!(ZOE));
        assert_eq!(zoe["image"], json!("new.png"));
        assert_eq!(zoe["group"], json!("User"));
        assert_eq!(zoe["group_id"], json!(4));
        assert_eq!(zoe["plex_token"], Value::Null);
        assert_eq!(zoe["auth_service"], json!("internal"));
        assert_eq!(user(&app, 70).await, Value::Null);
    })
    .await;
}

#[tokio::test]
async fn name_with_markup_is_stored_as_it_was_sent() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let name = "<b>Dee</b> & co";

        let body = json!({"username": name, "password": "pw", "email": "dee@example.com"});
        let added = call(&app, As::Admin, "POST", "/users", Some(body)).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);

        let dee = user(&app, 6).await;
        assert_eq!(dee["username"], json!(name));
        assert_eq!(dee["group"], json!("User"));
        assert_eq!(dee["locked"], json!(0));
        assert_eq!(dee["password"], Value::Null);
        assert!(
            dee["image"]
                .as_str()
                .is_some_and(|image| image.starts_with("https://www.gravatar.com/avatar/")),
            "{dee}"
        );
    })
    .await;
}

/// Whether the session cookie still names a session.
async fn is_logged_in(app: &Router, cookie: &str) -> bool {
    let me = call(app, As::User(cookie), "GET", "/token/me", None).await;
    me.data()["loggedin"] == json!(true)
}

#[tokio::test]
async fn password_change_ends_the_other_sessions() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let here = log_in(&app, "zoe", seed::ZOE_PASSWORD).await;
        let elsewhere = log_in(&app, "zoe", seed::ZOE_PASSWORD).await;

        // A change of the own password keeps the session of the change and ends the
        // others, such as one that somebody stole (D-146).
        let body = json!({
            "password": "a new password of zoe",
            "currentPassword": seed::ZOE_PASSWORD,
        });
        let answer = call(
            &app,
            As::User(&here),
            "PUT",
            &format!("/users/{ZOE}"),
            Some(body),
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert!(is_logged_in(&app, &here).await);
        assert!(!is_logged_in(&app, &elsewhere).await);

        // A change by an Admin ends every session of the user.
        let again = log_in(&app, "zoe", "a new password of zoe").await;
        change(
            &app,
            ZOE,
            json!({"password": "the password the admin chose"}),
        )
        .await;
        assert!(!is_logged_in(&app, &here).await);
        assert!(!is_logged_in(&app, &again).await);

        // A change of another field ends nothing.
        let kept = log_in(&app, "zoe", "the password the admin chose").await;
        change(&app, ZOE, json!({"email": "zoe@example.org"})).await;
        assert!(is_logged_in(&app, &kept).await);
    })
    .await;
}

#[tokio::test]
async fn password_is_stored_as_a_new_hash_and_has_a_limit() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let stored = async || {
            database
                .users()
                .find(UserId::new(ZOE))
                .await
                .expect("the lookup works")
                .expect("the user exists")
                .password_hash
                .expose()
                .to_owned()
        };
        let before = stored().await;

        let too_long = json!({"password": "x".repeat(201)});
        let answer = call(
            &app,
            As::Admin,
            "PUT",
            &format!("/users/{ZOE}"),
            Some(too_long),
        )
        .await;
        assert_refused(
            &answer,
            StatusCode::CONFLICT,
            "String is over limit of: 200",
        );
        assert_eq!(stored().await, before);

        let new_password = "y".repeat(200);
        change(&app, ZOE, json!({"password": new_password})).await;
        let after = stored().await;
        assert_ne!(after, before);
        assert!(after.starts_with("$argon2id$"), "{after}");
        assert!(!after.contains(&new_password));
        log_in(&app, "zoe", &new_password).await;
    })
    .await;
}

#[tokio::test]
async fn only_an_admin_gives_or_takes_the_admin_level() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let cora = 6;
        let only_admins = "Only admins can make others admins";

        // A Co-Admin cannot make another user or the own account an Admin.
        for id in [ZOE, cora] {
            let path = format!("/users/{id}");
            let body = Some(json!({"group_id": 0}));
            let answer = call(&app, As::User(&co_admin), "PUT", &path, body).await;
            assert_refused(&answer, StatusCode::UNAUTHORIZED, only_admins);
        }
        assert_eq!(user(&app, ZOE).await["group_id"], json!(4));
        assert_eq!(user(&app, cora).await["group_id"], json!(1));
        // A Co-Admin can give every other level.
        let path = format!("/users/{ZOE}");
        let body = Some(json!({"group_id": 1}));
        let answer = call(&app, As::User(&co_admin), "PUT", &path, body).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(user(&app, ZOE).await["group"], json!("Co-Admin"));

        // The Admin can. Then the Co-Admin can neither change nor delete that user.
        change(&app, ZOE, json!({"group_id": 0})).await;
        assert_eq!(user(&app, ZOE).await["group"], json!("Admin"));
        let body = Some(json!({"image": "x.png"}));
        let answer = call(&app, As::User(&co_admin), "PUT", &path, body).await;
        assert_refused(
            &answer,
            StatusCode::UNAUTHORIZED,
            "Cannot update admin unless you are admin",
        );
        let answer = call(&app, As::User(&co_admin), "DELETE", &path, None).await;
        assert_refused(
            &answer,
            StatusCode::UNAUTHORIZED,
            "Cannot delete admin unless you are admin",
        );
        assert_eq!(user(&app, ZOE).await["group_id"], json!(0));
    })
    .await;
}

#[tokio::test]
async fn install_always_has_an_admin() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let admin = format!("/users/{ADMIN}");

        // With the API key nobody deletes the "own" user, so this needs its own rule.
        let deleted = call(&app, As::Admin, "DELETE", &admin, None).await;
        assert_refused(
            &deleted,
            StatusCode::CONFLICT,
            "Cannot delete the last admin",
        );
        let moved = call(&app, As::Admin, "PUT", &admin, Some(json!({"group_id": 4}))).await;
        assert_refused(
            &moved,
            StatusCode::CONFLICT,
            "Cannot change the group of the last admin",
        );
        // The level that the Admin has is no change.
        change(&app, ADMIN, json!({"group_id": 0, "image": "admin.png"})).await;
        assert_eq!(user(&app, ADMIN).await["group_id"], json!(0));
        assert_eq!(user(&app, ADMIN).await["image"], json!("admin.png"));

        // With a second Admin, the first one can leave the group or be deleted.
        change(&app, ZOE, json!({"group_id": 0})).await;
        change(&app, ADMIN, json!({"group_id": 1})).await;
        assert_eq!(user(&app, ADMIN).await["group"], json!("Co-Admin"));
        change(&app, ADMIN, json!({"group_id": 0})).await;
        let deleted = call(&app, As::Admin, "DELETE", &admin, None).await;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.body);
        // Now "zoe" is the last one.
        let path = format!("/users/{ZOE}");
        let deleted = call(&app, As::Admin, "DELETE", &path, None).await;
        assert_refused(
            &deleted,
            StatusCode::CONFLICT,
            "Cannot delete the last admin",
        );
    })
    .await;
}

#[tokio::test]
async fn admin_stays_an_admin_in_a_change_of_the_own_account() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let admin = log_in(&app, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        let path = format!("/users/{ADMIN}");

        // As in PHP, the request is taken and the level stays.
        let body = Some(json!({"group_id": 4, "image": "boss.png"}));
        let answer = call(&app, As::User(&admin), "PUT", &path, body).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let stored = user(&app, ADMIN).await;
        assert_eq!(stored["group_id"], json!(0));
        assert_eq!(stored["image"], json!("boss.png"));

        let answer = call(&app, As::User(&admin), "DELETE", &path, None).await;
        assert_refused(&answer, StatusCode::CONFLICT, "Cannot delete your own user");
        // The Admin can lock and unlock another user.
        let zoe = format!("/users/{ZOE}");
        let answer = call(
            &app,
            As::User(&admin),
            "PUT",
            &zoe,
            Some(json!({"locked": true})),
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(user(&app, ZOE).await["locked"], json!(1));
    })
    .await;
}

#[tokio::test]
async fn user_changes_some_fields_of_the_own_account_only() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let zoe = log_in(&app, "zoe", seed::ZOE_PASSWORD).await;
        let path = format!("/users/{ZOE}");

        let body = Some(json!({
            "username": "Zoe Z",
            "email": "zoe.z@example.com",
            "currentPassword": seed::ZOE_PASSWORD,
        }));
        let answer = call(&app, As::User(&zoe), "PUT", &path, body).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let stored = user(&app, ZOE).await;
        assert_eq!(stored["username"], json!("Zoe Z"));
        assert_eq!(stored["email"], json!("zoe.z@example.com"));

        for locked in [json!(1), json!(0)] {
            let body = Some(json!({"locked": locked}));
            let answer = call(&app, As::User(&zoe), "PUT", &path, body).await;
            assert_refused(
                &answer,
                StatusCode::UNAUTHORIZED,
                "Cannot change your own locked",
            );
        }
        // The level of the own group is refused, also when it is the level the user has.
        let answer = call(
            &app,
            As::User(&zoe),
            "PUT",
            &path,
            Some(json!({"group_id": 4})),
        )
        .await;
        assert_refused(
            &answer,
            StatusCode::UNAUTHORIZED,
            "Cannot change your own group_id",
        );
        let stored = user(&app, ZOE).await;
        assert_eq!(stored["locked"], json!(0));
        assert_eq!(stored["group_id"], json!(4));
    })
    .await;
}

/// A change of the own password or e-mail address needs the current password, so that
/// a stolen session alone cannot take the account over (D-185).
#[tokio::test]
async fn own_password_and_address_need_the_current_password() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let zoe = log_in(&app, "zoe", seed::ZOE_PASSWORD).await;
        let path = format!("/users/{ZOE}");
        let address = user(&app, ZOE).await["email"].clone();
        let put = async |body: Value| call(&app, As::User(&zoe), "PUT", &path, Some(body)).await;

        for mut body in [
            json!({"password": "a password of the thief"}),
            json!({"email": "thief@example.com"}),
        ] {
            let answer = put(body.clone()).await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                "Enter the password of the account",
            );
            body["currentPassword"] = json!("not the password");
            let answer = put(body).await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                "The password is not the password of the account",
            );
        }
        assert_eq!(user(&app, ZOE).await["email"], address);
        log_in(&app, "zoe", seed::ZOE_PASSWORD).await;

        // Other fields need no password, and the address the account has is no change.
        let body = json!({"username": "Zoe Z", "email": address, "image": "zoe.png"});
        let answer = put(body).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

        let body = json!({"email": "zoe.new@example.com", "currentPassword": seed::ZOE_PASSWORD});
        let answer = put(body).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(user(&app, ZOE).await["email"], json!("zoe.new@example.com"));
        let body =
            json!({"password": "a new password of zoe", "currentPassword": seed::ZOE_PASSWORD});
        let answer = put(body).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        log_in(&app, "Zoe Z", "a new password of zoe").await;

        // The Admin needs it for the own account too, and not for another account.
        let admin = log_in(&app, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        let own = format!("/users/{ADMIN}");
        let body = Some(json!({"password": "a password of the thief"}));
        let answer = call(&app, As::User(&admin), "PUT", &own, body).await;
        assert_refused(
            &answer,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Enter the password of the account",
        );
        let body = Some(json!({"password": "the password the admin chose"}));
        let answer = call(&app, As::User(&admin), "PUT", &path, body).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        log_in(&app, "Zoe Z", "the password the admin chose").await;
    })
    .await;
}

#[tokio::test]
async fn list_with_groups_needs_the_key_in_the_query() {
    on_each_engine(async |database| {
        let app = install(database).await;

        for query in ["includeGroups", "includeGroups=0", "a=1&includeGroups=true"] {
            let answer = call(&app, As::Admin, "GET", &format!("/users?{query}"), None).await;
            let data = answer.data();
            assert_eq!(data["users"].as_array().map(Vec::len), Some(5), "{query}");
            assert_eq!(data["groups"].as_array().map(Vec::len), Some(6), "{query}");
        }
        for query in ["", "?includegroups", "?groups"] {
            let answer = call(&app, As::Admin, "GET", &format!("/users{query}"), None).await;
            assert!(answer.data().is_array(), "{query}: {}", answer.body);
        }
    })
    .await;
}

#[tokio::test]
async fn checks_of_a_new_user_run_in_the_order_of_php() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let long_name = "x".repeat(51);

        // The address is checked before the lengths.
        let body = json!({"username": long_name, "password": "pw", "email": "not an address"});
        let answer = call(&app, As::Admin, "POST", "/users", Some(body)).await;
        assert_refused(&answer, StatusCode::CONFLICT, "Email is not a valid email");
        assert_eq!(answer.data(), json!({"email": "not an address"}));

        // The lengths are checked before the names that are taken.
        let body =
            json!({"username": "zoe", "password": "x".repeat(201), "email": "new@example.com"});
        let answer = call(&app, As::Admin, "POST", "/users", Some(body)).await;
        assert_refused(
            &answer,
            StatusCode::CONFLICT,
            "String is over limit of: 200",
        );
        assert_eq!(users(&app).await.len(), 5);
    })
    .await;
}

#[tokio::test]
async fn guest_is_refused_before_the_id_is_looked_at() {
    on_each_engine(async |database| {
        let app = install(database).await;

        for id in ["0", "99", "abc"] {
            let path = format!("/users/{id}");
            let answer = call(
                &app,
                As::Guest,
                "PUT",
                &path,
                Some(json!({"image": "x.png"})),
            )
            .await;
            assert_refused(&answer, StatusCode::UNAUTHORIZED, "Not Authorized");
        }
    })
    .await;
}

#[tokio::test]
async fn co_admin_cannot_take_over_another_co_admin() {
    on_each_engine(async |database| {
        let app = install(database).await;
        let cora = seed::logged_in(database, &app, "cora", 1).await;
        let _cole = seed::logged_in(database, &app, "cole", 1).await;
        let (cora_id, cole_id) = (6, 7);
        let cole = format!("/users/{cole_id}");
        let update = "Cannot update admin unless you are admin";

        // Cora cannot change the password, the address, the lock, the group or the name
        // of Cole: a login service finds the account by its name (D-188).
        for body in [
            json!({"password": "taken-over"}),
            json!({"email": "cora@example.com"}),
            json!({"locked": true}),
            json!({"group_id": 4}),
            json!({"username": "colette"}),
        ] {
            let answer = call(&app, As::User(&cora), "PUT", &cole, Some(body.clone())).await;
            assert_refused(&answer, StatusCode::UNAUTHORIZED, update);
        }
        let stored = user(&app, cole_id).await;
        assert_eq!(stored["locked"], json!(0));
        assert_eq!(stored["group_id"], json!(1));
        assert_ne!(stored["email"], json!("cora@example.com"));
        log_in(&app, "cole", seed::USER_PASSWORD).await;
        // Nor lock or open the account, nor remove its second factor.
        for (method, path, message) in [
            (
                "POST",
                format!("/users/lock/{cole_id}"),
                "Only admins can lock or unlock admins",
            ),
            (
                "POST",
                format!("/users/unlock/{cole_id}"),
                "Only admins can lock or unlock admins",
            ),
            (
                "DELETE",
                format!("/users/2fa/{cole_id}"),
                "Only admins can remove the second factor of admins",
            ),
        ] {
            let answer = call(&app, As::User(&cora), method, &path, None).await;
            assert_refused(&answer, StatusCode::UNAUTHORIZED, message);
        }
        assert_eq!(user(&app, cole_id).await["locked"], json!(0));

        // Nor delete Cole: the account could then be made again with Cole's name and
        // address and a password of Cora's (D-188).
        let answer = call(&app, As::User(&cora), "DELETE", &cole, None).await;
        assert_refused(
            &answer,
            StatusCode::UNAUTHORIZED,
            "Cannot delete admin unless you are admin",
        );
        assert_eq!(user(&app, cole_id).await["username"], json!("cole"));

        // The other fields of Cole, the own account and the accounts of users stay open.
        let body = Some(json!({"username": "cole", "image": "c.png"}));
        let answer = call(&app, As::User(&cora), "PUT", &cole, body).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let body =
            Some(json!({"email": "cora@example.com", "currentPassword": seed::USER_PASSWORD}));
        let own = format!("/users/{cora_id}");
        let answer = call(&app, As::User(&cora), "PUT", &own, body).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let zoe = format!("/users/{ZOE}");
        for body in [json!({"password": "new-one"}), json!({"locked": true})] {
            let answer = call(&app, As::User(&cora), "PUT", &zoe, Some(body)).await;
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        }
        let answer = call(
            &app,
            As::User(&cora),
            "POST",
            &format!("/users/unlock/{ZOE}"),
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

        // The Admin still changes Cole.
        change(
            &app,
            cole_id,
            json!({"password": "by-the-admin", "locked": true}),
        )
        .await;
        assert_eq!(user(&app, cole_id).await["locked"], json!(1));
        let answer = call(
            &app,
            As::Admin,
            "DELETE",
            &format!("/users/2fa/{cole_id}"),
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    })
    .await;
}
