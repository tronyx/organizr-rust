//! The `settings`, `plugins` and `sso` parts of `GET /launch`, through the real route table.
//!
//! The recorded PHP answers show these parts for the default settings. These tests change
//! settings and look at what the answer then says, for a guest and for an admin.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::SocketAddr;

use axum::Router;
use axum::extract::ConnectInfo;
use organizr_auth::password;
use organizr_db::Database;
use organizr_domain::{GroupLevel, NewUser, Timestamp};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{request, send};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

/// An address on the local network, and one that is not.
const LOCAL: &str = "192.168.1.20:50000";
const ELSEWHERE: &str = "203.0.113.7:50000";

/// Who asks: a guest, or a script with the API key, which counts as an Admin.
#[derive(Clone, Copy)]
enum Who {
    Guest,
    Admin,
}

/// The `data` of the launch answer.
async fn launch(app: &Router, who: Who, peer: &str, cookie: &str) -> Value {
    let mut headers = Vec::new();
    if matches!(who, Who::Admin) {
        headers.push(("token", seed::API_KEY));
    }
    if !cookie.is_empty() {
        headers.push(("cookie", cookie));
    }
    let mut request = request("GET", "/api/v2/launch", &headers, "");
    let peer: SocketAddr = peer.parse().expect("the address is valid");
    request.extensions_mut().insert(ConnectInfo(peer));
    let answer = send(app, request).await;
    assert_eq!(answer.status, 200, "{}", answer.body);
    answer.data()
}

const CO_ADMIN_PASSWORD: &str = "the password of cora";

/// Stores a Co-Admin, logs in, and returns the session cookie.
async fn co_admin_cookie(database: &Database, app: &Router) -> String {
    let group = database
        .groups()
        .find_by_level(GroupLevel::CO_ADMIN)
        .await
        .expect("the lookup works")
        .expect("the seed has the group Co-Admin");
    database
        .users()
        .insert(NewUser {
            username: "cora".to_owned(),
            email: None,
            password_hash: password::hash(CO_ADMIN_PASSWORD).expect("the password can be hashed"),
            group: group.id,
            image: None,
            created_at: Timestamp::from_unix_seconds(1_800_000_000),
        })
        .await
        .expect("the user can be stored");
    let body = json!({"username": "cora", "password": CO_ADMIN_PASSWORD}).to_string();
    let headers = [("content-type", "application/json")];
    let answer = send(app, request("POST", "/api/v2/login", &headers, &body)).await;
    answer
        .cookies
        .iter()
        .find(|cookie| cookie.starts_with("organizr_token_") && !cookie.contains("=;"))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap_or_else(|| panic!("the login sets the session cookie: {}", answer.body))
        .to_owned()
}

fn keys(object: &Value) -> Vec<&str> {
    object
        .as_object()
        .expect("the value is an object")
        .keys()
        .map(String::as_str)
        .collect()
}

#[tokio::test]
async fn level_of_a_setting_decides_what_a_caller_may_do() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("homepageOmbiAuth", json!("999")),
                ("homepageOmbiRequestAuth", json!("1")),
                ("homepageOverseerrAuth", json!("0")),
                ("homepageOverseerrRequestAuth", json!("0")),
                ("debugAreaAuth", json!("998")),
                ("mediaSearchAuth", json!("999")),
                ("mediaSearch", json!(true)),
                ("plexToken", json!("a-token")),
            ],
        );

        let guest = launch(&app, Who::Guest, ELSEWHERE, "").await;
        let admin = launch(&app, Who::Admin, ELSEWHERE, "").await;

        let ombi = &guest["settings"]["homepage"]["ombi"];
        assert_eq!(ombi["authView"], json!(true));
        assert_eq!(ombi["authRequest"], json!(false));
        assert_eq!(guest["settings"]["misc"]["debugArea"], json!(false));
        assert_eq!(
            guest["settings"]["homepage"]["search"]["enabled"],
            json!(true)
        );
        let ombi = &admin["settings"]["homepage"]["ombi"];
        assert_eq!(ombi["authView"], json!(true));
        assert_eq!(ombi["authRequest"], json!(true));
        assert_eq!(admin["settings"]["misc"]["debugArea"], json!(true));
        // Level 0 is the Admin alone.
        let overseerr = &admin["settings"]["homepage"]["overseerr"];
        assert_eq!(overseerr["authView"], json!(true));
        assert_eq!(overseerr["userSelectTv"], json!(false));
    })
    .await;
}

