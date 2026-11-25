//! The items of the homepage that show the state of a service: AdGuard Home, Netdata,
//! Monitorr, Speedtest Tracker, Netronome, OctoPrint and UniFi, against stand-ins for
//! them.

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
use organizr_testkit::homepage_services::{
    self, ADGUARD_LOGIN, OCTOPRINT_INNER_KEY, OCTOPRINT_KEY, UNIFI_LOGIN, UNIFI_TOKEN,
};
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const NOT_APPROVED: &str = "User not approved to view this homepage item";

/// An address that nothing listens at.
const NOWHERE: &str = "http://127.0.0.1:9";

type Settings = Vec<(&'static str, Value)>;

async fn get(app: &Router, path: &str) -> Answer {
    call(app, As::Admin, "GET", path, None).await
}

/// Whether the list of the homepage has an item of a kind for the caller.
async fn lists(app: &Router, who: As<'_>, kind: &str) -> bool {
    let items = call(app, who, "GET", "/homepage/items", None).await;
    items
        .data()
        .as_array()
        .expect("the items are a list")
        .iter()
        .any(|item| item["kind"] == json!(kind))
}

/// What holds for every item. `settings` starts with the switch of the item, and has
/// each setting that must hold a value after it.
async fn is_refused_as_every_item(
    database: &Database,
    path: &str,
    kind: &str,
    settings: &Settings,
) {
    let app = installed_app_with(database, settings);
    assert!(lists(&app, As::Admin, kind).await, "{kind} is listed");

    // The level of each of these items is the one of a Co-Admin, when the settings
    // name none.
    let user = seed::logged_in(database, &app, "Robin", 4).await;
    for who in [As::Guest, As::User(&user)] {
        let answer = call(&app, who, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{path}");
        assert_eq!(answer.message(), json!(NOT_APPROVED));
        assert!(!lists(&app, who, kind).await, "{kind} is not listed");
    }

    let switch = settings[0].0;
    let off = installed_app_with(database, &settings[1..]);
    let answer = get(&off, path).await;
    assert_eq!(answer.status, StatusCode::CONFLICT, "{path}");
    assert_eq!(
        answer.message(),
        json!(format!("{switch} module is not enabled"))
    );
    assert!(!lists(&off, As::Admin, kind).await);

    for missing in 1..settings.len() {
        let mut without = settings.clone();
        let (key, _) = without.remove(missing);
        let app = installed_app_with(database, &without);
        let answer = get(&app, path).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{key}");
        assert_eq!(answer.message(), json!(format!("{key} was not supplied")));
        assert!(!lists(&app, As::Admin, kind).await);
    }
}

#[tokio::test]
async fn adguard_gives_the_statistics_of_each_install_by_its_host() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let first = homepage_services::adguard().await;
        let second = homepage_services::adguard_with_login().await;
        // Both stand-ins are on this machine. The second one is named by its host name,
        // so that the two have different hosts.
        let second_url = second.url().replace("127.0.0.1", "localhost");
        let settings: Settings = vec![
            ("homepageAdGuardEnabled", json!(true)),
            (
                "adguardURL",
                json!(format!("{}/,{second_url}", first.url())),
            ),
            ("adguardDomainListToggle", json!(true)),
        ];
        let app = installed_app_with(database, &settings);

        let answer = get(&app, "/homepage/adguard/stats").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        // The install with a login refuses, and is left out.
        assert_eq!(
            data["data"]
                .as_object()
                .map(|map| map.keys().map(String::as_str).collect()),
            Some(vec!["127.0.0.1"])
        );
        assert_eq!(data["data"]["127.0.0.1"]["num_dns_queries"], json!(12000));
        assert_eq!(
            data["filters"]["127.0.0.1"]["filters"][0]["rules_count"],
            json!(100_000)
        );
        assert_eq!(
            data["options"],
            json!({
                "combine": false,
                "title": true,
                "queries": true,
                "blocked_count": true,
                "blocked_percent": true,
                "processing_time": true,
                "domain_count": true,
            })
        );
        let paths: Vec<String> = first.seen().into_iter().map(|seen| seen.path).collect();
        assert_eq!(paths, ["/control/stats", "/control/filtering/status"]);
        // Without a user name, the requests carry no login.
        assert_eq!(first.seen()[0].header("authorization"), None);

        // The test names the install that refused.
        let answer = call(&app, As::Admin, "POST", "/test/adguard", None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.message(), json!("localhost: Unknown Failure"));
        let alone: Settings = vec![("adguardURL", json!(first.url()))];
        let app = installed_app_with(database, &alone);
        let answer = call(&app, As::Admin, "POST", "/test/adguard", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

        // With the login of the settings, the install with a login answers too, and so
        // does the test (D-102).
        let (username, password, header) = ADGUARD_LOGIN;
        let with_login = |password: &str| -> Settings {
            let mut settings = settings.clone();
            settings.push(("adGuardUsername", json!(username)));
            settings.push(("adGuardPassword", json!(password)));
            settings
        };
        let logged_in = installed_app_with(database, &with_login(password));
        let answer = get(&logged_in, "/homepage/adguard/stats").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data()["data"]["localhost"]["num_dns_queries"],
            json!(12000)
        );
        assert_eq!(
            second
                .seen()
                .last()
                .and_then(|seen| seen.header("authorization")),
            Some(header)
        );
        let answer = call(&logged_in, As::Admin, "POST", "/test/adguard", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let wrong = installed_app_with(database, &with_login("a wrong password"));
        let answer = call(&wrong, As::Admin, "POST", "/test/adguard", None).await;
        assert_eq!(answer.message(), json!("localhost: Unknown Failure"));
        assert!(!answer.body.contains("a wrong password"), "{}", answer.body);

        let user = seed::logged_in(database, &app, "Robin", 4).await;
        let answer = call(&app, As::User(&user), "POST", "/test/adguard", None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        let app = installed_app_with(database, &[]);
        let answer = call(&app, As::Admin, "POST", "/test/adguard", None).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(answer.message(), json!("AdGuard URL is not defined"));
    })
    .await;
}

#[tokio::test]
async fn adguard_is_refused_as_every_item_and_fails_without_its_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let settings: Settings = vec![
            ("homepageAdGuardEnabled", json!(true)),
            ("adguardURL", json!(NOWHERE)),
        ];
        is_refused_as_every_item(database, "/homepage/adguard/stats", "adguard", &settings).await;

        let app = installed_app_with(database, &settings);
        let answer = get(&app, "/homepage/adguard/stats").await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);
    })
    .await;
}

