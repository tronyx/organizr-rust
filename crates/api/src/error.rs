//! The one error type of the API. Every failure of a handler becomes an envelope here.

use std::time::Duration;

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use organizr_auth::install::InstallError;
use organizr_auth::login::LoginError;
use organizr_auth::registration::RegistrationError;
use organizr_db::DbError;
use serde::Serialize;
use thiserror::Error;

use crate::cookies::CookieError;
use crate::envelope::{Envelope, NoData};
use crate::state::ChangeError;

/// Why a request failed.
#[derive(Debug, Error)]
pub enum ApiError {
    /// No route matches the method and the path.
    #[error("no route for {method} {endpoint}")]
    NotFound {
        /// The request path.
        endpoint: String,
        /// The request method.
        method: String,
    },

    /// A browser on another site sent a request that would change something.
    #[error("a request from another site to {route} was refused")]
    CrossSite {
        /// The request path.
        route: String,
    },

    /// The caller is not logged in, or the group of the caller does not have the access
    /// that the route asks for.
    #[error("the caller does not have the access that the route asks for")]
    NotAuthorized,

    /// The request lacks something the route needs. The text says what, for the caller.
    #[error("the request cannot be carried out: {0}")]
    Unprocessable(&'static str),

    /// Too many logins failed from the address of the caller, or for the name. No login
    /// is taken from it, or for it, until the time is over.
    #[error("too many logins failed; the next one is taken in {} seconds", retry_after.as_secs())]
    LockedOut {
        /// How long the lock still lasts.
        retry_after: Duration,
    },

    /// A login failed: because of the name or the password, or because something broke.
    #[error(transparent)]
    Login(#[from] LoginError),

    /// The set-up of the install broke. The caller gets no details; the log does.
    #[error(transparent)]
    Install(#[from] InstallError),

    /// A registration broke. The caller gets no details; the log does.
    #[error(transparent)]
    Registration(#[from] RegistrationError),

    /// A cookie of the answer could not be written. The caller gets no details; the log
    /// does.
    #[error(transparent)]
    Cookie(#[from] CookieError),

    /// The database failed. The caller gets no details; the log does.
    #[error(transparent)]
    Database(#[from] DbError),

    /// The folder of the uploaded pictures could not be read or written. The caller
    /// gets no details; the log does.
    #[error("the folder of the uploaded pictures failed: {0}")]
    PictureFolder(#[source] std::io::Error),

    /// A change of settings could not be stored. The caller gets no details; the log
    /// does.
    #[error(transparent)]
    Settings(#[from] ChangeError),
}

/// The answer to a failure that is not the fault of the caller. The log has the details.
fn internal_error(error: &dyn std::error::Error, what: &str) -> Response {
    tracing::error!(%error, "{what}");
    Envelope::error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal Server Error",
        NoData,
    )
    .into_response()
}

/// The `data` of the "not found" answer.
#[derive(Serialize)]
struct NotFoundData {
    endpoint: String,
    method: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound { endpoint, method } => Envelope::error(
                StatusCode::NOT_FOUND,
                "Endpoint Not Found or Defined",
                NotFoundData { endpoint, method },
            )
            .into_response(),
            // The status and the text are those of the PHP application for a request
            // without a valid form key.
            Self::CrossSite { route } => Envelope::error(
                StatusCode::UNAUTHORIZED,
                format!("Not authorized for current Route: {route}"),
                NoData,
            )
            .into_response(),
            Self::NotAuthorized => {
                Envelope::error(StatusCode::UNAUTHORIZED, "Not Authorized", NoData).into_response()
            }
            Self::Unprocessable(message) => {
                Envelope::error(StatusCode::UNPROCESSABLE_ENTITY, message, NoData).into_response()
            }
            // The status and the text are those of the PHP application. The header
            // tells a client how many seconds to wait, rounded up.
            Self::LockedOut { retry_after } => {
                let seconds = retry_after.as_secs() + u64::from(retry_after.subsec_nanos() > 0);
                (
                    [(header::RETRY_AFTER, seconds.max(1).to_string())],
                    Envelope::error(StatusCode::FORBIDDEN, "User is locked out", NoData),
                )
                    .into_response()
            }
            // One answer for an unknown name and for a wrong password, as in the PHP
            // application.
            Self::Login(LoginError::WrongCredentials) => Envelope::error(
                StatusCode::UNAUTHORIZED,
                "User credentials incorrect",
                NoData,
            )
            .into_response(),
            // The texts and the result `warning` are those of the PHP application.
            Self::Login(LoginError::SecondFactorNeeded) => {
                Envelope::warning(StatusCode::UNPROCESSABLE_ENTITY, "2FA Code Needed", NoData)
                    .into_response()
            }
            Self::Login(LoginError::WrongSecondFactor) => {
                Envelope::error(StatusCode::UNPROCESSABLE_ENTITY, "Wrong 2FA", NoData)
                    .into_response()
            }
            Self::Login(error) => internal_error(&error, "a login could not be completed"),
            Self::Install(error) => internal_error(&error, "the set-up could not be completed"),
            Self::Registration(error) => {
                internal_error(&error, "a registration could not be completed")
            }
            Self::Cookie(error) => internal_error(&error, "a cookie could not be written"),
            Self::Database(error) => internal_error(&error, "a request failed in the database"),
            Self::Settings(error) => internal_error(&error, "settings could not be changed"),
            Self::PictureFolder(_) => {
                internal_error(&self, "a picture could not be read or written")
            }
        }
    }
}
