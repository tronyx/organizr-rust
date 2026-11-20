//! `GET /help/smtp`: takes the settings of the shared SMTP account that the PHP project
//! Organizr offers to its installs, and stores them.

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use organizr_domain::GroupLevel;
use organizr_integrations::organizr_app;
use serde_json::Value;

use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::no_requests;
use crate::same_origin::FromThisSite;
use crate::state::AppState;

/// The message of a stored account, as in the PHP application.
const ACTIVATED: &str = "SMTP activated with Organizr SMTP account";

/// Whether the API of the project may set a setting: the switch of the e-mails and the
/// settings of the SMTP server. The PHP application stores every key of the answer.
fn is_mail_server_setting(key: &str) -> bool {
    key == "PHPMAILER-enabled" || key.starts_with("PHPMAILER-smtpHost")
}

/// A value of the answer as the PHP application stores it: the texts `true` and `false`
/// are switches, and `null` is the empty text. A number is stored as text, as the
/// settings of the server hold it.
fn stored_value(value: &Value) -> Value {
    match value {
        Value::String(text) if text == "true" => Value::Bool(true),
        Value::String(text) if text == "false" => Value::Bool(false),
        Value::Null => Value::from(""),
        Value::Number(number) => Value::from(number.to_string()),
        other => other.clone(),
    }
}

/// `GET /help/smtp`: asks the API of the project for the account and stores the
/// settings of the SMTP server that it names. `data` is `true` when the API answered,
/// and `false` when it refused the request.
///
/// The request changes settings, so it must come from this site or carry the API key
/// (D-03).
pub async fn smtp(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    _: FromThisSite,
) -> Result<Envelope<bool>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let http = state.outbound().ok_or_else(no_requests)?;
    let answer = match organizr_app::shared_smtp(http, state.organizr_api()).await {
        Ok(Some(answer)) => answer,
        // The PHP application answers as if nothing had happened.
        Ok(None) => return Ok(Envelope::success(false)),
        Err(reason) => {
            tracing::warn!(reason, "the API of Organizr cannot be reached");
            return Err(Refusal::RequestWithData(
                StatusCode::INTERNAL_SERVER_ERROR,
                reason,
                Value::Bool(false),
            ));
        }
    };
    let items = answer
        .pointer("/response/data")
        .and_then(Value::as_object)
        .filter(|items| !items.is_empty());
    let Some(items) = items else {
        // PHP refuses the empty list and then writes its message over the refusal.
        return Ok(Envelope::error(StatusCode::CONFLICT, ACTIVATED, true));
    };
    let mut changes = Vec::new();
    for (key, value) in items {
        if is_mail_server_setting(key) {
            changes.push((key.clone(), stored_value(value)));
        } else {
            tracing::warn!(
                key,
                "the API of Organizr named a setting that it may not set"
            );
        }
    }
    let keys: Vec<&str> = changes.iter().map(|(key, _)| key.as_str()).collect();
    tracing::info!(settings = ?keys, "the shared SMTP account of Organizr was stored");
    state
        .change_settings(changes)
        .await
        .map_err(ApiError::from)?;
    Ok(Envelope::success(true).with_message(ACTIVATED))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{is_mail_server_setting, stored_value};

    #[test]
    fn only_the_settings_of_the_mail_server_are_taken() {
        for key in [
            "PHPMAILER-enabled",
            "PHPMAILER-smtpHost",
            "PHPMAILER-smtpHostPassword",
            "PHPMAILER-smtpHostSenderEmail",
        ] {
            assert!(is_mail_server_setting(key), "{key}");
        }
        for key in [
            "organizrAPI",
            "customJava",
            "PHPMAILER-template",
            "smtpHost",
        ] {
            assert!(!is_mail_server_setting(key), "{key}");
        }
    }

    #[test]
    fn values_are_stored_as_php_stores_them() {
        assert_eq!(stored_value(&json!("true")), json!(true));
        assert_eq!(stored_value(&json!("false")), json!(false));
        assert_eq!(stored_value(&json!(null)), json!(""));
        assert_eq!(stored_value(&json!("587")), json!("587"));
        assert_eq!(stored_value(&json!(587)), json!("587"));
    }
}