/// The settings of a chart of the Netdata item.
fn chart(number: usize, source: &str) -> Settings {
    let key = |name: &str| -> &'static str { format!("netdata{number}{name}").leak() };
    vec![
        (key("Enabled"), json!(true)),
        (key("Data"), json!(source)),
        (key("Title"), json!(format!("Chart {number}"))),
        (key("Chart"), json!("gauge")),
    ]
}

#[tokio::test]
async fn netdata_gives_one_number_for_each_chart_that_is_on() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::netdata().await;
        let mut settings: Settings = vec![
            ("homepageNetdataEnabled", json!(true)),
            ("netdataURL", json!(service.url())),
            ("netdata3Colour", json!("46e302")),
            ("netdata3Size", json!("lg")),
            ("netdata3sm", json!(false)),
            (
                "netdataCustom",
                json!(r#"{"5": {"url": "/api/v1/data?chart=system.swap&format=array", "value": "result,0", "units": "GB", "max": 6, "mutator": "*2"}}"#),
            ),
        ];
        for (number, source) in [
            (1, "disk-read"),
            (2, "disk-write"),
            (3, "cpu"),
            (4, "disk-avail"),
            (5, "custom"),
            (6, "custom"),
            (7, "cpu-temp-c"),
        ] {
            settings.extend(chart(number, source));
        }
        // A chart that is off is left out.
        settings.retain(|(key, _)| *key != "netdata4Enabled");
        let app = installed_app_with(database, &settings);

        let answer = get(&app, "/homepage/netdata/data").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(data["url"], json!(service.url()));
        let charts = data["data"].as_array().expect("the charts are a list");
        assert_eq!(charts.len(), 6);
        assert_eq!(
            charts[0],
            json!({
                "value": 1.5, "percent": 25, "units": "MiB/s", "max": 6000,
                "title": "Chart 1", "colour": "", "chart": "gauge", "size": "",
                "lg": true, "md": true, "sm": true,
            })
        );
        // What the disks write is counted below 0, and shows above 0.
        assert_eq!(charts[1]["value"], json!(3));
        assert_eq!(charts[1]["percent"], json!(50));
        assert_eq!(charts[2]["value"], json!(12.5));
        assert_eq!(charts[2]["percent"], json!(12.5));
        assert_eq!(charts[2]["units"], json!("%"));
        assert_eq!(charts[2]["colour"], json!("46e302"));
        assert_eq!(charts[2]["size"], json!("lg"));
        assert_eq!(charts[2]["sm"], json!(false));
        // The custom chart: 1.5 times 2, of 6 at most.
        assert_eq!(charts[3]["title"], json!("Chart 5"));
        assert_eq!(charts[3]["value"], json!(3));
        assert_eq!(charts[3]["percent"], json!(50));
        assert_eq!(charts[3]["units"], json!("GB"));
        assert_eq!(charts[3]["max"], json!(6));
        assert_eq!(charts[4]["error"], json!("custom definition not found"));
        // A source that PHP names and has no code for.
        assert_eq!(charts[5]["value"], json!(0));
        assert_eq!(charts[5]["units"], json!("N/A"));

        let paths: Vec<String> = service.seen().into_iter().map(|seen| seen.path).collect();
        assert_eq!(paths.len(), 4);
        assert_eq!(
            paths[0],
            "/api/v1/data?chart=system.io&dimensions=in&format=array&points=540&group=average&gtime=0&options=absolute|jsonwrap|nonzero&after=-540"
        );
        assert_eq!(paths[2], "/api/v1/data?chart=system.cpu&format=array");
        assert_eq!(paths[3], "/api/v1/data?chart=system.swap&format=array");
    })
    .await;
}

