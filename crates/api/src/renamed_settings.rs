//! Moves the stored values of settings that PHP's defaults name differently from the key
//! that PHP reads (D-112), so that an install keeps them after an update.
//!
//! Two kinds of install hold such a value:
//!
//! - An install that stored a value under the name of PHP's default, for example
//!   `ProwlarrURL`, before this application read the other name. The row gets the new
//!   name. A secret is sealed again, because the name of a setting is part of its seal.
//! - An install imported before this application knew the name that PHP reads, for
//!   example `prowlarrURL`. The import kept such a value, sealed, in the table
//!   `legacy_settings`. It moves to the settings.
//!
//! A setting that already has a value under the new name keeps it.

use organizr_auth::secrets::{SecretBox, legacy_setting_name};
use organizr_db::{Database, DbError, StoredSetting};
use organizr_settings::{RENAMED_SETTINGS, Settings};
use serde_json::Value;

/// Moves the values, and tells what it did and what it could not do, one line each.
pub async fn adopt_renamed_settings(
    database: &Database,
    secrets: &SecretBox,
) -> Result<Vec<String>, DbError> {
    let mut notes = Vec::new();
    let stored = database.settings().all().await?;
    let has = |key: &str| stored.iter().any(|setting| setting.key == key);

    for (old, new) in RENAMED_SETTINGS {
        let Some(setting) = stored.iter().find(|setting| setting.key == old) else {
            continue;
        };
        if has(new) {
            database.settings().apply(&[], &[old.to_owned()]).await?;
            notes.push(format!("`{old}` is removed: `{new}` holds the value"));
            continue;
        }
        match renamed_form(old, new, &setting.value, secrets) {
            Some(value) => {
                let put = StoredSetting {
                    key: new.to_owned(),
                    value,
                };
                database.settings().apply(&[put], &[old.to_owned()]).await?;
                notes.push(format!("the value of `{old}` is now stored as `{new}`"));
            }
            None => notes.push(format!(
                "the value of `{old}` cannot be opened with the key of this install, so `{new}` stays empty"
            )),
        }
    }

    let stored = database.settings().all().await?;
    for legacy in database.import().legacy_settings().await? {
        let key = legacy.key.as_str();
        let wanted = RENAMED_SETTINGS.iter().any(|(_, new)| *new == key);
        if !wanted || stored.iter().any(|setting| setting.key == key) {
            continue;
        }
        match adopted_form(key, &legacy.value, secrets) {
            Adopted::Store(value) => {
                database.settings().put(key, &value).await?;
                database.import().forget_legacy_setting(key).await?;
                notes.push(format!("the imported value of `{key}` is now used"));
            }
            Adopted::Default => {
                database.import().forget_legacy_setting(key).await?;
            }
            Adopted::Unusable => notes.push(format!(
                "the imported value of `{key}` cannot be used, so the setting stays empty"
            )),
        }
    }
    Ok(notes)
}

/// The stored form of a value under its new name. A secret is opened under the old name
/// and sealed under the new one. `None` when the secret does not open.
fn renamed_form(old: &str, new: &str, stored: &str, secrets: &SecretBox) -> Option<String> {
    if !Settings::is_secret(new) {
        return Some(stored.to_owned());
    }
    let Ok(Value::String(sealed)) = serde_json::from_str::<Value>(stored) else {
        return Some(stored.to_owned());
    };
    let secret = secrets.open(old, &sealed).ok()?;
    let resealed = secrets.seal(new, &secret).ok()?;
    Some(Value::String(resealed).to_string())
}

/// What becomes of a value of `legacy_settings`.
enum Adopted {
    /// The value, in its stored form.
    Store(String),
    /// The value is the default, which needs no row.
    Default,
    /// The value does not open, or does not fit the setting.
    Unusable,
}

/// What becomes of the sealed value of `key` in `legacy_settings`.
fn adopted_form(key: &str, sealed: &str, secrets: &SecretBox) -> Adopted {
    stored_form_of_legacy(key, sealed, secrets).unwrap_or(Adopted::Unusable)
}

/// The value of `legacy_settings` as a setting, or `None` when it cannot be one.
fn stored_form_of_legacy(key: &str, sealed: &str, secrets: &SecretBox) -> Option<Adopted> {
    let text = secrets.open(&legacy_setting_name(key), sealed).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    let mut settings = Settings::defaults();
    settings.set(key, &value).ok()?;
    let now = settings.get(key)?;
    if Settings::defaults().get(key).as_ref() == Some(&now) {
        return Some(Adopted::Default);
    }
    match now {
        Value::String(secret) if Settings::is_secret(key) => {
            let resealed = secrets.seal(key, &secret).ok()?;
            Some(Adopted::Store(Value::String(resealed).to_string()))
        }
        other => Some(Adopted::Store(other.to_string())),
    }
}
