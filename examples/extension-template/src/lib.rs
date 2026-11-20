//! A small extension for Organizr.
//!
//! It answers three routes, which show what an extension can do:
//!
//! - `GET /hello` greets the caller by name.
//! - `POST /count` counts its calls in the storage of the extension.
//! - `GET /echo` sends back what it got.
//!
//! Build it with `cargo build --release --target wasm32-wasip2`.

wit_bindgen::generate!({
    path: "../../crates/plugins/wit",
    world: "extension",
});

use organizr::extension::host;

struct Template;

fn text(status: u16, body: String) -> Response {
    Response {
        status,
        headers: vec![("content-type".to_owned(), "text/plain; charset=utf-8".to_owned())],
        body: body.into_bytes(),
    }
}

impl Guest for Template {
    fn handle(request: Request) -> Response {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/hello") => {
                let name = request.caller.username.unwrap_or_else(|| "guest".to_owned());
                host::log(host::LogLevel::Info, &format!("a greeting for {name}"));
                text(200, format!("Hello, {name}!"))
            }
            ("POST", "/count") => {
                let count = host::get_value("count")
                    .and_then(|count| count.parse::<u64>().ok())
                    .unwrap_or(0)
                    + 1;
                match host::set_value("count", &count.to_string()) {
                    Ok(()) => text(200, count.to_string()),
                    Err(error) => text(500, error),
                }
            }
            ("GET", "/echo") => text(
                200,
                format!(
                    "{} {}?{} level={} body={}",
                    request.method,
                    request.path,
                    request.query,
                    request.caller.level,
                    String::from_utf8_lossy(&request.body)
                ),
            ),
            _ => text(404, "This extension has no such route.".to_owned()),
        }
    }
}

export!(Template);