#[tokio::test]
async fn netdata_is_refused_as_every_item_and_shows_no_value_without_netdata() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let mut settings: Settings = vec![
            ("homepageNetdataEnabled", json!(true)),
            ("netdataURL", json!(NOWHERE)),
        ];
        is_refused_as_every_item(database, "/homepage/netdata/data", "netdata", &settings).await;

        // Netdata cannot be reached: the charts come without a value, as in PHP.
        settings.extend(chart(1, "cpu"));
        settings.extend(chart(2, "net-out"));
        let app = installed_app_with(database, &settings);
        let answer = get(&app, "/homepage/netdata/data").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(data["data"][0].get("value"), None);
        assert_eq!(data["data"][0]["title"], json!("Chart 1"));
        assert_eq!(data["data"][1]["value"], json!(0));

        // A chart that Netdata does not have comes without a value too.
        let service = homepage_services::netdata().await;
        let mut settings: Settings = vec![
            ("homepageNetdataEnabled", json!(true)),
            ("netdataURL", json!(service.url())),
            (
                "netdataCustom",
                json!(r#"{"1": {"url": "api/v1/data?chart=none", "value": "result,0"}}"#),
            ),
        ];
        settings.extend(chart(1, "custom"));
        let app = installed_app_with(database, &settings);
        let answer = get(&app, "/homepage/netdata/data").await;
        assert_eq!(answer.data()["data"][0].get("value"), None);
        assert_eq!(answer.data()["data"][0].get("error"), None);
    })
    .await;
}

