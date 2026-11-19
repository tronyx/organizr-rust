//! Sending one request to the application and taking the answer apart.

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// The parts of an answer that tests look at.
#[derive(Debug)]
pub struct Answer {
    /// The status code.
    pub status: StatusCode,
    /// All headers.
    pub headers: HeaderMap,
    /// The `Set-Cookie` values, in order.
    pub cookies: Vec<String>,
    /// The body as text.
    pub body: String,
}

impl Answer {
    /// The body as JSON.
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("the body is not JSON ({error}): {}", self.body))
    }

    /// The `data` of the envelope.
    pub fn data(&self) -> Value {
        self.json()
            .pointer("/response/data")
            .cloned()
            .unwrap_or_else(|| panic!("the body has no `response.data`: {}", self.body))
    }

    /// The `message` of the envelope.
    pub fn message(&self) -> Value {
        self.json()
            .pointer("/response/message")
            .cloned()
            .unwrap_or_else(|| panic!("the body has no `response.message`: {}", self.body))
    }

    /// The `Set-Cookie` values for the cookie of the given name.
    pub fn cookies_named(&self, name: &str) -> Vec<&str> {
        let prefix = format!("{name}=");
        self.cookies
            .iter()
            .map(String::as_str)
            .filter(|cookie| cookie.starts_with(&prefix))
            .collect()
    }

    /// The cookie of the given name as a browser sends it back: `name=value`. `None`
    /// when the answer does not set it.
    pub fn cookie(&self, name: &str) -> Option<String> {
        self.cookies_named(name)
            .first()
            .and_then(|cookie| cookie.split(';').next())
            .map(str::to_owned)
    }
}

/// Sends a request to the application.
pub async fn send(app: &Router, request: Request<Body>) -> Answer {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers");
    let status = response.status();
    let headers = response.headers().clone();
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().expect("a cookie is text").to_owned())
        .collect();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("the body can be read")
        .to_bytes();
    Answer {
        status,
        headers,
        cookies,
        body: String::from_utf8(body.to_vec()).expect("the body is UTF-8"),
    }
}

/// A request to the host `organizr.example.com` with the given headers and body.
pub fn request(method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::HOST, "organizr.example.com");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    request
        .body(Body::from(body.to_owned()))
        .expect("the request is valid")
}

/// Who sends a request.
#[derive(Debug, Clone, Copy)]
pub enum As<'a> {
    /// Nobody: no cookie and no key.
    Guest,
    /// A script with the API key of the seed, which counts as the Admin.
    Admin,
    /// A logged-in user, by the session cookie.
    User(&'a str),
}

/// Sends a request to the API. `path` is the path below `/api/v2`. A body is sent as JSON.
pub async fn call(
    app: &Router,
    who: As<'_>,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Answer {
    let mut headers: Vec<(&str, &str)> = Vec::new();
    match who {
        As::Guest => {}
        As::Admin => headers.push(("token", crate::seed::API_KEY)),
        As::User(cookie) => headers.push(("cookie", cookie)),
    }
    let text = body.as_ref().map_or_default(Value::to_string);
    if body.is_some() {
        headers.push(("content-type", "application/json"));
    }
    let path = format!("/api/v2{path}");
    send(app, request(method, &path, &headers, &text)).await
}
