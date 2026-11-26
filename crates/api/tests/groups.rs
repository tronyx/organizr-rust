//! The `/groups` routes through the real route table: what the recorded PHP answers
//! cannot show.
//!
//! The recording covers what both applications answer alike. These tests cover callers
//! of other levels, and the requests where this application is stricter than PHP (D-51).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_db::Database;
use organizr_domain::{GroupLevel, NewGroup};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_app, seed};
use serde_json::{Value, json};

/// The ids of groups of the seed.
const ADMIN: i64 = 1;
const USER: i64 = 5;
/// The id of the first group that a test adds.
const FIRST_NEW: i64 = 7;

fn editors() -> Value {
    json!({"group": "Editors", "image": "editors.png"})
}

/// The groups as `GET /groups` lists them.
async fn groups(app: &Router) -> Vec<Value> {
    let answer = call(app, As::Admin, "GET", "/groups", None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer
        .data()
        .as_array()
        .expect("the data is a list")
        .clone()
}

/// The group with the given id as `GET /groups/{id}` shows it.
async fn group(app: &Router, id: i64) -> Value {
    let answer = call(app, As::Admin, "GET", &format!("/groups/{id}"), None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer.data()
}

/// Adds the group "Editors", which gets the id [`FIRST_NEW`].
async fn add_editors(app: &Router) {
    let answer = call(app, As::Admin, "POST", "/groups", Some(editors())).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
}

fn assert_refused(answer: &Answer, status: StatusCode, message: &str) {
    assert_eq!(answer.status, status, "{}", answer.body);
    assert_eq!(answer.message(), json!(message));
}

#[tokio::test]
async fn co_admin_manages_groups_and_other_users_do_not() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let super_user = seed::logged_in(database, &app, "sam", 2).await;

        let requests = [
            ("GET", "/groups".to_owned(), None),
            ("GET", format!("/groups/{USER}"), None),
            ("POST", "/groups".to_owned(), Some(editors())),
            (
                "PUT",
                format!("/groups/{FIRST_NEW}"),
                Some(json!({"image": "new.png"})),
            ),
            ("DELETE", format!("/groups/{FIRST_NEW}"), None),
        ];
        for (method, path, body) in &requests {
            let answer = call(&app, As::User(&super_user), method, path, body.clone()).await;
            assert_refused(&answer, StatusCode::UNAUTHORIZED, "Not Authorized");
        }
        assert_eq!(groups(&app).await.len(), 6, "nothing was added");

        for (method, path, body) in requests {
            let answer = call(&app, As::User(&co_admin), method, &path, body).await;
            assert!(
                answer.status.is_success(),
                "{method} {path}: {}",
                answer.body
            );
        }
        assert_eq!(
            groups(&app).await.len(),
            6,
            "the new group was deleted again"
        );
    })
    .await;
}

#[tokio::test]
async fn value_must_be_of_the_kind_of_its_field() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        add_editors(&app).await;
        let before = groups(&app).await;

        let wrong = [
            (json!({"group": ["Writers"], "image": "i"}), "group"),
            (json!({"group": {"name": "Writers"}, "image": "i"}), "group"),
            (json!({"group": "Writers", "image": ["i"]}), "image"),
            (
                json!({"group": "Writers", "image": "i", "default": "yes"}),
                "default",
            ),
            (
                json!({"group": "Writers", "image": "i", "default": 2}),
                "default",
            ),
            (
                json!({"group": "Writers", "image": "i", "default": []}),
                "default",
            ),
        ];
        for (body, field) in wrong {
            let message = format!("Group {field} is not valid");
            let added = call(&app, As::Admin, "POST", "/groups", Some(body.clone())).await;
            assert_refused(&added, StatusCode::UNPROCESSABLE_ENTITY, &message);
            let path = format!("/groups/{FIRST_NEW}");
            let changed = call(&app, As::Admin, "PUT", &path, Some(body)).await;
            assert_refused(&changed, StatusCode::UNPROCESSABLE_ENTITY, &message);
        }

        assert_eq!(groups(&app).await, before);
    })
    .await;
}

