//! The Rust application must answer the recorded requests as the PHP application did.
//!
//! Each test replays one group of `parity/cases` on every database engine. See
//! `organizr_testkit::replay` for what a replay compares and how a difference is allowed.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_db::{Database, Engine};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::replay::Replay;
use organizr_testkit::{NO_LOGIN_LIMIT, app, installed_app, installed_app_with, seed};
use serde_json::{Value, json};

/// The differences that every status answer has.
fn status_differences(replay: Replay, case: &str, database: &Database) -> Replay {
    // There is no PHP, so the PHP version has a fixed value.
    let replay = replay.deviation("D-21", case, "/response/data/php", json!("n/a"));
    // The answer names the database engine, which the set-up wizard cannot choose.
    let engine = match database.engine() {
        Engine::Sqlite => "sqlite",
        Engine::Postgres => "postgresql",
    };
    let replay = replay.deviation_adds("D-46", case, "/response/data", "database", json!(engine));
    if cfg!(windows) {
        replay.environment(
            "the recording was made on Linux and this test runs on Windows",
            case,
            "/response/data/os",
            json!("win"),
        )
    } else {
        replay
    }
}

/// The differences that every launch answer has.
fn launch_differences(replay: Replay, case: &str) -> Replay {
    let version = json!(env!("CARGO_PKG_VERSION"));
    let mut replay = replay
        // There is no PHP, so the PHP version and the PHP user have fixed values.
        .deviation("D-21", case, "/response/data/status/php", json!("n/a"))
        .deviation("D-21", case, "/response/data/status/php_user", json!("n/a"))
        .environment(
            "the recording was made with Organizr 2.1.5000; this is another version",
            case,
            "/response/data/version",
            version.clone(),
        )
        .environment(
            "the recording was made with Organizr 2.1.5000; this is another version",
            case,
            "/response/data/status/version",
            version,
        );
    if cfg!(windows) {
        replay = replay.environment(
            "the recording was made on Linux and this test runs on Windows",
            case,
            "/response/data/status/os",
            json!("win"),
        );
    }
    // The recording of a group that changed no setting has these lists in the order of
    // the default settings.
    if UNSORTED_LAUNCHES.contains(&case) {
        for list in ["settings/homepage/refresh", "plugins", "plugins/includes"] {
            replay = replay.deviation_sorts_keys("D-50", case, &format!("/response/data/{list}"));
        }
    }
    // There is no configuration file whose path could be sent.
    replay.deviation_without("D-29", case, "/response/data/status/userConfigPath")
}

/// The launch cases that were recorded before PHP wrote its configuration file anew.
const UNSORTED_LAUNCHES: [&str; 5] = [
    "launch_before_the_set_up",
    "launch_as_guest",
    "launch_as_guest_with_a_trailing_slash",
    "launch_as_admin",
    "launch_with_cookies_of_other_services",
];

/// Where the launch answer holds the tables of the PHP application, for an admin.
const SCHEMA: &str = "/response/data/settings/misc/schema";

/// The switches of a tab that PHP sends as `null` when they were never set. Here a
/// switch is 0 or 1.
const TAB_SWITCHES: [&str; 4] = ["splash", "ping", "timeout", "preload"];

/// The cookie difference that every answer has: there is no PHP session.
fn without_php_session(replay: Replay) -> Replay {
    replay.without_cookie("D-27", "PHPSESSID")
}

#[tokio::test]
async fn fresh_install_answers_like_php() {
    on_each_engine(async |database| {
        let mut replay = without_php_session(Replay::group("fresh_install"));
        for case in [
            "status",
            "status_before_the_set_up",
            "status_after_the_set_up",
        ] {
            replay = status_differences(replay, case, database);
        }
        // The admin sees no token, and which session is the one of the request (D-18).
        // The only session is the one that the set-up started.
        let me = "me_after_the_set_up";
        let session = "/response/data/tokenList/0";
        replay = replay
            .deviation("D-18", me, "/response/data/token", Value::Null)
            .deviation("D-18", me, &format!("{session}/token"), Value::Null)
            .deviation_adds("D-18", me, session, "current", json!(true));

        for case in [
            "launch_before_the_set_up",
            "launch_as_guest",
            "launch_as_guest_with_a_trailing_slash",
            "launch_as_admin",
            "launch_with_cookies_of_other_services",
        ] {
            replay = launch_differences(replay, case);
        }
        // The admin in the launch answer, as in `me_after_the_set_up` (D-18).
        let launch = "launch_as_admin";
        let session = "/response/data/user/tokenList/0";
        replay = replay
            .deviation("D-18", launch, "/response/data/user/token", Value::Null)
            .deviation("D-18", launch, &format!("{session}/token"), Value::Null)
            .deviation_adds("D-18", launch, session, "current", json!(true))
            // PHP sends the SQL text of its tables to an admin.
            .deviation("D-50", launch, SCHEMA, json!([]));
        // The one tab an admin sees after the set-up is "Settings". The set-up of PHP
        // leaves its switches empty (`null`). Here a switch is on or off.
        for switch in TAB_SWITCHES {
            replay = replay.deviation(
                "D-28",
                launch,
                &format!("/response/data/tabs/0/{switch}"),
                json!(0),
            );
        }

        // The application starts on a database without an install. The recorded
        // requests carry the set-up code of the test applications, which PHP ignores.
        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn basic_routes_answer_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = Replay::group("basics");
        let replay = status_differences(replay, "status", database);
        let replay = status_differences(replay, "status_with_trailing_slash", database);
        let replay = without_php_session(replay);

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn login_answers_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = without_php_session(Replay::group("login"));
        // The group fails more logins from one address than the login limit allows. PHP
        // has no limit on the server; the group `lockout` is about the limit.
        let app = installed_app_with(database, &[("loginAttempts", json!(NO_LOGIN_LIMIT))]);

        replay.run(&app).await;
    })
    .await;
}