#[tokio::test]
async fn media_search_needs_the_switch_and_a_plex_token() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        for (switch, token, expected) in [
            (true, "a-token", true),
            (false, "a-token", false),
            (true, "", false),
            // PHP takes the text "0" for false.
            (true, "0", false),
        ] {
            let app = installed_app_with(
                database,
                &[("mediaSearch", json!(switch)), ("plexToken", json!(token))],
            );

            let admin = launch(&app, Who::Admin, ELSEWHERE, "").await;

            assert_eq!(
                admin["settings"]["homepage"]["search"]["enabled"],
                json!(expected),
                "switch {switch}, token `{token}`"
            );
            // The text "0" is a token all the same.
            assert_eq!(
                admin["settings"]["sso"]["plex"]["token"],
                json!(!token.is_empty()),
                "token `{token}`"
            );
            // The search is for Admins and Co-Admins, as long as the level is not changed.
            let guest = launch(&app, Who::Guest, ELSEWHERE, "").await;
            assert_eq!(
                guest["settings"]["homepage"]["search"]["enabled"],
                json!(false)
            );
        }
    })
    .await;
}

#[tokio::test]
async fn requests_are_on_only_with_the_levels_the_switches_and_the_cookie() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let all_on = [
            ("homepageOmbiEnabled", json!(true)),
            ("ssoOmbi", json!(true)),
            ("homepageOverseerrEnabled", json!(true)),
            ("ssoOverseerr", json!(true)),
        ];
        let cookies = "Auth=ombi; connect.sid=overseerr";
        let enabled = |data: &Value| {
            let homepage = &data["settings"]["homepage"];
            (
                homepage["ombi"]["enabled"].clone(),
                homepage["overseerr"]["enabled"].clone(),
            )
        };

        let app = installed_app_with(database, &all_on);
        let on = (json!(true), json!(true));
        let off = (json!(false), json!(false));
        assert_eq!(
            enabled(&launch(&app, Who::Admin, ELSEWHERE, cookies).await),
            on
        );
        assert_eq!(enabled(&launch(&app, Who::Admin, ELSEWHERE, "").await), off);
        // A guest has not the level of the two homepage items.
        assert_eq!(
            enabled(&launch(&app, Who::Guest, ELSEWHERE, cookies).await),
            off
        );

        for missing in ["homepageOmbiEnabled", "ssoOmbi"] {
            let settings: Vec<(&str, Value)> = all_on
                .iter()
                .filter(|(key, _)| *key != missing)
                .cloned()
                .collect();
            let app = installed_app_with(database, &settings);

            let data = launch(&app, Who::Admin, ELSEWHERE, cookies).await;

            assert_eq!(
                enabled(&data),
                (json!(false), json!(true)),
                "without {missing}"
            );
        }

        // A caller needs the level to see the item and the level to make a request.
        let for_guests = |keys: &[&'static str]| -> Vec<(&'static str, Value)> {
            let levels = keys.iter().map(|key| (*key, json!("999")));
            all_on.iter().cloned().chain(levels).collect()
        };
        for (levels, expected) in [
            (vec!["homepageOmbiRequestAuth"], false),
            (vec!["homepageOmbiAuth"], false),
            (vec!["homepageOmbiAuth", "homepageOmbiRequestAuth"], true),
        ] {
            let app = installed_app_with(database, &for_guests(&levels));

            let data = launch(&app, Who::Guest, ELSEWHERE, cookies).await;

            assert_eq!(
                data["settings"]["homepage"]["ombi"]["enabled"],
                json!(expected),
                "a guest with {levels:?}"
            );
        }
        // The switch of Overseerr is its own.
        let settings: Vec<(&str, Value)> = all_on
            .iter()
            .filter(|(key, _)| *key != "homepageOverseerrEnabled")
            .cloned()
            .collect();
        let app = installed_app_with(database, &settings);
        let data = launch(&app, Who::Admin, ELSEWHERE, cookies).await;
        assert_eq!(enabled(&data), (json!(true), json!(false)));
    })
    .await;
}