#[tokio::test]
async fn switch_as_text_or_as_true_is_taken() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        for (number, default) in [json!("1"), json!(true), json!(1)].into_iter().enumerate() {
            let body =
                json!({"group": format!("Group {number}"), "image": "i", "default": default});
            let answer = call(&app, As::Admin, "POST", "/groups", Some(body)).await;
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

            let defaults: Vec<Value> = groups(&app)
                .await
                .into_iter()
                .filter(|group| group["default"] == json!(1))
                .map(|group| group["group"].clone())
                .collect();
            assert_eq!(defaults, [json!(format!("Group {number}"))]);
        }
    })
    .await;
}

#[tokio::test]
async fn group_needs_a_name_and_a_picture() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        add_editors(&app).await;
        let before = groups(&app).await;
        let path = format!("/groups/{FIRST_NEW}");

        for name in ["", "   ", "\t\n"] {
            let body = json!({"group": name, "image": "i"});
            let added = call(&app, As::Admin, "POST", "/groups", Some(body)).await;
            assert_refused(
                &added,
                StatusCode::UNPROCESSABLE_ENTITY,
                "Group name was not supplied",
            );
            let changed = call(&app, As::Admin, "PUT", &path, Some(json!({"group": name}))).await;
            assert_refused(&changed, StatusCode::CONFLICT, "Group was set but empty");
        }
        for image in [json!("  "), Value::Null] {
            let body = json!({"group": "Writers", "image": image});
            let added = call(&app, As::Admin, "POST", "/groups", Some(body)).await;
            assert_refused(
                &added,
                StatusCode::UNPROCESSABLE_ENTITY,
                "Group image cannot be empty",
            );
            let changed = call(&app, As::Admin, "PUT", &path, Some(json!({"image": image}))).await;
            assert_refused(&changed, StatusCode::CONFLICT, "Image was set but empty");
        }

        assert_eq!(groups(&app).await, before);
    })
    .await;
}

#[tokio::test]
async fn row_id_in_a_request_is_ignored() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        let body = json!({"id": 70, "group": "Editors", "image": "i"});
        let added = call(&app, As::Admin, "POST", "/groups", Some(body)).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
        assert_eq!(group(&app, FIRST_NEW).await["group"], json!("Editors"));
        assert_eq!(group(&app, 70).await, Value::Null);

        let path = format!("/groups/{FIRST_NEW}");
        let body = json!({"id": 71, "image": "new.png"});
        let changed = call(&app, As::Admin, "PUT", &path, Some(body)).await;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        assert_eq!(group(&app, FIRST_NEW).await["image"], json!("new.png"));
        assert_eq!(group(&app, 71).await, Value::Null);
    })
    .await;
}

#[tokio::test]
async fn name_is_compared_as_text() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        // In a pattern of SQL, `_` stands for any character and `%` for any text. PHP
        // compares names with such a pattern.
        for name in ["Use_", "%", "U%"] {
            let body = json!({"group": name, "image": "i"});
            let answer = call(&app, As::Admin, "POST", "/groups", Some(body)).await;
            assert_eq!(answer.status, StatusCode::OK, "{name}: {}", answer.body);
        }
        let body = json!({"group": "uSER", "image": "i"});
        let answer = call(&app, As::Admin, "POST", "/groups", Some(body)).await;
        assert_refused(
            &answer,
            StatusCode::CONFLICT,
            "Group name: uSER is already taken",
        );
    })
    .await;
}

#[tokio::test]
async fn taken_name_is_reported_before_a_missing_or_empty_picture() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        add_editors(&app).await;

        for body in [
            json!({"group": "user"}),
            json!({"group": "user", "image": ""}),
        ] {
            let answer = call(&app, As::Admin, "POST", "/groups", Some(body)).await;
            assert_refused(
                &answer,
                StatusCode::CONFLICT,
                "Group name: user is already taken",
            );
        }
        let path = format!("/groups/{FIRST_NEW}");
        let body = json!({"group": "user", "image": ""});
        let answer = call(&app, As::Admin, "PUT", &path, Some(body)).await;
        assert_refused(
            &answer,
            StatusCode::CONFLICT,
            "Group name: user is already taken",
        );
    })
    .await;
}

