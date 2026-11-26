//! The tests of the settings pages that test no service of the homepage: whether a
//! page can be shown in a frame, a folder, a cron schedule, the scheduler, the
//! webhook for the logs, and the two tests of the set-up of the PHP application that
//! have nothing to test here.
//!
//! Only an Admin or a Co-Admin can run a test.

use std::path::Path as FilePath;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Extension;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use organizr_domain::GroupLevel;
use organizr_integrations::{frames, slack};
use serde_json::Value;

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::client::Client;
use crate::cron::Schedule;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::{no_requests, tab_address};
use crate::state::AppState;

/// What a test answers.
type Outcome = Result<Envelope<Value>, Refusal>;

/// How old the last sign of the scheduler may be, in seconds. The PHP application
/// takes its cron file for set up when it was touched in the last 120 seconds.
const SCHEDULER_LIMIT: i64 = 120;

fn passed(message: &str) -> Envelope<Value> {
    Envelope::success(Value::Null).with_message(message)
}

fn refused(status: StatusCode, message: impl Into<String>) -> Refusal {
    Refusal::Request(status, message.into())
}

/// `POST /test/iframe`: whether the page at the address of the body lets Organizr
/// show it in a frame.
pub async fn test_iframe(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Outcome {
    caller.require(GroupLevel::CO_ADMIN)?;
    let named = data.text("url").unwrap_or_default();
    if named.is_empty() {
        return Err(Refusal::not_found("URL not supplied"));
    }
    // A tab address that is a path is on this site, as for the browser.
    let url = tab_address(&named, &headers, &client);
    // The host of Organizr, as the browser named it, without the port.
    let organizr = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(frames::host_name)
        .unwrap_or_default();
    let http = state.outbound().ok_or_else(no_requests)?;
    match frames::test(http, &url, &organizr).await {
        Ok(true) => Ok(passed("URL approved for iFrame")),
        Ok(false) => Err(Refusal::conflict("URL failed approval for iFrame")),
        Err(error) => {
            tracing::info!(%error, "the page of a frame test cannot be reached");
            Err(Refusal::conflict("URL failed approval for iFrame"))
        }
    }
}

/// `POST /test/path`: the test of the data folder that the set-up wizard of the PHP
/// application runs before an install. Here the database is there before the wizard
/// starts, and the wizard asks for no folder. So the answer is always the one of the
/// PHP application for an install that has its database.
pub async fn test_path() -> Refusal {
    refused(
        StatusCode::UNAUTHORIZED,
        "Endpoint disabled as database already exists",
    )
}

/// `POST /test/folder`: whether the folder of the body can be used, for example for
/// the backups. The folder passes when it is there. It is not made.
pub async fn test_folder(Extension(caller): Extension<Caller>, data: ApiData) -> Outcome {
    caller.require(GroupLevel::CO_ADMIN)?;
    let folder = data.text("folder").unwrap_or_default();
    if folder.is_empty() || folder == "0" {
        return Err(Refusal::conflict("Folder was not supplied"));
    }
    let is_folder = tokio::fs::metadata(FilePath::new(&folder))
        .await
        .is_ok_and(|found| found.is_dir());
    if is_folder {
        Ok(passed("Folder approved for logs"))
    } else {
        Err(Refusal::conflict(
            "Folder path is not valid or permissions insufficient",
        ))
    }
}

/// The name of a database driver, as the PHP application writes it.
fn driver_name(named: Option<String>) -> String {
    let driver = named.unwrap_or_else(|| "sqlite3".to_owned()).to_lowercase();
    match driver.as_str() {
        "sqlite" | "sqlite3" => "sqlite3".to_owned(),
        "mysql" | "mysqli" => "mysqli".to_owned(),
        "postgre" | "postgres" | "postgresql" => "postgre".to_owned(),
        _ => driver,
    }
}

/// `POST /test/database`: the test of a database connection of the set-up wizard of
/// the PHP application. Here the database is chosen before the first start, and a
/// request cannot name another one. So no driver has a test, and the answer is the
/// one of the PHP application for such a driver.
pub async fn test_database(Extension(caller): Extension<Caller>, data: ApiData) -> Outcome {
    caller.require(GroupLevel::CO_ADMIN)?;
    Err(Refusal::conflict(format!(
        "Database connection test not available for driver: {}",
        driver_name(data.text("driver"))
    )))
}

/// The schedule that a body names. The interface of the PHP application sends the
/// schedule as one JSON text. A form has the schedule as the name of its first field,
/// where PHP writes each space as `_`.
fn schedule_of(content_type: &str, body: &[u8]) -> String {
    let from_name = |name: &str| name.replace('_', " ");
    if content_type
        .to_ascii_lowercase()
        .contains("application/json")
    {
        return match serde_json::from_slice::<Value>(body) {
            Ok(Value::String(schedule)) => schedule,
            Ok(Value::Object(fields)) => fields
                .keys()
                .next()
                .map_or_else(String::new, |name| from_name(name)),
            _ => String::new(),
        };
    }
    form_urlencoded::parse(body)
        .next()
        .map_or_else(String::new, |(name, _)| from_name(&name))
}

/// `POST /test/cron`: whether the text of the body is a cron schedule that the
/// scheduler can read.
pub async fn test_cron_schedule(
    Extension(caller): Extension<Caller>,
    headers: HeaderMap,
    body: Bytes,
) -> Outcome {
    caller.require(GroupLevel::CO_ADMIN)?;
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let schedule = schedule_of(content_type, &body);
    if schedule.trim().is_empty() {
        return Err(Refusal::conflict("Schedule was not supplied"));
    }
    match schedule.parse::<Schedule>() {
        Ok(_) => Ok(passed("Schedule was validated")),
        Err(reason) => Err(refused(StatusCode::INTERNAL_SERVER_ERROR, reason)),
    }
}

/// Whether the scheduler gave a sign of life in the last two minutes.
fn scheduler_runs(beat: i64, now: i64) -> bool {
    beat > 0 && now - beat < SCHEDULER_LIMIT
}

/// `GET /test/cron`: whether the scheduler runs. The PHP application needs an entry
/// in the cron table of the system, and looks at the file that the entry touches.
/// Here the scheduler is a part of the server.
pub async fn test_cron(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Outcome {
    caller.require(GroupLevel::CO_ADMIN)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        });
    if scheduler_runs(state.scheduler_beat(), now) {
        Ok(passed("Cron file is setup"))
    } else {
        Err(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Cron file is not setup correctly",
        ))
    }
}

