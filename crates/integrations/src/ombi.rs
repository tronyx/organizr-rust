//! Ombi: the requests for films and series, for the homepage.

use std::time::Duration;

use reqwest::{Method, header};
use serde_json::{Map, Value, json};

use crate::http::{Outbound, without_address};

/// How long Ombi has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// The picture of a request that has none.
const NO_POSTER: &str = "plugins/images/homepage/no-list.png";

/// Ombi cannot be reached, or it did not answer in time.
#[derive(Debug, thiserror::Error)]
#[error("Ombi cannot be reached: {0}")]
pub struct OmbiError(String);

/// What Ombi answered.
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    /// Whether the status is a success.
    pub is_success: bool,
    /// The body as JSON, or `null` when it is none.
    pub body: Value,
}

/// An install of Ombi, and how it is asked.
#[derive(Debug, Clone)]
pub struct Install {
    /// The address, complete.
    pub url: String,
    /// The API key.
    pub key: String,
    /// How long the install has to answer.
    pub timeout: Option<Duration>,
    /// Whether the certificate of the install is left unchecked.
    pub never_check_certificate: bool,
}

/// Who a request to Ombi comes from.
#[derive(Debug, Clone, Copy)]
pub enum As<'a> {
    /// The install of Organizr, with its API key.
    Application,
    /// A user, with the token of the `Auth` cookie that single sign-on gave the
    /// browser.
    User(&'a str),
}

impl Install {
    /// Sends a request to a path under `/api`, with a JSON body when one is given.
    pub async fn send(
        &self,
        http: &Outbound,
        method: Method,
        path: &str,
        body: Option<&Value>,
        who: As<'_>,
    ) -> Result<Reply, OmbiError> {
        let url = format!("{}/api/{path}", self.url.trim_end_matches('/'));
        let mut request = http
            .keyed_client(&url, self.never_check_certificate)
            .request(method, &url)
            .header(header::ACCEPT, "application/json")
            .timeout(self.timeout.unwrap_or(TIME_LIMIT));
        request = match who {
            As::Application => request.header("ApiKey", &self.key),
            As::User(token) => request.bearer_auth(token),
        };
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request
            .send()
            .await
            .map_err(|error| OmbiError(without_address(error)))?;
        let is_success = response.status().is_success();
        let body = response.json().await.unwrap_or(Value::Null);
        Ok(Reply { is_success, body })
    }

    /// Reads a path under `/api` with the API key.
    pub async fn get(&self, http: &Outbound, path: &str) -> Result<Reply, OmbiError> {
        self.send(http, Method::GET, path, None, As::Application)
            .await
    }
}

/// Which requests a caller sees.
#[derive(Debug, Clone, Copy)]
pub struct Viewer<'a> {
    /// Whether the caller sees the requests of everybody.
    pub sees_all: bool,
    /// The user name of the caller. A guest has none.
    pub name: &'a str,
    /// The user whose requests everybody sees: the fallback user of single sign-on.
    pub fallback: &'a str,
}

impl Viewer<'_> {
    fn sees(&self, requester: &str) -> bool {
        let is = |name: &str| !name.is_empty() && requester.eq_ignore_ascii_case(name);
        self.sees_all || is(self.name) || is(self.fallback)
    }
}

fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or_default()
}

fn field(value: &Value, name: &str) -> Value {
    value.get(name).cloned().unwrap_or(Value::Null)
}

/// The address of the wide picture of a request, or an empty text without one.
fn background(entry: &Value) -> String {
    match text(entry, "background") {
        "" => String::new(),
        path => format!("https://image.tmdb.org/t/p/w1280/{path}"),
    }
}

/// The fields of a row that a film and a series share. `state` is the part of the
/// answer that holds the state and the user: the film itself, or the first child
/// request of a series.
fn row(entry: &Value, state: &Value, id: &str, poster: &str) -> Map<String, Value> {
    let user = state.get("requestedUser").unwrap_or(&Value::Null);
    let mut row = Map::new();
    for (key, value) in [
        ("id", field(entry, id)),
        ("title", field(entry, "title")),
        ("overview", field(entry, "overview")),
        ("poster", json!(poster)),
        ("background", json!(background(entry))),
        ("approved", field(state, "approved")),
        ("available", field(state, "available")),
        ("denied", field(state, "denied")),
        ("deniedReason", field(state, "deniedReason")),
        ("user", field(user, "userName")),
        ("userAlias", field(user, "userAlias")),
        ("request_id", field(entry, "id")),
        ("request_date", field(state, "requestedDate")),
        ("release_date", field(entry, "releaseDate")),
    ] {
        row.insert(key.to_owned(), value);
    }
    row
}

fn finish(mut row: Map<String, Value>, kind: &str, icon: &str, color: &str) -> Value {
    row.insert("type".to_owned(), json!(kind));
    row.insert("icon".to_owned(), json!(icon));
    row.insert("color".to_owned(), json!(color));
    Value::Object(row)
}

/// The requests for films of an answer of Ombi that the viewer sees, in the form of
/// the PHP application.
pub fn film_rows(answer: &Value, viewer: Viewer<'_>) -> Vec<Value> {
    let entries = answer.as_array().map_or(&[][..], Vec::as_slice);
    entries
        .iter()
        .filter(|entry| viewer.sees(text(&entry["requestedUser"], "userName")))
        .map(|entry| {
            let poster = match text(entry, "posterPath") {
                "" => NO_POSTER.to_owned(),
                path => format!("https://image.tmdb.org/t/p/w300/{path}"),
            };
            finish(
                row(entry, entry, "theMovieDbId", &poster),
                "movie",
                "mdi mdi-filmstrip",
                "palette-Deep-Purple-900 bg white",
            )
        })
        .collect()
}