#[tokio::test]
async fn there_is_always_one_default_group() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        add_editors(&app).await;
        let before = groups(&app).await;
        let user = format!("/groups/{USER}");
        let editors = format!("/groups/{FIRST_NEW}");

        // The mark cannot be taken from the default group, in any way to write "off".
        for off in [json!(0), json!(false), json!("0"), Value::Null] {
            let answer = call(&app, As::Admin, "PUT", &user, Some(json!({"default": off}))).await;
            assert_refused(
                &answer,
                StatusCode::CONFLICT,
                "Cannot remove default from group: User as a default group is needed",
            );
        }
        let deleted = call(&app, As::Admin, "DELETE", &user, None).await;
        assert_refused(
            &deleted,
            StatusCode::CONFLICT,
            "Cannot delete group: User as it is the default group",
        );
        // "Off" for a group that is not the default changes nothing.
        let answer = call(
            &app,
            As::Admin,
            "PUT",
            &editors,
            Some(json!({"default": 0})),
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(groups(&app).await, before);

        // Another group takes the mark. Then the group that had it can go.
        let answer = call(
            &app,
            As::Admin,
            "PUT",
            &editors,
            Some(json!({"default": 1})),
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(group(&app, USER).await["default"], json!(0));
        assert_eq!(group(&app, FIRST_NEW).await["default"], json!(1));
        let deleted = call(&app, As::Admin, "DELETE", &user, None).await;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.body);
        assert_eq!(deleted.body, "");
    })
    .await;
}

#[tokio::test]
async fn group_with_users_is_not_deleted_whether_default_or_not() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        add_editors(&app).await;
        seed::logged_in(database, &app, "eve", 5).await;
        let editors = format!("/groups/{FIRST_NEW}");
        let has_users = "Cannot delete group as group still has users assigned to it";

        let answer = call(&app, As::Admin, "DELETE", &editors, None).await;
        assert_refused(&answer, StatusCode::CONFLICT, has_users);
        // The users are named before the mark of the default group.
        let answer = call(
            &app,
            As::Admin,
            "PUT",
            &editors,
            Some(json!({"default": 1})),
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let answer = call(&app, As::Admin, "DELETE", &editors, None).await;
        assert_refused(&answer, StatusCode::CONFLICT, has_users);
        assert_eq!(groups(&app).await.len(), 7);
    })
    .await;
}

#[tokio::test]
async fn id_without_a_group_is_not_found() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        for id in ["99", "abc", "-1", "1.5", "7 "] {
            let path = format!("/groups/{}", id.replace(' ', "%20"));
            let deleted = call(&app, As::Admin, "DELETE", &path, None).await;
            assert_refused(&deleted, StatusCode::NOT_FOUND, "id not found");
            let changed = call(&app, As::Admin, "PUT", &path, Some(json!({"image": "i"}))).await;
            assert_refused(&changed, StatusCode::NOT_FOUND, "No category info found");
            let read = call(&app, As::Admin, "GET", &path, None).await;
            assert_eq!(read.status, StatusCode::OK, "{}", read.body);
            assert_eq!(read.data(), Value::Null);
        }
        assert_eq!(groups(&app).await.len(), 6);
    })
    .await;
}

#[tokio::test]
async fn level_of_a_new_group_follows_the_highest_one_below_guest() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        store_group(database, 997, "Almost Guest").await;

        add_editors(&app).await;
        let listed = groups(&app).await;
        let levels: Vec<&Value> = listed.iter().map(|group| &group["group_id"]).collect();
        assert_eq!(levels, [0, 1, 2, 3, 4, 997, 998, 999]);
        assert_eq!(listed[6]["group"], json!("Editors"));

        // The next level would be the one of guests.
        let body = json!({"group": "Writers", "image": "i"});
        let answer = call(&app, As::Admin, "POST", "/groups", Some(body)).await;
        assert_refused(
            &answer,
            StatusCode::CONFLICT,
            "Cannot add group as no level is free",
        );
        assert_eq!(groups(&app).await.len(), 8);
    })
    .await;
}

