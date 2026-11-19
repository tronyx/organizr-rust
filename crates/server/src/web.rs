//! The files of the web interface: the page, its script and its style sheet.
//!
//! The interface is built on its own (`npm run build` in `web/`), which writes the files
//! to `web/dist`. A binary that is built with the feature `embedded-web` holds those
//! files and hands them out. A binary without the feature answers the address of the
//! page with a short notice, so the server and its tests can be built without Node.
//!
//! The interface finds its pages by the part of the address after the `#`, which a
//! browser does not send. So the server only hands out files, and the interface works
//! at the root of a host and in a sub-folder alike.

use std::borrow::Cow;

use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderValue, Method, header};
use axum::middleware::Next;
use axum::response::Response;

/// What the page may load and do. Scripts and styles come only from the server itself,
/// and none of them is written into the page, so a text that an attacker gets into the
/// page cannot run as a script. Pictures and frames come from anywhere, because the
/// tabs are other web applications.
const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; script-src 'self'; \
    style-src 'self'; img-src 'self' data: http: https:; font-src 'self'; \
    connect-src 'self'; frame-src http: https:; frame-ancestors 'self'; base-uri 'none'; \
    form-action 'self'; object-src 'none'";

/// The name of the file that is the page.
const PAGE: &str = "index.html";

/// The files of the built interface.
#[cfg(feature = "embedded-web")]
#[derive(rust_embed::RustEmbed)]
#[folder = "../../web/dist"]
struct Files;

/// The content of a file of the interface.
#[cfg(feature = "embedded-web")]
fn file(name: &str) -> Option<Cow<'static, [u8]>> {
    Files::get(name).map(|file| file.data)
}

/// What a binary without the interface shows at the address of the page.
#[cfg(not(feature = "embedded-web"))]
const NOT_BUILT_IN: &str = "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
    <title>Organizr</title>\n</head>\n<body>\n<h1>Organizr</h1>\n\
    <p>The server is running. This build does not hold the web interface.</p>\n\
    <p>Build the interface in the folder <code>web</code>, and then build the server with \
    the feature <code>embedded-web</code>.</p>\n</body>\n</html>\n";

/// The content of a file of the interface. This binary holds only a notice.
#[cfg(not(feature = "embedded-web"))]
fn file(name: &str) -> Option<Cow<'static, [u8]>> {
    (name == PAGE).then_some(Cow::Borrowed(NOT_BUILT_IN.as_bytes()))
}

/// The content type of a file, by the ending of its name.
fn content_type(name: &str) -> &'static str {
    let ending = name.rsplit_once('.').map_or("", |(_, ending)| ending);
    match ending {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// How long a browser can keep a file.
///
/// The build gives each file under `assets/` a name that changes with its content, so a
/// browser can keep such a file for good. The pictures under `plugins/` change only with
/// a new version of the application, so a browser keeps them for a day. Every other file
/// is checked again each time.
fn caching(name: &str) -> &'static str {
    if name.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else if name.starts_with("plugins/") {
        "public, max-age=86400"
    } else {
        "no-cache"
    }
}

/// The name of the file of the interface that a request asks for, when it asks for one:
/// the page for `/`, and else the path without its first slash. A request for the API,
/// or one that does not read, asks for no file.
fn file_name<'a>(method: &Method, path: &'a str) -> Option<&'a str> {
    if method != Method::GET && method != Method::HEAD {
        return None;
    }
    let name = path.strip_prefix('/')?;
    if name.is_empty() {
        Some(PAGE)
    } else if name.starts_with("api/") {
        None
    } else {
        Some(name)
    }
}