#[tokio::test]
async fn logout_answers_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = without_php_session(Replay::group("logout"));

        replay.run(&installed_app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn session_routes_answer_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        seed::wizard_session(database).await;
        let mut replay = without_php_session(Replay::group("session"));
        // Session 1 is the one of the set-up wizard, 2 is browser A and 3 is browser B.
        //
        // `GET /token/me` holds no token, and says which session is the one of the
        // request. PHP shows the tokens of all sessions of the user.
        for (case, sessions, current) in [
            ("me_in_browser_a", 3, 1),
            ("me_in_browser_a_after_the_delete", 2, 1),
        ] {
            replay = replay.deviation("D-18", case, "/response/data/token", Value::Null);
            for index in 0..sessions {
                let entry = format!("/response/data/tokenList/{index}");
                replay = replay
                    .deviation("D-18", case, &format!("{entry}/token"), Value::Null)
                    .deviation_adds("D-18", case, &entry, "current", json!(index == current));
            }
        }
        // `POST /token/validate` gives back the token that the caller sent, but not the
        // tokens of the other sessions of the user.
        for case in [
            "validate_token_in_the_query",
            "validate_token_in_a_form",
            "validate_token_of_another_browser",
        ] {
            for index in [0, 2] {
                let token = format!("/response/data/tokenList/{index}/token");
                replay = replay.deviation("D-02", case, &token, Value::Null);
            }
        }

        replay.run(&installed_app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn login_limit_answers_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // PHP remembers a lock-out in a cookie of the browser. Here the server remembers.
        let replay =
            without_php_session(Replay::group("lockout")).without_cookie("D-04", "lockout");

        replay.run(&installed_app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn api_key_answers_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = without_php_session(Replay::group("api_key"));

        replay.run(&installed_app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn registration_answers_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The registration password of the recording, stored as the set-up stores it.
        let hash = organizr_auth::password::hash("let-new-users-in")
            .expect("the registration password can be hashed");
        let app = installed_app_with(database, &[("registrationPassword", json!(hash.expose()))]);
        let replay = without_php_session(Replay::group("register"));

        replay.run(&app).await;
    })
    .await;
}

/// The local address of the recorded tab "Local" as the list of tabs shows it to a
/// script with the API key. `komga` is the value of the cookie `komga_token`.
fn placeholders_filled_in(komga: &str) -> String {
    format!(
        "http://organizr.test/Organizr API/organizr api//Admin/0/{komga}/{{other}}/organizr.test"
    )
}

#[tokio::test]
async fn tab_routes_answer_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let mut replay = without_php_session(Replay::group("tabs"));

        // Every answer that shows a tab whose switches were never set.
        for case in [
            "tabs",
            "tab",
            "tab_with_a_leading_zero_in_the_id",
            "added_tab",
            "tab_with_spaces_cut_off",
            "tab_with_numbers_as_text",
            "tab_with_placeholders",
            "tab_with_true_and_false",
            "tab_from_a_form",
            "tab_with_null_for_address_and_image",
            "tabs_after_adding",
            "tabs_with_a_komga_cookie",
            "tab_with_html_in_the_name",
            "tab_with_changed_texts",
            "tabs_after_changing_the_default",
            "tab_changed_to_null",
            "tabs_after_ordering",
            "tabs_after_deleting",
        ] {
            replay = replay.deviation_for_nulls("D-28", case, &TAB_SWITCHES, json!(0));
        }
        replay = replay
            // PHP asks a guest for the form key first. Here a request that does not come
            // from another site goes on to the check of the level.
            .deviation(
                "D-03",
                "add_as_guest",
                "/response/message",
                json!("Not Authorized"),
            )
            // Text is stored as it was sent, not with HTML entities.
            .deviation(
                "D-24",
                "tab_with_html_in_the_name",
                "/response/data/name",
                json!("A & <b>"),
            )
            // The local address of the tab "Local" holds every placeholder.
            // `{username_lower}` gives the name in lower case here.
            .deviation(
                "D-47",
                "tabs_after_adding",
                "/response/data/tabs/6/url_local",
                json!(placeholders_filled_in("")),
            )
            .deviation(
                "D-47",
                "tabs_with_a_komga_cookie",
                "/response/data/tabs/6/url_local",
                json!(placeholders_filled_in("komga-value")),
            )
            .deviation(
                "D-47",
                "tabs_after_changing_the_default",
                "/response/data/tabs/6/url_local",
                json!(placeholders_filled_in("")),
            )
            .deviation(
                "D-47",
                "tabs_after_ordering",
                "/response/data/tabs/1/url_local",
                json!(placeholders_filled_in("")),
            )
            .deviation(
                "D-47",
                "tabs_after_deleting",
                "/response/data/tabs/1/url_local",
                json!(placeholders_filled_in("")),
            )
            // The same tab in the menu of the script with the API key.
            .deviation(
                "D-47",
                "launch_with_tabs",
                "/response/data/tabs/4/url_local",
                json!(placeholders_filled_in("")),
            )
            .deviation(
                "D-47",
                "launch_with_tabs",
                "/response/data/tabs/4/access_url",
                json!(placeholders_filled_in("")),
            );
        replay = replay.deviation("D-50", "launch_with_tabs", SCHEMA, json!([]));
        for case in ["launch_with_tabs_as_guest", "launch_with_tabs"] {
            replay = launch_differences(replay, case).deviation_for_nulls(
                "D-28",
                case,
                &TAB_SWITCHES,
                json!(0),
            );
        }

        replay.run(&installed_app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn category_routes_answer_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let mut replay = without_php_session(Replay::group("categories"));

        // The list of categories is the list of the tab editor. It shows the two tabs of
        // the set-up, whose switches were never set.
        for case in [
            "categories",
            "categories_after_adding",
            "categories_with_html_in_a_name",
            "categories_after_changing_the_default",
            "categories_after_changing",
            "categories_after_ordering",
            "categories_after_cleaning_up",
        ] {
            replay = replay.deviation_for_nulls("D-28", case, &TAB_SWITCHES, json!(0));
        }
        replay = replay
            // PHP asks a guest for the form key first. Here a request that does not come
            // from another site goes on to the check of the level.
            .deviation(
                "D-03",
                "add_as_guest",
                "/response/message",
                json!("Not Authorized"),
            )
            // Text is stored as it was sent, not with HTML entities.
            .deviation(
                "D-24",
                "categories_with_html_in_a_name",
                "/response/data/categories/8/category",
                json!("A & <b>"),
            );

        replay.run(&installed_app(database)).await;
    })
    .await;
}

