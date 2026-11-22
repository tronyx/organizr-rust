//! Nextcloud for the invites plugin: the groups that a new account can join, and the
//! account that an invite makes, through the OCS API with the Admin account of the
//! settings, as the invites plugin of PHP Organizr asks for them.

use std::time::Duration;

use serde_json::Value;
use thiserror::Error;

use crate::http::{Outbound, without_address};

/// Why Nextcloud did not do what was asked.
#[derive(Debug, Error)]
pub enum NextcloudError {
    /// Nextcloud cannot be reached.
    #[error("{0}")]
    Unreachable(String),
    /// Nextcloud answered with an HTTP error.
    #[error("Nextcloud answered with HTTP {0}")]
    Status(u16),
    /// Nextcloud refused, with the status of its OCS API and its message.
    #[error("Nextcloud refused: {0}")]
    Refused(String),
    /// The answer of Nextcloud was not what it sends.
    #[error("Nextcloud sent an answer that is not one of its OCS API")]
    Answer,
}

/// The Admin account that asks Nextcloud.
#[derive(Debug, Clone, Copy)]
pub struct Admin<'a> {
    /// The address of Nextcloud, without a slash at the end.
    pub url: &'a str,
    /// The name of the Admin account.
    pub user: &'a str,
    /// Its password.
    pub password: &'a str,
}

/// The account that an invite makes in Nextcloud.
#[derive(Debug, Clone, Copy)]
pub struct NewUser<'a> {
    /// The id of the account: the e-mail address, or `PlexTv-<id>` for the Plex app.
    pub user_id: &'a str,
    /// The first password, which nobody is told: the person logs in another way.
    pub password: &'a str,
    /// The e-mail address.
    pub email: &'a str,
    /// The name that the person typed at the invite.
    pub display_name: &'a str,
    /// The groups that the account joins.
    pub groups: &'a [String],
    /// The storage quota, for example `10GB`, or the empty text for none.
    pub quota: &'a str,
}

/// The answer of the OCS API, after a check of its status. Version 1 of the API answers
/// HTTP 200 also for a refusal, with the status 100 for success in `ocs.meta`.
async fn ocs(response: reqwest::Response) -> Result<Value, NextcloudError> {
    if !response.status().is_success() {
        return Err(NextcloudError::Status(response.status().as_u16()));
    }
    let body: Value = response.json().await.map_err(|_| NextcloudError::Answer)?;
    match body.pointer("/ocs/meta/statuscode").and_then(Value::as_i64) {
        Some(100 | 200) => Ok(body.pointer("/ocs/data").cloned().unwrap_or(Value::Null)),
        Some(code) => Err(NextcloudError::Refused(format!(
            "{code} {}",
            body.pointer("/ocs/meta/message")
                .and_then(Value::as_str)
                .unwrap_or_default()
        ))),
        None => Err(NextcloudError::Answer),
    }
}

/// The groups of Nextcloud.
pub async fn groups(
    http: &Outbound,
    admin: Admin<'_>,
    timeout: Duration,
) -> Result<Vec<String>, NextcloudError> {
    let target = format!("{}/ocs/v1.php/cloud/groups", admin.url);
    let response = http
        .client_for(&target)
        .get(&target)
        .timeout(timeout)
        .basic_auth(admin.user, Some(admin.password))
        .header("OCS-APIRequest", "true")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| NextcloudError::Unreachable(without_address(error)))?;
    let data = ocs(response).await?;
    Ok(data
        .get("groups")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |groups| {
            groups
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        }))
}

/// Makes an account in Nextcloud, with the form of PHP. PHP keeps only the last group,
/// because it writes each one to the same key; here the account joins each group.
pub async fn create_user(
    http: &Outbound,
    admin: Admin<'_>,
    user: NewUser<'_>,
    timeout: Duration,
) -> Result<(), NextcloudError> {
    let target = format!("{}/ocs/v1.php/cloud/users", admin.url);
    let mut form: Vec<(&str, &str)> = vec![
        ("userid", user.user_id),
        ("password", user.password),
        ("email", user.email),
        ("displayName", user.display_name),
    ];
    form.extend(user.groups.iter().map(|group| ("groups[]", group.as_str())));
    if !user.quota.is_empty() {
        form.push(("quota", user.quota));
    }
    let response = http
        .client_for(&target)
        .post(&target)
        .timeout(timeout)
        .basic_auth(admin.user, Some(admin.password))
        .header("OCS-APIRequest", "true")
        .header("Accept", "application/json")
        .form(&form)
        .send()
        .await
        .map_err(|error| NextcloudError::Unreachable(without_address(error)))?;
    ocs(response).await.map(|_| ())
}
