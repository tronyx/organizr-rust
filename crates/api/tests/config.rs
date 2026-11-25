//! The `/config` routes through the real route table: what the recorded PHP answers
//! cannot show.
//!
//! The recording covers what both applications answer alike. These tests cover secrets,
//! callers of other levels, the requests that this application refuses (D-06, D-07,
//! D-53), and PHP's form routes, which it does not build (D-96).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::collections::BTreeMap;

use axum::Router;
use axum::http::StatusCode;
use organizr_api::stored_settings;
use organizr_auth::secrets::PREFIX;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_app, installed_app_with, seed};
use serde_json::{Value, json};

const SECURE: &str = "***Secure***";

/// The value of one setting as `GET /config` shows it to the Admin.
async fn setting(app: &Router, key: &str) -> Value {
    let answer = call(app, As::Admin, "GET", "/config", None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer.data()[key].clone()
}

/// Changes settings with the API key, and expects that the change is taken.
async fn change(app: &Router, body: Value) {
    let answer = call(app, As::Admin, "PUT", "/config", Some(body)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.message(), json!("Config items updated"));
}

fn assert_refused(answer: &Answer, status: StatusCode, message: &str) {
    assert_eq!(answer.status, status, "{}", answer.body);
    assert_eq!(answer.message(), json!(message));
}

/// The rows of the settings table: key to stored text.
async fn rows(database: &Database) -> BTreeMap<String, String> {
    database
        .settings()
        .all()
        .await
        .expect("the settings can be read")
        .into_iter()
        .map(|setting| (setting.key, setting.value))
        .collect()
}

#[tokio::test]
async fn configuration_is_for_admins_and_co_admins() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let super_user = seed::logged_in(database, &app, "sam", 2).await;

        let requests = [
            ("GET", "/config", None),
            ("GET", "/config/title", None),
            ("GET", "/config/search/title", None),
            ("PUT", "/config", Some(json!({"title": "Home"}))),
        ];
        for (method, path, body) in &requests {
            let answer = call(&app, As::User(&super_user), method, path, body.clone()).await;
            assert_refused(&answer, StatusCode::UNAUTHORIZED, "Not Authorized");
        }
        assert_eq!(setting(&app, "title").await, json!("Organizr V2"));

        for (method, path, body) in requests {
            let answer = call(&app, As::User(&co_admin), method, path, body).await;
            assert_eq!(
                answer.status,
                StatusCode::OK,
                "{method} {path}: {}",
                answer.body
            );
        }
        assert_eq!(setting(&app, "title").await, json!("Home"));
    })
    .await;
}

#[tokio::test]
async fn secret_is_never_shown() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[("plexToken", json!("a-plex-token"))]);

        let all = call(&app, As::Admin, "GET", "/config", None).await;
        let one = call(&app, As::Admin, "GET", "/config/plexToken", None).await;
        let found = call(&app, As::Admin, "GET", "/config/search/token", None).await;

        assert_eq!(all.data()["plexToken"], json!(SECURE));
        assert_eq!(one.data(), json!(SECURE));
        assert_eq!(found.data()["plexToken"], json!(SECURE));
        for answer in [&all, &one, &found] {
            assert!(!answer.body.contains("a-plex-token"), "{}", answer.body);
        }
        // A secret without a value is shown as empty, so a page can tell the two apart.
        assert_eq!(all.data()["embyToken"], json!(""));
        assert_eq!(found.data()["embyToken"], json!(""));
        let empty = call(&app, As::Admin, "GET", "/config/embyToken", None).await;
        assert_eq!(empty.status, StatusCode::OK);
        assert_eq!(empty.data(), Value::Null);
    })
    .await;
}

#[tokio::test]
async fn api_key_is_shown_to_the_admin_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;

        for path in ["/config", "/config/search/organizrAPI"] {
            let for_admin = call(&app, As::Admin, "GET", path, None).await;
            assert_eq!(for_admin.data()["organizrAPI"], json!(seed::API_KEY));
            let for_co_admin = call(&app, As::User(&co_admin), "GET", path, None).await;
            assert_eq!(for_co_admin.data()["organizrAPI"], json!(SECURE));
            assert!(!for_co_admin.body.contains(seed::API_KEY));
        }
        let for_admin = call(&app, As::Admin, "GET", "/config/organizrAPI", None).await;
        assert_eq!(for_admin.data(), json!(seed::API_KEY));
        let for_co_admin = call(
            &app,
            As::User(&co_admin),
            "GET",
            "/config/organizrAPI",
            None,
        )
        .await;
        assert_eq!(for_co_admin.data(), json!(SECURE));
    })
    .await;
}