#[tokio::test]
async fn monitorr_gives_the_services_of_its_page_with_the_offline_ones_first() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::monitorr().await;
        let settings: Settings = vec![
            ("homepageMonitorrEnabled", json!(true)),
            ("monitorrURL", json!(format!("{}/", service.url()))),
            ("monitorrCompact", json!(true)),
        ];
        let app = installed_app_with(database, &settings);

        let answer = get(&app, "/homepage/monitorr/data").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        let services = data["services"].as_object().expect("an object");
        let names: Vec<&String> = services.keys().collect();
        assert_eq!(names, ["Backup & Sync", "Plex", "Router"]);
        assert_eq!(
            services["Plex"],
            json!({
                "status": true,
                "sort": 0,
                // The picture comes with the answer.
                "image": "data:image/png;base64,/9j/4AAQSkZJRg==",
                "link": "https://plex.example.com/web",
            })
        );
        // A picture that cannot be loaded is the picture of PHP for "none".
        assert_eq!(
            services["Router"],
            json!({
                "status": "unresponsive",
                "sort": 1,
                "image": "plugins/images/homepage/no-list.png",
            })
        );
        assert_eq!(services["Backup & Sync"]["status"], json!(false));
        assert_eq!(services["Backup & Sync"]["sort"], json!(2));
        assert_eq!(
            data["options"],
            json!({"title": "Monitorr", "titleToggle": true, "compact": true})
        );
        let seen = service.seen();
        assert_eq!(seen[0].path, "/assets/php/loop.php");
        // The API key of Organizr does not go to Monitorr.
        assert_eq!(seen[0].header("token"), None);
        assert!(
            seen.iter()
                .any(|seen| seen.path == "/assets/data/usrimg/plex.png")
        );
    })
    .await;
}

#[tokio::test]
async fn monitorr_is_refused_as_every_item_and_fails_as_in_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let settings: Settings = vec![
            ("homepageMonitorrEnabled", json!(true)),
            ("monitorrURL", json!(NOWHERE)),
        ];
        is_refused_as_every_item(database, "/homepage/monitorr/data", "monitorr", &settings).await;

        // PHP answers 401 for a Monitorr that cannot be reached.
        let app = installed_app_with(database, &settings);
        let answer = get(&app, "/homepage/monitorr/data").await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);

        // A service that is no Monitorr has no page for the tiles.
        let other = homepage_services::netdata().await;
        let settings: Settings = vec![
            ("homepageMonitorrEnabled", json!(true)),
            ("monitorrURL", json!(other.url())),
        ];
        let app = installed_app_with(database, &settings);
        let answer = get(&app, "/homepage/monitorr/data").await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            answer.message(),
            json!("Monitorr answered with the status 404")
        );
    })
    .await;
}

#[tokio::test]
async fn speed_test_item_gives_the_last_result_of_speedtest_tracker() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::speedtest_tracker().await;
        let settings: Settings = vec![
            ("homepageSpeedtestEnabled", json!(true)),
            ("speedtestURL", json!(service.url())),
            ("speedtestHeader", json!("Line")),
        ];
        let app = installed_app_with(database, &settings);

        let answer = get(&app, "/homepage/speedtest/data").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(data["data"]["current"]["ping"], json!(12.345));
        assert_eq!(data["data"]["current"]["download"], json!(250.12));
        assert_eq!(data["data"]["average"]["upload"], json!(39));
        // An old version names the highest values `max`.
        assert_eq!(data["data"]["maximum"]["download"], json!(260.5));
        assert_eq!(data["data"]["minimum"]["ping"], json!(9.5));
        assert_eq!(data["data"].get("max"), None);
        assert_eq!(
            data["options"],
            json!({"title": "Line", "titleToggle": true})
        );
        assert_eq!(service.seen()[0].path, "/api/speedtest/latest");

        is_refused_as_every_item(
            database,
            "/homepage/speedtest/data",
            "speedtest",
            &vec![
                ("homepageSpeedtestEnabled", json!(true)),
                ("speedtestURL", json!(NOWHERE)),
            ],
        )
        .await;
    })
    .await;
}

#[tokio::test]
async fn speed_test_item_fails_with_the_answers_of_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let settings = |url: String| -> Settings {
            vec![
                ("homepageSpeedtestEnabled", json!(true)),
                ("speedtestURL", json!(url)),
            ]
        };
        let app = installed_app_with(database, &settings(NOWHERE.to_owned()));
        let answer = get(&app, "/homepage/speedtest/data").await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);

        // A service that is no Speedtest Tracker refuses the request.
        let other = homepage_services::netdata().await;
        let app = installed_app_with(database, &settings(other.url()));
        let answer = get(&app, "/homepage/speedtest/data").await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), json!("SpeedTest connection error"));
    })
    .await;
}

