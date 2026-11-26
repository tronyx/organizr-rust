//! SickRage, and the applications that share its API: the episodes for the calendar
//! of the homepage.
//!
//! The functions that format an answer give the events in the form of the PHP
//! application, which the calendar of the homepage reads.

use std::time::Duration;

use jiff::Timestamp;
use jiff::civil::{Date, Time};
use jiff::tz::TimeZone;
use serde_json::{Value, json};

use crate::arr::NO_PICTURE;
use crate::http::{Outbound, body_within, without_address};
use crate::probe::Answer;

/// How long an install has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// Why an install gave no usable answer. The text names no address, because the
/// address holds the API key.
#[derive(Debug, thiserror::Error)]
pub enum SickRageError {
    /// The install cannot be reached, or it did not answer in time.
    #[error("the application cannot be reached: {0}")]
    Unreachable(String),
    /// The install answered with a status that is no success.
    #[error("the application answered with the status {0}")]
    Status(u16),
    /// The answer is not JSON.
    #[error("the answer is not JSON")]
    Unreadable,
    /// The answer is larger than [`ANSWER_LIMIT`].
    #[error("the answer is larger than {ANSWER_LIMIT} bytes")]
    TooLarge,
}

/// The largest answer of an install, in bytes.
pub const ANSWER_LIMIT: usize = 32 * 1024 * 1024;

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
    /// Sends a command of the API. The key is a part of the path, as SickRage wants
    /// it.
    async fn ask(
        &self,
        http: &Outbound,
        command: &[(&str, &str)],
        limit: Duration,
    ) -> Result<Answer, SickRageError> {
        let url = format!("{}/api/{}/", self.url.trim_end_matches('/'), self.key);
        let response = http
            .client(&url, self.never_check_certificate)
            .get(&url)
            .query(command)
            .timeout(limit)
            .send()
            .await
            .map_err(|error| SickRageError::Unreachable(without_address(error)))?;
        let status = response.status().as_u16();
        let body = match body_within(response, ANSWER_LIMIT).await {
            Ok(Some(bytes)) => String::from_utf8_lossy(&bytes).into_owned(),
            Ok(None) => return Err(SickRageError::TooLarge),
            Err(_) => String::new(),
        };
        Ok(Answer { status, body })
    }

    async fn data(
        &self,
        http: &Outbound,
        command: &[(&str, &str)],
    ) -> Result<Value, SickRageError> {
        let answer = self.ask(http, command, TIME_LIMIT).await?;
        if !answer.is_success() {
            return Err(SickRageError::Status(answer.status));
        }
        serde_json::from_str(&answer.body).map_err(|_| SickRageError::Unreadable)
    }

    /// The episodes that were missed, and the ones of today and of the days to come.
    pub async fn future(&self, http: &Outbound) -> Result<Value, SickRageError> {
        let command = [
            ("cmd", "future"),
            ("sort", "date"),
            ("type", "missed|today|soon|later"),
        ];
        self.data(http, &command).await
    }

    /// The last 100 episodes that were downloaded.
    pub async fn history(&self, http: &Outbound) -> Result<Value, SickRageError> {
        let command = [("cmd", "history"), ("limit", "100"), ("type", "downloaded")];
        self.data(http, &command).await
    }

    /// What the install says about itself, for a connection test.
    pub async fn about(&self, http: &Outbound, limit: Duration) -> Result<Answer, SickRageError> {
        self.ask(http, &[("cmd", "sb")], limit).await
    }
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

fn two_digits(value: &Value) -> String {
    let number = match value {
        Value::String(text) => text.trim().parse().unwrap_or(0),
        other => other.as_i64().unwrap_or(0),
    };
    format!("{number:02}")
}

/// A field of an entry, or the given value when the entry has none.
fn or_else(entry: &Value, field: &str, default: Value) -> Value {
    entry
        .get(field)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(default)
}

/// The time of day in a text such as `Sunday 9:00 PM`. A text without a time is
/// midnight.
fn air_time(airs: &str) -> Time {
    let mut parts = airs.split(' ').skip(1);
    let (clock, half) = (parts.next().unwrap_or_default(), parts.next());
    let mut numbers = clock.split(':').map(|part| part.parse::<i8>().ok());
    let (Some(Some(hour)), Some(Some(minute))) = (numbers.next(), numbers.next()) else {
        return Time::midnight();
    };
    let hour = match half.map(str::to_ascii_uppercase).as_deref() {
        Some("PM") if hour < 12 => hour + 12,
        Some("AM") if hour == 12 => 0,
        _ => hour,
    };
    Time::new(hour, minute, 0, 0).unwrap_or_else(|_| Time::midnight())
}

fn event(id: &str, series: &str, start: &Value, show: &str, state: &str, details: &Value) -> Value {
    json!({
        "id": id,
        "title": series,
        "start": start,
        "className": format!("inline-popups bg-calendar calendar-item tvID--{show}"),
        "imagetype": format!("tv {state}"),
        "imagetypeFilter": "tv",
        "downloadFilter": state,
        "bgColor": state.replace("text", "bg"),
        "details": details,
    })
}

