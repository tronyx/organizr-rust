//! The `/config` routes: the settings of the install, for the people who manage it.
//!
//! Every route needs an Admin or a Co-Admin. A secret is never shown: the answer says
//! `***Secure***` for a secret that has a value. A change is checked against the settings
//! registry first, and all settings of a request are stored, or none is.
//! `docs/deviations.md` lists the differences from the PHP application as D-06, D-07 and
//! D-53.

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use organizr_auth::api_key::API_KEY_LENGTH;
use organizr_auth::registration::{LONGEST_PASSWORD, hash_password};
use organizr_domain::GroupLevel;
use organizr_settings::Settings;
use serde_json::{Map, Value};

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::fields::{Refusal, is_nothing_in_php};
use crate::state::AppState;

/// The level a caller needs for the configuration routes.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// What the answer shows for a secret that has a value.
pub(crate) const SECURE: &str = "***Secure***";

/// The setting that holds the API key. Only an Admin sees it and changes it.
const API_KEY: &str = "organizrAPI";

/// The setting that holds the registration password. A request sends the password, and
/// the install stores its hash.
const REGISTRATION_PASSWORD: &str = "registrationPassword";

/// The id of the install. It is made once, by the set-up, and is part of the name of the
/// session cookie.
const INSTALL_ID: &str = "uuid";

/// Whether only an Admin can change a setting: the API key, the settings that let a
/// proxy say who the caller is, the scripts that run on every page, and the default look
/// (D-125).
fn is_for_admins_only(key: &str) -> bool {
    key == API_KEY
        || key.starts_with("authProxy")
        || key == "customJava"
        || key == "customThemeJava"
        || key == "layout"
        || key == "modernTheme"
        || key == "modernColourSet"
}

/// A setting as the API shows it to a caller of the given level.
pub(crate) fn shown(key: &str, value: Value, level: GroupLevel) -> Value {
    let is_empty = value.as_str().is_some_and(str::is_empty);
    let may_see = key == API_KEY && level == GroupLevel::ADMIN;
    if Settings::is_secret(key) && !is_empty && !may_see {
        Value::String(SECURE.to_owned())
    } else {
        value
    }
}

/// `GET /config`: every setting, sorted by key.
pub async fn all(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Map<String, Value>>, Refusal> {
    caller.require(MANAGER)?;
    Ok(Envelope::success(matching(
        &state.settings(),
        caller.level(),
        "",
    )))
}

/// The settings whose key holds the text, in any letter case, sorted by key.
fn matching(settings: &Settings, level: GroupLevel, part: &str) -> Map<String, Value> {
    let part = part.to_ascii_lowercase();
    let mut keys: Vec<&str> = Settings::KEYS
        .iter()
        .copied()
        .filter(|key| key.to_ascii_lowercase().contains(&part))
        .collect();
    keys.sort_unstable();
    keys.into_iter()
        .filter_map(|key| {
            settings
                .get(key)
                .map(|value| (key.to_owned(), shown(key, value, level)))
        })
        .collect()
}

/// `GET /config/{item}`: one setting, by its key.
pub async fn item(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(item): Path<String>,
) -> Result<Response, Refusal> {
    read(&state, &caller, &item, "")
}

/// `GET /config/{item}/{term}`: with `search` as the item, the settings whose key holds
/// the term. With another item, that setting: the term is ignored.
pub async fn item_with_term(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path((item, term)): Path<(String, String)>,
) -> Result<Response, Refusal> {
    read(&state, &caller, &item, &term)
}

fn read(state: &AppState, caller: &Caller, item: &str, term: &str) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    let settings = state.settings();
    if item.eq_ignore_ascii_case("search") {
        let found = matching(&settings, caller.level(), term);
        // PHP sends `null` for an empty result.
        let data = (!found.is_empty()).then_some(found);
        return Ok(Envelope::success(data)
            .with_message(format!("Search results for term: {term}"))
            .into_response());
    }
    let Some(value) = settings.get(item) else {
        return Err(Refusal::not_found(&format!(
            "{item} is not defined or is blank"
        )));
    };
    let value = shown(item, value, caller.level());
    let data = (!is_nothing_in_php(&value)).then_some(value);
    Ok(Envelope::success(data)
        .with_message(format!("The value for {item}"))
        .into_response())
}

