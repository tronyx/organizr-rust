//! `GET /news` and `POST /news/{id}`: the news items of the PHP project that the Admin
//! chose to ignore.
//!
//! The interface of the PHP application reads the news from the project itself. These
//! routes only keep the ids of the items that no longer get a mark, in the setting
//! `ignoredNewsIds`.

use axum::Extension;
use axum::extract::{Path, State};
use organizr_domain::GroupLevel;
use serde_json::Value;

use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The setting that holds the ignored ids.
const IGNORED: &str = "ignoredNewsIds";

/// `GET /news`: the ids of the ignored news items. The PHP application sends `null`
/// for an empty list.
pub async fn ignored(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let ids = state.settings().ignored_news_ids.clone();
    Ok(Envelope::success(if ids.is_empty() {
        Value::Null
    } else {
        Value::Array(ids)
    }))
}

/// `POST /news/{id}`: adds an id to the ignored ones. The id is read as a whole
/// number, as `intval` of PHP reads it, and an id that the list holds is not added a
/// second time.
pub async fn ignore(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    if id.is_empty() || id == "0" {
        return Err(Refusal::conflict("News id was not supplied"));
    }
    let mut ids = state.settings().ignored_news_ids.clone();
    ids.push(Value::from(php_intval(&id)));
    let mut seen = Vec::new();
    ids.retain(|entry| {
        let text = php_string(entry);
        let new = !seen.contains(&text);
        seen.push(text);
        new
    });
    state
        .change_settings(vec![(IGNORED.to_owned(), Value::Array(ids))])
        .await
        .map_err(ApiError::from)?;
    Ok(Envelope::success(NoData).with_message("News id is now ignored"))
}

/// A whole number as `intval` of PHP reads it from a text: the space at the start, a
/// sign and the digits after it. A text without such digits is `0`.
fn php_intval(text: &str) -> i64 {
    let trimmed = text.trim_start_matches([' ', '\t', '\n', '\r', '\x0B', '\x0C']);
    let (negative, digits) = match trimmed.as_bytes().first() {
        Some(b'-') => (true, trimmed.get(1..).unwrap_or_default()),
        Some(b'+') => (false, trimmed.get(1..).unwrap_or_default()),
        _ => (false, trimmed),
    };
    let mut number: i64 = 0;
    for digit in digits.bytes().take_while(u8::is_ascii_digit) {
        number = number
            .saturating_mul(10)
            .saturating_add(i64::from(digit - b'0'));
    }
    if negative {
        number.saturating_neg()
    } else {
        number
    }
}

/// The text that `array_unique` of PHP compares an entry by.
fn php_string(entry: &Value) -> String {
    match entry {
        Value::String(text) => text.clone(),
        Value::Bool(true) => "1".to_owned(),
        Value::Bool(false) | Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::php_intval;

    #[test]
    fn id_is_read_as_intval_reads_it() {
        for (text, number) in [
            ("12", 12),
            (" 7", 7),
            ("12abc", 12),
            ("abc", 0),
            ("-3", -3),
            ("+4", 4),
            ("99999999999999999999", i64::MAX),
        ] {
            assert_eq!(php_intval(text), number, "{text}");
        }
    }
}
