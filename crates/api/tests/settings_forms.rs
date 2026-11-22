//! The forms of the settings pages: the `/settings/*` routes and the `settings` routes of
//! the plugins (D-22, D-96).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test reads the fields of answers it knows, and its helpers may panic"
)]

use axum::http::StatusCode;
use organizr_settings::Settings;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

/// Every route of the forms, except those of one homepage item.
const ROUTES: [&str; 11] = [
    "/settings/appearance",
    "/settings/plugin",
    "/settings/theme",
    "/settings/main",
    "/settings/sso",
    "/settings/homepage",
    "/plugins/bookmark/settings",
    "/plugins/healthchecks/settings",
    "/plugins/invites/settings",
    "/plugins/php-mailer/settings",
    "/plugins/speedtest/settings",
];

/// What every planted secret holds, so a test finds it in any answer.
const PLANTED: &str = "planted-secret-7f3a";

/// Every secret setting with a planted value, except the API key, which the tests
/// send. An address of Sonarr holds a password, which the debug route must hide.
fn planted_secrets() -> Vec<(&'static str, Value)> {
    let mut settings: Vec<(&'static str, Value)> = Settings::KEYS
        .iter()
        .copied()
        .filter(|key| Settings::is_secret(key) && *key != "organizrAPI")
        .map(|key| (key, json!(format!("{PLANTED}-{key}"))))
        .collect();
    settings.push((
        "sonarrURL",
        json!(format!("http://me:{PLANTED}@sonarr:8989")),
    ));
    settings.push(("homepageSonarrEnabled", json!(true)));
    settings
}

/// The names of the homepage items, from `GET /settings/homepage`.
async fn item_names(app: &axum::Router) -> Vec<String> {
    let answer = call(app, As::Admin, "GET", "/settings/homepage", None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer
        .data()
        .as_array()
        .expect("a list")
        .iter()
        .map(|item| item["name"].as_str().expect("a name").to_owned())
        .collect()
}

#[tokio::test]
async fn only_admins_and_co_admins_get_the_forms() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let mut paths: Vec<String> = ROUTES.iter().map(|path| (*path).to_owned()).collect();
        paths.push("/settings/homepage/sonarr".to_owned());
        paths.push("/settings/homepage/sonarr/debug".to_owned());
        for path in &paths {
            for who in [As::Guest, As::User(&user)] {
                let answer = call(&app, who, "GET", path, None).await;
                assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{path}");
                assert_eq!(answer.message(), json!("Not Authorized"));
            }
            let answer = call(&app, As::User(&co_admin), "GET", path, None).await;
            assert_eq!(answer.status, StatusCode::OK, "{path}: {}", answer.body);
        }
        for path in [
            "/settings/homepage/no-such-item",
            "/settings/homepage/no-such-item/debug",
        ] {
            let answer = call(&app, As::Admin, "GET", path, None).await;
            assert_eq!(answer.status, StatusCode::NOT_FOUND, "{path}");
            assert_eq!(answer.message(), json!("Homepage item was not found"));
        }
    })
    .await;
}

#[tokio::test]
async fn no_route_sends_a_secret() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &planted_secrets());
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let mut paths: Vec<String> = ROUTES.iter().map(|path| (*path).to_owned()).collect();
        for name in item_names(&app).await {
            paths.push(format!("/settings/homepage/{name}"));
            paths.push(format!("/settings/homepage/{name}/debug"));
        }
        assert!(paths.len() > 80, "{}", paths.len());
        for path in &paths {
            for who in [As::Admin, As::User(&co_admin)] {
                let answer = call(&app, who, "GET", path, None).await;
                assert_eq!(answer.status, StatusCode::OK, "{path}");
                let planted_in_the_clear = answer
                    .body
                    .match_indices(PLANTED)
                    .any(|(at, _)| !answer.body[..at].ends_with("http://me:"));
                assert!(
                    !planted_in_the_clear,
                    "{path} sends a secret: {}",
                    answer.body
                );
                if path.ends_with("/debug") {
                    assert!(!answer.body.contains(PLANTED), "{path}: {}", answer.body);
                    assert!(!answer.body.contains("***Secure***"), "{path}");
                }
            }
        }
        // The forms mask a secret as `GET /config` does.
        let answer = call(&app, As::Admin, "GET", "/settings/main", None).await;
        let ldap = answer.data()["Authentication"]
            .as_array()
            .expect("a list")
            .iter()
            .find(|field| field["name"] == "ldapBindPassword")
            .cloned()
            .expect("the field is there");
        assert_eq!(ldap["value"], json!("***Secure***"));
    })
    .await;
}

#[tokio::test]
async fn the_debug_route_hides_secrets_and_leaves_out_what_php_leaves_out() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &planted_secrets());
        let answer = call(
            &app,
            As::Admin,
            "GET",
            "/settings/homepage/Sonarr/debug",
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        let groups: Vec<&String> = data.as_object().expect("an object").keys().collect();
        assert_eq!(
            groups,
            ["Enable", "Connection", "API SOCKS", "Queue", "Calendar"]
        );
        assert_eq!(data["Enable"]["homepageSonarrEnabled"], json!(true));
        assert_eq!(data["Connection"]["sonarrURL"], json!("***redacted***"));
        assert_eq!(data["Connection"]["sonarrToken"], json!("***redacted***"));
        assert_eq!(data["Calendar"]["calendarStart"], json!("14"));
    })
    .await;
}

