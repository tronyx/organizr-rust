//! The themes, the theme marketplace, and the style sheet, the scripts and the icons of
//! the Admin.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test reads the fields of answers it knows, and its helpers may panic"
)]

use axum::Router;
use axum::http::{StatusCode, header};
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app_with, installed_settings, seed, state_with};
use serde_json::{Value, json};

/// The application with the switch `ORGANIZR_CUSTOM_JS` on or off.
fn app_with_custom_js(database: &Database, on: bool, settings: &[(&str, Value)]) -> Router {
    let state = state_with(database, installed_settings(settings)).with_custom_js(on);
    organizr_api::router(state).expect("the route table is valid")
}

/// Two scripts of the Admin.
const SCRIPTS: [(&str, &str); 2] = [
    ("customThemeJava", "window.themeScript = 1;"),
    ("customJava", "window.customScript = 2;"),
];

fn scripts() -> Vec<(&'static str, Value)> {
    SCRIPTS
        .iter()
        .map(|(key, script)| (*key, json!(script)))
        .collect()
}

/// The routes of the themes, which need an Admin or a Co-Admin.
const ROUTES: [(&str, &str); 4] = [
    ("GET", "/themes"),
    ("GET", "/themes/marketplace"),
    ("POST", "/themes/manage/nord"),
    ("DELETE", "/themes/manage/nord"),
];

#[tokio::test]
async fn only_admins_and_co_admins_use_the_theme_routes() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        for (method, path) in ROUTES {
            for who in [As::Guest, As::User(&user)] {
                let answer = call(&app, who, method, path, None).await;
                assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{method} {path}");
                assert_eq!(answer.message(), json!("Not Authorized"));
            }
        }
        let answer = call(&app, As::User(&co_admin), "GET", "/themes", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let answer = call(
            &app,
            As::User(&co_admin),
            "POST",
            "/themes/manage/nord",
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    })
    .await;
}

#[tokio::test]
async fn new_install_has_the_two_themes_of_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        let answer = call(&app, As::Admin, "GET", "/themes", None).await;

        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(
            answer.headers[header::CONTENT_TYPE],
            "application/json;charset=UTF-8"
        );
        assert_eq!(
            answer.data(),
            json!({
                "Blue": { "name": "Blue", "repo": null, "version": "1.0.0", "path": "css/themes" },
                "Organizr": {
                    "name": "Organizr", "repo": null, "version": "1.0.0", "path": "css/themes"
                }
            })
        );
    })
    .await;
}

#[tokio::test]
async fn marketplace_lists_the_eleven_themes_with_the_fields_of_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        let answer = call(&app, As::Admin, "GET", "/themes/marketplace", None).await;

        assert_eq!(answer.status, StatusCode::OK);
        let data = answer.data();
        let themes = data.as_object().expect("the list is an object");
        let names: Vec<&str> = themes.keys().map(String::as_str).collect();
        assert_eq!(
            names,
            [
                "Nord",
                "Dracula",
                "Overseerr",
                "Onedark",
                "Aquamarine",
                "Space_Gray",
                "Plex_theme_park",
                "Dark_theme_park",
                "Hotline",
                "Hotpink",
                "Plex"
            ]
        );
        let nord = &data["Nord"];
        for field in [
            "repo",
            "author",
            "category",
            "description",
            "icon",
            "contact",
            "website",
            "version",
            "minimum_organizr_version",
            "release_date",
            "last_updated",
            "github_folder",
            "project_folder",
            "license",
            "images",
            "css",
            "javascript",
        ] {
            assert!(nord.get(field).is_some(), "{field}");
        }
        assert_eq!(nord["project_folder"], json!("Nord_theme"));
        assert_eq!(nord["installed"], json!(false));
        assert_eq!(nord["installed_version"], Value::Null);
        assert_eq!(nord["needs_update"], json!(false));
        assert_eq!(nord["status"], json!("Not Installed"));
    })
    .await;
}

#[tokio::test]
async fn install_and_removal_change_the_themes_of_the_install() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);

        // The name is found in any letter case, with a space or a dash for `_`.
        let answer = call(&app, As::Admin, "POST", "/themes/manage/space-gray", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("Theme installed"));
        // The answer shows the theme as it was before the install, as PHP does.
        assert_eq!(answer.data()["status"], json!("Not Installed"));
        assert_eq!(answer.data()["version"], json!("2.0.2"));

        let themes = call(&app, As::Admin, "GET", "/themes", None).await.data();
        assert_eq!(
            themes["Space_Gray"],
            json!({
                "name": "Space_Gray",
                "version": "2.0.2",
                "repo": "https://github.com/Organizr/Organizr-Themes",
                "path": "data/themes/Space_Gray_theme"
            })
        );
        let market = call(&app, As::Admin, "GET", "/themes/marketplace", None)
            .await
            .data();
        assert_eq!(market["Space_Gray"]["installed"], json!(true));
        assert_eq!(market["Space_Gray"]["installed_version"], json!("2.0.2"));
        assert_eq!(market["Space_Gray"]["status"], json!("Up to date"));
        // The setting holds what PHP stores, and an Admin sees it in the launch answer.
        let launch = call(&app, As::Admin, "GET", "/launch", None).await.data();
        assert_eq!(
            launch["settings"]["misc"]["installedThemes"]["Space_Gray"]["path"],
            json!("data/themes/Space_Gray_theme")
        );

        let answer = call(&app, As::Admin, "DELETE", "/themes/manage/SPACE_GRAY", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("Theme removed"));
        let themes = call(&app, As::Admin, "GET", "/themes", None).await.data();
        assert!(themes.get("Space_Gray").is_none(), "{themes}");

        let answer = call(&app, As::Admin, "DELETE", "/themes/manage/space_gray", None).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND);
        assert_eq!(answer.message(), json!("Theme is not installed"));
    })
    .await;
}

