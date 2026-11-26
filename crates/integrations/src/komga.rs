//! Komga for the invites plugin: the libraries that an invite can share, and the account
//! that an invite makes, as the invites plugin of PHP Organizr asks for them. Komga takes
//! the API key of the settings in the header `X-API-Key`.

use std::time::Duration;

use serde::Deserialize;
use serde_json::json;
use thiserror::Error;

use crate::http::{Outbound, without_address};

/// Why Komga did not do what was asked.
#[derive(Debug, Error)]
pub enum KomgaError {
    /// Komga cannot be reached.
    #[error("{0}")]
    Unreachable(String),
    /// Komga answered with an error.
    #[error("Komga answered with HTTP {0}")]
    Status(u16),
    /// The answer of Komga was not what it sends.
    #[error("Komga sent an answer that is not a list of libraries")]
    Answer,
}

/// A library of Komga.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Library {
    /// The id of the library.
    pub id: String,
    /// The name of the library.
    pub name: String,
}

/// The libraries of the Komga at `url`.
pub async fn libraries(
    http: &Outbound,
    url: &str,
    api_key: &str,
    timeout: Duration,
) -> Result<Vec<Library>, KomgaError> {
    let target = format!("{url}/api/v1/libraries");
    let response = http
        .keyed_client_for(&target)
        .get(&target)
        .timeout(timeout)
        .header("Accept", "application/json")
        .header("X-API-Key", api_key)
        .send()
        .await
        .map_err(|error| KomgaError::Unreachable(without_address(error)))?;
    if !response.status().is_success() {
        return Err(KomgaError::Status(response.status().as_u16()));
    }
    response.json().await.map_err(|_| KomgaError::Answer)
}

/// The account that an invite makes in Komga.
#[derive(Debug, Clone, Copy)]
pub struct NewAccount<'a> {
    /// The e-mail address, which is the name of an account in Komga.
    pub email: &'a str,
    /// The first password: the setting `INVITES-komga-default-user-password`.
    pub password: &'a str,
    /// The roles, for example `FILE_DOWNLOAD`.
    pub roles: &'a [String],
    /// The ids of the libraries that the account sees.
    pub library_ids: &'a [String],
}

/// Makes an account in the Komga at `url`, with the body of PHP.
pub async fn create_account(
    http: &Outbound,
    url: &str,
    api_key: &str,
    account: NewAccount<'_>,
    timeout: Duration,
) -> Result<(), KomgaError> {
    let target = format!("{url}/api/v2/users");
    let response = http
        .keyed_client_for(&target)
        .post(&target)
        .timeout(timeout)
        .header("Accept", "application/json")
        .header("X-API-Key", api_key)
        .json(&json!({
            "email": account.email,
            "password": account.password,
            "roles": account.roles,
            "sharedLibraries": {"all": false, "libraryIds": account.library_ids},
        }))
        .send()
        .await
        .map_err(|error| KomgaError::Unreachable(without_address(error)))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(KomgaError::Status(response.status().as_u16()))
    }
}
