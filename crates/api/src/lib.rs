//! The HTTP API of Organizr: router, response envelope, extractors, error type and handlers.

pub mod api_data;
pub mod caller;
pub mod client;
pub mod cookie_domain;
pub mod cookies;
pub mod device_cookie;
pub mod envelope;
pub mod error;
mod handlers;
pub mod php_json;
pub mod php_url;
pub mod ping;
mod router;
mod same_origin;
pub mod slim_route;
mod state;

pub use crate::router::{API_BASE, router};
pub use crate::state::{AppState, stored_settings};
