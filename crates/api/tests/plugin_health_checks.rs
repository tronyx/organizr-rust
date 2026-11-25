//! The health checks plugin, against stand-ins for the services and for Healthchecks.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call};
use organizr_testkit::services::StandIn;
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const PATH: &str = "/plugins/healthchecks/run";

/// A service behind the forward auth of Organizr: it answers only a request that
/// carries the API key of the install.
async fn guarded(headers: HeaderMap) -> StatusCode {
    let key = headers.get("token").and_then(|value| value.to_str().ok());
    if key == Some(seed::API_KEY) {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    }
}

async fn services() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/up", get(|| async { StatusCode::NO_CONTENT }))
            .route("/guarded", get(guarded))
            .route("/down", get(|| async { StatusCode::INTERNAL_SERVER_ERROR }))
            .route("/refuses", get(|| async { StatusCode::FORBIDDEN })),
    )
    .await
}

/// A stand-in for the ping address of Healthchecks. It takes every ping.
async fn healthchecks() -> StandIn {
    StandIn::start(Router::new().fallback(|| async { "OK" })).await
}

fn item(name: &str, id: &str, internal: &str, external: &str, enabled: bool) -> Value {
    json!([
        {"type": "input", "label": "Service Name", "value": name},
        {"type": "input", "label": "UUID", "value": id},
        {"type": "input", "label": "External URL", "value": external},
        {"type": "input", "label": "Internal URL", "value": internal},
        {"type": "switch", "label": "Enabled", "value": enabled},
    ])
}

#[tokio::test]
async fn run_asks_each_service_and_pings_healthchecks_with_the_result() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (service, pings) = (services().await, healthchecks().await);
        let at = |path: &str| format!("{}{path}", service.url());
        let items = json!({
            // Inside the network it answers, so the address outside is not asked.
            "0": item("Both", "id-both", &at("/up"), &at("/down"), true),
            // Inside it is down, and outside it answers with the key of Organizr.
            "1": item("Guarded", "id-guarded", &at("/down"), &at("/guarded"), true),
            "2": item("Down", "id-down", "", &at("/down"), true),
            "3": item("Refuses", "id-refuses", &at("/refuses"), "", true),
            "4": item("Off", "id-off", &at("/up"), "", false),
            "5": item("No address", "id-none", "", "", true),
        });
        let settings = |more: &[(&'static str, Value)]| {
            let mut settings = vec![
                ("HEALTHCHECKS-enabled", json!(true)),
                ("HEALTHCHECKS-all-items", items.clone()),
                ("HEALTHCHECKS-PingURL", json!(format!("{}/", pings.url()))),
            ];
            settings.extend(more.iter().cloned());
            settings
        };
        let app = installed_app_with(database, &settings(&[]));

        let answer = call(&app, As::Admin, "GET", PATH, None).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(
            data["0"]["results"],
            json!({"internal": "Success", "external": "Not needed"})
        );
        assert_eq!(
            data["1"]["results"],
            json!({"internal": "Error", "external": "Success"})
        );
        assert_eq!(data["2"]["results"], json!({"external": "Error"}));
        assert_eq!(data["3"]["results"], json!({"internal": "Error"}));
        assert_eq!(data["0"]["Service Name"], "Both");
        // The service that is off is not in the answer. The one without an address
        // is, without results.
        assert_eq!(data.get("4"), None);
        assert_eq!(data["5"].get("results"), None);

        // An install of one's own gets its pings under `/ping`.
        let pinged: Vec<String> = pings.seen().into_iter().map(|seen| seen.path).collect();
        assert_eq!(
            pinged,
            [
                "/ping/id-both/start",
                "/ping/id-both",
                "/ping/id-guarded/start",
                "/ping/id-guarded",
                "/ping/id-down/start",
                "/ping/id-down/fail",
                "/ping/id-refuses/start",
                "/ping/id-refuses/fail",
                // No address: no start, and the check fails.
                "/ping/id-none/fail",
            ]
        );
        let asked: Vec<String> = service.seen().into_iter().map(|seen| seen.path).collect();
        assert_eq!(asked, ["/up", "/down", "/guarded", "/down", "/refuses"]);

        // With the setting, a refusal counts as an answer.
        let loose = installed_app_with(
            database,
            &settings(&[("HEALTHCHECKS-403-enabled", json!(true))]),
        );
        let answer = call(&loose, As::Admin, "GET", PATH, None).await;
        assert_eq!(
            answer.data()["3"]["results"],
            json!({"internal": "Success"})
        );
    })
    .await;
}