#[tokio::test]
async fn single_sign_on_part_describes_the_services() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("ssoPlex", json!(true)),
                ("plexID", json!("0123456789abcdef0123456789abcdef01234567")),
                ("plexToken", json!("a-token")),
                ("plexAdmin", json!("owner@example.com")),
                ("plexStrictFriends", json!(false)),
                ("plexoAuth", json!(true)),
                ("authBackend", json!("plex")),
                ("tautulliURL", json!("https://tautulli.example.com")),
                ("ssoTautulli", json!(true)),
                ("overseerrToken", json!("a-key")),
                ("jellyfinSSOURL", json!("https://jellyfin.example.com")),
                ("rememberMe", json!(false)),
                ("rememberMeDays", json!("30")),
            ],
        );

        let data = launch(&app, Who::Guest, ELSEWHERE, "").await;

        let sso = &data["settings"]["sso"];
        assert_eq!(
            sso["plex"],
            json!({
                "enabled": true,
                "cookie": false,
                "machineID": true,
                "token": true,
                "plexAdmin": "email",
                "strict": false,
                "oAuthEnabled": true,
                "backend": true,
            })
        );
        assert_eq!(
            sso["tautulli"],
            json!({"enabled": true, "cookie": false, "url": "https://tautulli.example.com"})
        );
        assert_eq!(sso["overseerr"]["api"], json!(true));
        assert_eq!(sso["overseerr"]["url"], json!(false));
        assert_eq!(
            sso["jellyfin"],
            json!({"enabled": false, "url": false, "ssoUrl": "https://jellyfin.example.com"})
        );
        assert_eq!(
            sso["misc"],
            json!({"oAuthLogin": false, "rememberMe": false, "rememberMeDays": "30"})
        );
        // The secrets themselves are nowhere in the answer.
        let text = data.to_string();
        assert!(
            !text.contains("a-token") && !text.contains("a-key"),
            "{text}"
        );
    })
    .await;
}

#[tokio::test]
async fn plex_admin_is_a_name_or_an_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        for (plex_admin, expected) in [
            ("", json!(false)),
            ("owner", json!("username")),
            ("first.last", json!("username")),
            ("owner@example.com", json!("email")),
        ] {
            // The id of a Plex server has 40 characters. This one has 41.
            let app = installed_app_with(
                database,
                &[
                    ("plexAdmin", json!(plex_admin)),
                    ("plexID", json!("0123456789abcdef0123456789abcdef012345678")),
                ],
            );

            let data = launch(&app, Who::Guest, ELSEWHERE, "").await;

            assert_eq!(data["settings"]["sso"]["plex"]["plexAdmin"], expected);
            assert_eq!(data["settings"]["sso"]["plex"]["machineID"], json!(false));
        }
    })
    .await;
}

#[tokio::test]
async fn login_without_a_password_is_offered_to_a_local_caller_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let bypass = async |settings: &[(&str, Value)], peer: &str| {
            let app = installed_app_with(database, settings);
            let data = launch(&app, Who::Guest, peer, "").await;
            let user = &data["settings"]["user"];
            assert_eq!(user["local"], json!(peer == LOCAL));
            assert_eq!(
                user["ip"],
                json!(peer.rsplit_once(':').expect("the address has a port").0)
            );
            user["bypass"].clone()
        };
        let on = [("bypassLoginForLocal", json!(true))];
        let no_user = [
            ("bypassLoginForLocal", json!(true)),
            ("localLoginUserId", json!("0")),
        ];

        assert_eq!(bypass(&on, LOCAL).await, json!(true));
        assert_eq!(bypass(&on, ELSEWHERE).await, json!(false));
        assert_eq!(bypass(&[], LOCAL).await, json!(false));
        assert_eq!(bypass(&no_user, LOCAL).await, json!(false));
    })
    .await;
}

#[tokio::test]
async fn some_values_are_for_admins_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("installedPlugins", json!(["a-plugin"])),
                ("installedThemes", json!("a-theme")),
            ],
        );

        let guest = launch(&app, Who::Guest, ELSEWHERE, "").await;
        let admin = launch(&app, Who::Admin, ELSEWHERE, "").await;
        let cookie = co_admin_cookie(database, &app).await;
        let co_admin = launch(&app, Who::Guest, ELSEWHERE, &cookie).await;

        assert_eq!(co_admin["user"]["groupID"], json!(1));
        assert_eq!(co_admin["settings"]["misc"], admin["settings"]["misc"]);
        for key in [
            "installedPlugins",
            "installedThemes",
            "docker",
            "githubCommit",
            "schema",
        ] {
            assert_eq!(guest["settings"]["misc"][key], json!(""), "{key}");
        }
        let misc = &admin["settings"]["misc"];
        assert_eq!(misc["installedPlugins"], json!(["a-plugin"]));
        assert_eq!(misc["installedThemes"], json!("a-theme"));
        assert_eq!(misc["docker"], json!(false));
        assert_eq!(misc["githubCommit"], Value::Null);
        assert_eq!(misc["schema"], json!([]));
        // Both get the id of the install.
        assert_eq!(guest["settings"]["misc"]["uuid"], json!(seed::INSTALL_ID));
        assert_eq!(misc["uuid"], json!(seed::INSTALL_ID));
    })
    .await;
}