/// The requests for series of an answer of Ombi that the viewer sees. The state and
/// the user of a series are the ones of its first child request, and a series
/// without a child request is left out.
pub fn series_rows(answer: &Value, viewer: Viewer<'_>) -> Vec<Value> {
    let entries = answer.as_array().map_or(&[][..], Vec::as_slice);
    entries
        .iter()
        .filter_map(|entry| {
            let child = entry.get("childRequests")?.as_array()?.first()?;
            if !viewer.sees(text(&child["requestedUser"], "userName")) {
                return None;
            }
            let poster = match text(entry, "posterPath") {
                "" => NO_POSTER.to_owned(),
                path if path.starts_with('/') => format!("https://image.tmdb.org/t/p/w300/{path}"),
                address => address.to_owned(),
            };
            Some(finish(
                row(entry, child, "tvDbId", &poster),
                "tv",
                "mdi mdi-television",
                "grayish-blue-bg",
            ))
        })
        .collect()
}

/// Puts the newest request first. Requests of the same moment keep their order.
pub fn newest_first(rows: &mut [Value]) {
    rows.sort_by(|a, b| text(b, "request_date").cmp(text(a, "request_date")));
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Viewer, film_rows, newest_first, series_rows};

    const ALL: Viewer<'static> = Viewer {
        sees_all: true,
        name: "",
        fallback: "",
    };

    fn film(user: &str) -> serde_json::Value {
        json!({
            "id": 31, "theMovieDbId": 815, "title": "The Film", "overview": "A film.",
            "posterPath": "/poster815.jpg", "background": "/back815.jpg",
            "approved": true, "available": false, "denied": false, "deniedReason": null,
            "requestedUser": {"userName": user, "userAlias": "The Alias"},
            "requestedDate": "2020-01-03T10:00:00", "releaseDate": "2019-05-01T00:00:00",
        })
    }

    #[test]
    fn film_has_the_form_of_the_php_application() {
        let rows = film_rows(&json!([film("robin")]), ALL);

        assert_eq!(
            rows,
            [json!({
                "id": 815,
                "title": "The Film",
                "overview": "A film.",
                "poster": "https://image.tmdb.org/t/p/w300//poster815.jpg",
                "background": "https://image.tmdb.org/t/p/w1280//back815.jpg",
                "approved": true,
                "available": false,
                "denied": false,
                "deniedReason": null,
                "user": "robin",
                "userAlias": "The Alias",
                "request_id": 31,
                "request_date": "2020-01-03T10:00:00",
                "release_date": "2019-05-01T00:00:00",
                "type": "movie",
                "icon": "mdi mdi-filmstrip",
                "color": "palette-Deep-Purple-900 bg white",
            })]
        );
    }

    #[test]
    fn series_takes_its_state_from_the_first_child_request() {
        let series = json!([
            {
                "id": 32, "tvDbId": 4711, "title": "The Show", "overview": "A show.",
                "posterPath": "https://art.example.net/show.jpg", "releaseDate": "2018-09-01T00:00:00",
                "childRequests": [{
                    "approved": false, "available": false, "denied": true, "deniedReason": "No",
                    "requestedUser": {"userName": "pat", "userAlias": null},
                    "requestedDate": "2020-01-05T10:00:00",
                }],
            },
            {"id": 33, "tvDbId": 4712, "title": "No Children", "childRequests": []},
        ]);

        let rows = series_rows(&series, ALL);

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row["id"], 4711);
        assert_eq!(row["denied"], true);
        assert_eq!(row["deniedReason"], "No");
        assert_eq!(row["user"], "pat");
        assert_eq!(row["request_date"], "2020-01-05T10:00:00");
        // A poster that is an address stays as it is.
        assert_eq!(row["poster"], "https://art.example.net/show.jpg");
        assert_eq!(row["background"], "");
        assert_eq!(row["type"], "tv");
        assert_eq!(row["icon"], "mdi mdi-television");
        // An answer that is no list has no rows.
        assert_eq!(
            series_rows(&json!({"message": "no"}), ALL),
            Vec::<serde_json::Value>::new()
        );
    }

    #[test]
    fn limited_viewer_sees_the_own_requests_and_the_ones_of_the_fallback_user() {
        let films = json!([film("Robin")]);
        let seen = |sees_all, name, fallback| {
            !film_rows(
                &films,
                Viewer {
                    sees_all,
                    name,
                    fallback,
                },
            )
            .is_empty()
        };

        assert!(seen(true, "", ""));
        assert!(seen(false, "robin", ""));
        assert!(seen(false, "zed", "ROBIN"));
        assert!(!seen(false, "zed", "pat"));
        // A guest has no name, and the name of nobody is nobody.
        let nobody = Viewer {
            sees_all: false,
            name: "",
            fallback: "",
        };
        assert_eq!(
            film_rows(&json!([film("")]), nobody),
            Vec::<serde_json::Value>::new()
        );
    }

    #[test]
    fn newest_request_comes_first() {
        let mut rows = vec![
            json!({"request_id": 1, "request_date": "2020-01-03T10:00:00"}),
            json!({"request_id": 2, "request_date": "2020-01-05T10:00:00"}),
            json!({"request_id": 3, "request_date": "2020-01-03T10:00:00"}),
        ];

        newest_first(&mut rows);

        let ids: Vec<_> = rows.iter().map(|row| row["request_id"].clone()).collect();
        assert_eq!(ids, [json!(2), json!(1), json!(3)]);
    }
}
