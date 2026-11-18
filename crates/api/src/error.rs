//! The one error type of the API. Every failure of a handler becomes an envelope here.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use organizr_db::DbError;
use serde::Serialize;
use thiserror::Error;

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

    /// The database failed. The caller gets no details; the log does.
    #[error(transparent)]
    Database(#[from] DbError),
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
            Self::Database(error) => {
                tracing::error!(%error, "a request failed in the database");
                Envelope::error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    NoData,
                )
                .into_response()
            }
        }
    }
}