#[tokio::test]
async fn netronome_item_gives_the_last_test_and_the_average_of_the_time_range() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::netronome().await;
        let settings = |range: &str| -> Settings {
            vec![
                ("homepageNetronomeEnabled", json!(true)),
                ("netronomeURL", json!(service.url())),
                ("homepageNetronomeHeader", json!("Line")),
                ("homepageNetronomeTimeRange", json!(range)),
            ]
        };
        let app = installed_app_with(database, &settings("week"));

        let answer = get(&app, "/homepage/netronome/data").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(
            data["data"]["current"],
            json!({
                "ping": 12.345, "download": 250.12, "upload": 40.26, "jitter": 1.25,
                "server": "A server", "type": "speedtest", "time": "2026-11-03T10:00:00Z",
            })
        );
        assert_eq!(data["data"]["maximum"]["download"], json!(260.5));
        assert_eq!(data["data"]["maximum"]["ping"], json!(20));
        // A latency without a unit is a count of milliseconds.
        assert_eq!(data["data"]["minimum"]["ping"], json!(9.5));
        assert_eq!(data["data"]["minimum"]["upload"], json!(35));
        let average = data["data"]["average"]["download"].as_f64();
        assert!(average.is_some_and(|speed| (speed - 236.9567).abs() < 0.001));
        assert_eq!(data["data"]["tests"], json!(3));
        assert_eq!(
            data["options"],
            json!({"title": "Line", "titleToggle": true, "timeRange": "week"})
        );
        // The request is for the public history, and it carries no login.
        let seen = service.seen();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0].path,
            "/api/speedtest/public/history?timeRange=week&page=1&limit=1000"
        );
        assert_eq!(seen[0].header("authorization"), None);
        assert_eq!(seen[0].header("cookie"), None);

        // A time range without a test: the last test is an older one, and there is no
        // average.
        let app = installed_app_with(database, &settings("24h"));
        let answer = get(&app, "/homepage/netronome/data").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(
            data["data"]["current"]["time"],
            json!("2026-11-03T10:00:00Z")
        );
        assert_eq!(data["data"].get("average"), None);
        assert_eq!(data["options"]["timeRange"], json!("24h"));
        assert_eq!(
            service.seen()[2].path,
            "/api/speedtest/public/history?timeRange=all&page=1&limit=1"
        );

        // A text that is no time range of Netronome never goes into the request.
        let app = installed_app_with(database, &settings("all&limit=5"));
        let answer = get(&app, "/homepage/netronome/data").await;
        assert_eq!(answer.data()["options"]["timeRange"], json!("week"));
        assert!(service.seen()[3].path.contains("timeRange=week&"));

        is_refused_as_every_item(
            database,
            "/homepage/netronome/data",
            "netronome",
            &vec![
                ("homepageNetronomeEnabled", json!(true)),
                ("netronomeURL", json!(NOWHERE)),
            ],
        )
        .await;
    })
    .await;
}

#[tokio::test]
async fn netronome_item_fails_as_the_speed_test_item() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let settings = |url: String| -> Settings {
            vec![
                ("homepageNetronomeEnabled", json!(true)),
                ("netronomeURL", json!(url)),
            ]
        };
        let app = installed_app_with(database, &settings(NOWHERE.to_owned()));
        let answer = get(&app, "/homepage/netronome/data").await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);

        // A service that is no Netronome refuses the request, or answers with something
        // that is no history.
        for other in [
            homepage_services::netdata().await,
            homepage_services::speedtest_tracker().await,
        ] {
            let app = installed_app_with(database, &settings(other.url()));
            let answer = get(&app, "/homepage/netronome/data").await;
            assert_eq!(answer.status, StatusCode::CONFLICT);
            assert_eq!(answer.message(), json!("Netronome connection error"));
        }
    })
    .await;
}

