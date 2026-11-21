//! Custom HTML: up to eight items of the homepage that hold HTML of the Admin.
//!
//! The PHP application writes the HTML into its own page, where a script of it can do
//! all that the person who looks at the page can do. Here each item is a document of
//! its own, `GET /homepage/html/{number}`, which the interface shows in a frame. The
//! answer tells the browser to treat the document as a page of no site: its scripts
//! run, but they cannot read the cookies of Organizr or call its routes as the person.
//!
//! An item with the switch `homepageCustomHTML{number}Trusted` is a page of this site,
//! as in the PHP application. It can embed pages that only let this site frame them,
//! and its scripts can do all that the person can do.

use axum::Extension;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

use organizr_settings::Settings;
use serde_json::Value;

use super::Permissions;
use crate::caller::Caller;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The numbers of the items: `01` to `08`.
const NUMBERS: [&str; 8] = ["01", "02", "03", "04", "05", "06", "07", "08"];

/// What the browser may do with the document. `sandbox` without `allow-same-origin`
/// makes it a page of no site. A link can open a window or, after a click, change the
/// address of the window of Organizr. The HTML itself may load from anywhere.
pub(crate) const POLICY: &str = "sandbox allow-scripts allow-forms allow-popups \
    allow-popups-to-escape-sandbox allow-top-navigation-by-user-activation; \
    default-src * data: blob: 'unsafe-inline' 'unsafe-eval'; frame-ancestors 'self'";

/// What the browser may do with the document of a trusted item: all that a page of
/// this site may do. Only this site can frame it.
pub(crate) const TRUSTED_POLICY: &str =
    "default-src * data: blob: 'unsafe-inline' 'unsafe-eval'; frame-ancestors 'self'";

/// Whether the Admin trusts the item with the given number: its document is then a
/// page of this site.
pub fn is_trusted(settings: &Settings, number: &str) -> bool {
    NUMBERS.contains(&number)
        && settings.get(&format!("homepageCustomHTML{number}Trusted")) == Some(Value::Bool(true))
}

/// The style that every custom HTML item starts with. See the file.
const BASE_STYLE: &str = include_str!("custom_html.css");

/// Tells the page of Organizr how high the content is, so the frame can fit it.
const SIZE_SCRIPT: &str = "<script>(function () {\
    function tell() {\
        parent.postMessage({ organizrFrameHeight: Math.ceil(document.body.getBoundingClientRect().height) }, '*');\
    }\
    new ResizeObserver(tell).observe(document.body);\
    addEventListener('load', tell);\
    tell();\
})();</script>";

/// Turns a click on a link to a pop-up of PHP Organizr (`href="#new-request"`) into a
/// message to the page of Organizr, which opens its own pop-up for it (D-111). Nothing
/// else goes to the page: only the name of the pop-up, and only for the names above.
const POPUP_SCRIPT: &str = "<script>(function () {\
    var popups = ['new-request', 'mediaSearch-area', 'speedtest-area', 'account-area', 'invite-area'];\
    document.addEventListener('click', function (event) {\
        var link = event.target instanceof Element ? event.target.closest('[href]') : null;\
        var href = link ? link.getAttribute('href') || '' : '';\
        var name = href.charAt(0) === '#' ? href.slice(1) : '';\
        if (popups.indexOf(name) < 0) { return; }\
        event.preventDefault();\
        event.stopPropagation();\
        parent.postMessage({ organizrPopup: name }, '*');\
    }, true);\
})();</script>";

/// The permissions of the item with the given number, or nothing when there is no
/// such item.
pub fn permissions(number: &str) -> Option<Permissions> {
    NUMBERS.contains(&number).then(|| Permissions {
        enabled: vec![format!("homepageCustomHTML{number}Enabled")],
        auth: vec![format!("homepageCustomHTML{number}Auth")],
        not_empty: vec![format!("customHTML{number}")],
    })
}

/// The address of the style sheet of the interface from the query `style=`, when it is
/// a file in the `assets` folder of this server and nothing else.
fn style_sheet(query: Option<&str>) -> Option<&str> {
    let path = query?
        .split('&')
        .find_map(|pair| pair.strip_prefix("style="))?;
    let name = path.strip_prefix("/assets/")?.strip_suffix(".css")?;
    let is_plain = !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character));
    is_plain.then_some(path)
}