/// The name and the picture of the group that the recorder adds with markup in both, as
/// this application stores them: as they were sent.
const MARKUP_NAME: &str = "<b>Bold</b> & co";
const MARKUP_IMAGE: &str = "a&b.png";

#[tokio::test]
async fn group_routes_answer_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The recorder runs this group after the group `register`.
        seed::registered_users(database).await;
        let mut replay = without_php_session(Replay::group("groups"))
            // PHP asks a guest for the form key first. Here a request that does not come
            // from another site goes on to the check of the level.
            .deviation(
                "D-03",
                "add_as_guest",
                "/response/message",
                json!("Not Authorized"),
            )
            // A user that was never locked has `null` in PHP.
            .deviation_for_nulls("D-51", "groups_with_users", &["locked"], json!(0))
            // In PHP a new default group leaves the old one as a second default group.
            .deviation(
                "D-51",
                "groups_after_adding_a_default",
                "/response/data/4/default",
                json!(0),
            );
        // The hash of a password does not leave the server.
        for user in 0..=seed::REGISTERED_USERS.len() {
            replay = replay.deviation(
                "D-05",
                "groups_with_users",
                &format!("/response/data/users/{user}/password"),
                Value::Null,
            );
        }
        // Text is stored as it was sent, not with HTML entities.
        for (case, place) in [
            ("groups_after_adding", 10),
            ("groups_after_adding_a_default", 10),
            ("groups_after_the_default_is_back", 10),
            ("groups_after_a_new_default", 10),
            ("groups_after_changing", 10),
            ("groups_after_deleting", 9),
        ] {
            let group = format!("/response/data/{place}");
            replay = replay
                .deviation("D-24", case, &format!("{group}/group"), json!(MARKUP_NAME))
                .deviation("D-24", case, &format!("{group}/image"), json!(MARKUP_IMAGE));
        }

        replay.run(&installed_app(database)).await;
    })
    .await;
}
