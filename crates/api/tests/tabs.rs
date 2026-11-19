//! The `/tabs` routes through the real route table: what the recorded PHP answers cannot
//! show.
//!
//! The recording covers what both applications answer alike. These tests cover callers
//! of other levels, and the requests where this application is stricter than PHP: PHP
//! stores any value in any field, and some requests make it crash.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_auth::password;
use organizr_db::Database;
use organizr_domain::{GroupLevel, LevelRange, NewTab, NewUser, Tab, TabKind, Timestamp};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, request, send};
use organizr_testkit::{installed_app, seed};
use serde_json::{Value, json};

const PASSWORD: &str = "the password of the test users";

/// Who sends a request.
#[derive(Clone, Copy)]
enum As<'a> {
    /// Nobody: no cookie and no key.
    Guest,
    /// A script with the API key, which counts as the Admin.
    Admin,
    /// A logged-in user, by the session cookie.
    User(&'a str),
}

fn level(value: u16) -> GroupLevel {
    GroupLevel::new(value).expect("the level is valid")
}

/// Sends a request to a tab route. A body is sent as JSON.
async fn call(app: &Router, who: As<'_>, method: &str, path: &str, body: Option<Value>) -> Answer {
    let mut headers: Vec<(&str, &str)> = Vec::new();
    match who {
        As::Guest => {}
        As::Admin => headers.push(("token", seed::API_KEY)),
        As::User(cookie) => headers.push(("cookie", cookie)),
    }
    let text = body.as_ref().map_or_default(Value::to_string);
    if body.is_some() {
        headers.push(("content-type", "application/json"));
    }
    let path = format!("/api/v2{path}");
    send(app, request(method, &path, &headers, &text)).await
}

/// Stores a user in the group of the given level, logs the user in, and returns the
/// session cookie as a browser sends it back.
async fn logged_in(database: &Database, app: &Router, name: &str, group_level: u16) -> String {
    let group = database
        .groups()
        .find_by_level(level(group_level))
        .await
        .expect("the lookup works")
        .expect("the seed has the group");
    database
        .users()
        .insert(NewUser {
            username: name.to_owned(),
            email: Some(format!("{name}@example.com")),
            password_hash: password::hash(PASSWORD).expect("the password can be hashed"),
            group: group.id,
            image: None,
            created_at: Timestamp::from_unix_seconds(1_800_000_000),
        })
        .await
        .expect("the user can be stored");
    let body = json!({"username": name, "password": PASSWORD}).to_string();
    let headers = [("content-type", "application/json")];
    let answer = send(app, request("POST", "/api/v2/login", &headers, &body)).await;
    answer
        .cookies
        .iter()
        .find(|cookie| cookie.starts_with("organizr_token_"))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap_or_else(|| panic!("the login sets the session cookie: {}", answer.body))
        .to_owned()
}

/// Stores a tab in the category of the seed.
async fn store_tab(database: &Database, name: &str, change: impl FnOnce(&mut NewTab)) -> Tab {
    let category = database
        .categories()
        .list()
        .await
        .expect("the categories can be read")
        .into_iter()
        .next()
        .expect("the seed has a category");
    let mut new = NewTab {
        position: 10,
        category: category.id,
        name: name.to_owned(),
        url: Some("https://plex.example.com/".to_owned()),
        url_local: None,
        is_default: false,
        enabled: true,
        access: LevelRange::up_to(level(4)),
        add_to_admin: false,
        image: Some("plugins/images/tabs/plex.png".to_owned()),
        kind: TabKind::Iframe,
        splash: false,
        ping: false,
        ping_url: None,
        timeout: false,
        timeout_ms: None,
        preload: false,
    };
    change(&mut new);
    database
        .tabs()
        .insert(new)
        .await
        .expect("the tab can be stored")
}

/// All tabs as the database holds them, in menu order.
async fn stored_tabs(database: &Database) -> Vec<Tab> {
    database.tabs().list().await.expect("the tabs can be read")
}

