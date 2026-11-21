//! The Ombi item: the requests for films and series, and what a person can do with
//! them.

use std::time::Duration;

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use organizr_domain::GroupLevel;
use organizr_integrations::http::time_limit;
use organizr_integrations::ombi::{
    As, Install, OmbiError, Viewer, film_rows, newest_first, series_rows,
};
use organizr_settings::Settings;
use serde_json::{Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::client::Client;
use crate::cookies::cookie_value;
use crate::envelope::{Envelope, NoData};
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// How long Ombi has to answer a change of a request, as in the PHP application.
const CHANGE_LIMIT: Duration = Duration::from_secs(60);

/// What must hold before a caller sees the requests.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageOmbiEnabled".to_owned()],
        auth: vec!["homepageOmbiAuth".to_owned()],
        not_empty: vec!["ombiURL".to_owned(), "ombiToken".to_owned()],
    }
}

/// What must hold before a caller makes a request.
pub fn request_permissions() -> Permissions {
    let mut permissions = permissions();
    permissions.auth.push("homepageOmbiRequestAuth".to_owned());
    permissions
}

/// The settings that say how the interface shows the item.
pub fn options(settings: &Settings) -> Value {
    json!({
        "alias": settings.ombi_alias,
        "filters": {
            "available": settings.ombi_default_filter_available,
            "unavailable": settings.ombi_default_filter_unavailable,
            "approved": settings.ombi_default_filter_approved,
            "unapproved": settings.ombi_default_filter_unapproved,
            "denied": settings.ombi_default_filter_denied,
        },
    })
}

fn install(
    settings: &Settings,
    headers: &HeaderMap,
    client: &Client,
    timeout: Option<Duration>,
) -> Install {
    Install {
        url: service_url(&settings.ombi_url, headers, client),
        key: settings.ombi_token.clone(),
        timeout,
        never_check_certificate: settings.ombi_disable_cert_check,
    }
}

/// The answer when Ombi cannot be reached.
fn unreachable(error: &OmbiError) -> Refusal {
    tracing::warn!(%error, "Ombi gave no answer");
    Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn failed(message: &str) -> Refusal {
    Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, message.to_owned())
}

/// The answer after Ombi refused a request of this application.
fn refused() -> Refusal {
    failed("Ombi Error Occurred")
}

/// A number of a path, or nothing for a text that is no number. A text that is no
/// number could name another path of Ombi.
fn number(text: &str) -> Option<u64> {
    (text.len() <= 12 && text.chars().all(|digit| digit.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

/// The kind of a request for a path of Ombi. As in the PHP application, every kind
/// that is no series is a film.
fn kind(text: &str) -> &'static str {
    match text {
        "tv" | "season" => "tv",
        _ => "movie",
    }
}

/// A whole number of a path as PHP reads it: a text that is no number is 0. A number
/// below 0 is 0 too.
fn whole(text: &str) -> usize {
    text.trim().parse().unwrap_or(0)
}

async fn list(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    headers: &HeaderMap,
    (kind, limit, offset): (&str, Option<&str>, Option<&str>),
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, caller)?;
    let http = &state
        .outbound_for(settings.ombi_use_custom_certificate)
        .ok_or_else(no_requests)?;
    let limit_of_settings = time_limit(&settings.ombi_refresh.to_string());
    let install = install(&settings, headers, client, limit_of_settings);
    let limit = limit.map_or_else(
        || usize::try_from(settings.ombi_limit).unwrap_or(usize::MAX),
        whole,
    );
    let offset = offset.map_or(0, whole);
    // A person sees all requests when the list is not limited, and an Admin or a
    // Co-Admin always.
    let viewer = Viewer {
        sees_all: !settings.ombi_limit_user || caller.level() <= GroupLevel::CO_ADMIN,
        name: caller
            .user()
            .map_or("", |user| user.authenticated.user.username.as_str()),
        fallback: &settings.ombi_fallback_user,
    };

    let read = async |path: &str| {
        let reply = install
            .get(http, path)
            .await
            .map_err(|error| unreachable(&error))?;
        Ok::<_, Refusal>(if reply.is_success {
            reply.body
        } else {
            Value::Null
        })
    };
    let films = if kind == "tv" {
        Vec::new()
    } else {
        film_rows(&read("v1/Request/movie").await?, viewer)
    };
    let series = if kind == "movie" {
        Vec::new()
    } else {
        series_rows(&read("v1/Request/tv").await?, viewer)
    };
    let count = json!({
        "movie": films.len(),
        "tv": series.len(),
        "limit": limit,
        "offset": offset,
    });
    let mut rows = films;
    rows.extend(series);
    newest_first(&mut rows);
    let content: Vec<Value> = rows.into_iter().skip(offset).take(limit).collect();
    Ok(Envelope::success(
        json!({ "count": count, "content": content }),
    ))
}

/// `GET /homepage/ombi/requests`.
pub async fn requests(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    list(&state, &caller, &client, &headers, ("both", None, None)).await
}

/// `GET /homepage/ombi/requests/{type}`: the requests for films with `movie`, the
/// ones for series with `tv`, and all with another word.
pub async fn requests_of_type(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path(kind): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    list(&state, &caller, &client, &headers, (&kind, None, None)).await
}

/// `GET /homepage/ombi/requests/{type}/{limit}`.
pub async fn requests_limited(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path((kind, limit)): Path<(String, String)>,
) -> Result<Envelope<Value>, Refusal> {
    list(
        &state,
        &caller,
        &client,
        &headers,
        (&kind, Some(&limit), None),
    )
    .await
}

/// `GET /homepage/ombi/requests/{type}/{limit}/{offset}`.
pub async fn requests_paged(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path((kind, limit, offset)): Path<(String, String, String)>,
) -> Result<Envelope<Value>, Refusal> {
    list(
        &state,
        &caller,
        &client,
        &headers,
        (&kind, Some(&limit), Some(&offset)),
    )
    .await
}

/// What an Admin or a Co-Admin can do with a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Approve,
    Available,
    Unavailable,
    Deny,
    Delete,
}

