//! The weather and air item: weather, air quality and pollen for one place.

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use organizr_integrations::http::time_limit;
use organizr_integrations::weather::{self, Part, Place};
use serde_json::{Value, json};

use super::{Permissions, no_requests};
use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// What must hold before a caller gets the item.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageWeatherAndAirEnabled".to_owned()],
        auth: vec!["homepageWeatherAndAirAuth".to_owned()],
        not_empty: vec![
            "homepageWeatherAndAirLatitude".to_owned(),
            "homepageWeatherAndAirLongitude".to_owned(),
        ],
    }
}

/// `GET /homepage/weather/data`: the kinds of data that are switched on, each as the
/// API of BreezoMeter gives it, or `false`.
pub async fn data(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = state.outbound().ok_or_else(no_requests)?;
    let place = Place {
        api: state.breezometer(),
        latitude: &settings.homepage_weather_and_air_latitude,
        longitude: &settings.homepage_weather_and_air_longitude,
        units: &settings.homepage_weather_and_air_units,
        key: &settings.breezometer_token,
        timeout: time_limit(&settings.homepage_weather_and_air_refresh.to_string()),
    };

    let mut content = [Value::Bool(false), Value::Bool(false), Value::Bool(false)];
    let wanted = [
        (
            Part::Weather,
            settings.homepage_weather_and_air_weather_enabled,
        ),
        (
            Part::Air,
            settings.homepage_weather_and_air_air_quality_enabled,
        ),
        (
            Part::Pollen,
            settings.homepage_weather_and_air_pollen_enabled,
        ),
    ];
    for (slot, (part, is_on)) in content.iter_mut().zip(wanted) {
        if is_on {
            *slot = weather::part(http, place, part).await.map_err(|error| {
                tracing::warn!(%error, "the weather could not be loaded");
                Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
            })?;
        }
    }
    let [weather, air, pollen] = content;
    Ok(Envelope::success(json!({
        "content": { "weather": weather, "air": air, "pollen": pollen },
    })))
}