/// The events of an answer to the `future` command. `number` is the place of the
/// install in the settings, from 0. The times of the answer are times of `zone`.
pub fn future_events(answer: &Value, number: usize, now: Timestamp, zone: &TimeZone) -> Vec<Value> {
    let mut events = Vec::new();
    let mut count = 0_usize;
    // As in the PHP application: after the first episode that was not aired yet,
    // every later one of the answer counts as not aired.
    let mut not_aired = false;
    for (group, word) in [
        ("missed", "Miss"),
        ("today", "Today"),
        ("soon", "Soon"),
        ("later", "Later"),
    ] {
        let entries = answer["data"].get(group).and_then(Value::as_array);
        for child in entries.into_iter().flatten() {
            count += 1;
            let Ok(day) = text(&child["airdate"]).parse::<Date>() else {
                continue;
            };
            let aired = day.to_datetime(air_time(text(&child["airs"])));
            if let Ok(moment) = aired.to_zoned_datetime(zone.clone()) {
                not_aired = not_aired || now < moment.timestamp();
            }
            let state = if not_aired {
                "text-info"
            } else {
                "text-danger"
            };
            let name = text(&child["show_name"]);
            let details = json!({
                "seasonCount": "",
                "status": child["show_status"],
                "topTitle": name,
                "bottomTitle": format!(
                    "S{}E{} - {}",
                    two_digits(&child["season"]),
                    two_digits(&child["episode"]),
                    text(&child["ep_name"])
                ),
                "overview": or_else(child, "ep_plot", json!("")),
                "runtime": "",
                "image": NO_PICTURE,
                "ratings": "",
                "videoQuality": or_else(child, "quality", json!("")),
                "audioChannels": "",
                "audioCodec": "",
                "videoCodec": "",
                "size": "",
                "genres": "",
            });
            events.push(event(
                &format!("Sick-{number}-{word}-{count}"),
                name,
                &json!(aired.strftime("%Y-%m-%d %H:%M:%S").to_string()),
                &written(&child["tvdbid"]),
                state,
                &details,
            ));
        }
    }
    events
}

/// The events of an answer to the `history` command: the episodes that were
/// downloaded.
pub fn history_events(answer: &Value, number: usize) -> Vec<Value> {
    let entries = answer["data"].as_array();
    entries
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, child)| {
            let name = text(&child["show_name"]);
            let series = &child["series"];
            let details = json!({
                "seasonCount": "",
                "status": child["status"],
                "topTitle": name,
                "bottomTitle": format!(
                    "S{}E{}",
                    two_digits(&child["season"]),
                    two_digits(&child["episode"])
                ),
                "overview": "",
                "runtime": or_else(series, "runtime", json!(30)),
                "image": NO_PICTURE,
                "ratings": or_else(&series["ratings"], "value", json!("unknown")),
                "videoQuality": or_else(child, "quality", json!("unknown")),
                "audioChannels": "",
                "audioCodec": "",
                "videoCodec": "",
                "size": "",
                "genres": "",
            });
            event(
                &format!("Sick-{number}-History-{}", index + 1),
                name,
                &child["date"],
                &written(&child["tvdbid"]),
                "text-success",
                &details,
            )
        })
        .collect()
}