async fn act(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    headers: &HeaderMap,
    (kind_text, id): (&str, &str),
    action: Action,
) -> Result<Envelope<NoData>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, caller)?;
    // The PHP application lets everybody who sees the requests change them.
    caller.require(GroupLevel::CO_ADMIN)?;
    let Some(id) = number(id) else {
        return Err(Refusal::unprocessable("Id was not supplied"));
    };
    let kind = kind(kind_text);
    let http = &state
        .outbound_for(settings.ombi_use_custom_certificate)
        .ok_or_else(no_requests)?;
    let install = install(&settings, headers, client, Some(CHANGE_LIMIT));
    let body = json!({ "id": id });
    let (method, path, body, message) = match action {
        Action::Approve => (
            Method::POST,
            format!("v1/Request/{kind}/approve"),
            Some(&body),
            "Ombi Request has been approved",
        ),
        Action::Available => (
            Method::POST,
            format!("v1/Request/{kind}/available"),
            Some(&body),
            "Ombi Request has been marked available",
        ),
        Action::Unavailable => (
            Method::POST,
            format!("v1/Request/{kind}/unavailable"),
            Some(&body),
            "Ombi Request has been marked unavailable",
        ),
        Action::Deny => (
            Method::PUT,
            format!("v1/Request/{kind}/deny"),
            Some(&body),
            "Ombi Request has been denied",
        ),
        Action::Delete => (
            Method::DELETE,
            format!("v1/Request/{kind}/{id}"),
            None,
            "Ombi Request has been deleted",
        ),
    };
    let reply = install
        .send(http, method, &path, body, As::Application)
        .await
        .map_err(|error| unreachable(&error))?;
    if reply.is_success {
        Ok(Envelope::success(NoData).with_message(message))
    } else {
        Err(refused())
    }
}