#[tokio::test]
async fn some_settings_are_changed_by_the_admin_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;

        for (key, value) in [
            ("organizrAPI", json!("abcdefghij0123456789")),
            ("authProxyEnabled", json!(true)),
            ("authProxyHeaderName", json!("X-User")),
            ("customJava", json!("alert(1)")),
            ("customThemeJava", json!("alert(2)")),
            ("layout", json!("modern")),
            ("modernTheme", json!("organizr")),
            ("modernColorSet", json!("light")),
        ] {
            // The other setting of the request is not changed either.
            let body = json!({"title": "Home", key: value});
            let answer = call(&app, As::User(&co_admin), "PUT", "/config", Some(body)).await;
            assert_refused(
                &answer,
                StatusCode::UNAUTHORIZED,
                &format!("Config item: {key} can only be changed by an admin"),
            );
        }
        assert_eq!(setting(&app, "title").await, json!("Organizr V2"));
        assert_eq!(setting(&app, "customJava").await, json!(""));

        change(
            &app,
            json!({"customJava": "console.log(1)", "authProxyEnabled": "true"}),
        )
        .await;
        assert_eq!(setting(&app, "customJava").await, json!("console.log(1)"));
        assert_eq!(setting(&app, "authProxyEnabled").await, json!(true));
        assert_eq!(setting(&app, "layout").await, json!("og"));
        change(
            &app,
            json!({"layout": "modern", "modernTheme": "organizr", "modernColorSet": "light"}),
        )
        .await;
        assert_eq!(setting(&app, "layout").await, json!("modern"));
        assert_eq!(setting(&app, "modernColorSet").await, json!("light"));
        // A Co-Admin changes the other settings, the style sheets among them.
        let body = json!({"customCss": "body { margin: 0; }"});
        let answer = call(&app, As::User(&co_admin), "PUT", "/config", Some(body)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    })
    .await;
}

#[tokio::test]
async fn key_that_is_no_setting_and_value_that_does_not_fit_are_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        let unknown = json!({"title": "Home", "noSuchSetting": 1});
        let answer = call(&app, As::Admin, "PUT", "/config", Some(unknown)).await;
        assert_refused(
            &answer,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Config item: noSuchSetting is not a setting",
        );
        // Keys of the PHP configuration file are no settings.
        for key in ["organizrHash", "dbName", "driver", "configVersion"] {
            let answer = call(&app, As::Admin, "PUT", "/config", Some(json!({key: "x"}))).await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                &format!("Config item: {key} is not a setting"),
            );
        }

        let wrong = [
            ("rememberMeDays", json!("soon")),
            ("rememberMeDays", json!("-1")),
            ("rememberMeDays", json!("")),
            ("rememberMeDays", json!(true)),
            ("lockScreen", json!("yes")),
            ("lockScreen", json!(1)),
            ("lockScreen", Value::Null),
            ("title", json!(42)),
            ("title", json!(["Home"])),
            ("unsortedTabs", json!("left")),
            ("ignoredNewsIds", json!("4")),
            ("maxSocksDebugSize", json!("100")),
            ("plexToken", json!(["a-token"])),
        ];
        for (key, value) in wrong {
            let body = json!({"description": "Changed", key: value.clone()});
            let answer = call(&app, As::Admin, "PUT", "/config", Some(body)).await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                &format!("Config item: {key} is not valid"),
            );
            assert!(!answer.body.contains("a-token"), "{}", answer.body);
        }

        // Nothing of a refused request is stored.
        assert_eq!(setting(&app, "title").await, json!("Organizr V2"));
        assert_eq!(
            setting(&app, "description").await,
            json!("Organizr - Accept no others")
        );
        assert_eq!(rows(database).await, BTreeMap::new());
    })
    .await;
}

#[tokio::test]
async fn request_without_settings_changes_nothing() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        // PHP answers the first of these with an error page.
        for body in [
            None,
            Some(json!({})),
            Some(json!([1, 2])),
            Some(json!("title")),
        ] {
            let answer = call(&app, As::Admin, "PUT", "/config", body).await;
            assert_refused(&answer, StatusCode::CONFLICT, "No data submitted");
        }
        // The form key alone is taken, and it is no setting.
        change(&app, json!({"formKey": "anything"})).await;
        assert_eq!(rows(database).await, BTreeMap::new());
    })
    .await;
}

