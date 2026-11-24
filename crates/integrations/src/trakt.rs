//! Trakt: the episodes and the films of the calendar of an account, and the OAuth
//! requests that give Organizr the tokens of that account.
//!
//! The functions that format an answer give the events in the form of the PHP
//! application, which the calendar of the homepage reads.

use std::time::Duration;

use jiff::Timestamp;
use serde_json::{Value, json};

use crate::arr::NO_PICTURE;
use crate::http::{Outbound, escaped, without_address};

/// The address of the API of Trakt.
pub const TRAKT_API: &str = "https://api.trakt.tv";

/// The address of the pages of Trakt, where a person allows the access.
pub const TRAKT_SITE: &str = "https://trakt.tv";

/// How long Trakt has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// How long new tokens are good for when Trakt does not say: the 90 days of the PHP
/// application.
const DEFAULT_LIFETIME: i64 = 90 * 24 * 60 * 60;

/// Why Trakt gave no usable answer. The text names no token and no secret.
#[derive(Debug, thiserror::Error)]
pub enum TraktError {
    /// Trakt cannot be reached, or it did not answer in time.
    #[error("Trakt cannot be reached: {0}")]
    Unreachable(String),
    /// Trakt refused the request for tokens.
    #[error("Trakt refused the request for tokens: {0}")]
    Refused(String),
    /// The answer holds no tokens.
    #[error("the answer of Trakt holds no tokens")]
    Unreadable,
}

/// An account of Trakt, and how its calendar is asked for.
#[derive(Debug, Clone, Copy)]
pub struct Account<'a> {
    /// The address of the API: [`TRAKT_API`], or a stand-in for a test.
    pub api: &'a str,
    /// The client id of the API application of the Admin.
    pub client_id: &'a str,
    /// The access token of the account.
    pub access_token: &'a str,
    /// How long Trakt has to answer.
    pub timeout: Option<Duration>,
}

/// Which of the two calendars of an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Calendar {
    /// The episodes of the series of the account.
    Shows,
    /// The films of the account.
    Movies,
}

impl Account<'_> {
    /// One calendar of the account: `days` days from the day `start` on. Nothing when
    /// Trakt refused the request, for example for a token that is no longer good.
    pub async fn calendar(
        &self,
        http: &Outbound,
        calendar: Calendar,
        start: &str,
        days: u32,
    ) -> Result<Option<Value>, TraktError> {
        let kind = match calendar {
            Calendar::Shows => "shows",
            Calendar::Movies => "movies",
        };
        let url = format!(
            "{}/calendars/my/{kind}/{start}/{days}",
            self.api.trim_end_matches('/')
        );
        let response = http
            .client(&url, false)
            .get(&url)
            .query(&[("extended", "full")])
            .header("Content-Type", "application/json")
            .bearer_auth(self.access_token)
            .header("trakt-api-version", "2")
            .header("trakt-api-key", self.client_id)
            .timeout(self.timeout.unwrap_or(TIME_LIMIT))
            .send()
            .await
            .map_err(|error| TraktError::Unreachable(without_address(error)))?;
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), kind, "Trakt refused the request for a calendar");
            return Ok(None);
        }
        Ok(Some(response.json().await.unwrap_or(Value::Null)))
    }
}

/// A value as text: a text as it is, a number in digits, and nothing else.
fn written(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        _ => String::new(),
    }
}

/// A field, or an empty text when the answer does not have it.
fn or_empty(value: &Value) -> Value {
    if value.is_null() {
        json!("")
    } else {
        value.clone()
    }
}

/// A whole number of a field. A field without one counts as 0.
fn whole(value: &Value) -> i64 {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
        .unwrap_or(0)
}

/// The class that says how the calendar colors an event.
fn background(state: &str) -> String {
    state.replace("text", "bg")
}