#[tokio::test]
async fn a_form_has_the_structure_of_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[("socksDebug", json!(true))]);
        let answer = call(&app, As::Admin, "GET", "/settings/main", None).await;
        let data = answer.data();
        let groups: Vec<&String> = data.as_object().expect("an object").keys().collect();
        assert_eq!(
            groups,
            [
                "Socks",
                "Settings Page",
                "Other",
                "Github",
                "API",
                "Authentication",
                "Security",
                "Logs",
                "Cron",
                "Login",
                "Auth Proxy",
                "Ping"
            ]
        );
        // What PHP's settingsOption('switch', 'socksDebug', [...]) gives, in its order.
        assert_eq!(
            serde_json::to_string(&data["Socks"][0]).expect("JSON"),
            serde_json::to_string(&json!({
                "type": "switch",
                "label": "Enable Debug Log",
                "name": "socksDebug",
                "value": true,
                "help": "Enable the option to have socks output to logs",
            }))
            .expect("JSON")
        );
        let lockout = data["Security"]
            .as_array()
            .expect("a list")
            .iter()
            .find(|field| field["name"] == "lockoutMinAuth")
            .cloned()
            .expect("the field is there");
        assert!(
            lockout["options"]
                .as_array()
                .expect("a list")
                .contains(&json!({"name": "Admin", "value": 0}))
        );

        // The list of the homepage items, and the form of one of them.
        let answer = call(&app, As::Admin, "GET", "/settings/homepage", None).await;
        let items = answer.data();
        let first = items[0].as_object().expect("an object");
        let fields: Vec<&String> = first.keys().collect();
        assert_eq!(
            fields,
            ["name", "enabled", "image", "category", "settingsArray"]
        );
        assert_eq!(items[0]["name"], json!("Bookmarks"));
        let answer = call(&app, As::Admin, "GET", "/settings/homepage/sonarr", None).await;
        let item = answer.data();
        let fields: Vec<&String> = item.as_object().expect("an object").keys().collect();
        assert_eq!(
            fields,
            [
                "name",
                "enabled",
                "image",
                "category",
                "settingsArray",
                "docs",
                "debug",
                "settings"
            ]
        );
        assert_eq!(item["enabled"], json!(true));
        assert_eq!(item["settings"]["About"][0]["type"], json!("about"));
        assert!(!answer.body.contains("\"html\""));
        assert!(!answer.body.contains("\"attr\""));

        // A form that PHP fills only with HTML is empty, and PHP writes it as `[]`.
        let answer = call(&app, As::Admin, "GET", "/plugins/bookmark/settings", None).await;
        assert_eq!(answer.data(), json!([]));
    })
    .await;
}

#[tokio::test]
async fn the_shuck_stop_plugin_has_no_form() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        let answer = call(&app, As::Admin, "GET", "/plugins/shuck-stop/settings", None).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND);
        let names = item_names(&app).await;
        assert!(
            !names
                .iter()
                .any(|name| name.to_lowercase().contains("shuck"))
        );
    })
    .await;
}

#[tokio::test]
async fn the_form_of_prowlarr_has_its_connection_as_in_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[("prowlarrURL", json!("http://prowlarr:9696"))]);
        let answer = call(&app, As::Admin, "GET", "/settings/homepage/prowlarr", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let connection = &answer.data()["settings"]["Connection"];
        let names: Vec<&str> = connection
            .as_array()
            .expect("the group Connection")
            .iter()
            .map(|field| field["name"].as_str().expect("a name"))
            .collect();
        assert_eq!(
            names,
            [
                "prowlarrURL",
                "prowlarrToken",
                "prowlarrDisableCertCheck",
                "prowlarrUseCustomCertificate"
            ]
        );
        assert_eq!(connection[0]["value"], json!("http://prowlarr:9696"));
    })
    .await;
}

#[tokio::test]
async fn the_form_of_netdata_has_the_charts_of_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("netdata3Title", json!("CPU of the server")),
                ("netdataCustom", json!("{\"1\": {}}")),
            ],
        );
        let answer = call(&app, As::Admin, "GET", "/settings/homepage/netdata", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let settings = &answer.data()["settings"];
        let groups: Vec<&String> = settings.as_object().expect("groups").keys().collect();
        assert_eq!(
            groups,
            [
                "Enable",
                "Connection",
                "Chart 1",
                "Chart 2",
                "Chart 3",
                "Chart 4",
                "Chart 5",
                "Chart 6",
                "Chart 7",
                "Custom data",
                "Options"
            ]
        );
        let names = |group: &str| -> Vec<Value> {
            settings[group]
                .as_array()
                .expect("a group")
                .iter()
                .map(|field| field["name"].clone())
                .collect()
        };
        assert_eq!(
            names("Chart 3"),
            [
                json!("netdata3Enabled"),
                Value::Null,
                json!("netdata3Title"),
                json!("netdata3Data"),
                json!("netdata3Chart"),
                json!("netdata3Colour"),
                json!("netdata3Size"),
                Value::Null,
                json!("netdata3lg"),
                json!("netdata3md"),
                json!("netdata3sm")
            ]
        );
        assert_eq!(settings["Chart 3"][2]["value"], json!("CPU of the server"));
        let data = &settings["Chart 3"][3]["options"];
        assert_eq!(data[2], json!({"name": "CPU", "value": "cpu"}));
        assert_eq!(names("Options"), [json!("homepageNetdataRefresh")]);
        let custom = &settings["Custom data"][1];
        assert_eq!(custom["name"], json!("netdataCustom"));
        assert_eq!(custom["value"], json!("{\"1\": {}}"));
    })
    .await;
}
