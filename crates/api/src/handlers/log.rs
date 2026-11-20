//! `GET /log[/{number}[/{trace_id}]]` and `DELETE /log[/{number}]`: the log of Organizr,
//! for Admins and Co-Admins.
//!
//! `number` picks a file from the list of log files, the newest first, or `all` and
//! `combined-logs` for all of them. It is never a path: a value that is not the place of
//! a file on the list picks the newest file, as in PHP.

use std::path::PathBuf;

use axum::Extension;
use axum::extract::{RawPathParams, RawQuery, State};
use axum::http::StatusCode;
use organizr_domain::GroupLevel;
use serde::Serialize;
use serde_json::Value;

use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::handlers::fields::Refusal;
use crate::log_files::{self, LogFile, Record};
use crate::state::AppState;

/// The level that the log routes ask for.
const READER: GroupLevel = GroupLevel::CO_ADMIN;

/// The number of records of a page when the query names none.
const PAGE_SIZE: usize = 1000;

/// The folder of the log, after the checks of the routes.
fn folder_for(state: &AppState, caller: &Caller) -> Result<PathBuf, Refusal> {
    caller.require(READER)?;
    let data_dir = state
        .data_folder()
        .ok_or_else(|| Refusal::conflict("Logging not setup"))?;
    Ok(log_files::folder(data_dir))
}

/// The log files, the newest first.
async fn files_in(folder: PathBuf) -> Result<Vec<LogFile>, Refusal> {
    tokio::task::spawn_blocking(move || log_files::list(&folder))
        .await
        .map_err(|_| unreadable())?
        .map_err(|error| {
            tracing::warn!(%error, "the log files cannot be listed");
            unreadable()
        })
}

fn unreadable() -> Refusal {
    Refusal::Request(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Log could not be read".to_owned(),
    )
}

/// Whether `number` names all the files.
fn is_all(number: &str) -> bool {
    number == "all" || number == "combined-logs"
}

/// The place on the list that `number` names, as PHP finds an array key: a whole number
/// written without a sign of plus, without leading zeros and without spaces.
fn place_of(number: &str) -> Option<usize> {
    let canonical = number == "0"
        || (!number.is_empty()
            && !number.starts_with('0')
            && number.bytes().all(|byte| byte.is_ascii_digit()));
    if canonical { number.parse().ok() } else { None }
}

/// The values of the query, the last one of each name, as PHP's `$_GET` holds them.
fn query_value(query: Option<&str>, name: &str) -> Option<String> {
    form_urlencoded::parse(query.unwrap_or_default().as_bytes())
        .filter(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
        .last()
}

/// A whole number of the query, or `422` when the value is not one.
fn number_of(query: Option<&str>, name: &str, default: usize) -> Result<(usize, Value), Refusal> {
    match query_value(query, name) {
        None => Ok((default, Value::from(default))),
        Some(text) => {
            let number = text
                .parse::<usize>()
                .ok()
                .filter(|_| text.bytes().all(|byte| byte.is_ascii_digit()))
                .ok_or_else(|| Refusal::unprocessable(&format!("{name} is not valid")))?;
            // PHP sends the value back as the text of the query.
            Ok((number, Value::String(text)))
        }
    }
}

/// `GET /log[/{number}[/{trace_id}]]`: a page of the records of a log file, or the record
/// with a trace id.
pub async fn read(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    parts: RawPathParams,
    RawQuery(query): RawQuery,
) -> Result<Envelope<LogAnswer>, Refusal> {
    let folder = folder_for(&state, &caller)?;
    let mut parts = parts.iter().map(|(_, value)| value.to_owned());
    let number = parts.next().unwrap_or_else(|| "0".to_owned());
    let trace_id = parts.next().filter(|trace| !trace.is_empty());
    let query = query.as_deref();
    let (size, size_value) = number_of(query, "pageSize", PAGE_SIZE)?;
    if size == 0 {
        return Err(Refusal::unprocessable("pageSize is not valid"));
    }
    let (offset, _) = number_of(query, "offset", 0)?;
    let level =
        log_files::filter_of(&query_value(query, "filter").unwrap_or_else(|| "NONE".to_owned()));

    let files = files_in(folder).await?;
    let names = files
        .iter()
        .map(|file| file.date.clone())
        .collect::<Vec<_>>();
    let (chosen, name) = if is_all(&number) {
        (files, "combined-logs".to_owned())
    } else {
        let file = place_of(&number)
            .and_then(|place| files.get(place))
            .or_else(|| files.first())
            .cloned();
        let Some(file) = file else {
            // PHP answers `200` with `false` when there is no log file.
            let message = trace_id.as_ref().map_or_else(
                || "Results for log: ".to_owned(),
                |trace| format!("Results for trace_id: {trace}"),
            );
            return Ok(Envelope::success(LogAnswer::Nothing(false)).with_message(message));
        };
        let name = file.name();
        (vec![file], name)
    };
    let lines = tokio::task::spawn_blocking(move || log_files::read_lines(&chosen))
        .await
        .map_err(|_| unreadable())?
        .map_err(|error| {
            tracing::warn!(%error, "a log file cannot be read");
            unreadable()
        })?;

    // As in PHP, a level in the filter wins over the trace id.
    if let (None, Some(trace)) = (level, &trace_id)
        && let Some(record) = log_files::find_trace(&lines, trace)
    {
        return Ok(Envelope::success(LogAnswer::Record(record))
            .with_message(format!("Results for trace_id: {trace}")));
    }
    let (total, records) = if level.is_none() && trace_id.is_some() {
        (0, Vec::new())
    } else {
        log_files::page(&lines, level, offset, size)
    };
    let page = if offset >= total {
        -1
    } else {
        i64::try_from(offset / size + 1).unwrap_or(i64::MAX)
    };
    let message = trace_id.as_ref().map_or_else(
        || format!("Results for log: {name}"),
        |trace| format!("Results for trace_id: {trace}"),
    );
    Ok(Envelope::success(LogAnswer::Page(LogPage {
        page_info: PageInfo {
            results: total,
            total_pages: total.div_ceil(size),
            page_size: size_value,
            page,
        },
        results: records,
        // The days of the files, for the choice of a file on the page of the log.
        files: trace_id.is_none().then_some(names),
    }))
    .with_message(message))
}

/// The `data` of `GET /log`.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum LogAnswer {
    /// `false`: there is no log file.
    Nothing(bool),
    /// The record of a trace id.
    Record(Option<Record>),
    /// A page of records.
    Page(LogPage),
}

