//! The one error type of the API. Every failure of a handler becomes an envelope here.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use organizr_auth::login::LoginError;
use organizr_db::DbError;
use serde::Serialize;
use thiserror::Error;

use crate::cookies::CookieError;
use crate::envelope::{Envelope, NoData};

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

    /// A login failed: because of the name or the password, or because something broke.
    #[error(transparent)]
    Login(#[from] LoginError),

    /// A cookie of the answer could not be written. The caller gets no details; the log
    /// does.
    #[error(transparent)]
    Cookie(#[from] CookieError),

    /// The database failed. The caller gets no details; the log does.
    #[error(transparent)]
    Database(#[from] DbError),
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
            // One answer for an unknown name and for a wrong password, as in the PHP
            // application.
            Self::Login(LoginError::WrongCredentials) => Envelope::error(
                StatusCode::UNAUTHORIZED,
                "User credentials incorrect",
                NoData,
            )
            .into_response(),
            Self::Login(error) => internal_error(&error, "a login could not be completed"),
            Self::Cookie(error) => internal_error(&error, "a cookie could not be written"),
            Self::Database(error) => internal_error(&error, "a request failed in the database"),
        }
    }
}
