//! The routes of the second factor of a login: `POST /2fa/{type}`, `POST /2fa`,
//! `PUT /2fa` and `DELETE /2fa`.
//!
//! A user sets a second factor up in three steps. `POST /2fa/google` makes a secret and
//! a QR code for an authenticator app. `POST /2fa` checks a code of the app against the
//! secret. `PUT /2fa` saves the secret for the account. From then on, a login of the
//! account needs a code. `DELETE /2fa` removes the second factor.

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use organizr_auth::secrets::two_factor_name;
use organizr_auth::two_factor::{self, KIND};
use organizr_db::SealedTwoFactor;
use organizr_domain::GroupLevel;
use serde::Serialize;
use serde_json::Value;

use crate::api_data::ApiData;
use crate::caller::{Caller, now_stamp};
use crate::envelope::{Envelope, NoContent, NoData};
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The name that an authenticator app shows for a script with the API key.
const API_USER: &str = "Organizr API";

/// What `POST /2fa/{type}` answers.
#[derive(Debug, Serialize)]
pub struct Created {
    /// The kind of second factor.
    #[serde(rename = "type")]
    kind: &'static str,
    /// The secret, in Base32, for a person who types it into the app.
    secret: String,
    /// A QR code of the secret, as a `data:` address of a PNG picture.
    url: String,
}

/// The answer for a kind of second factor that the application does not have.
fn unknown_kind(kind: &str, data: Value) -> Refusal {
    Refusal::RequestWithData(
        StatusCode::NOT_FOUND,
        format!("{kind} is not an available to be setup"),
        data,
    )
}

/// A field that PHP counts as given: it has text, and the text is not `0`.
fn given(data: &ApiData, name: &str) -> Option<String> {
    data.text(name)
        .filter(|text| !text.is_empty() && text != "0")
}

/// `POST /2fa/{type}`: makes a new secret. Nothing is stored.
pub async fn create(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(kind): Path<String>,
) -> Result<Envelope<Created>, Refusal> {
    caller.require(GroupLevel::LOGGED_IN)?;
    if kind != KIND {
        return Err(unknown_kind(&kind, Value::Null));
    }

    let account = caller
        .user()
        .map_or(API_USER, |user| user.authenticated.user.username.as_str());
    let secret = two_factor::new_secret();
    let url = two_factor::picture(&secret, &state.settings().title, account).map_err(|error| {
        tracing::error!(%error, "the QR code of a second factor could not be made");
        Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
    })?;
    Ok(Envelope::success(Created {
        kind: KIND,
        secret,
        url,
    })
    .with_message("2FA code created - awaiting verification"))
}

/// `POST /2fa`: says whether a code is the code of a secret at this moment.
///
/// The answer for a wrong code is `401` with the result `success`, as in the PHP
/// application.
pub async fn verify(
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    caller.require(GroupLevel::LOGGED_IN)?;
    let missing = |message: &str| {
        Refusal::RequestWithData(
            StatusCode::UNPROCESSABLE_ENTITY,
            message.to_owned(),
            Value::Bool(false),
        )
    };
    let secret =
        given(&data, "secret").ok_or_else(|| missing("Secret was not supplied or left blank"))?;
    let code =
        given(&data, "code").ok_or_else(|| missing("Code was not supplied or left blank"))?;
    let kind =
        given(&data, "type").ok_or_else(|| missing("Type was not supplied or left blank"))?;
    if kind != KIND {
        return Err(unknown_kind(&kind, Value::Bool(false)));
    }

    let now = now_stamp(std::time::SystemTime::now()).unix_seconds();
    let now = u64::try_from(now).unwrap_or_default();
    let valid =
        two_factor::verify(&secret, &code, now).map_err(|_| missing("Secret is not valid"))?;
    Ok(if valid {
        Envelope::success(true)
            .with_message("Verification code verified")
            .into_response()
    } else {
        Envelope::success(false)
            .with_message("Verification code invalid")
            .with_status(StatusCode::UNAUTHORIZED)
            .into_response()
    })
}

/// `PUT /2fa`: saves a secret as the second factor of the account of the caller.
pub async fn save(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(GroupLevel::LOGGED_IN)?;
    let secret = given(&data, "secret")
        .ok_or_else(|| Refusal::unprocessable("Secret was not supplied or left blank"))?;
    let kind = given(&data, "type")
        .ok_or_else(|| Refusal::unprocessable("Type was not supplied or left blank"))?;
    if kind != KIND {
        return Err(unknown_kind(&kind, Value::Null));
    }
    if !two_factor::is_secret(&secret) {
        return Err(Refusal::unprocessable("Secret is not valid"));
    }
    // A script with the API key has no account that could get a second factor.
    let user = &caller
        .user()
        .ok_or(ApiError::NotAuthorized)?
        .authenticated
        .user;

    let sealed_secret = state
        .secrets()
        .seal(&two_factor_name(user.id), &secret)
        .map_err(|error| {
            tracing::error!(%error, "the secret of a second factor could not be sealed");
            Refusal::Request(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the secret could not be stored".to_owned(),
            )
        })?;
    state
        .database()
        .users()
        .set_two_factor(
            user.id,
            Some(SealedTwoFactor {
                kind,
                sealed_secret,
            }),
        )
        .await
        .map_err(ApiError::from)?;
    tracing::info!(user = %user.username, "a user added a second factor");
    Ok(Envelope::success(NoData).with_message("2FA Added"))
}

/// `DELETE /2fa`: removes the second factor of the account of the caller.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Response, ApiError> {
    caller.require(GroupLevel::LOGGED_IN)?;
    if let Some(user) = caller.user().map(|caller| &caller.authenticated.user) {
        state
            .database()
            .users()
            .set_two_factor(user.id, None)
            .await?;
        tracing::info!(user = %user.username, "a user removed the second factor");
    }
    Ok(NoContent.into_response())
}
