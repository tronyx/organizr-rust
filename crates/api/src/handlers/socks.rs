//! The API SOCKS: `/socks/{app}/{route:.*}` and
//! `/multiple/socks/{app}/{instance}/{route:.*}`.
//!
//! Organizr passes a request on to Sonarr, Radarr, Lidarr, SABnzbd, NZBGet, Tautulli or
//! qBittorrent at the address in the settings, so that a script can reach the API of
//! the application without a reverse proxy. The answer holds the body of the answer of
//! the application, always with the status 200 and the content type of JSON, as in the
//! PHP application.
//!
//! The caller brings the key of the application, as in the PHP application: in the
//! header `X-Api-Key` for Sonarr, Radarr and Lidarr, in the header `Authorization` for
//! NZBGet, or in the query. Organizr never adds the key of its settings. It sends no
//! cookie and no other header of the caller, only the key header and `Content-Type`.
//!
//! An address in the settings that is only a path, such as `/sonarr`, is a path on the
//! host of the request, as `qualifyURL` of the PHP application makes it.

use axum::Extension;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use organizr_auth::api_key::is_api_key;
use organizr_integrations::socks::{self, Forward, PathError, SocksError};
use organizr_settings::Settings;
use serde_json::Value;

use crate::caller::Caller;
use crate::client::Client;
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::{is_empty, level_of, no_requests, service_url};
use crate::router::API_BASE;
use crate::state::AppState;

/// The most bytes of the body of a request.
pub const BODY_LIMIT: usize = 2 * 1024 * 1024;

/// An application that the API SOCKS reaches: `socksListing` of the PHP application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct App {
    /// The name for the log.
    name: &'static str,
    /// The setting with the address, or the addresses with commas between them.
    url: &'static str,
    /// The switch of the API SOCKS of the application.
    enabled: &'static str,
    /// The setting with the level that may use it.
    auth: &'static str,
    /// The header of the caller that carries the key of the application.
    header: Option<&'static str>,
    /// The switch that leaves the certificate of the application unchecked.
    never_check: &'static str,
}

/// The application of a name in the path. The names are those of the PHP application,
/// in lower case.
fn listing(name: &str) -> Option<App> {
    let app = |name, url, enabled, auth, header, never_check| App {
        name,
        url,
        enabled,
        auth,
        header,
        never_check,
    };
    Some(match name {
        "sonarr" => app(
            "Sonarr",
            "sonarrURL",
            "sonarrSocksEnabled",
            "sonarrSocksAuth",
            Some("X-Api-Key"),
            "sonarrDisableCertCheck",
        ),
        "radarr" => app(
            "Radarr",
            "radarrURL",
            "radarrSocksEnabled",
            "radarrSocksAuth",
            Some("X-Api-Key"),
            "radarrDisableCertCheck",
        ),
        "lidarr" => app(
            "Lidarr",
            "lidarrURL",
            "lidarrSocksEnabled",
            "lidarrSocksAuth",
            Some("X-Api-Key"),
            "lidarrDisableCertCheck",
        ),
        "sabnzbd" => app(
            "SABnzbd",
            "sabnzbdURL",
            "sabnzbdSocksEnabled",
            "sabnzbdSocksAuth",
            None,
            "sabnzbdDisableCertCheck",
        ),
        // The user name and the password of NZBGet, as the caller sends them. The other
        // applications never get the `Authorization` header of the caller.
        "nzbget" => app(
            "NZBGet",
            "nzbgetURL",
            "nzbgetSocksEnabled",
            "nzbgetSocksAuth",
            Some("Authorization"),
            "nzbgetDisableCertCheck",
        ),
        "tautulli" => app(
            "Tautulli",
            "tautulliURL",
            "tautulliSocksEnabled",
            "tautulliSocksAuth",
            None,
            "tautulliDisableCertCheck",
        ),
        "qbittorrent" => app(
            "qBittorrent",
            "qBittorrentURL",
            "qBittorrentSocksEnabled",
            "qBittorrentSocksAuth",
            None,
            "qBittorrentDisableCertCheck",
        ),
        _ => return None,
    })
}

/// The parts of the path of a request, as they came, with their `%xx` codes.
#[derive(Debug, PartialEq, Eq)]
struct Parts<'a> {
    app: &'a str,
    /// The instance of the multiple form.
    instance: Option<&'a str>,
    /// The rest of the path, after the slash that ends the app or the instance.
    route: &'a str,
}

/// Takes the path of a request apart.
fn parts(path: &str) -> Option<Parts<'_>> {
    let rest = path.strip_prefix(API_BASE)?;
    let (rest, multiple) = match rest.strip_prefix("/socks/") {
        Some(rest) => (rest, false),
        None => (rest.strip_prefix("/multiple/socks/")?, true),
    };
    let (app, rest) = rest.split_once('/')?;
    let (instance, route) = if multiple {
        let (instance, route) = rest.split_once('/')?;
        (Some(instance), route)
    } else {
        (None, rest)
    };
    Some(Parts {
        app,
        instance,
        route,
    })
}