/// A body for `POST /tabs` that passes, with other values for some fields.
fn new_tab(name: &str, fields: &Value) -> Value {
    let mut body = json!({
        "name": name,
        "url": "https://plex.example.com/",
        "image": "plugins/images/tabs/plex.png",
    });
    let object = body.as_object_mut().expect("the body is an object");
    for (key, value) in fields.as_object().expect("the fields are an object") {
        object.insert(key.clone(), value.clone());
    }
    body
}

fn assert_refused(answer: &Answer, status: StatusCode, message: &str, context: &str) {
    assert_eq!(answer.status, status, "{context}: {}", answer.body);
    assert_eq!(answer.message(), json!(message), "{context}");
    assert_eq!(
        answer.json()["response"]["result"],
        json!("error"),
        "{context}"
    );
}

#[tokio::test]
async fn tab_routes_are_for_admins_and_co_admins() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let plex = store_tab(database, "Plex", |_| {}).await;
        let zoe = logged_in(database, &app, "zoe", 4).await;
        let super_user = logged_in(database, &app, "sam", 2).await;
        let co_admin = logged_in(database, &app, "cora", 1).await;
        let before = stored_tabs(database).await;
        let one = format!("/tabs/{}", plex.id);
        let requests = [
            ("GET", "/tabs", None),
            ("GET", one.as_str(), None),
            ("POST", "/tabs", Some(new_tab("Sonarr", &json!({})))),
            (
                "PUT",
                "/tabs",
                Some(json!([{"id": plex.id.get(), "order": 50}])),
            ),
            ("PUT", one.as_str(), Some(json!({"name": "Changed"}))),
            ("DELETE", one.as_str(), None),
        ];

        for (method, path, body) in &requests {
            for who in [As::Guest, As::User(&zoe), As::User(&super_user)] {
                let answer = call(&app, who, method, path, body.clone()).await;
                assert_refused(
                    &answer,
                    StatusCode::UNAUTHORIZED,
                    "Not Authorized",
                    &format!("{method} {path}"),
                );
            }
        }
        assert_eq!(stored_tabs(database).await, before, "nothing was changed");

        // A Co-Admin passes on every route. The delete comes last.
        for (method, path, body) in requests {
            let answer = call(&app, As::User(&co_admin), method, path, body).await;
            assert!(
                answer.status.is_success(),
                "{method} {path}: {}",
                answer.body
            );
        }
    })
    .await;
}

#[tokio::test]
async fn one_tab_is_shown_to_the_levels_that_see_it() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let for_admins = store_tab(database, "Vault", |tab| {
            tab.access = LevelRange::up_to(GroupLevel::ADMIN);
        })
        .await;
        let co_admin = logged_in(database, &app, "cora", 1).await;
        let path = format!("/tabs/{}", for_admins.id);

        let refused = call(&app, As::User(&co_admin), "GET", &path, None).await;
        let shown = call(&app, As::Admin, "GET", &path, None).await;

        assert_refused(
            &refused,
            StatusCode::UNAUTHORIZED,
            "Not Authorized",
            "co-admin",
        );
        assert_eq!(shown.status, StatusCode::OK);
        assert_eq!(shown.data()["name"], json!("Vault"));
        // The list for the tab editor shows every tab to a Co-Admin, as in PHP.
        let list = call(&app, As::User(&co_admin), "GET", "/tabs", None).await;
        let names: Vec<Value> = list.data()["tabs"]
            .as_array()
            .expect("the tabs are a list")
            .iter()
            .map(|tab| tab["name"].clone())
            .collect();
        assert!(names.contains(&json!("Vault")), "{names:?}");
    })
    .await;
}

