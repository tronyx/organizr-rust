//! A small page with a title and one sentence, for a window that another site sent
//! back to Organizr: after a payment, and after the login at Trakt. It is the page
//! of `showHTML` of the PHP application.

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

/// What the browser may load for the page: only the style and the script that the
/// page itself holds. Both are fixed texts of this file.
const POLICY: &str = "default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; frame-ancestors 'none'";

const STYLE: &str = "body{font-family:system-ui,sans-serif;background:#1b1e24;color:#e9edf2;\
display:grid;place-items:center;min-height:100vh;margin:0}\
main{text-align:center;padding:2rem}h3{font-size:1.5rem;margin:0 0 .5rem}sup{opacity:.7}";

/// The text for a place in HTML.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The HTML of the page. With `closes`, the window closes itself after three
/// seconds, when the browser lets it.
pub fn document(title: &str, notice: &str, closes: bool) -> String {
    let (title, notice) = (escaped(title), escaped(notice));
    let (on_load, closing) = if closes {
        (
            " onload=\"setTimeout(function(){window.close()},3000)\"",
            "<p><sup>(This window will close automatically)</sup></p>",
        )
    } else {
        ("", "")
    };
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"description\" content=\"{title}\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\
<title>{title}</title>\n<style>{STYLE}</style>\n</head>\n\
<body{on_load}>\n<main>\n<section>\n<div>\n<h3>{title}</h3>\n<p>{notice}</p>\n{closing}\n\
</div>\n</section>\n</main>\n</body>\n</html>\n"
    )
}

/// The answer with the page.
pub fn page(status: StatusCode, title: &str, notice: &str, closes: bool) -> Response {
    let mut response = (status, document(title, notice, closes)).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html;charset=UTF-8"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(POLICY),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

/// An answer of plain text, for the words that the PHP application prints without a
/// page.
pub fn plain(status: StatusCode, text: &str) -> Response {
    let mut response = (status, text.to_owned()).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain;charset=UTF-8"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::document;

    #[test]
    fn page_names_its_title_and_closes_only_when_asked() {
        let closing = document("Donation Success", "Thank you for donating!", true);
        assert!(closing.contains("<title>Donation Success</title>"));
        assert!(closing.contains("<h3>Donation Success</h3>"));
        assert!(closing.contains("<p>Thank you for donating!</p>"));
        assert!(closing.contains("window.close()"));
        assert!(closing.contains("(This window will close automatically)"));

        let staying = document("A <b> & \"c\"", "<script>", false);
        assert!(staying.contains("<h3>A &lt;b&gt; &amp; &quot;c&quot;</h3>"));
        assert!(staying.contains("<p>&lt;script&gt;</p>"));
        assert!(!staying.contains("window.close()"));
    }
}