/// The episodes of an answer as events of the calendar. Trakt knows nothing of
/// downloads, so an episode has the state of one that is not monitored when it was
/// not aired yet, and the state "missing" after that, as in the PHP application.
pub fn show_events(entries: &Value, now: Timestamp) -> Vec<Value> {
    let entries = entries.as_array();
    entries
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, child)| {
            let aired: Timestamp = child["first_aired"].as_str()?.parse().ok()?;
            let (show, episode) = (&child["show"], &child["episode"]);
            let is_premiere = whole(&episode["number"]) == 1;
            let state = match (now < aired, is_premiere) {
                (true, true) => "text-primary animated flash",
                (true, false) => "text-dark",
                (false, _) => "text-danger",
            };
            // The PHP application moves every episode but the first one of a season by
            // one second, so the first one stands in front on its day.
            let start = if is_premiere {
                child["first_aired"].clone()
            } else {
                let moved = aired
                    .checked_add(jiff::Span::new().seconds(1))
                    .unwrap_or(aired);
                json!(moved.strftime("%Y-%m-%dT%H:%M:%SZ").to_string())
            };
            let title = written(&show["title"]);
            Some(json!({
                "id": format!("Trakt-Tv-{}", index + 1),
                "title": title,
                "start": start,
                "className": format!(
                    "inline-popups bg-calendar calendar-item get-tmdb-image tmdb-tv tmdbID--{}",
                    written(show.pointer("/ids/tmdb").unwrap_or(&Value::Null))
                ),
                "imagetype": format!("tv {state}"),
                "imagetypeFilter": "tv",
                "downloadFilter": state,
                "bgColor": background(state),
                "details": {
                    "seasonCount": episode["season"],
                    "status": "dunno",
                    "topTitle": title,
                    "bottomTitle": format!(
                        "S{:02}E{:02} - {}",
                        whole(&episode["season"]),
                        whole(&episode["number"]),
                        written(&episode["title"])
                    ),
                    "overview": or_empty(&episode["overview"]),
                    "runtime": or_empty(&episode["runtime"]),
                    "image": NO_PICTURE,
                    "ratings": or_empty(&show["rating"]),
                    "videoQuality": "unknown",
                    "audioChannels": "unknown",
                    "audioCodec": "unknown",
                    "videoCodec": "unknown",
                    "size": "unknown",
                    "genres": or_empty(&show["genres"]),
                },
            }))
        })
        .collect()
}

/// Whether a text is a day in the form `2026-11-20`.
fn is_day(text: &str) -> bool {
    text.parse::<jiff::civil::Date>().is_ok()
}

/// The films of an answer as events of the calendar, each one on the day of its
/// release. A film without such a day is left out.
pub fn film_events(entries: &Value) -> Vec<Value> {
    let entries = entries.as_array();
    entries
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, child)| {
            let film = &child["movie"];
            let released = film["released"]
                .as_str()
                .or_else(|| child["released"].as_str())
                .filter(|day| is_day(day))?;
            let state = "text-dark";
            let title = written(&film["title"]);
            Some(json!({
                "id": format!("Trakt-Movie-{}", index + 1),
                "title": title,
                "start": released,
                "className": format!(
                    "inline-popups bg-calendar calendar-item get-tmdb-image tmdb-movie tmdbID--{}",
                    written(film.pointer("/ids/tmdb").unwrap_or(&Value::Null))
                ),
                "imagetype": format!("film {state}"),
                "imagetypeFilter": "film",
                "downloadFilter": state,
                "bgColor": background(state),
                "details": {
                    "topTitle": title,
                    "bottomTitle": film["tagline"],
                    "status": film["status"],
                    "overview": film["overview"],
                    "runtime": film["runtime"],
                    "image": NO_PICTURE,
                    "ratings": or_empty(&film["rating"]),
                    "videoQuality": "unknown",
                    "audioChannels": "",
                    "audioCodec": "",
                    "videoCodec": "",
                    "genres": film["genres"],
                    "year": or_empty(&film["year"]),
                    // The PHP application names the year here too.
                    "studio": or_empty(&film["year"]),
                },
            }))
        })
        .collect()
}

/// The address of the page of Trakt where a person allows the access of the API
/// application. Trakt sends the browser back to `redirect` with a code and the `state`.
pub fn authorize_address(site: &str, client_id: &str, redirect: &str, state: &str) -> String {
    let query: String = form_pairs(&[
        ("response_type", "code"),
        ("client_id", client_id),
        ("redirect_uri", redirect),
        ("state", state),
    ]);
    format!("{}/oauth/authorize?{query}", site.trim_end_matches('/'))
}

