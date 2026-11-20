//! CouchPotato: the films for the calendar of the homepage.
//!
//! The function that formats an answer gives the events in the form of the PHP
//! application, which the calendar of the homepage reads.

use std::time::Duration;

use jiff::civil::{Date, Time};
use serde_json::{Value, json};

use crate::arr::{Events, NO_PICTURE, Picture};
use crate::http::{Outbound, without_address};
use crate::ical::Window;

/// How long an install has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(60);

/// Why an install gave no usable answer. The text names no address, because the
/// address holds the API key.
#[derive(Debug, thiserror::Error)]
pub enum CouchPotatoError {
    /// The install cannot be reached, or it did not answer in time.
    #[error("the application cannot be reached: {0}")]
    Unreachable(String),
    /// The install answered with a status that is no success.
    #[error("the application answered with the status {0}")]
    Status(u16),
    /// The answer is not JSON.
    #[error("the answer is not JSON")]
    Unreadable,
}

/// One install, and how it is asked.
#[derive(Debug, Clone, Copy)]
pub struct Install<'a> {
    /// The address, complete.
    pub url: &'a str,
    /// The API key.
    pub key: &'a str,
    /// Whether the certificate of the install is left unchecked.
    pub never_check_certificate: bool,
}

impl Install<'_> {
    /// The films that CouchPotato looks for, and the ones that it has. The key is a
    /// part of the path, as CouchPotato wants it.
    pub async fn films(&self, http: &Outbound) -> Result<Value, CouchPotatoError> {
        let url = format!(
            "{}/api/{}/media.list",
            self.url.trim_end_matches('/'),
            self.key
        );
        let response = http
            .client(&url, self.never_check_certificate)
            .get(&url)
            .query(&[("status", "active,done")])
            .timeout(TIME_LIMIT)
            .send()
            .await
            .map_err(|error| CouchPotatoError::Unreachable(without_address(error)))?;
        if !response.status().is_success() {
            return Err(CouchPotatoError::Status(response.status().as_u16()));
        }
        response
            .json()
            .await
            .map_err(|_| CouchPotatoError::Unreadable)
    }
}

/// A part of an answer, or `null` when the answer has no such part.
fn part<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    path.iter().fold(value, |value, name| &value[*name])
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

/// A number or a text as PHP writes it into a text.
fn written(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        _ => String::new(),
    }
}

/// A field of an entry, or the given value when the entry has none.
fn or_else(entry: &Value, field: &str, default: Value) -> Value {
    entry
        .get(field)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(default)
}

/// The first picture of a list that holds one.
fn first_picture(images: &Value, field: &str) -> Option<String> {
    let address = text(images.get(field)?.as_array()?.first()?);
    (!address.is_empty()).then(|| address.to_owned())
}

/// The events of an answer of CouchPotato: one for each film whose release is in
/// the days of the window. `number` is the place of the install in the settings,
/// from 0.
pub fn film_events(answer: &Value, number: usize, window: &Window) -> Events {
    let mut out = Events::default();
    let films = answer["movies"].as_array();
    for (index, child) in films.into_iter().flatten().enumerate() {
        let info = &child["info"];
        // The release for the home, or else the one in cinemas.
        let release = [&info["released"], part(info, &["release_date", "theater"])]
            .into_iter()
            .find(|value| !value.is_null())
            .and_then(|value| text(value).get(..10)?.parse::<Date>().ok());
        let Some(day) = release else {
            continue;
        };
        let Ok(start) = day
            .to_datetime(Time::midnight())
            .to_zoned_datetime(window.zone.clone())
        else {
            continue;
        };
        if !window.holds(start.timestamp()) {
            continue;
        }
        let state = if text(&child["status"]) != "active" {
            "text-success"
        } else if window.now < start.timestamp() {
            "text-info"
        } else {
            "text-danger"
        };

        let id = written(&info["tmdb_id"]);
        let source = first_picture(&info["images"], "backdrop_original")
            .or_else(|| first_picture(&info["images"], "backdrop"));
        let picture = match source {
            Some(source) if !id.is_empty() => {
                let name = format!("movie-{id}");
                let path = format!("data/cache/{name}.jpg");
                out.pictures.push(Picture { name, source });
                path
            }
            _ => NO_PICTURE.to_owned(),
        };
        let release_of_file = child["releases"].get(0).unwrap_or(&Value::Null);
        let has_file = match part(release_of_file, &["files", "movie"]) {
            Value::Array(list) => !list.is_empty(),
            Value::String(file) => !file.is_empty(),
            _ => false,
        };
        let name = &info["original_title"];
        // The PHP application sends the year as the studio too.
        let year = or_else(info, "year", json!(""));
        let details = json!({
            "topTitle": name,
            "bottomTitle": info["tagline"],
            "status": child["status"],
            "overview": info["plot"],
            "runtime": info["runtime"],
            "image": picture,
            "ratings": part(info, &["rating", "imdb"]).get(0).cloned().unwrap_or_else(|| json!("")),
            "videoQuality": if has_file { release_of_file["quality"].clone() } else { json!("unknown") },
            "audioChannels": "",
            "audioCodec": "",
            "videoCodec": "",
            "genres": info["genres"],
            "year": year,
            "studio": year,
        });
        out.events.push(json!({
            "id": format!("CouchPotato-{number}-{}", index + 1),
            "title": name,
            "start": day.strftime("%Y-%m-%d").to_string(),
            "className": format!("inline-popups bg-calendar calendar-item movieID--{id}"),
            "imagetype": format!("film {state}"),
            "imagetypeFilter": "film",
            "downloadFilter": state,
            "bgColor": state.replace("text", "bg"),
            "details": details,
        }));
    }
    out
}