/// `POST /test/slack-logs`: sends one message to the webhook of Slack or of Discord
/// of the settings.
pub async fn test_slack_logs(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Outcome {
    caller.require(GroupLevel::CO_ADMIN)?;
    let settings = state.settings();
    if !settings.send_logs_to_slack {
        return Err(Refusal::conflict("sendLogsToSlack is disabled"));
    }
    let webhook = settings.slack_log_webhook.trim();
    // A webhook of Discord needs no channel.
    let needs_channel = !webhook.to_lowercase().contains("discord");
    if webhook.is_empty() || (needs_channel && settings.slack_log_web_hook_channel.is_empty()) {
        return Err(Refusal::conflict("slackLogWebhook is empty"));
    }
    let http = state.outbound().ok_or_else(no_requests)?;
    let user = caller.user().map_or("the API key", |user| {
        user.authenticated.user.username.as_str()
    });
    slack::send_test(http, webhook, &settings.slack_log_web_hook_channel, user)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "the test message for the logs could not be sent");
            refused(StatusCode::INTERNAL_SERVER_ERROR, error)
        })?;
    Ok(passed(
        "Slack test connection completed - Please check Slack/Discord Channel",
    ))
}

#[cfg(test)]
mod tests {
    use super::{driver_name, schedule_of, scheduler_runs};

    #[test]
    fn schedule_is_the_text_or_the_first_name_of_the_body() {
        assert_eq!(
            schedule_of("application/json", b"\"*/5 * * * *\""),
            "*/5 * * * *"
        );
        assert_eq!(
            schedule_of("application/json; charset=utf-8", b"\"@daily\""),
            "@daily"
        );
        assert_eq!(
            schedule_of("application/json", b"{\"0_3_*_*_*\": \"\"}"),
            "0 3 * * *"
        );
        assert_eq!(schedule_of("application/json", b"5"), "");
        assert_eq!(schedule_of("application/json", b"no json"), "");
        assert_eq!(
            schedule_of("application/x-www-form-urlencoded", b"*%2F5+*+*+*+*="),
            "*/5 * * * *"
        );
        assert_eq!(
            schedule_of("application/x-www-form-urlencoded", b"0_3_*_*_*=&other=1"),
            "0 3 * * *"
        );
        assert_eq!(schedule_of("", b""), "");
    }

    #[test]
    fn driver_has_the_name_of_the_php_application() {
        assert_eq!(driver_name(None), "sqlite3");
        assert_eq!(driver_name(Some("SQLite".to_owned())), "sqlite3");
        assert_eq!(driver_name(Some("mysql".to_owned())), "mysqli");
        assert_eq!(driver_name(Some("PostgreSQL".to_owned())), "postgre");
        assert_eq!(driver_name(Some("Oracle".to_owned())), "oracle");
    }

    #[test]
    fn scheduler_runs_when_it_woke_in_the_last_two_minutes() {
        assert!(scheduler_runs(1000, 1000));
        assert!(scheduler_runs(1000, 1119));
        assert!(!scheduler_runs(1000, 1120));
        assert!(!scheduler_runs(0, 60));
    }
}
