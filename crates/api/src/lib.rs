//! The HTTP API of Organizr: router, response envelope, extractors, error type and handlers.

pub mod cookie_domain;
pub mod device_cookie;
pub mod envelope;
pub mod error;
mod handlers;
pub mod php_json;
mod router;
pub mod slim_route;
mod state;

pub use crate::router::{API_BASE, router};
pub use crate::state::AppState;