/// Answers a request for a file of the interface, and lets every other request through
/// to the API. Use it as a middleware around the router of the API.
pub async fn serve_files(request: Request, next: Next) -> Response {
    let found = file_name(request.method(), request.uri().path())
        .and_then(|name| file(name).map(|content| (name.to_owned(), content)));
    let Some((name, content)) = found else {
        return next.run(request).await;
    };

    let body = if request.method() == Method::HEAD {
        Body::empty()
    } else {
        Body::from(content.into_owned())
    };
    let mut response = Response::new(body);
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type(&name)),
    );
    // A browser must not guess another type than the one named here.
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(caching(&name)),
    );
    if name == PAGE {
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CONTENT_SECURITY_POLICY),
        );
        // The address of the page does not go to the applications in the tabs.
        headers.insert(
            header::REFERRER_POLICY,
            HeaderValue::from_static("same-origin"),
        );
    }
    response
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode, header};
    use axum::middleware::from_fn;
    use axum::routing::get;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::{CONTENT_SECURITY_POLICY, caching, content_type, file_name, serve_files};

    /// A stand-in for the API behind the middleware.
    fn app() -> Router {
        Router::new()
            .route("/api/v2/ping", get(|| async { "pong" }))
            .fallback(|| async { (StatusCode::NOT_FOUND, "the API has no such route") })
            .layer(from_fn(serve_files))
    }

    struct Answer {
        status: StatusCode,
        headers: axum::http::HeaderMap,
        body: String,
    }

    impl Answer {
        fn header(&self, name: header::HeaderName) -> Option<&str> {
            self.headers.get(name).and_then(|value| value.to_str().ok())
        }
    }

    async fn send(method: Method, path: &str) -> Answer {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .expect("the request is valid");
        let response = app().oneshot(request).await.expect("the router answers");
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("the body can be read")
            .to_bytes();
        Answer {
            status,
            headers,
            body: String::from_utf8_lossy(&body).into_owned(),
        }
    }

    #[test]
    fn request_that_reads_asks_for_a_file_unless_it_is_for_the_api() {
        assert_eq!(file_name(&Method::GET, "/"), Some("index.html"));
        assert_eq!(file_name(&Method::HEAD, "/"), Some("index.html"));
        assert_eq!(
            file_name(&Method::GET, "/assets/index-abc.js"),
            Some("assets/index-abc.js")
        );
        assert_eq!(file_name(&Method::GET, "/favicon.svg"), Some("favicon.svg"));
        assert_eq!(file_name(&Method::GET, "/api/v2/ping"), None);
        assert_eq!(file_name(&Method::GET, "/api/"), None);
        assert_eq!(file_name(&Method::POST, "/"), None);
        assert_eq!(file_name(&Method::DELETE, "/favicon.svg"), None);
        assert_eq!(file_name(&Method::GET, ""), None);
    }

    #[test]
    fn browser_keeps_files_whose_name_follows_their_content() {
        assert_eq!(
            caching("assets/index-abc.js"),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(
            caching("plugins/images/tabs/plex.png"),
            "public, max-age=86400"
        );
        assert_eq!(caching("index.html"), "no-cache");
        assert_eq!(caching("favicon.svg"), "no-cache");
        assert_eq!(caching("other/assets/file.js"), "no-cache");
    }

    #[test]
    fn content_type_follows_the_ending_of_the_name() {
        assert_eq!(content_type("assets/themify-abc.woff"), "font/woff");
        assert_eq!(content_type("assets/icons-abc.woff2"), "font/woff2");
        assert_eq!(content_type("plugins/images/tabs/plex.png"), "image/png");
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(
            content_type("assets/index-abc.js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(
            content_type("assets/index.abc.css"),
            "text/css; charset=utf-8"
        );
        assert_eq!(content_type("favicon.svg"), "image/svg+xml");
        assert_eq!(content_type("file"), "application/octet-stream");
        assert_eq!(content_type("file.exe"), "application/octet-stream");
    }

    #[tokio::test]
    async fn page_comes_with_the_headers_that_protect_it() {
        let answer = send(Method::GET, "/").await;

        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(
            answer.header(header::CONTENT_TYPE),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(
            answer.header(header::CONTENT_SECURITY_POLICY),
            Some(CONTENT_SECURITY_POLICY)
        );
        assert_eq!(
            answer.header(header::X_CONTENT_TYPE_OPTIONS),
            Some("nosniff")
        );
        assert_eq!(answer.header(header::REFERRER_POLICY), Some("same-origin"));
        assert_eq!(answer.header(header::CACHE_CONTROL), Some("no-cache"));
        assert!(
            answer.body.starts_with("<!doctype html>"),
            "{}",
            answer.body
        );
    }

    #[test]
    fn policy_lets_scripts_and_styles_come_from_the_server_only() {
        let policy = format!("; {CONTENT_SECURITY_POLICY};");
        for rule in [
            "; default-src 'self';",
            "; script-src 'self';",
            "; style-src 'self';",
            "; object-src 'none';",
            "; base-uri 'none';",
            "; frame-ancestors 'self';",
        ] {
            assert!(policy.contains(rule), "{rule}");
        }
        assert!(!policy.contains("unsafe"), "{policy}");
    }

    #[tokio::test]
    async fn head_request_gets_the_headers_without_the_page() {
        let answer = send(Method::HEAD, "/").await;

        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(
            answer.header(header::CONTENT_TYPE),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(answer.body, "");
    }

    #[tokio::test]
    async fn other_requests_go_to_the_api() {
        let ping = send(Method::GET, "/api/v2/ping").await;
        assert_eq!((ping.status, ping.body.as_str()), (StatusCode::OK, "pong"));
        assert_eq!(ping.header(header::CONTENT_SECURITY_POLICY), None);

        // A file that the interface does not have, and a request that does not read.
        for (method, path) in [
            (Method::GET, "/assets/no-such-file.js"),
            (Method::GET, "/../Cargo.toml"),
            (Method::POST, "/"),
        ] {
            let answer = send(method.clone(), path).await;
            assert_eq!(answer.status, StatusCode::NOT_FOUND, "{method} {path}");
            assert_eq!(answer.body, "the API has no such route", "{method} {path}");
        }
    }

    #[cfg(not(feature = "embedded-web"))]
    #[tokio::test]
    async fn binary_without_the_interface_says_so() {
        let answer = send(Method::GET, "/").await;

        assert!(
            answer
                .body
                .contains("This build does not hold the web interface."),
            "{}",
            answer.body
        );
    }

    /// The name of the script that the page loads.
    #[cfg(feature = "embedded-web")]
    fn script_of(page: &str) -> &str {
        let start = page.find("src=\"./").expect("the page loads a script") + "src=\"./".len();
        let rest = page.get(start..).expect("the script has a name");
        let end = rest.find('"').expect("the name of the script ends");
        rest.get(..end).expect("the name of the script ends")
    }

    #[cfg(feature = "embedded-web")]
    #[tokio::test]
    async fn binary_with_the_interface_hands_out_the_page_and_its_files() {
        let page = send(Method::GET, "/").await;
        assert!(page.body.contains("<div id=\"app\">"), "{}", page.body);
        // Nothing is written into the page that the policy would stop.
        assert!(!page.body.contains("<script>"), "{}", page.body);
        assert!(!page.body.contains("<style"), "{}", page.body);

        let script = format!("/{}", script_of(&page.body));
        assert!(script.starts_with("/assets/"), "{script}");
        let answer = send(Method::GET, &script).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(
            answer.header(header::CONTENT_TYPE),
            Some("text/javascript; charset=utf-8")
        );
        assert_eq!(
            answer.header(header::CACHE_CONTROL),
            Some("public, max-age=31536000, immutable")
        );
        assert_eq!(answer.header(header::CONTENT_SECURITY_POLICY), None);

        let icon = send(Method::GET, "/favicon.svg").await;
        assert_eq!(icon.header(header::CONTENT_TYPE), Some("image/svg+xml"));
        assert_eq!(icon.header(header::CACHE_CONTROL), Some("no-cache"));
    }
}