async fn store_group(database: &Database, level: u16, name: &str) {
    database
        .groups()
        .insert(NewGroup {
            level: GroupLevel::new(level).expect("the level is valid"),
            name: name.to_owned(),
            image: Some("i".to_owned()),
            is_default: false,
        })
        .await
        .expect("the group can be stored");
}

#[tokio::test]
async fn fixed_groups_stay_and_are_never_the_default() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let admin = format!("/groups/{ADMIN}");

        // Every way to name the switch is refused for the group of the Admin.
        for default in [json!(1), json!(0), json!(true)] {
            let answer = call(
                &app,
                As::Admin,
                "PUT",
                &admin,
                Some(json!({"default": default})),
            )
            .await;
            assert_refused(
                &answer,
                StatusCode::CONFLICT,
                "Setting Admin as default group is not allowed",
            );
        }
        let answer = call(&app, As::Admin, "PUT", &admin, Some(json!({"group_id": 0}))).await;
        assert_refused(&answer, StatusCode::CONFLICT, "Cannot change group_id");
        assert_eq!(group(&app, ADMIN).await["group_id"], json!(0));
        assert_eq!(group(&app, USER).await["default"], json!(1));
    })
    .await;
}

#[tokio::test]
async fn list_with_users_shows_no_secret_of_a_user() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        seed::logged_in(database, &app, "eve", 4).await;

        for query in ["includeUsers", "includeUsers=0", "a=1&includeUsers=true"] {
            let answer = call(&app, As::Admin, "GET", &format!("/groups?{query}"), None).await;
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            let data = answer.data();
            assert_eq!(data["groups"].as_array().map(Vec::len), Some(6), "{query}");
            let users = data["users"].as_array().expect("the users are a list");
            assert_eq!(users.len(), 2);
            let eve = &users[1];
            assert_eq!(eve["username"], json!("eve"));
            assert_eq!(eve["password"], Value::Null);
            assert_eq!(eve["plex_token"], Value::Null);
            assert_eq!(eve["group"], json!("User"));
            assert_eq!(eve["group_id"], json!(4));
            assert_eq!(eve["locked"], json!(0));
            assert_eq!(eve["register_date"], json!("2027-01-15 08:00:00"));
            assert_eq!(eve["auth_service"], json!("internal"));
            assert!(!answer.body.contains("$argon2"), "{}", answer.body);
            assert!(!answer.body.contains("$2y$"), "{}", answer.body);
        }
        // Without the key, the answer is the list of groups alone.
        for query in ["", "?includeusers", "?users"] {
            let answer = call(&app, As::Admin, "GET", &format!("/groups{query}"), None).await;
            assert!(answer.data().is_array(), "{query}: {}", answer.body);
        }
    })
    .await;
}

#[tokio::test]
async fn group_of_the_co_admin_never_becomes_the_default() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let co_admin = seed::logged_in(database, &app, "casey", 1).await;
        let co_admins = database
            .groups()
            .find_by_level(GroupLevel::CO_ADMIN)
            .await
            .expect("the lookup works")
            .expect("the seed has the group of the Co-Admin");
        let path = format!("/groups/{}", co_admins.id.get());

        // New users would be Co-Admins then, so nobody can make it the default.
        for who in [As::Admin, As::User(&co_admin)] {
            let body = json!({"default": true});
            let answer = call(&app, who, "PUT", &path, Some(body)).await;
            assert_refused(
                &answer,
                StatusCode::CONFLICT,
                &format!("Setting {} as default group is not allowed", co_admins.name),
            );
        }
        assert_eq!(group(&app, USER).await["default"], json!(1));
        // Other changes of the group still work.
        let body = json!({"default": false, "image": "other.png"});
        let answer = call(&app, As::Admin, "PUT", &path, Some(body)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    })
    .await;
}