#[tokio::test]
async fn unknown_theme_gets_the_texts_of_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);

        let answer = call(&app, As::Admin, "POST", "/themes/manage/Not-There", None).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND);
        assert_eq!(
            answer.message(),
            json!("Theme [not_there] does not exist in marketplace")
        );
        // PHP sends the marketplace with the names in lower case.
        assert_eq!(
            answer.data()["space_gray"]["project_folder"],
            json!("Space_Gray_theme")
        );

        let answer = call(&app, As::Admin, "DELETE", "/themes/manage/not-there", None).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND);
        assert_eq!(
            answer.message(),
            json!("Theme does not exist in marketplace")
        );
        assert_eq!(answer.data(), Value::Null);
    })
    .await;
}

#[tokio::test]
async fn launch_names_the_theme_that_the_page_shows() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let theme_of = async |settings: &[(&str, Value)]| {
            let app = installed_app_with(database, settings);
            call(&app, As::Guest, "GET", "/launch", None).await.data()["theme"].clone()
        };

        assert_eq!(theme_of(&[]).await, json!("Organizr"));
        assert_eq!(theme_of(&[("theme", json!("Blue"))]).await, json!("Blue"));
        // A marketplace theme that is not installed falls back to the default, as in PHP.
        assert_eq!(
            theme_of(&[("theme", json!("Nord"))]).await,
            json!("Organizr")
        );
        let installed = json!({ "Nord": { "name": "Nord", "version": "1.0.2" } });
        assert_eq!(
            theme_of(&[("theme", json!("Nord")), ("installedThemes", installed)]).await,
            json!("Nord")
        );
    })
    .await;
}

#[tokio::test]
async fn custom_css_is_a_style_sheet_for_every_caller() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("customCss", json!("body { color: red; }")),
                ("customThemeCss", json!(".org-header { color: blue; }")),
            ],
        );
        let answer = call(&app, As::Guest, "GET", "/appearance/custom.css", None).await;

        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(
            answer.headers[header::CONTENT_TYPE],
            "text/css; charset=utf-8"
        );
        assert_eq!(answer.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(
            answer.headers[header::CONTENT_SECURITY_POLICY],
            "default-src 'none'; frame-ancestors 'none'"
        );
        // No cache keeps it: a change applies on the next load.
        assert_eq!(
            answer.headers[header::CACHE_CONTROL],
            "no-store, no-cache, must-revalidate"
        );
        // The theme CSS comes first, as in PHP, so the custom CSS wins.
        assert_eq!(
            answer.body,
            ".org-header { color: blue; }\nbody { color: red; }\n"
        );

        let empty = installed_app_with(database, &[]);
        let answer = call(&empty, As::Guest, "GET", "/appearance/custom.css", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.body, "");
    })
    .await;
}

#[tokio::test]
async fn scripts_of_the_admin_are_not_found_while_the_switch_is_off() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with_custom_js(database, false, &scripts());

        let answer = call(&app, As::Admin, "GET", "/appearance/custom.js", None).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND);
        let launch = call(&app, As::Guest, "GET", "/launch", None).await.data();
        assert_eq!(launch["appearance"]["customJsEnabled"], json!(false));
        // PHP gives the texts to every caller, and so does this application.
        assert_eq!(
            launch["appearance"]["customJava"],
            json!("window.customScript = 2;")
        );
    })
    .await;
}

#[tokio::test]
async fn scripts_of_the_admin_are_one_file_for_every_caller_while_the_switch_is_on() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with_custom_js(database, true, &scripts());

        // A guest gets it too: PHP runs the scripts on the login page.
        let answer = call(&app, As::Guest, "GET", "/appearance/custom.js", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(
            answer.headers[header::CONTENT_TYPE],
            "text/javascript; charset=utf-8"
        );
        assert_eq!(answer.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(
            answer.headers[header::CACHE_CONTROL],
            "no-store, no-cache, must-revalidate"
        );
        // The theme script comes first, as in PHP.
        assert_eq!(
            answer.body,
            "window.themeScript = 1;
window.customScript = 2;
"
        );
        let launch = call(&app, As::Guest, "GET", "/launch", None).await.data();
        assert_eq!(launch["appearance"]["customJsEnabled"], json!(true));

        // Without a script, there is no file.
        let empty = app_with_custom_js(database, true, &[("customJava", json!("  "))]);
        let answer = call(&empty, As::Admin, "GET", "/appearance/custom.js", None).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn launch_has_only_the_icon_links_of_the_fav_icon_setting() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let html = concat!(
            r#"<link rel="icon" type="image/png" sizes="32x32" href="data/favicon/favicon-32x32.png">"#,
            r#"<link rel="manifest" href="data/favicon/site.webmanifest">"#,
            r#"<script>alert(1)</script>"#,
            r#"<link rel="shortcut icon" href="javascript:alert(1)">"#,
            r#"<link rel="apple-touch-icon" href="https://cdn.example.com/touch.png">"#,
        );
        let app = installed_app_with(database, &[("favIcon", json!(html))]);
        let launch = call(&app, As::Guest, "GET", "/launch", None).await.data();
        assert_eq!(
            launch["appearance"]["favIcons"],
            json!([
                {
                    "rel": "icon",
                    "href": "data/favicon/favicon-32x32.png",
                    "type": "image/png",
                    "sizes": "32x32"
                },
                { "rel": "apple-touch-icon", "href": "https://cdn.example.com/touch.png" }
            ])
        );

        let plain = installed_app_with(database, &[]);
        let launch = call(&plain, As::Guest, "GET", "/launch", None).await.data();
        assert_eq!(launch["appearance"]["favIcons"], json!([]));
    })
    .await;
}