/// What is wrong with the answer of an install to a connection test, or nothing.
pub fn fault(answer: &Answer) -> Option<String> {
    let Ok(body) = serde_json::from_str::<Value>(&answer.body) else {
        return Some("Response was not JSON".to_owned());
    };
    if let Some(error) = body.get("error") {
        return Some(
            error
                .get("msg")
                .and_then(Value::as_str)
                .or_else(|| error.as_str())
                .unwrap_or("An error occurred")
                .to_owned(),
        );
    }
    // SickRage answers a wrong key with the status 200 and the result "denied".
    let field = |name: &str| body.get(name).and_then(Value::as_str).unwrap_or_default();
    let result = field("result");
    if matches!(result, "denied" | "error" | "failure" | "fatal") {
        let message = field("message");
        return Some(if message.is_empty() { result } else { message }.to_owned());
    }
    if !answer.is_success() {
        return Some(format!("The answer has the status {}", answer.status));
    }
    if body.is_array() || body.is_object() {
        None
    } else {
        Some("Response was not JSON".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use jiff::civil::Time;
    use jiff::tz::TimeZone;
    use serde_json::json;

    use super::{
        ANSWER_LIMIT, Answer, Install, SickRageError, air_time, fault, future_events,
        history_events,
    };

    #[tokio::test]
    async fn answer_larger_than_the_limit_is_refused() {
        use axum::Router;
        use axum::routing::get;
        use organizr_testkit::services::StandIn;

        let install = StandIn::start(
            Router::new().route("/api/key/", get(|| async { "a".repeat(ANSWER_LIMIT + 1) })),
        )
        .await;
        let url = install.url();
        let http = crate::http::Outbound::new().expect("the client can be built");
        let answer = Install {
            url: &url,
            key: "key",
            never_check_certificate: false,
        }
        .future(&http)
        .await;
        assert!(matches!(answer, Err(SickRageError::TooLarge)), "{answer:?}");
    }

    #[test]
    fn time_of_day_is_read_from_the_text_of_sickrage() {
        let time = |hour, minute| Time::new(hour, minute, 0, 0).expect("a time");
        assert_eq!(air_time("Sunday 9:00 PM"), time(21, 0));
        assert_eq!(air_time("Monday 12:30 AM"), time(0, 30));
        assert_eq!(air_time("Monday 12:30 PM"), time(12, 30));
        assert_eq!(air_time("Friday 20:15"), time(20, 15));
        assert_eq!(air_time(""), Time::midnight());
        assert_eq!(air_time("Sunday"), Time::midnight());
    }

    fn episode(day: &str) -> serde_json::Value {
        json!({
            "airdate": day, "airs": "Sunday 9:00 PM", "ep_name": "The One", "ep_plot": "What happens.",
            "episode": 3, "season": 2, "show_name": "The Show", "show_status": "Continuing",
            "tvdbid": 4711, "quality": "HD720p",
        })
    }

    #[test]
    fn future_episodes_are_events_in_the_form_of_the_php_application() {
        let answer = json!({"data": {
            "missed": [episode("2020-01-05")],
            "today": [],
            "soon": [episode("2020-01-12")],
            "later": [],
        }, "result": "success"});
        let now: Timestamp = "2020-01-08T12:00:00Z".parse().expect("a moment");

        let events = future_events(&answer, 1, now, &TimeZone::UTC);

        assert_eq!(
            events[0],
            json!({
                "id": "Sick-1-Miss-1",
                "title": "The Show",
                "start": "2020-01-05 21:00:00",
                "className": "inline-popups bg-calendar calendar-item tvID--4711",
                "imagetype": "tv text-danger",
                "imagetypeFilter": "tv",
                "downloadFilter": "text-danger",
                "bgColor": "bg-danger",
                "details": {
                    "seasonCount": "",
                    "status": "Continuing",
                    "topTitle": "The Show",
                    "bottomTitle": "S02E03 - The One",
                    "overview": "What happens.",
                    "runtime": "",
                    "image": "/plugins/images/homepage/no-np.png",
                    "ratings": "",
                    "videoQuality": "HD720p",
                    "audioChannels": "",
                    "audioCodec": "",
                    "videoCodec": "",
                    "size": "",
                    "genres": "",
                },
            })
        );
        // The count goes on over the groups.
        assert_eq!(events[1]["id"], "Sick-1-Soon-2");
        assert_eq!(events[1]["downloadFilter"], "text-info");
        assert_eq!(events[1]["start"], "2020-01-12 21:00:00");
        // An answer without the groups has no events.
        assert_eq!(
            future_events(&json!({"result": "denied"}), 0, now, &TimeZone::UTC),
            Vec::<serde_json::Value>::new()
        );
    }

    #[test]
    fn downloaded_episodes_are_events_with_the_time_of_the_download() {
        let answer = json!({"data": [{
            "date": "2020-01-06 22:10", "episode": 3, "season": 2, "show_name": "The Show",
            "status": "Downloaded", "tvdbid": 4711, "quality": "HD720p",
        }]});

        let events = history_events(&answer, 0);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["id"], "Sick-0-History-1");
        assert_eq!(events[0]["start"], "2020-01-06 22:10");
        assert_eq!(events[0]["downloadFilter"], "text-success");
        assert_eq!(events[0]["details"]["bottomTitle"], "S02E03");
        assert_eq!(events[0]["details"]["runtime"], 30);
        assert_eq!(events[0]["details"]["ratings"], "unknown");
    }

    #[test]
    fn answer_to_a_test_is_read_for_its_fault() {
        let fault = |status: u16, body: &str| {
            fault(&Answer {
                status,
                body: body.to_owned(),
            })
        };
        assert_eq!(fault(200, r#"{"data": {}, "result": "success"}"#), None);
        assert_eq!(
            fault(
                200,
                r#"{"data": {}, "message": "WRONG API KEY", "result": "denied"}"#
            )
            .as_deref(),
            Some("WRONG API KEY")
        );
        assert_eq!(
            fault(200, r#"{"error": {"msg": "Broken"}}"#).as_deref(),
            Some("Broken")
        );
        assert_eq!(
            fault(200, "<html>").as_deref(),
            Some("Response was not JSON")
        );
        assert_eq!(
            fault(404, "{}").as_deref(),
            Some("The answer has the status 404")
        );
    }
}