macro_rules! action_route {
    ($(#[$doc:meta])* $name:ident, $action:expr) => {
        $(#[$doc])*
        pub async fn $name(
            State(state): State<AppState>,
            Extension(caller): Extension<Caller>,
            Extension(client): Extension<Client>,
            headers: HeaderMap,
            Path((kind, id)): Path<(String, String)>,
        ) -> Result<Envelope<NoData>, Refusal> {
            act(&state, &caller, &client, &headers, (&kind, &id), $action).await
        }
    };
}

action_route!(
    /// `POST /homepage/ombi/requests/{type}/{id}/approve`.
    approve,
    Action::Approve
);
action_route!(
    /// `POST /homepage/ombi/requests/{type}/{id}/available`.
    available,
    Action::Available
);
action_route!(
    /// `POST /homepage/ombi/requests/{type}/{id}/unavailable`.
    unavailable,
    Action::Unavailable
);
action_route!(
    /// `PUT /homepage/ombi/requests/{type}/{id}/deny`.
    deny,
    Action::Deny
);
action_route!(
    /// `DELETE /homepage/ombi/requests/{type}/{id}`.
    delete,
    Action::Delete
);

/// Whether a field of an answer of Ombi is true.
fn is_true(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).unwrap_or(false)
}

/// The id of a series in the TVDB, from what Ombi knows about the series. Ombi
/// writes the id as a number or as a text.
fn tvdb_id(series: &Value) -> Option<u64> {
    let id = series.get("theTvDbId")?;
    id.as_u64()
        .or_else(|| id.as_str()?.trim().parse().ok())
        .filter(|id| *id != 0)
}

/// The body of a new request for a series, with the seasons of the setting
/// `ombiTvDefault`.
fn series_request(tvdb_id: u64, default: &str) -> Value {
    json!({
        "tvDbId": tvdb_id,
        "requestAll": default == "all",
        "latestSeason": default == "last",
        "firstSeason": default == "first",
    })
}

/// `POST /homepage/ombi/requests/{type}/{id}`: a new request for a film, or for a
/// series with the seasons of the settings. The id is the one of the film or the
/// series in the TMDB.
pub async fn add_request(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path((kind_text, id)): Path<(String, String)>,
) -> Result<Envelope<NoData>, Refusal> {
    let settings = state.settings();
    request_permissions().check(&settings, &caller)?;
    let Some(id) = number(&id) else {
        return Err(Refusal::unprocessable("Id was not supplied"));
    };
    let kind = kind(&kind_text);
    // The request is made as the user, with the token that single sign-on gave.
    let Some(token) = cookie_value(&headers, "Auth").filter(|token| !token.is_empty()) else {
        return Err(failed("User does not have Auth Cookie"));
    };
    let http = &state
        .outbound_for(settings.ombi_use_custom_certificate)
        .ok_or_else(no_requests)?;
    let install = install(&settings, &headers, &client, None);
    let send = async |method: Method, path: &str, body: Option<&Value>| {
        let reply = install
            .send(http, method, path, body, As::User(token))
            .await
            .map_err(|error| unreachable(&error))?;
        if reply.is_success {
            Ok(reply.body)
        } else {
            Err(refused())
        }
    };

    // What Ombi knows about the film or the series says whether a request is needed.
    let lookup = if kind == "tv" {
        format!("v2/Search/tv/moviedb/{id}")
    } else {
        format!("v1/Search/movie/info/{id}")
    };
    let known = send(Method::GET, &lookup, None).await?;
    let body = if kind == "tv" {
        let tvdb_id =
            tvdb_id(&known).ok_or_else(|| Refusal::unprocessable("Could not get TVDB Id"))?;
        series_request(tvdb_id, &settings.ombi_tv_default)
    } else {
        json!({ "theMovieDbId": id })
    };
    if is_true(&known, "available") {
        return Err(Refusal::conflict("Request is already available"));
    }
    if is_true(&known, "requested") {
        return Err(Refusal::conflict("Request is already requested"));
    }
    send(Method::POST, &format!("v1/Request/{kind}"), Some(&body)).await?;
    Ok(Envelope::success(NoData).with_message("Ombi Request submitted"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{kind, number, series_request, tvdb_id, whole};

    #[test]
    fn parts_of_a_path_are_read_as_the_php_application_reads_them() {
        assert_eq!(number("815"), Some(815));
        for wrong in ["", "8 15", "815/..", "-1", "1234567890123"] {
            assert_eq!(number(wrong), None, "{wrong}");
        }
        assert_eq!(kind("tv"), "tv");
        assert_eq!(kind("season"), "tv");
        assert_eq!(kind("movie"), "movie");
        assert_eq!(kind("../user"), "movie");
        assert_eq!(whole("25"), 25);
        assert_eq!(whole("many"), 0);
        assert_eq!(whole("-5"), 0);
    }

    #[test]
    fn new_request_for_a_series_follows_the_setting() {
        assert_eq!(
            series_request(1234, "last"),
            json!({"tvDbId": 1234, "requestAll": false, "latestSeason": true, "firstSeason": false})
        );
        assert_eq!(series_request(1, "all")["requestAll"], true);
        assert_eq!(series_request(1, "first")["firstSeason"], true);
        assert_eq!(tvdb_id(&json!({"theTvDbId": "1234"})), Some(1234));
        assert_eq!(tvdb_id(&json!({"theTvDbId": 1234})), Some(1234));
        for none in [
            json!({}),
            json!({"theTvDbId": "0"}),
            json!({"theTvDbId": null}),
        ] {
            assert_eq!(tvdb_id(&none), None, "{none}");
        }
    }
}