/// A part of the path with its `%xx` codes read, or nothing for bytes that are no
/// UTF-8.
fn decoded(part: &str) -> Option<String> {
    let path = socks::route_path(part).ok()?;
    let text = path.strip_prefix('/')?;
    // One part, without codes that are left.
    (!text.contains(['/', '%'])).then(|| text.to_owned())
}

/// A setting as text.
fn text_of(settings: &Settings, key: &str) -> String {
    match settings.get(key) {
        Some(Value::String(text)) => text,
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

/// Whether PHP takes the text of the instance for "nothing": the empty text and `0`.
fn is_nothing(instance: Option<&str>) -> bool {
    instance.is_none_or(|text| text.is_empty() || text == "0")
}

/// The refusal of the PHP application for the settings and the caller. As in the PHP
/// application, each check replaces the answer of the one before, so the last one that
/// fails gives the answer.
fn refusal(app: &App, settings: &Settings, caller: &Caller, multiple: bool) -> Option<Refusal> {
    let mut refusal = None;
    if is_empty(settings.get(app.enabled).as_ref()) {
        refusal = Some(Refusal::conflict("SOCKS module is not enabled"));
    }
    if u32::from(caller.level().get()) > level_of(settings.get(app.auth).as_ref()) {
        refusal = Some(Refusal::Request(
            StatusCode::UNAUTHORIZED,
            "Not Authorized".to_owned(),
        ));
    }
    let has_comma = text_of(settings, app.url).contains(',');
    if has_comma && !multiple {
        refusal = Some(Refusal::conflict(
            "Multiple URLs found in field, please use /api/v2/multiple/socks endpoint",
        ));
    }
    if !has_comma && multiple {
        refusal = Some(Refusal::conflict(
            "Multiple endpoint accessed but multiple URLs not found in field, please use \
             /api/v2/socks endpoint",
        ));
    }
    refusal
}

/// The query for the application: the fields of the query of the caller, without the
/// field `apikey` when it holds the API key of Organizr.
fn query_for_app(query: Option<&str>, settings: &Settings) -> String {
    let Some(query) = query.filter(|query| !query.is_empty()) else {
        return String::new();
    };
    let mut out = form_urlencoded::Serializer::new(String::new());
    for (name, value) in form_urlencoded::parse(query.as_bytes()) {
        if name == "apikey" && is_api_key(&value, &settings.api_key) {
            continue;
        }
        out.append_pair(&name, &value);
    }
    out.finish()
}

/// The method for the application, and whether a body goes with it, as the PHP
/// application decides: GET, POST, PUT and DELETE go as they are, and every other method
/// as GET. A POST takes a body of JSON or of a form, a PUT one of JSON.
fn method_for_app(method: &Method, content_type: &str) -> (Method, bool) {
    let content_type = content_type.to_ascii_lowercase();
    let json = content_type.contains("application/json");
    let form = content_type
        .trim_start()
        .starts_with("application/x-www-form-urlencoded");
    match *method {
        Method::POST => (Method::POST, json || form),
        Method::PUT => (Method::PUT, json),
        Method::DELETE => (Method::DELETE, false),
        _ => (Method::GET, false),
    }
}

/// The address of the request to the application: the address of the settings, or of
/// the instance of the multiple form, with the path and the query of the caller.
fn url_for_app(
    app: &App,
    settings: &Settings,
    parts: &Parts<'_>,
    uri: &Uri,
    headers: &HeaderMap,
    client: &Client,
) -> Result<String, Refusal> {
    let addresses = text_of(settings, app.url);
    // The multiple form with the instance `0` gets here only without commas.
    let address = if parts.instance.is_some() {
        let place = parts
            .instance
            .filter(|text| text.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|text| text.parse::<usize>().ok())
            .and_then(|number| number.checked_sub(1));
        place
            .and_then(|place| addresses.split(',').nth(place))
            .ok_or_else(|| Refusal::conflict("Instance not found in field"))?
            .to_owned()
    } else {
        addresses
    };
    let address: String = address.chars().filter(|c| !c.is_whitespace()).collect();
    // A path such as `/sonarr` gets the scheme and the host of the request in front, as
    // in the PHP application and on the rest of the homepage.
    let base = service_url(&address, headers, client);
    let climbs = || {
        Refusal::Request(
            StatusCode::BAD_REQUEST,
            "The path must not leave the address of the application".to_owned(),
        )
    };
    let path = socks::route_path(parts.route).map_err(|_| climbs())?;
    let query = query_for_app(uri.query(), settings);
    socks::target(&base, &path, &query).map_err(|error| match error {
        PathError::Climbs => climbs(),
        PathError::Address => Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            "The address of the application is not valid".to_owned(),
        ),
    })
}

/// `ANY /socks/{app}/{route:.*}` and `ANY /multiple/socks/{app}/{instance}/{route:.*}`.
pub async fn socks(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, Refusal> {
    let not_supported = || Refusal::not_found("Application not supported for socks");
    let parts = parts(uri.path()).ok_or_else(not_supported)?;
    let app = decoded(parts.app)
        .and_then(|name| listing(&name))
        .ok_or_else(not_supported)?;
    let settings = state.settings();
    let multiple = !is_nothing(parts.instance);
    if let Some(refusal) = refusal(&app, &settings, &caller, multiple) {
        return Err(refusal);
    }
    let url = url_for_app(&app, &settings, &parts, &uri, &headers, &client)?;

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok());
    let (sent_method, with_body) = method_for_app(&method, content_type.unwrap_or_default());
    let bytes = axum::body::to_bytes(body, BODY_LIMIT).await.map_err(|_| {
        Refusal::Request(
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("The body of a SOCKS request has {BODY_LIMIT} bytes at most"),
        )
    })?;
    let mut sent_headers = Vec::new();
    if let Some(name) = app.header
        && let Some(value) = headers.get(name).and_then(|value| value.to_str().ok())
    {
        sent_headers.push((name, value.to_owned()));
    }
    if let Some(content_type) = content_type {
        sent_headers.push(("Content-Type", content_type.to_owned()));
    }

    let http = state.outbound().ok_or_else(no_requests)?;
    let request = Forward {
        method: sent_method.clone(),
        url: &url,
        headers: sent_headers,
        body: with_body.then(|| bytes.to_vec()),
        never_check_certificate: !is_empty(settings.get(app.never_check).as_ref()),
    };
    let answer = socks::forward(http, request).await.map_err(|error| {
        tracing::warn!(app = app.name, method = %sent_method, %error, "a SOCKS request failed");
        let message = match error {
            SocksError::Unreachable(text) => text,
            SocksError::TooLarge => error.to_string(),
        };
        Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, message)
    })?;
    tracing::debug!(
        app = app.name,
        method = %sent_method,
        status = answer.status,
        bytes = answer.body.len(),
        "a SOCKS request was passed on"
    );

    let mut response = Body::from(answer.body).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json;charset=UTF-8"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use axum::http::Method;
    use organizr_settings::Settings;

    use super::{Parts, listing, method_for_app, parts, query_for_app};

    #[test]
    fn path_is_taken_apart_as_it_came() {
        assert_eq!(
            parts("/api/v2/socks/sonarr/api/v3/queue"),
            Some(Parts {
                app: "sonarr",
                instance: None,
                route: "api/v3/queue"
            })
        );
        assert_eq!(
            parts("/api/v2/multiple/socks/radarr/2/api/%2e%2e"),
            Some(Parts {
                app: "radarr",
                instance: Some("2"),
                route: "api/%2e%2e"
            })
        );
        assert_eq!(
            parts("/api/v2/socks/sonarr/"),
            Some(Parts {
                app: "sonarr",
                instance: None,
                route: ""
            })
        );
        assert_eq!(parts("/api/v2/socks/sonarr"), None);
    }

    #[test]
    fn listing_has_the_seven_applications_of_php() {
        for name in [
            "sonarr",
            "radarr",
            "lidarr",
            "sabnzbd",
            "nzbget",
            "tautulli",
            "qbittorrent",
        ] {
            let app = listing(name).expect("the application is listed");
            for key in [app.url, app.enabled, app.auth, app.never_check] {
                assert!(Settings::KEYS.contains(&key), "{key}");
            }
        }
        assert_eq!(listing("Sonarr"), None);
        assert_eq!(listing("plex"), None);
    }

    #[test]
    fn method_and_body_go_as_php_sends_them() {
        let json = "application/json; charset=utf-8";
        let form = "application/x-www-form-urlencoded";
        assert_eq!(method_for_app(&Method::POST, json), (Method::POST, true));
        assert_eq!(method_for_app(&Method::POST, form), (Method::POST, true));
        assert_eq!(
            method_for_app(&Method::POST, "text/plain"),
            (Method::POST, false)
        );
        assert_eq!(method_for_app(&Method::PUT, json), (Method::PUT, true));
        assert_eq!(method_for_app(&Method::PUT, form), (Method::PUT, false));
        assert_eq!(
            method_for_app(&Method::DELETE, json),
            (Method::DELETE, false)
        );
        assert_eq!(method_for_app(&Method::PATCH, json), (Method::GET, false));
        assert_eq!(method_for_app(&Method::OPTIONS, ""), (Method::GET, false));
        assert_eq!(method_for_app(&Method::GET, json), (Method::GET, false));
    }

    #[test]
    fn api_key_of_organizr_stays_out_of_the_query() {
        let mut settings = Settings::defaults();
        settings.api_key = "abcdefghij0123456789".to_owned();
        assert_eq!(
            query_for_app(Some("apikey=abcdefghij0123456789&mode=queue"), &settings),
            "mode=queue"
        );
        assert_eq!(
            query_for_app(Some("apikey=key-of-sabnzbd&mode=queue"), &settings),
            "apikey=key-of-sabnzbd&mode=queue"
        );
        assert_eq!(query_for_app(Some("q=a b&x=%2F"), &settings), "q=a+b&x=%2F");
        assert_eq!(query_for_app(None, &settings), "");
    }
}