#[tokio::test]
async fn new_tab_with_a_value_of_the_wrong_kind_is_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let before = stored_tabs(database).await;
        let wrong = [
            ("enabled", json!("yes")),
            ("default", json!(2)),
            ("splash", json!([1])),
            ("type", json!(3)),
            ("type", json!("frame")),
            ("group_id", json!(1000)),
            ("group_id", json!(-1)),
            ("group_id_max", json!("admin")),
            // No category has this number.
            ("category_id", json!(77)),
            ("order", json!("first")),
            ("order", json!(1.5)),
            ("timeout_ms", json!("soon")),
            ("timeout_ms", json!(true)),
            ("name", json!(["Plex"])),
            ("url", json!({"address": "https://plex.example.com/"})),
            ("url_local", json!(["http://10.0.0.5/"])),
            ("ping_url", json!([])),
            ("image", json!({})),
        ];

        for (field, value) in wrong {
            let body = new_tab("Sonarr", &json!({field: value}));
            let answer = call(&app, As::Admin, "POST", "/tabs", Some(body.clone())).await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                &format!("Tab {field} is not valid"),
                &body.to_string(),
            );
        }
        assert_eq!(stored_tabs(database).await, before, "nothing was stored");
    })
    .await;
}

#[tokio::test]
async fn change_with_a_value_of_the_wrong_kind_is_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let plex = store_tab(database, "Plex", |_| {}).await;
        let path = format!("/tabs/{}", plex.id);
        let before = stored_tabs(database).await;
        let wrong = [
            ("enabled", json!("on")),
            ("type", json!(9)),
            ("group_id", json!(1000)),
            ("category_id", json!(77)),
            ("order", json!("last")),
            ("timeout_ms", json!([60000])),
            ("name", json!({"text": "Plex"})),
            ("url", json!([])),
            // A field that cannot be empty.
            ("order", Value::Null),
            ("type", Value::Null),
            ("group_id", Value::Null),
            ("group_id_max", Value::Null),
            ("category_id", Value::Null),
            ("order", json!("")),
        ];

        for (field, value) in wrong {
            // A valid field next to it must not be stored either.
            let body = json!({field: value, "ping": 1});
            let answer = call(&app, As::Admin, "PUT", &path, Some(body.clone())).await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                &format!("Tab {field} is not valid"),
                &body.to_string(),
            );
        }
        assert_eq!(stored_tabs(database).await, before, "nothing was changed");
    })
    .await;
}

#[tokio::test]
async fn tab_cannot_be_without_a_name() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let plex = store_tab(database, "Plex", |_| {}).await;
        let path = format!("/tabs/{}", plex.id);
        let before = stored_tabs(database).await;

        // PHP stores a tab without a name for each of these.
        for name in [json!(""), json!("  \t "), Value::Null, json!(false)] {
            let body = new_tab("x", &json!({"name": name}));
            let added = call(&app, As::Admin, "POST", "/tabs", Some(body)).await;
            let changed = call(&app, As::Admin, "PUT", &path, Some(json!({"name": name}))).await;
            for answer in [added, changed] {
                assert_refused(
                    &answer,
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Tab name was not supplied",
                    &name.to_string(),
                );
            }
        }
        assert_eq!(stored_tabs(database).await, before);
    })
    .await;
}

#[tokio::test]
async fn id_of_a_tab_cannot_be_chosen_or_changed() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let highest = |tabs: &[Tab]| tabs.iter().map(|tab| tab.id.get()).max().unwrap_or(0);
        let before = highest(&stored_tabs(database).await);

        let body = new_tab("Sonarr", &json!({"id": before + 40}));
        let added = call(&app, As::Admin, "POST", "/tabs", Some(body)).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
        let sonarr = before + 1;
        assert_eq!(highest(&stored_tabs(database).await), sonarr);

        let path = format!("/tabs/{sonarr}");
        let body = json!({"id": sonarr + 40, "enabled": 1});
        let changed = call(&app, As::Admin, "PUT", &path, Some(body)).await;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        let tabs = stored_tabs(database).await;
        assert_eq!(highest(&tabs), sonarr);
        let stored = tabs
            .iter()
            .find(|tab| tab.id.get() == sonarr)
            .expect("the tab kept its id");
        assert!(stored.enabled, "the other field of the request was stored");
    })
    .await;
}