/// The fields of a query, each name and each value written for an address.
fn form_pairs(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(name, value)| format!("{}={}", escaped(name), escaped(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// The tokens of an account.
#[derive(Clone, PartialEq, Eq)]
pub struct Tokens {
    /// The token that the requests for the calendar carry.
    pub access: String,
    /// The token that buys new tokens.
    pub refresh: String,
    /// When the access token ends, in the form `2026-11-20T10:00:00Z`.
    pub expires: String,
}

impl std::fmt::Debug for Tokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The tokens are secrets.
        f.debug_struct("Tokens")
            .field("expires", &self.expires)
            .finish_non_exhaustive()
    }
}

/// What buys tokens.
#[derive(Debug, Clone, Copy)]
pub enum Grant<'a> {
    /// The code that Trakt gave the browser after the person allowed the access.
    Code(&'a str),
    /// The refresh token of tokens that Organizr has.
    Refresh(&'a str),
}

/// The API application of the Admin at Trakt.
#[derive(Clone, Copy)]
pub struct Application<'a> {
    /// The address of the API: [`TRAKT_API`], or a stand-in for a test.
    pub api: &'a str,
    /// The client id.
    pub client_id: &'a str,
    /// The client secret.
    pub client_secret: &'a str,
    /// The address of Organizr that the application names as its redirect URI.
    pub redirect: &'a str,
}

/// The moment in the form that the settings store.
fn stamp(seconds: i64) -> String {
    Timestamp::from_second(seconds)
        .unwrap_or(Timestamp::UNIX_EPOCH)
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// The tokens of an answer of Trakt. The access token ends at the time that Trakt
/// names, or 90 days after `now`.
fn tokens_of(answer: &Value, now: Timestamp) -> Option<Tokens> {
    let text = |name: &str| {
        answer[name]
            .as_str()
            .filter(|token| !token.is_empty())
            .map(str::to_owned)
    };
    let created = answer["created_at"]
        .as_i64()
        .unwrap_or_else(|| now.as_second());
    let lifetime = answer["expires_in"]
        .as_i64()
        .filter(|seconds| *seconds > 0)
        .unwrap_or(DEFAULT_LIFETIME);
    Some(Tokens {
        access: text("access_token")?,
        refresh: text("refresh_token")?,
        expires: stamp(created.saturating_add(lifetime)),
    })
}

/// Asks Trakt for tokens.
pub async fn tokens(
    http: &Outbound,
    application: Application<'_>,
    grant: Grant<'_>,
    now: Timestamp,
) -> Result<Tokens, TraktError> {
    let url = format!("{}/oauth/token", application.api.trim_end_matches('/'));
    let (name, value, kind) = match grant {
        Grant::Code(code) => ("code", code, "authorization_code"),
        Grant::Refresh(token) => ("refresh_token", token, "refresh_token"),
    };
    let body = json!({
        "client_id": application.client_id,
        "client_secret": application.client_secret,
        "redirect_uri": application.redirect,
        name: value,
        "grant_type": kind,
    });
    let response = http
        .client(&url, false)
        .post(&url)
        .json(&body)
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(|error| TraktError::Unreachable(without_address(error)))?;
    let status = response.status();
    let answer: Value = response.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        // Trakt says in a few words what is wrong, for example "invalid_grant".
        let reason = answer
            .get("error_description")
            .and_then(Value::as_str)
            .or_else(|| answer.get("error").and_then(Value::as_str))
            .map_or_else(
                || format!("the status {}", status.as_u16()),
                |text| text.chars().take(200).collect(),
            );
        return Err(TraktError::Refused(reason));
    }
    tokens_of(&answer, now).ok_or(TraktError::Unreadable)
}

/// Whether the access token is over at `now`. A time that cannot be read is not over:
/// then nobody knows when the token ends, and Trakt decides.
pub fn is_over(expires: &str, now: Timestamp) -> bool {
    expires
        .trim()
        .parse::<Timestamp>()
        .is_ok_and(|end| now >= end)
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use serde_json::json;

    use super::{authorize_address, film_events, is_over, show_events, tokens_of};

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap_or(Timestamp::UNIX_EPOCH)
    }

    #[test]
    fn episodes_read_as_the_php_application_builds_them() {
        let entries = json!([
            {
                "first_aired": "2026-11-21T02:00:00.000Z",
                "episode": {"season": 3, "number": 1, "title": "The Return", "overview": "They are back.", "runtime": 45},
                "show": {"title": "The Show", "ids": {"tmdb": 4711}, "rating": 8.1, "genres": ["drama"]},
            },
            {
                "first_aired": "2026-11-19T02:00:00.000Z",
                "episode": {"season": 1, "number": 12, "title": "The End"},
                "show": {"title": "Another Show", "ids": {"tmdb": null}},
            },
            {"first_aired": "no time", "episode": {}, "show": {}},
        ]);
        let events = show_events(&entries, at("2026-11-20T10:00:00Z"));
        assert_eq!(events.len(), 2);

        let first = &events[0];
        assert_eq!(first["id"], "Trakt-Tv-1");
        assert_eq!(first["title"], "The Show");
        assert_eq!(first["start"], "2026-11-21T02:00:00.000Z");
        assert_eq!(first["downloadFilter"], "text-primary animated flash");
        assert_eq!(first["bgColor"], "bg-primary animated flash");
        assert_eq!(first["imagetype"], "tv text-primary animated flash");
        assert_eq!(
            first["className"],
            "inline-popups bg-calendar calendar-item get-tmdb-image tmdb-tv tmdbID--4711"
        );
        assert_eq!(first["details"]["bottomTitle"], "S03E01 - The Return");
        assert_eq!(first["details"]["seasonCount"], 3);
        assert_eq!(first["details"]["genres"], json!(["drama"]));
        assert_eq!(
            first["details"]["image"],
            "/plugins/images/homepage/no-np.png"
        );

        // An episode that was aired, and that is not the first one of its season.
        let second = &events[1];
        assert_eq!(second["start"], "2026-11-19T02:00:01Z");
        assert_eq!(second["downloadFilter"], "text-danger");
        assert_eq!(second["details"]["overview"], "");
        assert_eq!(second["details"]["ratings"], "");
        assert!(
            second["className"]
                .as_str()
                .is_some_and(|class| class.ends_with("tmdbID--"))
        );
    }

    #[test]
    fn films_stand_on_the_day_of_their_release() {
        let entries = json!([
            {
                "released": "2026-11-25",
                "movie": {
                    "title": "The Film", "released": "2026-11-25", "tagline": "One more time",
                    "status": "released", "overview": "A film.", "runtime": 110, "rating": 7.5,
                    "genres": ["action"], "year": 2026, "ids": {"tmdb": 815},
                },
            },
            {"movie": {"title": "No Day"}},
        ]);
        let events = film_events(&entries);
        assert_eq!(events.len(), 1);
        let film = &events[0];
        assert_eq!(film["id"], "Trakt-Movie-1");
        assert_eq!(film["start"], "2026-11-25");
        assert_eq!(film["imagetypeFilter"], "film");
        assert_eq!(film["downloadFilter"], "text-dark");
        assert_eq!(film["bgColor"], "bg-dark");
        assert_eq!(film["details"]["bottomTitle"], "One more time");
        assert_eq!(film["details"]["year"], 2026);
        assert_eq!(film["details"]["studio"], 2026);
        assert!(
            film["className"]
                .as_str()
                .is_some_and(|class| class.ends_with("tmdb-movie tmdbID--815"))
        );
    }

    #[test]
    fn address_of_the_page_of_trakt_names_the_application_and_the_state() {
        assert_eq!(
            authorize_address(
                "https://trakt.tv/",
                "the id",
                "https://organizr.example.com/api/v2/oauth/trakt",
                "abc-123"
            ),
            "https://trakt.tv/oauth/authorize?response_type=code&client_id=the%20id&redirect_uri=https%3A%2F%2Forganizr.example.com%2Fapi%2Fv2%2Foauth%2Ftrakt&state=abc-123"
        );
    }

    #[test]
    fn tokens_end_when_trakt_says_or_after_ninety_days() {
        let now = at("2026-11-20T10:00:00Z");
        let answer = json!({
            "access_token": "made-up-access", "refresh_token": "made-up-refresh",
            "created_at": now.as_second(), "expires_in": 86400,
        });
        let tokens = tokens_of(&answer, now).expect("tokens");
        assert_eq!(tokens.expires, "2026-11-21T10:00:00Z");
        assert!(!format!("{tokens:?}").contains("made-up"));

        let plain = json!({"access_token": "a", "refresh_token": "r"});
        assert_eq!(
            tokens_of(&plain, now).expect("tokens").expires,
            "2027-02-18T10:00:00Z"
        );
        assert!(tokens_of(&json!({"access_token": "a"}), now).is_none());
        assert!(tokens_of(&json!({"access_token": "", "refresh_token": "r"}), now).is_none());
    }

    #[test]
    fn token_is_over_at_its_end_and_not_when_the_end_is_unknown() {
        let now = at("2026-11-20T10:00:00Z");
        assert!(is_over("2026-11-20T10:00:00Z", now));
        assert!(is_over("2026-01-01T00:00:00Z", now));
        assert!(!is_over("2026-11-20T10:00:01Z", now));
        assert!(!is_over("", now));
        assert!(!is_over("soon", now));
    }
}
