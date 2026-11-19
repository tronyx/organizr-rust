//! Weather, air quality and pollen from the API of BreezoMeter.

use std::time::Duration;

use serde_json::Value;

use crate::http::Outbound;

/// The address of the API.
pub const BREEZOMETER: &str = "https://api.breezometer.com";

/// How long the API has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// The API cannot be reached, or it did not answer in time. The text names no address,
/// because the address holds the API key.
#[derive(Debug, thiserror::Error)]
#[error("the weather service cannot be reached: {0}")]
pub struct WeatherError(String);

/// One of the three kinds of data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The weather of the next 120 hours.
    Weather,
    /// The air quality at this moment.
    Air,
    /// The pollen of today.
    Pollen,
}

impl Part {
    const fn path(self) -> &'static str {
        match self {
            Self::Weather => "/weather/v1/forecast/hourly",
            Self::Air => "/air-quality/v2/current-conditions",
            Self::Pollen => "/pollen/v2/forecast/daily",
        }
    }

    const fn fields(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Weather => &[("hours", "120"), ("metadata", "true")],
            Self::Air => &[
                (
                    "features",
                    "breezometer_aqi,local_aqi,health_recommendations,sources_and_effects,dominant_pollutant_concentrations,pollutants_concentrations,pollutants_aqi_information",
                ),
                ("metadata", "true"),
            ],
            Self::Pollen => &[
                ("features", "plants_information,types_information"),
                ("days", "1"),
                ("metadata", "true"),
            ],
        }
    }
}

/// A place, and how its data is asked for.
#[derive(Debug, Clone, Copy)]
pub struct Place<'a> {
    /// The address of the API: [`BREEZOMETER`], or a stand-in for a test.
    pub api: &'a str,
    /// The latitude, as text.
    pub latitude: &'a str,
    /// The longitude, as text.
    pub longitude: &'a str,
    /// `imperial` or `metric`.
    pub units: &'a str,
    /// The API key.
    pub key: &'a str,
    /// How long the API has to answer.
    pub timeout: Option<Duration>,
}

/// One kind of data for a place, as the API sent it. `false` when the API refused the
/// request or named an error in its answer, as in the PHP application.
pub async fn part(http: &Outbound, place: Place<'_>, part: Part) -> Result<Value, WeatherError> {
    let url = format!("{}{}", place.api.trim_end_matches('/'), part.path());
    let mut query = part.fields().to_vec();
    query.extend([
        ("lat", place.latitude),
        ("lon", place.longitude),
        ("units", place.units),
        ("key", place.key),
    ]);
    let response = http
        .client(&url, false)
        .get(&url)
        .query(&query)
        .timeout(place.timeout.unwrap_or(TIME_LIMIT))
        .send()
        .await
        .map_err(|error| WeatherError(error.without_url().to_string()))?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), ?part, "the weather service refused the request");
        return Ok(Value::Bool(false));
    }
    match response.json::<Value>().await {
        Ok(body) if body.get("error").is_none_or(Value::is_null) => Ok(body),
        _ => Ok(Value::Bool(false)),
    }
}