/// A service with the switch "Send API Key", which a service of PHP Organizr does not have.
fn item_with_key_switch(name: &str, id: &str, internal: &str, sends_key: &Value) -> Value {
    let mut fields = item(name, id, internal, "", true);
    fields
        .as_array_mut()
        .expect("the fields are a list")
        .push(json!({"type": "switch", "label": "Send API Key", "value": sends_key}));
    fields
}

#[tokio::test]
async fn api_key_goes_only_to_the_services_that_have_it_switched_on() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (service, pings) = (services().await, healthchecks().await);
        let at = |path: &str| format!("{}{path}", service.url());
        let items = json!({
            // A service of PHP Organizr has no switch: it gets the key, as before.
            "0": item("Old", "id-old", &at("/guarded"), "", true),
            "1": item_with_key_switch("On", "id-on", &at("/guarded"), &json!(true)),
            "2": item_with_key_switch("Off", "id-off", &at("/guarded"), &json!(false)),
            "3": item_with_key_switch("Open", "id-open", &at("/up"), &json!(false)),
        });
        let app = installed_app_with(
            database,
            &[
                ("HEALTHCHECKS-enabled", json!(true)),
                ("HEALTHCHECKS-all-items", items),
                ("HEALTHCHECKS-PingURL", json!(format!("{}/", pings.url()))),
            ],
        );

        let answer = call(&app, As::Admin, "GET", PATH, None).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(data["0"]["results"], json!({"internal": "Success"}));
        assert_eq!(data["1"]["results"], json!({"internal": "Success"}));
        // Without the key, the guarded service refuses the request.
        assert_eq!(data["2"]["results"], json!({"internal": "Error"}));
        assert_eq!(data["3"]["results"], json!({"internal": "Success"}));
        let tokens: Vec<Option<String>> = service
            .seen()
            .into_iter()
            .map(|seen| seen.headers.get("token").cloned())
            .collect();
        let key = Some(seed::API_KEY.to_owned());
        assert_eq!(tokens, [key.clone(), key, None, None]);
    })
    .await;
}

#[tokio::test]
async fn run_is_for_the_level_of_the_plugin_and_needs_the_plugin_to_be_ready() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let pings = healthchecks().await;
        let items = json!({"0": item("One", "id-one", "", "", true)});
        let ready = vec![
            ("HEALTHCHECKS-enabled", json!(true)),
            ("HEALTHCHECKS-all-items", items),
            ("HEALTHCHECKS-PingURL", json!(pings.url())),
        ];
        let app = installed_app_with(database, &ready);
        let user = seed::logged_in(database, &app, "zed", 4).await;

        // The level of the plugin is 1: Admins and Co-Admins.
        for who in [As::Guest, As::User(&user)] {
            let refused = call(&app, who, "GET", PATH, None).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        }
        assert_eq!(pings.seen().len(), 0);

        let mut open = ready.clone();
        open.push(("HEALTHCHECKS-Auth-include", json!("4")));
        let app = installed_app_with(database, &open);
        let passed = call(&app, As::User(&user), "GET", PATH, None).await;
        assert_eq!(passed.status, StatusCode::OK, "{}", passed.body);

        // Off, without services, or without a ping address: the answer of PHP.
        for changed in [
            ("HEALTHCHECKS-enabled", json!(false)),
            ("HEALTHCHECKS-all-items", json!("")),
            ("HEALTHCHECKS-PingURL", json!("")),
        ] {
            let mut settings = ready.clone();
            settings.push(changed.clone());
            let app = installed_app_with(database, &settings);
            let refused = call(&app, As::Admin, "GET", PATH, None).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{changed:?}");
            assert_eq!(refused.message(), json!("User does not have access"));
        }
    })
    .await;
}