#[tokio::test]
async fn new_tab_cannot_have_its_levels_the_wrong_way_round() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let before = stored_tabs(database).await;

        // Most access "User" (4), least access "Co-Admin" (1): no level is in between.
        let body = new_tab("Sonarr", &json!({"group_id": 1, "group_id_max": 4}));
        let answer = call(&app, As::Admin, "POST", "/tabs", Some(body)).await;

        assert_refused(
            &answer,
            StatusCode::CONFLICT,
            "Tab name: Sonarr cannot have a lower Group Id Max than Group Id Min",
            "swapped levels",
        );
        assert_eq!(stored_tabs(database).await, before);

        // The default group of the seed is "User" (4). A tab for Guests only, without a
        // `group_id`, would have the same fault.
        let body = new_tab("Sonarr", &json!({"group_id_max": 999}));
        let answer = call(&app, As::Admin, "POST", "/tabs", Some(body)).await;
        assert_eq!(answer.status, StatusCode::CONFLICT, "{}", answer.body);
    })
    .await;
}

#[tokio::test]
async fn new_default_tab_takes_the_mark_from_the_other_tabs() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        store_tab(database, "Plex", |tab| tab.is_default = true).await;

        let body = new_tab("Sonarr", &json!({"default": 1}));
        let answer = call(&app, As::Admin, "POST", "/tabs", Some(body)).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let defaults: Vec<String> = stored_tabs(database)
            .await
            .into_iter()
            .filter(|tab| tab.is_default)
            .map(|tab| tab.name)
            .collect();
        assert_eq!(defaults, ["Sonarr"]);
    })
    .await;
}

#[tokio::test]
async fn name_is_compared_as_text_not_as_a_pattern() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        store_tab(database, "Plex", |_| {}).await;

        // PHP compares with `LIKE`, where `_` and `%` stand for any characters, and so
        // takes these names for taken.
        for name in ["Pl_x", "P%", "%"] {
            let answer = call(
                &app,
                As::Admin,
                "POST",
                "/tabs",
                Some(new_tab(name, &json!({}))),
            )
            .await;
            assert_eq!(answer.status, StatusCode::OK, "{name}: {}", answer.body);
        }
        let answer = call(
            &app,
            As::Admin,
            "POST",
            "/tabs",
            Some(new_tab("pLEX", &json!({}))),
        )
        .await;
        assert_refused(
            &answer,
            StatusCode::CONFLICT,
            "Tab name: pLEX is already taken",
            "another letter case",
        );
    })
    .await;
}

#[tokio::test]
async fn values_can_be_written_in_the_ways_a_form_writes_them() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let plex = store_tab(database, "Plex", |tab| tab.timeout_ms = Some(60_000)).await;
        let path = format!("/tabs/{}", plex.id);

        let body = json!({
            "enabled": "false",
            "splash": "true",
            "ping": true,
            "preload": "1",
            "order": " 12 ",
            "type": "2",
            "group_id": "999",
            // An empty field of a form stands for "nothing".
            "timeout_ms": "",
        });
        let answer = call(&app, As::Admin, "PUT", &path, Some(body)).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let stored = database
            .tabs()
            .find(plex.id)
            .await
            .expect("the lookup works")
            .expect("the tab exists");
        assert_eq!(
            stored,
            Tab {
                enabled: false,
                splash: true,
                ping: true,
                preload: true,
                position: 12,
                kind: TabKind::NewWindow,
                access: LevelRange::up_to(GroupLevel::GUEST),
                timeout_ms: None,
                ..plex
            }
        );
    })
    .await;
}

#[tokio::test]
async fn one_malformed_entry_changes_no_place() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let before = stored_tabs(database).await;
        let first = before[0].id.get();
        let second = before[1].id.get();

        // PHP stores the entries before the malformed one.
        for malformed in [
            json!({"id": second}),
            json!({"id": second, "order": 0}),
            json!({"id": second, "order": 8, "name": "x"}),
            json!({"id": second, "order": "last"}),
            json!(5),
        ] {
            let body = json!([{"id": first, "order": 70}, malformed, {"id": second, "order": 71}]);
            let answer = call(&app, As::Admin, "PUT", "/tabs", Some(body.clone())).await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                "data is malformed",
                &body.to_string(),
            );
        }
        assert_eq!(stored_tabs(database).await, before);
    })
    .await;
}

