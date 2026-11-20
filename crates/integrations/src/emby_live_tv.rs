//! The live television of an Emby server: the timers, and the recordings.
//!
//! The functions give the numbers and the lists in the form of the PHP application.

use std::collections::HashMap;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::{Value, json};

use crate::emby::{EmbyError, Server};
use crate::http::Outbound;

/// The most recordings that the numbers are counted from.
const COUNTED: &str = "50";

/// The most recordings that the list of the numbers names.
const LISTED: usize = 10;

/// A text of a field that holds one. An empty text is none.
fn filled<'a>(object: &'a Value, name: &str) -> Option<&'a str> {
    object
        .get(name)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

/// The moment of a text of the server, for example `2026-11-20T20:00:00.0000000Z`.
fn moment(text: &str) -> Option<Timestamp> {
    text.trim().parse().ok()
}

/// The entries of a list of the server.
fn entries(answer: Option<&Value>) -> &[Value] {
    answer
        .and_then(|answer| answer.get("Items"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// A number of minutes in the words of the PHP application: `45m`, or `1h 30m`.
fn span(minutes: i64) -> String {
    let hours = minutes / 60;
    if hours > 0 {
        format!("{hours}h {}m", minutes % 60)
    } else {
        format!("{minutes}m")
    }
}

/// Asks the server, and takes an answer that did not come for an empty one, as the
/// PHP application does. The log names no address and no key.
async fn asked(
    http: &Outbound,
    server: &Server<'_>,
    path: &str,
    query: &[(&str, &str)],
) -> Option<Value> {
    match server.json(http, path, query).await {
        Ok(answer) => answer,
        Err(error) => {
            tracing::warn!(%error, path, "the live television of Emby gave no answer");
            None
        }
    }
}

/// The numbers of the recordings of an answer: how many were made today, how many
/// there are, and the newest ones.
fn recordings_of(answer: Option<&Value>, zone: &TimeZone, now: Timestamp) -> (usize, Value, Value) {
    let day = |moment: Timestamp| {
        moment
            .to_zoned_datetime(zone.clone())
            .date()
            .strftime("%Y-%m-%d")
            .to_string()
    };
    let today = day(now);
    let all = entries(answer);
    let mut of_today = 0;
    let mut recent = Vec::new();
    for recording in all {
        let Some(created) = filled(recording, "DateCreated") else {
            continue;
        };
        // PHP reads a text that is no date as the first day of 1970.
        let made = day(moment(created).unwrap_or(Timestamp::UNIX_EPOCH));
        if made == today {
            of_today += 1;
        }
        recent.push(json!({
            "date": made,
            "program": recording.get("Name").filter(|name| !name.is_null()).cloned().unwrap_or_else(|| json!("Unknown")),
            "series": recording.get("SeriesName").filter(|name| !name.is_null()).cloned().unwrap_or_else(|| json!("")),
            "channel": recording.get("ChannelName").filter(|name| !name.is_null()).cloned().unwrap_or_else(|| json!("Unknown Channel")),
            "status": "Completed",
        }));
    }
    recent.truncate(LISTED);
    let total = answer
        .and_then(|answer| answer.get("TotalRecordCount"))
        .filter(|count| !count.is_null())
        .cloned()
        .unwrap_or_else(|| json!(all.len()));
    (of_today, total, Value::Array(recent))
}

/// `GET /homepage/embyLiveTVTracker/stats`: the numbers of the timers and of the
/// recordings. A request that fails leaves its numbers at 0, as in the PHP
/// application.
pub async fn stats(http: &Outbound, server: &Server<'_>, zone: &TimeZone, now: Timestamp) -> Value {
    let timers = asked(http, server, "/emby/LiveTv/Timers", &[]).await;
    let series = asked(http, server, "/emby/LiveTv/SeriesTimers", &[]).await;
    let recordings = asked(
        http,
        server,
        "/emby/LiveTv/Recordings",
        &[
            ("StartIndex", "0"),
            ("Limit", COUNTED),
            ("Fields", "Overview,DateCreated"),
            ("SortBy", "DateCreated"),
            ("SortOrder", "Descending"),
        ],
    )
    .await;
    let (of_today, total, recent) = match recordings.as_ref() {
        Some(answer) => recordings_of(Some(answer), zone, now),
        None => (0, json!(0), json!([])),
    };
    json!({
        "activeTimers": entries(timers.as_ref()).len(),
        "seriesTimers": entries(series.as_ref()).len(),
        "todaysRecordings": of_today,
        "totalRecordings": total,
        "recentRecordings": recent,
    })
}

/// One timer in the form of the list. `users` holds the names of the users by their
/// ids, when the list names users.
fn timer_of(timer: &Value, users: Option<&HashMap<String, String>>, now: Timestamp) -> Value {
    let start = filled(timer, "StartDate").and_then(moment);
    let end = filled(timer, "EndDate").and_then(moment);
    let (mut duration, mut status) = ("-".to_owned(), "Scheduled");
    if let (Some(start), Some(end)) = (start, end) {
        let seconds = end.as_second() - start.as_second();
        // PHP rounds to the nearest minute.
        duration = span((seconds + 30).div_euclid(60));
        if now >= start && now <= end {
            status = "Recording";
        } else if now > end {
            status = "Completed";
        }
    }
    let program = &timer["ProgramInfo"];
    let channel = match (filled(timer, "ChannelName"), timer.get("ChannelId")) {
        (Some(name), _) => name.to_owned(),
        (None, Some(Value::String(id))) if !id.is_empty() => format!("Channel {id}"),
        (None, Some(Value::Number(id))) => format!("Channel {id}"),
        _ => "Unknown Channel".to_owned(),
    };
    let user = users
        .zip(filled(timer, "UserId"))
        .and_then(|(users, id)| users.get(id));
    let episode = filled(program, "EpisodeTitle").unwrap_or_default();
    let series = filled(timer, "SeriesName")
        .or_else(|| filled(program, "SeriesName"))
        .or_else(|| filled(program, "Name"))
        .or_else(|| filled(timer, "Name"))
        .unwrap_or_default();
    let name = if episode.is_empty() {
        timer
            .get("Name")
            .and_then(Value::as_str)
            .or_else(|| program.get("Name").and_then(Value::as_str))
            .unwrap_or("Unknown Program")
    } else {
        episode
    };
    json!({
        "date": timer.get("StartDate").and_then(Value::as_str).unwrap_or_default(),
        "name": name,
        "seriesName": series,
        "episodeTitle": episode,
        "channelName": channel,
        "userName": user,
        "duration": duration,
        "status": status,
        "type": "timer",
    })
}

/// The timers of an answer, the next one first, and at most `most` of them.
fn timers_of(
    answer: Option<&Value>,
    users: Option<&HashMap<String, String>>,
    most: usize,
    now: Timestamp,
) -> Vec<Value> {
    let mut timers: Vec<&Value> = entries(answer).iter().collect();
    let start = |timer: &Value| filled(timer, "StartDate").unwrap_or_default().to_owned();
    timers.sort_by_key(|timer| start(timer));
    timers
        .into_iter()
        .take(most)
        .map(|timer| timer_of(timer, users, now))
        .collect()
}

/// The recordings of an answer in the form of the list.
fn completed_of(answer: Option<&Value>) -> Vec<Value> {
    entries(answer)
        .iter()
        .filter_map(|recording| {
            let created = recording.get("DateCreated").filter(|date| !date.is_null())?;
            // A tick is one ten-millionth of a second.
            let ticks = recording
                .get("RunTimeTicks")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let duration = if ticks > 0 {
                span(ticks / 600_000_000)
            } else {
                "-".to_owned()
            };
            Some(json!({
                "date": created,
                "name": recording.get("Name").filter(|name| !name.is_null()).cloned().unwrap_or_else(|| json!("Unknown Program")),
                "seriesName": recording.get("SeriesName").filter(|name| !name.is_null()).cloned().unwrap_or_else(|| json!("")),
                // The server names no channel and no user for a recording that is made.
                "channelName": "Unknown Channel",
                "userName": null,
                "duration": duration,
                "status": "Completed",
                "type": "recording",
            }))
        })
        .collect()
}

/// What the list of the timers and the recordings holds.
#[derive(Debug, Clone, Copy)]
pub struct Shown {
    /// The most timers.
    pub timers: usize,
    /// Whether the list has the recordings that are made.
    pub completed: bool,
    /// The most recordings that are made.
    pub recordings: usize,
    /// Whether a timer names the user who set it.
    pub users: bool,
}

/// The names of the users of the server, by their ids.
async fn users_of(http: &Outbound, server: &Server<'_>) -> HashMap<String, String> {
    let answer = asked(http, server, "/emby/Users", &[]).await;
    let list = answer.as_ref().and_then(Value::as_array);
    list.into_iter()
        .flatten()
        .filter_map(|user| {
            Some((
                filled(user, "Id")?.to_owned(),
                filled(user, "Name")?.to_owned(),
            ))
        })
        .collect()
}

/// `GET /homepage/embyLiveTVTracker/activity`: the timers, and the recordings that
/// are made. A request that fails leaves its list empty, as in the PHP application.
pub async fn activity(http: &Outbound, server: &Server<'_>, shown: Shown, now: Timestamp) -> Value {
    let users = if shown.users {
        Some(users_of(http, server).await)
    } else {
        None
    };
    let timers = asked(
        http,
        server,
        "/emby/LiveTv/Timers",
        &[(
            "Fields",
            "ChannelName,ChannelId,SeriesName,ProgramInfo,StartDate,EndDate,UserId",
        )],
    )
    .await;
    let scheduled = timers_of(timers.as_ref(), users.as_ref(), shown.timers, now);
    let completed = if shown.completed {
        let limit = shown.recordings.to_string();
        let recordings = asked(
            http,
            server,
            "/emby/LiveTv/Recordings",
            &[
                ("StartIndex", "0"),
                ("Limit", limit.as_str()),
                ("Fields", "DateCreated,SeriesName,RunTimeTicks"),
                ("SortBy", "DateCreated"),
                ("SortOrder", "Descending"),
            ],
        )
        .await;
        completed_of(recordings.as_ref())
    } else {
        Vec::new()
    };
    json!({ "scheduledRecordings": scheduled, "completedRecordings": completed })
}

/// Why the connection test failed.
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    /// The server cannot be reached. The text names no address.
    #[error(transparent)]
    Unreachable(#[from] EmbyError),
    /// The server refused the request.
    #[error("Emby Connection Error")]
    Refused,
    /// The answer names no server.
    #[error("Invalid response from Emby server")]
    Unreadable,
}

/// The connection test: the name of the server, and whether it has live television.
/// The text is the message of the PHP application.
pub async fn test(http: &Outbound, server: &Server<'_>) -> Result<String, TestError> {
    let info = server
        .json(http, "/emby/System/Info", &[])
        .await?
        .ok_or(TestError::Refused)?;
    let name = filled(&info, "ServerName").ok_or(TestError::Unreadable)?;
    let message = match server.json(http, "/emby/LiveTv/Info", &[]).await {
        Ok(live) => {
            let has_services = live
                .as_ref()
                .and_then(|live| live.get("Services"))
                .and_then(Value::as_array)
                .is_some_and(|services| !services.is_empty());
            if has_services {
                format!("Successfully connected to {name} with LiveTV support enabled")
            } else {
                format!("Successfully connected to {name} (Warning: LiveTV may not be configured)")
            }
        }
        Err(_) => format!("Connected to {name} but LiveTV status unknown"),
    };
    Ok(message)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use jiff::Timestamp;
    use jiff::tz::TimeZone;
    use serde_json::{Value, json};

    use super::{completed_of, recordings_of, span, timers_of};

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap_or(Timestamp::UNIX_EPOCH)
    }

    #[test]
    fn minutes_read_as_hours_and_minutes() {
        assert_eq!(span(45), "45m");
        assert_eq!(span(60), "1h 0m");
        assert_eq!(span(135), "2h 15m");
        assert_eq!(span(0), "0m");
    }

    #[test]
    fn recordings_are_counted_for_today_and_listed() {
        let answer = json!({
            "TotalRecordCount": 31,
            "Items": [
                {"Name": "News", "SeriesName": "The News", "ChannelName": "One", "DateCreated": "2026-11-20T06:00:00.0000000Z"},
                {"Name": "Film", "DateCreated": "2026-11-18T20:15:00.0000000Z"},
                {"Name": "No date"},
            ],
        });
        let (today, total, recent) =
            recordings_of(Some(&answer), &TimeZone::UTC, at("2026-11-20T10:00:00Z"));
        assert_eq!(today, 1);
        assert_eq!(total, json!(31));
        assert_eq!(
            recent,
            json!([
                {"date": "2026-11-20", "program": "News", "series": "The News", "channel": "One", "status": "Completed"},
                {"date": "2026-11-18", "program": "Film", "series": "", "channel": "Unknown Channel", "status": "Completed"},
            ])
        );

        let (_, total, _) = recordings_of(
            Some(&json!({"Items": [{}, {}]})),
            &TimeZone::UTC,
            at("2026-11-20T10:00:00Z"),
        );
        assert_eq!(total, json!(2));
    }

    #[test]
    fn timers_come_in_the_order_of_their_start_with_their_state() {
        let answer = json!({"Items": [
            {
                "Name": "Late Show", "StartDate": "2026-11-20T22:00:00.0000000Z", "EndDate": "2026-11-20T23:30:00.0000000Z",
                "ChannelId": "12", "UserId": "u1",
                "ProgramInfo": {"EpisodeTitle": "Guests", "SeriesName": "The Late Show"},
            },
            {
                "Name": "News", "SeriesName": "The News", "StartDate": "2026-11-20T09:45:00.0000000Z",
                "EndDate": "2026-11-20T10:15:00.0000000Z", "ChannelName": "One",
            },
            {"StartDate": "2026-11-19T09:00:00.0000000Z", "EndDate": "2026-11-19T09:20:00.0000000Z"},
        ]});
        let users = HashMap::from([("u1".to_owned(), "robin".to_owned())]);
        let now = at("2026-11-20T10:00:00Z");

        let all = timers_of(Some(&answer), Some(&users), 10, now);
        let field =
            |name: &str| -> Vec<Value> { all.iter().map(|timer| timer[name].clone()).collect() };
        assert_eq!(
            field("status"),
            [json!("Completed"), json!("Recording"), json!("Scheduled")]
        );
        assert_eq!(
            field("duration"),
            [json!("20m"), json!("30m"), json!("1h 30m")]
        );
        assert_eq!(
            field("name"),
            [json!("Unknown Program"), json!("News"), json!("Guests")]
        );
        assert_eq!(
            field("seriesName"),
            [json!(""), json!("The News"), json!("The Late Show")]
        );
        assert_eq!(
            field("channelName"),
            [json!("Unknown Channel"), json!("One"), json!("Channel 12")]
        );
        assert_eq!(
            field("userName"),
            [json!(null), json!(null), json!("robin")]
        );
        assert_eq!(
            field("type"),
            [json!("timer"), json!("timer"), json!("timer")]
        );

        // Without the names of the users, and with a limit.
        let two = timers_of(Some(&answer), None, 2, now);
        assert_eq!(two.len(), 2);
        assert!(two.iter().all(|timer| timer["userName"].is_null()));
        assert_eq!(timers_of(None, None, 10, now).len(), 0);
    }

    #[test]
    fn recordings_that_are_made_have_their_length() {
        let answer = json!({"Items": [
            {"Name": "Film", "SeriesName": null, "DateCreated": "2026-11-18T20:15:00.0000000Z", "RunTimeTicks": 54_000_000_000_i64},
            {"Name": "Short", "DateCreated": "2026-11-17T20:15:00.0000000Z"},
            {"Name": "No date"},
        ]});
        assert_eq!(
            completed_of(Some(&answer)),
            [
                json!({
                    "date": "2026-11-18T20:15:00.0000000Z", "name": "Film", "seriesName": "",
                    "channelName": "Unknown Channel", "userName": null, "duration": "1h 30m",
                    "status": "Completed", "type": "recording",
                }),
                json!({
                    "date": "2026-11-17T20:15:00.0000000Z", "name": "Short", "seriesName": "",
                    "channelName": "Unknown Channel", "userName": null, "duration": "-",
                    "status": "Completed", "type": "recording",
                }),
            ]
        );
    }
}