#[cfg(test)]
mod tests {
    use jiff::tz::TimeZone;
    use serde_json::json;

    use super::{Picture, Window, film_events};

    fn window() -> Window {
        Window {
            now: "2020-01-15T12:00:00Z".parse().expect("a moment"),
            zone: TimeZone::UTC,
            days_before: 14,
            days_after: 14,
        }
    }

    fn film(title: &str, status: &str, released: &str) -> serde_json::Value {
        json!({
            "status": status,
            "info": {
                "original_title": title, "tmdb_id": 815, "released": released,
                "tagline": "A tagline.", "plot": "A film.", "runtime": 101, "year": 2020,
                "genres": ["Drama"], "rating": {"imdb": [7.2, 1000]},
                "images": {"backdrop_original": ["https://art.example.net/film.jpg"], "backdrop": []},
            },
            "releases": [{"quality": "1080p", "files": {"movie": ["/films/film.mkv"]}}],
        })
    }

    #[test]
    fn film_is_an_event_in_the_form_of_the_php_application() {
        let answer = json!({"movies": [film("The Film", "done", "2020-01-07")], "success": true});

        let found = film_events(&answer, 1, &window());

        assert_eq!(
            found.events,
            [json!({
                "id": "CouchPotato-1-1",
                "title": "The Film",
                "start": "2020-01-07",
                "className": "inline-popups bg-calendar calendar-item movieID--815",
                "imagetype": "film text-success",
                "imagetypeFilter": "film",
                "downloadFilter": "text-success",
                "bgColor": "bg-success",
                "details": {
                    "topTitle": "The Film",
                    "bottomTitle": "A tagline.",
                    "status": "done",
                    "overview": "A film.",
                    "runtime": 101,
                    "image": "data/cache/movie-815.jpg",
                    "ratings": 7.2,
                    "videoQuality": "1080p",
                    "audioChannels": "",
                    "audioCodec": "",
                    "videoCodec": "",
                    "genres": ["Drama"],
                    "year": 2020,
                    "studio": 2020,
                },
            })]
        );
        assert_eq!(
            found.pictures,
            [Picture {
                name: "movie-815".to_owned(),
                source: "https://art.example.net/film.jpg".to_owned(),
            }]
        );
    }

    #[test]
    fn state_of_a_film_follows_its_status_and_its_release() {
        let mut later = film("Later", "active", "2020-01-20");
        later["releases"] = json!([]);
        later["info"]["images"] = json!({});
        let mut cinema = film("In Cinemas", "active", "");
        cinema["info"]["released"] = json!(null);
        cinema["info"]["release_date"] = json!({"theater": "2020-01-10"});
        let answer = json!({"movies": [
            later,
            film("Too Old", "done", "2019-11-01"),
            cinema,
            film("No Date", "active", "soon"),
        ]});

        let events = film_events(&answer, 0, &window()).events;

        let ids: Vec<_> = events.iter().map(|event| event["id"].clone()).collect();
        // The count goes on over the films that are left out.
        assert_eq!(ids, [json!("CouchPotato-0-1"), json!("CouchPotato-0-3")]);
        assert_eq!(events[0]["downloadFilter"], "text-info");
        assert_eq!(events[0]["details"]["videoQuality"], "unknown");
        assert_eq!(
            events[0]["details"]["image"],
            "/plugins/images/homepage/no-np.png"
        );
        assert_eq!(events[1]["downloadFilter"], "text-danger");
        assert_eq!(events[1]["start"], "2020-01-10");
        let none = film_events(&json!({"success": false}), 0, &window());
        assert_eq!(none.events, Vec::<serde_json::Value>::new());
    }
}