#[tokio::test]
async fn octoprint_gives_the_job_and_only_the_webcam_of_the_settings() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::octoprint().await;
        let settings = |key: &str| -> Settings {
            vec![
                ("homepageOctoprintEnabled", json!(true)),
                ("octoprintURL", json!(service.url())),
                ("octoprintToken", json!(key)),
            ]
        };
        let app = installed_app_with(database, &settings(OCTOPRINT_KEY));

        let answer = get(&app, "/homepage/octoprint/data").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(data["data"]["url"], json!(service.url()));
        assert_eq!(data["data"]["job"]["state"], json!("Printing"));
        assert_eq!(
            data["data"]["job"]["job"]["file"]["display"],
            json!("Bracket.gcode")
        );
        assert_eq!(data["data"]["job"]["progress"]["completion"], json!(41.6));
        assert_eq!(
            data["data"]["settings"],
            json!({"webcam": {"webcamEnabled": true, "streamUrl": "/webcam/?action=stream"}})
        );
        // The settings of OctoPrint hold a key, which stays on the server.
        assert!(
            !answer.body.contains(OCTOPRINT_INNER_KEY),
            "{}",
            answer.body
        );
        assert!(!answer.body.contains(OCTOPRINT_KEY), "{}", answer.body);
        assert_eq!(
            data["options"],
            json!({"title": "Octoprint", "titleToggle": true})
        );
        let seen = service.seen();
        assert_eq!(seen[0].path, "/api/job");
        assert_eq!(seen[1].path, "/api/settings");
        assert_eq!(seen[0].header("x-api-key"), Some(OCTOPRINT_KEY));

        // OctoPrint refuses a wrong key.
        let app = installed_app_with(database, &settings("wrong"));
        let answer = get(&app, "/homepage/octoprint/data").await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), json!("OctoPrint connection error"));
    })
    .await;
}

#[tokio::test]
async fn octoprint_is_refused_as_every_item_and_fails_without_its_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let secret = "made-up-secret-key";
        let settings: Settings = vec![
            ("homepageOctoprintEnabled", json!(true)),
            ("octoprintURL", json!(NOWHERE)),
            ("octoprintToken", json!(secret)),
        ];
        is_refused_as_every_item(database, "/homepage/octoprint/data", "octoprint", &settings)
            .await;

        let app = installed_app_with(database, &settings);
        let answer = get(&app, "/homepage/octoprint/data").await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);
        assert!(!answer.body.contains(secret), "{}", answer.body);
    })
    .await;
}

fn unifi_settings(url: &str, password: &str, site: &str) -> Settings {
    vec![
        ("homepageUnifiEnabled", json!(true)),
        ("unifiURL", json!(url)),
        ("unifiUsername", json!(UNIFI_LOGIN.0)),
        ("unifiPassword", json!(password)),
        ("unifiSiteName", json!(site)),
    ]
}