/// A page of records, with the field names of PHP.
#[derive(Debug, Serialize)]
pub struct LogPage {
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
    results: Vec<Option<Record>>,
    /// The days of the log files, the newest first. PHP has no such field.
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<Vec<String>>,
}

/// Where a page stands.
#[derive(Debug, Serialize)]
pub struct PageInfo {
    /// The number of records that match.
    results: usize,
    #[serde(rename = "totalPages")]
    total_pages: usize,
    /// The `pageSize` of the query: a text when the query has it, as in PHP.
    #[serde(rename = "pageSize")]
    page_size: Value,
    /// The number of the page, from 1, or -1 after the last record.
    page: i64,
}

/// `DELETE /log[/{number}]`: deletes a log file.
pub async fn purge(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    parts: RawPathParams,
) -> Result<Envelope<NoData>, Refusal> {
    let folder = folder_for(&state, &caller)?;
    let number = parts.iter().next().map(|(_, value)| value.to_owned());
    let files = files_in(folder).await?;
    let file = match number.as_deref() {
        None => files
            .into_iter()
            .next()
            .ok_or_else(|| Refusal::not_found("Log does not exist"))?,
        Some(number) if is_all(number) => {
            return Err(Refusal::conflict(
                "Cannot delete log [all] as it is not a real log",
            ));
        }
        Some(number) => place_of(number)
            .and_then(|place| files.into_iter().nth(place))
            .ok_or_else(|| Refusal::not_found("Log not found"))?,
    };
    match tokio::fs::remove_file(&file.path).await {
        Ok(()) => {
            tracing::info!(log = file.date, "a log file was purged");
            Ok(Envelope::success(NoData).with_message("Log purged"))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(Refusal::not_found("Log does not exist"))
        }
        Err(error) => {
            tracing::warn!(%error, log = file.date, "a log file could not be purged");
            Err(Refusal::Request(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Log could not be purged".to_owned(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_all, place_of};

    #[test]
    fn a_number_is_a_place_only_in_the_form_of_an_array_key() {
        assert_eq!(place_of("0"), Some(0));
        assert_eq!(place_of("12"), Some(12));
        for number in ["", "01", "-1", "+1", " 1", "1.0", "abc", "../x"] {
            assert_eq!(place_of(number), None, "{number}");
        }
        assert!(is_all("all") && is_all("combined-logs") && !is_all("ALL"));
    }
}
