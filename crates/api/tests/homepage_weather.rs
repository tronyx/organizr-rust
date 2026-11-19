//! The weather and air item of the homepage, against a stand-in for BreezoMeter.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_api::router;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, WEATHER_KEY};
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_settings, seed, state_with};
use serde_json::{Value, json};

fn app(database: &Database, api: &str, changed: &[(&'static str, Value)]) -> Router {
    let mut settings = vec![
        ("homepageWeatherAndAirEnabled", json!(true)),
        ("homepageWeatherAndAirAuth", json!("999")),
        ("homepageWeatherAndAirLatitude", json!("52.52")),
        ("homepageWeatherAndAirLongitude", json!("-13.4")),
        ("homepageWeatherAndAirUnits", json!("metric")),
        ("breezometerToken", json!(WEATHER_KEY)),
    ];
    settings.extend(changed.iter().cloned());
    let state = state_with(database, installed_settings(&settings)).with_breezometer(api);
    router(state).expect("the route table is valid")
}

const PATH: &str = "/homepage/weather/data";

#[tokio::test]
async fn weather_air_and_pollen_come_for_the_kinds_that_are_switched_on() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::breezometer().await;

        // No kind is switched on: the service is not asked.
        let none = app(database, &service.url(), &[]);
        let answer = call(&none, As::Guest, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data(),
            json!({"content": {"weather": false, "air": false, "pollen": false}})
        );
        assert_eq!(service.seen().len(), 0);

        let all = app(
            database,
            &service.url(),
            &[
                ("homepageWeatherAndAirWeatherEnabled", json!(true)),
                ("homepageWeatherAndAirAirQualityEnabled", json!(true)),
                ("homepageWeatherAndAirPollenEnabled", json!(true)),
            ],
        );
        let answer = call(&all, As::Guest, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let content = &answer.data()["content"];
        assert_eq!(content["weather"]["data"][0]["weather_text"], "Partly cloudy");
        assert_eq!(content["weather"]["error"], Value::Null);
        assert_eq!(content["air"]["data"]["indexes"]["baqi"]["aqi"], 67);
        assert_eq!(content["pollen"]["data"][0]["types"]["tree"]["index"]["value"], 3);

        let asked: Vec<String> = service.seen().into_iter().map(|seen| seen.path).collect();
        let place = format!("&lat=52.52&lon=-13.4&units=metric&key={WEATHER_KEY}");
        assert_eq!(
            asked,
            [
                format!("/weather/v1/forecast/hourly?hours=120&metadata=true{place}"),
                format!(
                    "/air-quality/v2/current-conditions?features=breezometer_aqi%2Clocal_aqi%2Chealth_recommendations%2Csources_and_effects%2Cdominant_pollutant_concentrations%2Cpollutants_concentrations%2Cpollutants_aqi_information&metadata=true{place}"
                ),
                format!(
                    "/pollen/v2/forecast/daily?features=plants_information%2Ctypes_information&days=1&metadata=true{place}"
                ),
            ]
        );
        assert_eq!(
            call(&all, As::Guest, "GET", "/homepage/items", None).await.data(),
            json!([{"key": "homepageOrderWeatherAndAir", "kind": "weather", "id": ""}])
        );
    })
    .await;
}

#[tokio::test]
async fn weather_that_the_service_refuses_is_false_and_names_no_key() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::breezometer().await;
        let weather = ("homepageWeatherAndAirWeatherEnabled", json!(true));

        let wrong = app(
            database,
            &service.url(),
            &[weather.clone(), ("breezometerToken", json!("a-wrong-key"))],
        );
        let answer = call(&wrong, As::Admin, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data()["content"]["weather"], json!(false));

        // Nothing listens at this address.
        let away = app(database, "http://127.0.0.1:1", &[weather]);
        let answer = call(&away, As::Admin, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!answer.body.contains(WEATHER_KEY), "{}", answer.body);
        assert!(!answer.body.contains("key="), "{}", answer.body);

        // Without a place, the service is not asked.
        let nowhere = app(
            database,
            &service.url(),
            &[("homepageWeatherAndAirLongitude", json!(""))],
        );
        let refused = call(&nowhere, As::Admin, "GET", PATH, None).await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            refused.message(),
            json!("homepageWeatherAndAirLongitude was not supplied")
        );
    })
    .await;
}