#[tokio::test]
async fn values_are_stored_in_the_form_of_the_php_application() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        change(
            &app,
            json!({
                "rememberMeDays": 30,
                "loginAttempts": "9",
                "lockScreen": "true",
                "hideRegistration": true,
                "ignoredNewsIds": [4, 7],
                "maxSocksDebugSize": 250,
                "unsortedTabs": "bottom",
                "customCss": null,
            }),
        )
        .await;

        // A number that PHP keeps as text comes back as text.
        assert_eq!(setting(&app, "rememberMeDays").await, json!("30"));
        assert_eq!(setting(&app, "loginAttempts").await, json!("9"));
        assert_eq!(setting(&app, "lockScreen").await, json!(true));
        assert_eq!(setting(&app, "hideRegistration").await, json!(true));
        assert_eq!(setting(&app, "ignoredNewsIds").await, json!([4, 7]));
        assert_eq!(setting(&app, "maxSocksDebugSize").await, json!(250));
        assert_eq!(setting(&app, "unsortedTabs").await, json!("bottom"));
        assert_eq!(setting(&app, "customCss").await, json!(""));
    })
    .await;
}

#[tokio::test]
async fn change_is_used_at_once_and_is_still_there_after_a_restart() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        change(&app, json!({"title": "Home", "unsortedTabs": "bottom"})).await;

        let launch = call(&app, As::Guest, "GET", "/launch", None).await;
        assert_eq!(launch.data()["appearance"]["title"], json!("Home"));
        assert_eq!(
            launch.data()["settings"]["misc"]["unsortedTabs"],
            json!("bottom")
        );
        // What another start of the server reads from the database.
        let (stored, problems) = stored_settings(database, &seed::secrets())
            .await
            .expect("the settings can be read");
        assert_eq!(problems, []);
        assert_eq!(stored.title, "Home");
        assert_eq!(
            rows(database).await,
            BTreeMap::from([
                ("title".to_owned(), "\"Home\"".to_owned()),
                ("unsortedTabs".to_owned(), "\"bottom\"".to_owned()),
            ])
        );

        // A setting that gets its default has no row any more.
        change(&app, json!({"title": "Organizr V2"})).await;
        assert_eq!(
            rows(database).await.keys().collect::<Vec<_>>(),
            ["unsortedTabs"]
        );
        assert_eq!(setting(&app, "title").await, json!("Organizr V2"));
    })
    .await;
}

#[tokio::test]
async fn secret_is_stored_sealed_and_kept_when_the_form_sends_it_back() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        change(&app, json!({"plexToken": "a-plex-token", "title": "Home"})).await;

        let stored = rows(database).await;
        let sealed: String =
            serde_json::from_str(&stored["plexToken"]).expect("the stored value is JSON text");
        assert!(sealed.starts_with(PREFIX), "{sealed}");
        assert!(!stored["plexToken"].contains("a-plex-token"));
        assert_eq!(
            seed::secrets()
                .open("plexToken", &sealed)
                .expect("it opens"),
            "a-plex-token"
        );
        assert_eq!(setting(&app, "plexToken").await, json!(SECURE));

        // A form sends back what it was shown. That is no new value.
        change(&app, json!({"plexToken": SECURE, "title": "Home again"})).await;
        assert_eq!(rows(database).await["plexToken"], stored["plexToken"]);
        assert_eq!(setting(&app, "title").await, json!("Home again"));
        let (settings, _) = stored_settings(database, &seed::secrets())
            .await
            .expect("the settings can be read");
        assert_eq!(settings.plex_token, "a-plex-token");

        // For a setting that is no secret, the same text is a value like every other.
        change(&app, json!({"description": SECURE})).await;
        assert_eq!(setting(&app, "description").await, json!(SECURE));

        // An empty value takes the secret away.
        change(&app, json!({"plexToken": ""})).await;
        assert_eq!(setting(&app, "plexToken").await, json!(""));
        assert!(!rows(database).await.contains_key("plexToken"));
    })
    .await;
}

#[tokio::test]
async fn new_api_key_works_at_once_and_the_old_one_does_not() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let new_key = "abcdefghij0123456789";

        for wrong in ["short", "abcdefghij01234567890", ""] {
            let answer = call(
                &app,
                As::Admin,
                "PUT",
                "/config",
                Some(json!({"organizrAPI": wrong})),
            )
            .await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                "Config item: organizrAPI must have 20 characters",
            );
        }
        change(&app, json!({"organizrAPI": new_key})).await;

        let with_old = call(&app, As::Admin, "GET", "/config/title", None).await;
        assert_refused(&with_old, StatusCode::UNAUTHORIZED, "Not Authorized");
        let headers = [("token", new_key)];
        let request =
            organizr_testkit::http::request("GET", "/api/v2/config/organizrAPI", &headers, "");
        let with_new = organizr_testkit::http::send(&app, request).await;
        assert_eq!(with_new.status, StatusCode::OK, "{}", with_new.body);
        assert_eq!(with_new.data(), json!(new_key));
        assert!(!rows(database).await["organizrAPI"].contains(new_key));
    })
    .await;
}