#[tokio::test]
async fn folding_of_the_menu_needs_the_switch_that_allows_it() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        for (allowed, expected) in [(true, true), (false, false)] {
            let app = installed_app_with(
                database,
                &[
                    ("allowCollapsableSideMenu", json!(allowed)),
                    ("sideMenuCollapsed", json!(true)),
                    ("collapseSideMenuOnClick", json!(true)),
                ],
            );

            let data = launch(&app, Who::Guest, ELSEWHERE, "").await;

            let misc = &data["settings"]["misc"];
            assert_eq!(misc["sideMenuCollapsed"], json!(expected));
            assert_eq!(misc["collapseSideMenuOnClick"], json!(expected));
        }
    })
    .await;
}

#[tokio::test]
async fn refresh_list_holds_numbers_and_no_secret() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("traktRefreshToken", json!("123456")),
                ("calendarRefresh", json!("5000")),
            ],
        );

        let data = launch(&app, Who::Admin, ELSEWHERE, "").await;

        let refresh = &data["settings"]["homepage"]["refresh"];
        assert_eq!(refresh["calendarRefresh"], json!("5000"));
        assert_eq!(refresh.get("traktRefreshToken"), None);
        assert!(!data.to_string().contains("123456"));
        let listed = keys(refresh);
        let mut sorted = listed.clone();
        sorted.sort_unstable();
        assert_eq!(listed, sorted, "the keys are sorted");
        assert_eq!(listed.first(), Some(&"CHAT-userRefreshTimeout"));
    })
    .await;
}

#[tokio::test]
async fn homepage_items_come_in_the_order_of_their_places() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                // Two items on place 1, and one item before both.
                ("homepageOrderplexrecent", json!("1")),
                ("homepageOrderJellyStat", json!("0")),
                ("homepageOrdercustomhtml02", json!("100")),
            ],
        );

        let data = launch(&app, Who::Guest, ELSEWHERE, "").await;

        let order = keys(&data["settings"]["homepage"]["order"]);
        assert_eq!(
            order[..4],
            [
                "homepageOrderJellyStat",
                "homepageOrdercustomhtml01",
                "homepageOrderplexrecent",
                "homepageOrdertransmission",
            ]
        );
        assert_eq!(order.last(), Some(&"homepageOrdercustomhtml02"));
        // The 48 items of PHP, and the Netronome item (D-128).
        assert_eq!(order.len(), 49);
    })
    .await;
}

#[tokio::test]
async fn plugins_part_lists_the_switches_and_the_shared_settings() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("BOOKMARK-enabled", json!(true)),
                ("INVITES-type-include", json!("emby")),
                ("CHAT-secret", json!("a-secret")),
            ],
        );

        let data = launch(&app, Who::Guest, ELSEWHERE, "").await;

        let plugins = &data["plugins"];
        assert_eq!(plugins["BOOKMARK-enabled"], json!(true));
        assert_eq!(plugins["CHAT-enabled"], json!(false));
        assert_eq!(plugins["includes"]["INVITES-type-include"], json!("emby"));
        assert_eq!(keys(plugins).last(), Some(&"includes"));
        assert!(
            keys(plugins)
                .iter()
                .all(|key| key.ends_with("-enabled") || *key == "includes")
        );
        assert!(!data.to_string().contains("a-secret"));
    })
    .await;
}

#[tokio::test]
async fn cookies_of_other_services_are_sent_back() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);

        let none = launch(&app, Who::Guest, ELSEWHERE, "").await;
        let some = launch(
            &app,
            Who::Guest,
            ELSEWHERE,
            "mpt=one%20two; mpt=later; user-1=a; petio_jwt=p; oAuth=true; other=x; userid=9",
        )
        .await;

        assert_eq!(
            none["sso"],
            json!({
                "myPlexAccessToken": false,
                "id_token": false,
                "jellyfin_credentials": false,
                "komga_token": false,
            })
        );
        assert_eq!(
            some["sso"],
            json!({
                "myPlexAccessToken": "one two",
                "id_token": false,
                "jellyfin_credentials": false,
                "komga_token": false,
                "user-1": "a",
            })
        );
        let settings = &some["settings"];
        assert_eq!(settings["sso"]["plex"]["cookie"], json!(true));
        assert_eq!(settings["sso"]["petio"]["cookie"], json!(true));
        assert_eq!(settings["sso"]["komga"]["cookie"], json!(false));
        assert_eq!(settings["user"]["oAuthLogin"], json!(true));
        assert_eq!(none["settings"]["user"]["oAuthLogin"], json!(false));
    })
    .await;
}