#[tokio::test]
async fn order_request_that_php_crashes_on_gets_an_error_answer() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let before = stored_tabs(database).await;
        let first = before[0].id.get();

        // No body, and JSON that is neither a list nor an object.
        let without_body = call(&app, As::Admin, "PUT", "/tabs", None).await;
        let text = call(&app, As::Admin, "PUT", "/tabs", Some(json!("abc"))).await;
        for answer in [without_body, text] {
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                "data is empty or not in array",
                "no entries",
            );
        }
        // One entry that is not inside a list, and a list of numbers.
        for body in [json!({"id": first, "order": 13}), json!([first, 13])] {
            let answer = call(&app, As::Admin, "PUT", "/tabs", Some(body.clone())).await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                "data is malformed",
                &body.to_string(),
            );
        }
        assert_eq!(stored_tabs(database).await, before);
    })
    .await;
}

#[tokio::test]
async fn list_fills_in_the_placeholders_for_the_user_who_asks() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let address = "https://{domain}/{username}/{username_lower}/{email}/{group}/{group_id}";
        let plex = store_tab(database, "Plex", |tab| {
            tab.url = Some(address.to_owned());
            tab.url_local = Some(String::new());
        })
        .await;
        let co_admin = logged_in(database, &app, "Cora", 1).await;

        let list = call(&app, As::User(&co_admin), "GET", "/tabs", None).await;
        let one = call(
            &app,
            As::User(&co_admin),
            "GET",
            &format!("/tabs/{}", plex.id),
            None,
        )
        .await;

        let listed = list.data()["tabs"]
            .as_array()
            .expect("the tabs are a list")
            .iter()
            .find(|tab| tab["name"] == json!("Plex"))
            .cloned()
            .expect("the list has the tab");
        assert_eq!(
            listed["url"],
            json!("https://organizr.example.com/Cora/Cora/Cora@example.com/Co-Admin/1")
        );
        assert_eq!(listed["url_local"], Value::Null, "an empty address");
        // One tab alone is shown as it is stored, for the form that edits it.
        assert_eq!(one.data()["url"], json!(address));
        assert_eq!(one.data()["url_local"], json!(""));
    })
    .await;
}

#[tokio::test]
async fn routes_that_change_a_tab_send_the_plain_json_type() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let plex = store_tab(database, "Plex", |_| {}).await;
        let path = format!("/tabs/{}", plex.id);
        let content_type = |answer: &Answer| {
            answer
                .headers
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        };

        let read = call(&app, As::Admin, "GET", &path, None).await;
        let added = call(
            &app,
            As::Admin,
            "POST",
            "/tabs",
            Some(new_tab("Sonarr", &json!({}))),
        )
        .await;
        assert_eq!(
            content_type(&read).as_deref(),
            Some("application/json;charset=UTF-8")
        );
        assert_eq!(
            content_type(&added).as_deref(),
            Some("application/json;charset=UTF-8")
        );

        // Also for an answer that refuses, and for one without a body.
        let refused = call(&app, As::Guest, "PUT", &path, Some(json!({"ping": 1}))).await;
        let changed = call(&app, As::Admin, "PUT", &path, Some(json!({"ping": 1}))).await;
        let ordered = call(
            &app,
            As::Admin,
            "PUT",
            "/tabs",
            Some(json!([{"id": plex.id.get(), "order": 3}])),
        )
        .await;
        let deleted = call(&app, As::Admin, "DELETE", &path, None).await;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        assert_eq!(deleted.body, "");
        for answer in [refused, changed, ordered, deleted] {
            assert_eq!(
                content_type(&answer).as_deref(),
                Some("application/json"),
                "{answer:?}"
            );
        }
    })
    .await;
}
