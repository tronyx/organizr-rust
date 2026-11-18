//! The response envelope: the one shape every JSON answer of the API has.
//!
//! ```json
//! {
//!     "response": {
//!         "result": "success",
//!         "message": null,
//!         "data": "pong"
//!     }
//! }
//! ```

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Serialize, Serializer};

use crate::php_json;

/// The `result` field of an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// The request did what it asked for.
    Success,
    /// The request failed.
    Error,
    /// The request needs something more, for example a two-factor code.
    Warning,
}

/// The `Content-Type` of a JSON answer. The PHP application sends one of two forms,
/// depending on the route, and clients may compare the header as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonContentType {
    /// `application/json;charset=UTF-8`: what most routes send.
    WithCharset,
    /// `application/json`: what a few routes send, `GET /ping` among them.
    Plain,
}

impl JsonContentType {
    const fn header_value(self) -> HeaderValue {
        match self {
            Self::WithCharset => HeaderValue::from_static("application/json;charset=UTF-8"),
            Self::Plain => HeaderValue::from_static("application/json"),
        }
    }
}

/// The `data` of an answer that carries none. It is written as `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoData;

impl Serialize for NoData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}

/// One JSON answer of the API: a status code and the envelope fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope<T> {
    status: StatusCode,
    outcome: Outcome,
    message: Option<String>,
    data: T,
    content_type: JsonContentType,
}

impl<T> Envelope<T> {
    /// A `200` answer with `result: "success"`, no message and the given data.
    pub const fn success(data: T) -> Self {
        Self {
            status: StatusCode::OK,
            outcome: Outcome::Success,
            message: None,
            data,
            content_type: JsonContentType::WithCharset,
        }
    }

    /// An answer with `result: "error"`, the given status, message and data.
    pub fn error(status: StatusCode, message: impl Into<String>, data: T) -> Self {
        Self {
            status,
            outcome: Outcome::Error,
            message: Some(message.into()),
            data,
            content_type: JsonContentType::WithCharset,
        }
    }

    /// Sets the message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Sets the form of the `Content-Type` header.
    #[must_use]
    pub const fn with_content_type(mut self, content_type: JsonContentType) -> Self {
        self.content_type = content_type;
        self
    }
}

/// A `204` answer: the request was carried out and there is nothing to say. It has no
/// body, but it keeps the content type that the PHP application sends with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoContent;

impl IntoResponse for NoContent {
    fn into_response(self) -> Response {
        (
            StatusCode::NO_CONTENT,
            [(
                header::CONTENT_TYPE,
                JsonContentType::WithCharset.header_value(),
            )],
        )
            .into_response()
    }
}

/// The envelope as it goes over the wire.
#[derive(Serialize)]
struct Wire<'a, T> {
    response: Fields<'a, T>,
}

#[derive(Serialize)]
struct Fields<'a, T> {
    result: Outcome,
    message: Option<&'a str>,
    data: &'a T,
}

/// What is sent when an answer cannot be written as JSON. It has the envelope shape.
const UNWRITABLE_ANSWER: &str = "{\n    \"response\": {\n        \"result\": \"error\",\n        \"message\": \"Internal Server Error\",\n        \"data\": null\n    }\n}";

impl<T: Serialize> IntoResponse for Envelope<T> {
    fn into_response(self) -> Response {
        let wire = Wire {
            response: Fields {
                result: self.outcome,
                message: self.message.as_deref(),
                data: &self.data,
            },
        };
        let content_type = [(header::CONTENT_TYPE, self.content_type.header_value())];
        match php_json::to_vec(&wire) {
            Ok(body) => (self.status, content_type, body).into_response(),
            Err(error) => {
                tracing::error!(%error, "an answer could not be written as JSON");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    content_type,
                    UNWRITABLE_ANSWER,
                )
                    .into_response()
            }
        }
    }
}