/// The refusal for a value that does not fit its setting.
fn not_valid(key: &str) -> Refusal {
    Refusal::unprocessable(&format!("Config item: {key} is not valid"))
}

/// The value to store for one setting of a request, or `None` when the request leaves the
/// setting as it is.
///
/// `settings` is a copy of the settings that takes each value, so that a value that does
/// not fit is found before anything is stored.
async fn accepted(
    settings: &mut Settings,
    level: GroupLevel,
    key: &str,
    sent: &Value,
) -> Result<Option<Value>, Refusal> {
    if !Settings::KEYS.contains(&key) {
        return Err(Refusal::unprocessable(&format!(
            "Config item: {key} is not a setting"
        )));
    }
    if key == INSTALL_ID {
        return Err(Refusal::unprocessable(&format!(
            "Config item: {key} cannot be changed"
        )));
    }
    if is_for_admins_only(key) && level != GroupLevel::ADMIN {
        return Err(Refusal::Request(
            StatusCode::UNAUTHORIZED,
            format!("Config item: {key} can only be changed by an admin"),
        ));
    }
    // A form sends the text that the answer showed for a secret back when nobody changed it.
    if Settings::is_secret(key) && sent.as_str() == Some(SECURE) {
        return Ok(None);
    }

    let value = match (key, sent) {
        // PHP stores the empty text for "nothing".
        (_, Value::Null) => Value::String(String::new()),
        (REGISTRATION_PASSWORD, Value::String(password)) if !password.is_empty() => {
            if password.len() > LONGEST_PASSWORD {
                return Err(Refusal::over_limit(LONGEST_PASSWORD));
            }
            let hash = hash_password(password.clone())
                .await
                .map_err(Refusal::from)?;
            Value::String(hash.expose().to_owned())
        }
        (API_KEY, Value::String(key)) if key.chars().count() != API_KEY_LENGTH => {
            return Err(Refusal::unprocessable(&format!(
                "Config item: {API_KEY} must have {API_KEY_LENGTH} characters"
            )));
        }
        (_, value) => value.clone(),
    };
    if settings.set(key, &value).is_ok() {
        return Ok(Some(value));
    }
    // A form sends a switch as the text `true` or `false`.
    let switch = match value.as_str() {
        Some("true") => Value::Bool(true),
        Some("false") => Value::Bool(false),
        _ => return Err(not_valid(key)),
    };
    settings.set(key, &switch).map_err(|_| not_valid(key))?;
    Ok(Some(switch))
}

/// `PUT /config`: changes the settings that the body names.
///
/// The body is an object: each key is a setting, each value its new value. A key that is
/// no setting, and a value that does not fit its setting, are refused, and then nothing
/// changes. The field `formKey` is ignored.
pub async fn change(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    // A body that is a list names no setting. PHP would store its entries under numbers.
    if data.fields().next().is_none() {
        return Err(Refusal::conflict("No data submitted"));
    }
    // A copy that takes each value, so that a value that does not fit is found before
    // anything is stored. The settings are large, so the copy does not stand in this future.
    let mut settings = Box::new((*state.settings()).clone());
    let mut changes = Vec::new();
    for (key, sent) in data.fields() {
        if key.eq_ignore_ascii_case("formkey") {
            continue;
        }
        if let Some(value) = accepted(&mut settings, caller.level(), key, sent).await? {
            changes.push((key.to_owned(), value));
        }
    }

    let keys: Vec<&str> = changes.iter().map(|(key, _)| key.as_str()).collect();
    tracing::info!(settings = ?keys, "settings were changed");
    state
        .change_settings(changes)
        .await
        .map_err(ApiError::from)?;
    Ok(Envelope::success(NoData)
        .with_message("Config items updated")
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::is_for_admins_only;

    #[test]
    fn settings_for_admins_only() {
        for key in [
            "organizrAPI",
            "authProxyEnabled",
            "authProxyHeaderName",
            "authProxyWhitelist",
            "customJava",
            "customThemeJava",
            "layout",
            "modernTheme",
            "modernColourSet",
        ] {
            assert!(is_for_admins_only(key), "{key}");
        }
        for key in ["title", "customCss", "authBackend", "plexToken"] {
            assert!(!is_for_admins_only(key), "{key}");
        }
    }
}