/// The document around the HTML of the Admin.
fn document(custom: &str, style: Option<&str>) -> String {
    let link = style.map_or_else(String::new, |path| {
        format!("<link rel=\"stylesheet\" href=\"{path}\">")
    });
    // A link without a target changes the address of Organizr, as it does in the PHP
    // application: `#Plex` opens the tab "Plex".
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\">\
         <base href=\"/\" target=\"_top\">{link}\
         <style>{BASE_STYLE}</style>\
         </head><body>\n{custom}\n{SIZE_SCRIPT}{POPUP_SCRIPT}</body></html>\n"
    )
}

/// `GET /homepage/html/{number}`: one custom HTML item as a document for a frame.
pub async fn show(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(number): Path<String>,
    RawQuery(query): RawQuery,
) -> Result<Response, Refusal> {
    let Some(permissions) = permissions(&number) else {
        return Err(Refusal::not_found("Custom HTML item was not found"));
    };
    let settings = state.settings();
    permissions.check(&settings, &caller)?;
    let custom = settings
        .get(&format!("customHTML{number}"))
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default();

    let body = document(&custom, style_sheet(query.as_deref()));
    let mut response = (StatusCode::OK, body).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    let policy = if is_trusted(&settings, &number) {
        TRUSTED_POLICY
    } else {
        POLICY
    };
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(policy),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::{POLICY, POPUP_SCRIPT, document, permissions, style_sheet};

    #[test]
    fn there_are_eight_items() {
        assert!(permissions("01").is_some());
        assert!(permissions("08").is_some());
        for number in ["00", "09", "1", "wo", "", "001"] {
            assert!(permissions(number).is_none(), "{number}");
        }
    }

    #[test]
    fn style_sheet_is_a_file_of_the_assets_folder_only() {
        assert_eq!(
            style_sheet(Some("style=/assets/index-Ab_1.css")),
            Some("/assets/index-Ab_1.css")
        );
        assert_eq!(
            style_sheet(Some("a=b&style=/assets/x.css")),
            Some("/assets/x.css")
        );
        for query in [
            "style=https://evil.example.net/assets/x.css",
            "style=//evil.example.net/assets/x.css",
            "style=/assets/../x.css",
            "style=/assets/a/b.css",
            "style=/assets/x.css\"><script>",
            "style=/assets/x.js",
            "style=/assets/.css",
            "other=/assets/x.css",
        ] {
            assert_eq!(style_sheet(Some(query)), None, "{query}");
        }
        assert_eq!(style_sheet(None), None);
    }

    #[test]
    fn document_holds_the_html_as_it_is() {
        let page = document("<b onclick=\"go()\">Hi</b>", Some("/assets/x.css"));

        assert!(page.starts_with("<!doctype html>"));
        assert!(page.contains("\n<b onclick=\"go()\">Hi</b>\n"));
        assert!(page.contains("<link rel=\"stylesheet\" href=\"/assets/x.css\">"));
        assert!(page.contains("<base href=\"/\" target=\"_top\">"));
        assert!(!document("x", None).contains("stylesheet"));
    }

    #[test]
    fn document_sends_the_known_pop_ups_to_the_page_only() {
        let page = document("<a href=\"#new-request\">Search</a>", None);

        assert!(page.contains("organizrPopup"));
        for name in [
            "new-request",
            "mediaSearch-area",
            "speedtest-area",
            "account-area",
            "invite-area",
        ] {
            assert!(POPUP_SCRIPT.contains(&format!("'{name}'")), "{name}");
        }
    }

    #[test]
    fn policy_makes_the_document_a_page_of_no_site() {
        let sandbox = POLICY.split(';').next().unwrap_or_default();
        assert!(sandbox.starts_with("sandbox "));
        assert!(sandbox.contains("allow-scripts"));
        // With this word the scripts could act as the person who looks at the page.
        assert!(!POLICY.contains("allow-same-origin"));
        assert!(!sandbox.contains("allow-top-navigation "));
        assert!(POLICY.contains("frame-ancestors 'self'"));
    }
}
