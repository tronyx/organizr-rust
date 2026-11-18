//! Refuses a request from another site that would change something.
//!
//! A browser tells the server where a request comes from: current browsers in the
//! `Sec-Fetch-Site` header, older ones in `Origin`. A request that changes something
//! (any method other than GET, HEAD and OPTIONS) is let through when it comes from a
//! page of the application itself, or from a client that is not a browser and sends
//! neither header. `tower_http::csrf` makes that decision.
//!
//! This replaces the "form key" of the PHP application, which was the same for every
//! visitor, never changed and was checked on POST only. A refused request gets the answer
//! PHP gives for a wrong form key, so clients see the error they know.
//!
//! A few routes of the PHP application change something with a GET request, for example
//! `GET /logout`. The layer lets every GET through, so the handler of such a route asks
//! for [`FromThisSite`], which makes the same decision for any method.

use axum::extract::{FromRequestParts, Request};
use axum::http::request::Parts;
use axum::http::{HeaderMap, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tower_http::csrf::{CsrfLayer, ProtectionError};

use crate::caller::Caller;
use crate::client::Client;
use crate::error::ApiError;

/// The layer that makes the decision. A refused request leaves it as an empty 403 that
/// carries the reason; [`answer_refusal`] must wrap it to give the refusal its answer.
pub fn layer() -> CsrfLayer {
    CsrfLayer::new()
}

/// Turns a refusal of [`layer`] into the answer of the API. Use it as a middleware around
/// that layer.
pub async fn answer_refusal(request: Request, next: Next) -> Response {
    let route = request.uri().path().to_owned();
    let method = request.method().clone();
    let client = request.extensions().get::<Client>().copied();
    let came_from = request
        .headers()
        .get(header::ORIGIN)
        .or_else(|| request.headers().get(header::REFERER))
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let response = next.run(request).await;

    match response.extensions().get::<ProtectionError>() {
        Some(reason) => {
            tracing::warn!(
                %method,
                %route,
                %reason,
                came_from = came_from.as_deref().unwrap_or("unknown"),
                client = ?client.map(|client| client.address),
                "refused a request from another site"
            );
            ApiError::CrossSite { route }.into_response()
        }
        None => response,
    }
}

/// Proof that a request does not come from another site, or that it carries the API
/// key. A handler that changes something with a GET request takes it as an argument; a
/// request from another site then gets the refusal and the handler does not run.
#[derive(Debug, Clone, Copy)]
pub struct FromThisSite;

impl<S: Send + Sync> FromRequestParts<S> for FromThisSite {
    type Rejection = ApiError;

    // The decision needs nothing that must be waited for, so the answer is ready at once.
    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        std::future::ready(Self::check(parts))
    }
}

impl FromThisSite {
    fn check(parts: &Parts) -> Result<Self, ApiError> {
        // Another site cannot make a browser send the API key, so a request with the
        // key is the request of a script that someone gave the key to.
        let has_api_key = parts
            .extensions
            .get::<Caller>()
            .is_some_and(|caller| caller.has_api_key);
        if !has_api_key && comes_from_another_site(&parts.headers) {
            let route = parts.uri.path().to_owned();
            tracing::warn!(
                method = %parts.method,
                %route,
                client = ?parts.extensions.get::<Client>().map(|client| client.address),
                "refused a request from another site"
            );
            return Err(ApiError::CrossSite { route });
        }
        Ok(Self)
    }
}

/// The decision of [`layer`], for a request of any method.
///
/// - `Sec-Fetch-Site` says it: `same-origin` and `none` (the user typed the address or
///   used a bookmark) are this site, every other value is another site.
/// - Without that header, an `Origin` header must name the host of the request.
/// - A request with neither header does not come from a browser, so no other site made
///   the browser send it.
fn comes_from_another_site(headers: &HeaderMap) -> bool {
    // A value that is not text is none of the known values.
    let text = |name: &str| headers.get(name).map(|value| value.to_str().unwrap_or(""));

    match (text("sec-fetch-site"), text("origin")) {
        (Some(site), _) => !matches!(site, "same-origin" | "none"),
        (None, Some(origin)) => {
            let authority = origin.split_once("://").map(|(_, authority)| authority);
            authority.is_none() || authority != text("host")
        }
        (None, None) => false,
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderName, HeaderValue};

    use super::comes_from_another_site;

    fn request(headers: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        map.insert("host", HeaderValue::from_static("organizr.example.com"));
        for (name, value) in headers {
            map.insert(
                HeaderName::from_static(name),
                HeaderValue::from_static(value),
            );
        }
        map
    }

    #[test]
    fn request_of_the_application_itself_is_from_this_site() {
        let from_here: [&[(&str, &str)]; 6] = [
            // A current browser on a page of the application.
            &[("sec-fetch-site", "same-origin")],
            // The user typed the address or used a bookmark.
            &[("sec-fetch-site", "none")],
            // The fetch header decides, whatever else is sent.
            &[
                ("sec-fetch-site", "same-origin"),
                ("origin", "https://evil.example.net"),
            ],
            // An older browser on a page of the application.
            &[("origin", "https://organizr.example.com")],
            &[("origin", "http://organizr.example.com")],
            // A script or an API client: no browser headers at all.
            &[],
        ];

        for headers in from_here {
            assert!(!comes_from_another_site(&request(headers)), "{headers:?}");
        }
    }

    #[test]
    fn request_that_another_site_made_the_browser_send_is_from_another_site() {
        let from_elsewhere: [&[(&str, &str)]; 8] = [
            &[("sec-fetch-site", "cross-site")],
            // A sibling sub-domain, for example another application behind the same proxy.
            &[("sec-fetch-site", "same-site")],
            &[("sec-fetch-site", "something-new")],
            &[
                ("sec-fetch-site", "cross-site"),
                ("origin", "https://organizr.example.com"),
            ],
            &[("origin", "https://evil.example.net")],
            &[("origin", "https://organizr.example.com.evil.example.net")],
            &[("origin", "https://organizr.example.com:8443")],
            // What a browser sends from a sandboxed page or a local file.
            &[("origin", "null")],
        ];

        for headers in from_elsewhere {
            assert!(comes_from_another_site(&request(headers)), "{headers:?}");
        }
    }
}
