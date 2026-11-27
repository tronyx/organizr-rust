//! The routes of the second factor of a login: `POST /2fa/{type}`, `POST /2fa`,
//! `PUT /2fa` and `DELETE /2fa`.
//!
//! A user sets a second factor up in three steps. `POST /2fa/google` makes a secret and
//! a QR code for an authenticator app. `POST /2fa` checks a code of the app against the
//! secret. `PUT /2fa` saves the secret for the account. From then on, a login of the
//! account needs a code. `DELETE /2fa` removes the second factor.
//!
//! `DELETE /users/2fa/{id}` removes the second factor of another user, for an Admin or a
//! Co-Admin. PHP has no such route (D-129).
//!
//! A change of the own second factor needs the password of the account in the field
//! `password`, or a code of the second factor it has in the field `code` (D-186).

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use organizr_auth::secrets::two_factor_name;
use organizr_auth::two_factor::{self, KIND};
use organizr_db::SealedTwoFactor;
use organizr_domain::{GroupLevel, User};
use serde::Serialize;
use serde_json::Value;

use crate::api_data::ApiData;
use crate::caller::{Caller, now_stamp};
use crate::client::Client;
use crate::envelope::{Envelope, NoContent, NoData};
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::handlers::proof::{self, Proof};
use crate::handlers::users::{end_other_sessions, is_admin, is_another_co_admin, user_at};
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

/// Checks that a change of the own second factor comes from the owner: the password of
/// the account, or a code of the second factor it has (D-186). An account without a
/// second factor has no code, so only the password counts for it.
async fn owner_proof(
    state: &AppState,
    client: &Client,
    user: &User,
    data: &ApiData,
) -> Result<(), Refusal> {
    let proof = Proof::new(data.text("password"), data.text("code"));
    if has_second_factor(state, user).await? {
        proof::password_or_code(state, client, user, proof).await
    } else {
        proof::password(state, client, user, proof.password).await
    }
}

/// Whether the account has a second factor.
async fn has_second_factor(state: &AppState, user: &User) -> Result<bool, Refusal> {
    Ok(state
        .database()
        .users()
        .login_of(user.id)
        .await?
        .is_some_and(|login| login.two_factor.is_some()))
}

/// `PUT /2fa`: saves a secret as the second factor of the account of the caller.
pub async fn save(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
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
    owner_proof(&state, &client, user, &data).await?;

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
    end_other_sessions(state.database(), &caller, user, true, "a new second factor").await?;
    Ok(Envelope::success(NoData).with_message("2FA Added"))
}

/// `DELETE /2fa`: removes the second factor of the account of the caller. An account
/// without a second factor has nothing to remove, and needs no proof.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    data: ApiData,
) -> Result<Response, Refusal> {
    caller.require(GroupLevel::LOGGED_IN)?;
    if let Some(user) = caller.user().map(|caller| &caller.authenticated.user)
        && has_second_factor(&state, user).await?
    {
        owner_proof(&state, &client, user, &data).await?;
        state
            .database()
            .users()
            .set_two_factor(user.id, None)
            .await?;
        tracing::info!(user = %user.username, "a user removed the second factor");
        end_other_sessions(
            state.database(),
            &caller,
            user,
            true,
            "a removed second factor",
        )
        .await?;
    }
    Ok(NoContent.into_response())
}

/// The level that removes the second factor of another user.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// `DELETE /users/2fa/{id}`: removes the second factor of a user, for a person who has
/// lost the authenticator app. Only an Admin removes the second factor of an Admin, or of
/// a Co-Admin other than the caller (D-188). A
/// user without a second factor is no error. The own second factor needs the proof of
/// `DELETE /2fa` here too (D-186). PHP has no such route (D-129).
pub async fn remove_for_user(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    Path(id): Path<String>,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(MANAGER)?;
    let database = state.database();
    let not_found = || Refusal::not_found("User not found");
    let user = user_at(database, &id).await?.ok_or_else(not_found)?;
    let groups = database.groups().list().await?;
    if (is_admin(&groups, &user) && caller.level() != GroupLevel::ADMIN)
        || is_another_co_admin(&caller, &groups, &user)
    {
        return Err(Refusal::not_authorized(
            "Only admins can remove the second factor of admins",
        ));
    }
    let own = caller
        .user()
        .is_some_and(|caller| caller.authenticated.user.id == user.id);
    let had_one = has_second_factor(&state, &user).await?;
    if own && had_one {
        owner_proof(&state, &client, &user, &data).await?;
    }
    if !database.users().set_two_factor(user.id, None).await? {
        // Another request deleted the user after the lookup above.
        return Err(not_found());
    }
    let by = caller.user().map_or("a script with the API key", |caller| {
        caller.authenticated.user.username.as_str()
    });
    tracing::info!(user = %user.username, by, "the second factor of a user was removed");
    if had_one {
        end_other_sessions(database, &caller, &user, own, "a removed second factor").await?;
    }
    Ok(Envelope::success(NoData).with_message("Second factor removed"))
}