#[tokio::test]
async fn unifi_logs_in_to_a_classic_controller_and_reads_the_health_of_the_site() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::unifi_classic().await;
        let settings = |password: &str, site: &str| unifi_settings(&service.url(), password, site);
        let app = installed_app_with(database, &settings(UNIFI_LOGIN.1, "default"));

        let answer = get(&app, "/homepage/unifi/data").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let health = &answer.data()["content"]["unifi"];
        assert_eq!(health["meta"]["rc"], json!("ok"));
        assert_eq!(health["data"][0]["subsystem"], json!("wlan"));
        assert_eq!(health["data"][0]["num_user"], json!(14));
        assert_eq!(health["data"][1]["wan_ip"], json!("203.0.113.7"));
        let seen = service.seen();
        let paths: Vec<&str> = seen.iter().map(|seen| seen.path.as_str()).collect();
        assert_eq!(
            paths,
            ["/", "/manage", "/api/login", "/api/s/default/stat/health"]
        );
        assert_eq!(
            seen[2].json(),
            json!({
                "username": UNIFI_LOGIN.0,
                "password": UNIFI_LOGIN.1,
                "remember": true,
                "strict": true,
            })
        );
        assert_eq!(seen[2].header("x-csrf-token"), None);
        // The password stays on the server.
        assert!(!answer.body.contains(UNIFI_LOGIN.1), "{}", answer.body);

        // The test passes, and the list of the sites comes for the choice of one.
        let answer = call(&app, As::Admin, "POST", "/test/unifi", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("API Connection succeeded"));
        let answer = call(&app, As::Admin, "POST", "/test/unifi/site", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data()["data"][1]["name"], json!("x7k2m9qa"));
        assert_eq!(answer.data()["data"][1]["desc"], json!("Second house"));

        // A wrong password. PHP answers this with a success and no data.
        let app = installed_app_with(database, &settings("wrong", "default"));
        for (method, path) in [
            ("GET", "/homepage/unifi/data"),
            ("POST", "/test/unifi"),
            ("POST", "/test/unifi/site"),
        ] {
            let answer = call(&app, As::Admin, method, path, None).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{path}");
            assert_eq!(
                answer.message(),
                json!("Unifi response error - Check Credentials")
            );
        }

        // A site that the controller does not have, and a name that is no name.
        for site in ["other", "", "../../self/sites?"] {
            let app = installed_app_with(database, &settings(UNIFI_LOGIN.1, site));
            let answer = get(&app, "/homepage/unifi/data").await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{site}");
            assert_eq!(answer.message(), json!("Unifi response error3"));
        }
    })
    .await;
}

#[tokio::test]
async fn unifi_logs_in_to_a_console_with_unifi_os_with_its_token() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::unifi_os().await;
        // The name of the site does not count for a console.
        let settings = unifi_settings(&service.url(), UNIFI_LOGIN.1, "other");
        let app = installed_app_with(database, &settings);

        let answer = get(&app, "/homepage/unifi/data").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data()["content"]["unifi"]["data"][3]["subsystem"],
            json!("lan")
        );
        let seen = service.seen();
        let paths: Vec<&str> = seen.iter().map(|seen| seen.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "/",
                "/api/auth/login",
                "/proxy/network/api/s/default/stat/health"
            ]
        );
        assert_eq!(seen[1].header("x-csrf-token"), Some(UNIFI_TOKEN));
        assert_eq!(
            seen[1].form().get("username").map(String::as_str),
            Some(UNIFI_LOGIN.0)
        );
        assert_eq!(seen[2].header("x-csrf-token"), Some(UNIFI_TOKEN));

        let answer = call(&app, As::Admin, "POST", "/test/unifi", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        // A console has one site.
        let answer = call(&app, As::Admin, "POST", "/test/unifi/site", None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            answer.message(),
            json!("Unifi OS does not support Multi Site")
        );
    })
    .await;
}

#[tokio::test]
async fn unifi_is_refused_as_every_item_and_fails_without_its_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let secret = "made-up-secret-password";
        let mut settings = unifi_settings(NOWHERE, secret, "default");
        settings.truncate(4);
        is_refused_as_every_item(database, "/homepage/unifi/data", "unifi", &settings).await;

        let app = installed_app_with(database, &settings);
        for (method, path) in [
            ("GET", "/homepage/unifi/data"),
            ("POST", "/test/unifi"),
            ("POST", "/test/unifi/site"),
        ] {
            let answer = call(&app, As::Admin, method, path, None).await;
            assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR, "{path}");
            assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);
            assert!(!answer.body.contains(secret), "{}", answer.body);
        }

        // The tests need the level of a Co-Admin, and the settings of the login, but
        // not the switch of the item.
        let user = seed::logged_in(database, &app, "Sam", 4).await;
        for path in ["/test/unifi", "/test/unifi/site"] {
            let answer = call(&app, As::User(&user), "POST", path, None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{path}");
            let no_password = installed_app_with(database, &settings[1..3]);
            let answer = call(&no_password, As::Admin, "POST", path, None).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{path}");
            assert_eq!(answer.message(), json!("unifiPassword was not supplied"));
        }
    })
    .await;
}
