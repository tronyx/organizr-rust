//! `POST /plex/register` and `POST /emby/register`: a person with an invite makes an
//! account on Plex, or a user on the Emby server of the settings.
//!
//! The PHP application lets everybody call these routes. Here the body must carry a
//! usable invite code in the field `code` (D-83). A wrong or used code is refused and
//! counted as the invites plugin counts it (D-69). The code is used up when the account
//! is made, and stays usable when that fails. For Plex, the new account then gets the
//! access to the Plex server that the invite gives, as `POST /plugins/invites/{code}`
//! gives it.
//!
//! No log and no answer holds the password or the key of the Admin.

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_integrations::emby_join::{Join, JoinError};
use organizr_integrations::plex::PlexError;

use crate::api_data::ApiData;
use crate::client::Client;
use crate::envelope::{Envelope, NoData};
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::{no_requests, service_url};
use crate::handlers::invites::{self, Access};
use crate::state::AppState;

/// The fields of a new account. A field that is missing, empty or the text `0` counts
/// as not supplied, as in PHP.
struct Fields {
    username: String,
    email: String,
    password: String,
}

fn fields(data: &ApiData, status: StatusCode) -> Result<Fields, Refusal> {
    let field = |name: &str, message: &str| {
        data.text(name)
            .filter(|value| !value.is_empty() && value != "0")
            .ok_or_else(|| Refusal::Request(status, message.to_owned()))
    };
    Ok(Fields {
        username: field("username", "Username not supplied")?,
        email: field("email", "Email not supplied")?,
        password: field("password", "Password not supplied")?,
    })
}

/// Checks the invite code of the body, before anything is made.
async fn check_code(state: &AppState, client: &Client, data: &ApiData) -> Result<String, Refusal> {
    invites::check_enabled(state)?;
    let code = data.text("code").unwrap_or_default();
    invites::usable(state, client, &code).await?;
    Ok(code)
}

/// `POST /plex/register`: makes a Plex account with `username`, `email` and
/// `password`, and shares the Plex server with it when the invites are for Plex.
pub async fn plex(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    let new = fields(&data, StatusCode::CONFLICT)?;
    let code = check_code(&state, &client, &data).await?;
    let settings = state.settings();
    let shares = settings.invites_type_include == "plex";
    if shares && (settings.plex_token.is_empty() || settings.plex_id.is_empty()) {
        tracing::warn!("a Plex account cannot be made: the Plex token or the Plex id is not set");
        return Err(Refusal::conflict("Plex Token/ID not set"));
    }
    let Some(plex) = state.plex() else {
        return Err(Refusal::conflict("Plex cannot be asked"));
    };

    let invite_email = invites::invite_email(&state, &code).await?;
    invites::reserve(&state, &client, &code, Some(&new.username)).await?;
    let joined = plex
        .join(
            &settings.install_id,
            &new.username,
            &new.email,
            &new.password,
        )
        .await;
    let refusal = match joined {
        Ok(Ok(())) => None,
        Ok(Err(errors)) => Some(Refusal::Request(StatusCode::UNPROCESSABLE_ENTITY, errors)),
        Err(error) => {
            if let PlexError::Unreachable(_) = error {
                tracing::warn!(error = %error, "plex.tv cannot be reached for a new account");
            } else {
                tracing::warn!(error = %error, "plex.tv did not make a new account");
            }
            Some(Refusal::conflict("An Error Occurred"))
        }
    };
    if let Some(refusal) = refusal {
        state.database().invites().restore(&code).await?;
        return Err(refusal);
    }
    tracing::info!(client = %client.address, "a Plex account was made with an invite");

    if shares {
        match invites::share_plex_server(&plex, &settings, &new.email).await {
            Ok(Access::Given | Access::AlreadyHad) => {
                invites::after_share(&state, &settings, &invite_email, &new.username).await;
            }
            Err(message) => {
                // The account exists. The code stays usable, so that the person can ask
                // for the access with it.
                state.database().invites().restore(&code).await?;
                return Err(Refusal::conflict(format!(
                    "User has joined Plex, but has no access yet: {message}"
                )));
            }
        }
    }
    Ok(Envelope::success(NoData).with_message("User has joined Plex"))
}

/// `POST /emby/register`: makes a user on the Emby server of the settings with
/// `username` and `password`, with the policy of the template user of the invites
/// plugin, and links it to the Emby Connect account of `email`.
pub async fn emby(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    let new = fields(&data, StatusCode::UNPROCESSABLE_ENTITY)?;
    let code = check_code(&state, &client, &data).await?;
    let settings = state.settings();
    if settings.emby_url.is_empty() || settings.emby_token.is_empty() {
        tracing::warn!("an Emby user cannot be made: the Emby address or token is not set");
        return Err(Refusal::conflict("Emby URL/Token not set"));
    }
    let http = state.outbound().ok_or_else(no_requests)?;
    let url = service_url(&settings.emby_url, &headers, &client);

    invites::reserve(&state, &client, &code, Some(&new.username)).await?;
    let join = Join {
        url: &url,
        token: &settings.emby_token,
        never_check_certificate: settings.emby_disable_cert_check,
        template: &settings.invites_emby_template,
        username: &new.username,
        email: &new.email,
        password: &new.password,
    };
    if let Err(error) = join.run(http).await {
        tracing::warn!(%error, "an Emby user was not made");
        state.database().invites().restore(&code).await?;
        let status = match error {
            JoinError::Unreachable(_) => StatusCode::INTERNAL_SERVER_ERROR,
            JoinError::Refused(_) => StatusCode::CONFLICT,
        };
        return Err(Refusal::Request(status, error.to_string()));
    }
    tracing::info!(client = %client.address, "an Emby user was made with an invite");
    Ok(Envelope::success(NoData).with_message("User has joined Emby"))
}