#[tokio::test]
async fn id_of_the_install_cannot_be_changed() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        let body = json!({"uuid": "00000000-0000-4000-8000-000000000000"});
        let answer = call(&app, As::Admin, "PUT", "/config", Some(body)).await;

        assert_refused(
            &answer,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Config item: uuid cannot be changed",
        );
        assert_eq!(setting(&app, "uuid").await, json!(seed::INSTALL_ID));
    })
    .await;
}

#[tokio::test]
async fn registration_password_is_stored_as_a_hash_and_works() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let register = async |name: &str, password: &str| {
            let body = json!({
                "username": name,
                "password": "the password of the new user",
                "email": format!("{name}@example.com"),
                "registrationPassword": password,
            });
            call(&app, As::Guest, "POST", "/users/register", Some(body)).await
        };

        let too_long = json!({"registrationPassword": "x".repeat(201)});
        let answer = call(&app, As::Admin, "PUT", "/config", Some(too_long)).await;
        assert_refused(
            &answer,
            StatusCode::CONFLICT,
            "String is over limit of: 200",
        );

        change(&app, json!({"registrationPassword": "let them in"})).await;
        assert_eq!(setting(&app, "registrationPassword").await, json!(SECURE));
        let stored = &rows(database).await["registrationPassword"];
        assert!(
            !stored.contains("let them in") && !stored.contains("$argon2"),
            "{stored}"
        );
        assert_eq!(register("abe", "let them in").await.status, StatusCode::OK);
        assert_eq!(
            register("bea", "let me in").await.status,
            StatusCode::UNAUTHORIZED
        );

        // Without a registration password, nobody registers.
        change(&app, json!({"registrationPassword": ""})).await;
        assert_eq!(setting(&app, "registrationPassword").await, json!(""));
        assert!(!rows(database).await.contains_key("registrationPassword"));
        assert_eq!(
            register("cy", "let them in").await.status,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            register("cy", "").await.status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    })
    .await;
}

#[tokio::test]
async fn search_without_a_term_finds_every_setting() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        let everything = call(&app, As::Admin, "GET", "/config", None).await;
        let found = call(&app, As::Admin, "GET", "/config/search", None).await;

        assert_eq!(found.status, StatusCode::OK, "{}", found.body);
        assert_eq!(found.message(), json!("Search results for term: "));
        assert_eq!(found.data(), everything.data());
        assert!(
            found
                .data()
                .as_object()
                .is_some_and(|settings| settings.len() > 850)
        );
        // The keys are sorted, capital letters first.
        let data = everything.data();
        let keys: Vec<&String> = data.as_object().expect("an object").keys().collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted);
        assert_eq!(
            keys.first().map(|key| key.as_str()),
            Some("BOOKMARK-enabled")
        );
    })
    .await;
}

#[tokio::test]
async fn two_changes_at_the_same_time_are_both_kept() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        let (first, second) = tokio::join!(
            call(
                &app,
                As::Admin,
                "PUT",
                "/config",
                Some(json!({"title": "Home"}))
            ),
            call(
                &app,
                As::Admin,
                "PUT",
                "/config",
                Some(json!({"description": "My server"}))
            ),
        );

        assert_eq!(first.status, StatusCode::OK, "{}", first.body);
        assert_eq!(second.status, StatusCode::OK, "{}", second.body);
        assert_eq!(setting(&app, "title").await, json!("Home"));
        assert_eq!(setting(&app, "description").await, json!("My server"));
        assert_eq!(rows(database).await.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn the_form_routes_of_php_are_unknown() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The settings pages draw their forms from the definitions of the web application,
        // so PHP's form routes are not built (D-96).
        let app = installed_app(database);
        for path in [
            "/settings/appearance",
            "/settings/plugin",
            "/settings/theme",
            "/settings/main",
            "/settings/sso",
            "/settings/homepage",
            "/settings/homepage/plex",
            "/settings/homepage/plex/debug",
            "/plugins/bookmark/settings",
            "/plugins/chat/settings",
            "/plugins/healthchecks/settings",
            "/plugins/invites/settings",
            "/plugins/php-mailer/settings",
            "/plugins/speedtest/settings",
            "/plugins/plexlibraries/settings",
            // The HTML of the bookmark settings of PHP is not built either (D-68 (9)).
            "/plugins/bookmark/settings_tab_editor_bookmark_tabs",
            "/plugins/bookmark/settings_tab_editor_bookmark_categories",
        ] {
            let answer = call(&app, As::Admin, "GET", path, None).await;
            assert_eq!(
                answer.status,
                StatusCode::NOT_FOUND,
                "{path}: {}",
                answer.body
            );
            assert_eq!(
                answer.message(),
                json!("Endpoint Not Found or Defined"),
                "{path}"
            );
        }
    })
    .await;
}
