//! The Overseerr item: the requests for films and series, and what a person can do
//! with them.

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use organizr_domain::GroupLevel;
use organizr_integrations::http::{Outbound, time_limit};
use organizr_integrations::overseerr::{As, Install, OverseerrError, Reply, default_service};
use organizr_settings::Settings;
use serde_json::{Map, Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::client::Client;
use crate::cookies::cookie_value;
use crate::envelope::{Envelope, NoData};
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// How many requests for details run at the same time.
const AT_ONCE: usize = 8;

/// What must hold before a caller sees the requests.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageOverseerrEnabled".to_owned()],
        auth: vec!["homepageOverseerrAuth".to_owned()],
        not_empty: vec!["overseerrURL".to_owned(), "overseerrToken".to_owned()],
    }
}

/// What must hold before a caller makes a request, or reads the details for one.
pub fn request_permissions() -> Permissions {
    let mut permissions = permissions();
    permissions
        .auth
        .push("homepageOverseerrRequestAuth".to_owned());
    permissions
}

fn install(settings: &Settings, headers: &HeaderMap, client: &Client, with_limit: bool) -> Install {
    Install {
        url: service_url(&settings.overseerr_url, headers, client),
        key: settings.overseerr_token.clone(),
        timeout: with_limit
            .then(|| time_limit(&settings.overseerr_refresh.to_string()))
            .flatten(),
        never_check_certificate: settings.overseerr_disable_cert_check,
    }
}

/// The client for the requests to Overseerr.
fn outbound(state: &AppState, settings: &Settings) -> Result<Outbound, Refusal> {
    state
        .outbound_for(settings.overseerr_use_custom_certificate)
        .ok_or_else(no_requests)
}

/// The answer when Overseerr cannot be reached.
fn unreachable(error: &OverseerrError) -> Refusal {
    tracing::warn!(%error, "Overseerr gave no answer");
    Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn failed(message: &str) -> Refusal {
    Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, message.to_owned())
}

/// The answer after Overseerr refused a change: its own message, when it sent one.
fn refused(reply: &Reply) -> Refusal {
    failed(reply.message().unwrap_or("Overseerr Error Occurred"))
}

/// A number of a path, or nothing for a text that is no number. A text that is no
/// number could name another path of Overseerr.
fn number(text: &str) -> Option<&str> {
    (!text.is_empty() && text.len() <= 12 && text.chars().all(|digit| digit.is_ascii_digit()))
        .then_some(text)
}

/// The kind of a request, when it is one that Overseerr has a path for.
fn kind(text: &str) -> Option<&'static str> {
    match text {
        "movie" => Some("movie"),
        "tv" => Some("tv"),
        _ => None,
    }
}

fn text_of<'a>(value: &'a Value, path: &[&str]) -> &'a str {
    path.iter()
        .try_fold(value, |value, key| value.get(key))
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn number_of(value: &Value, path: &[&str]) -> Option<i64> {
    path.iter()
        .try_fold(value, |value, key| value.get(key))
        .and_then(Value::as_i64)
}

/// One request of the list of Overseerr in the form of the PHP application, without
/// the details of its film or series. Nothing for a request that the caller does not
/// see, or that names no film or series.
fn request_row(
    entry: &Value,
    sees_all: bool,
    caller_name: &str,
    fallback: &str,
) -> Option<(i64, &'static str, Map<String, Value>)> {
    let by = entry.get("requestedBy").unwrap_or(&Value::Null);
    let requester = match text_of(by, &["username"]) {
        "" => text_of(by, &["plexUsername"]),
        name => name,
    };
    let own = !caller_name.is_empty() && requester.eq_ignore_ascii_case(caller_name);
    let of_fallback = !fallback.is_empty() && requester.eq_ignore_ascii_case(fallback);
    if !(sees_all || own || of_fallback) {
        return None;
    }
    let kind = kind(text_of(entry, &["type"]))?;
    let id = number_of(entry, &["media", "tmdbId"])?;
    let is_film = kind == "movie";

    let mut row = Map::new();
    for (key, value) in [
        ("id", json!(id)),
        ("approved", json!(number_of(entry, &["status"]) == Some(2))),
        (
            "available",
            json!(number_of(entry, &["media", "status"]) == Some(5)),
        ),
        ("denied", json!(number_of(entry, &["status"]) == Some(3))),
        ("deniedReason", json!("n/a")),
        ("user", json!(requester)),
        (
            "userAlias",
            by.get("displayName").cloned().unwrap_or(Value::Null),
        ),
        (
            "request_id",
            entry.get("id").cloned().unwrap_or(Value::Null),
        ),
        (
            "request_date",
            entry.get("createdAt").cloned().unwrap_or(Value::Null),
        ),
        ("type", json!(kind)),
        // The PHP application sends this icon for both kinds.
        ("icon", json!("filmstrip")),
        (
            "color",
            json!(if is_film {
                "palette-Deep-Purple-900 bg white"
            } else {
                "grayish-blue-bg"
            }),
        ),
    ] {
        row.insert(key.to_owned(), value);
    }
    Some((id, kind, row))
}

/// Adds what Overseerr knows about the film or series to a row.
fn add_details(row: &mut Map<String, Value>, details: &Value) {
    let first = |fields: [&str; 2]| {
        fields
            .iter()
            .find_map(|field| details.get(field).filter(|value| !value.is_null()))
            .cloned()
            .unwrap_or(Value::Null)
    };
    let picture = |field: &str, size: &str, none: &str| match text_of(details, &[field]) {
        "" => none.to_owned(),
        path => format!("https://image.tmdb.org/t/p/{size}/{path}"),
    };
    row.insert("title".to_owned(), first(["title", "name"]));
    row.insert(
        "release_date".to_owned(),
        first(["releaseDate", "firstAirDate"]),
    );
    row.insert(
        "background".to_owned(),
        json!(picture("backdropPath", "w1280", "")),
    );
    row.insert(
        "poster".to_owned(),
        json!(picture(
            "posterPath",
            "w300",
            "plugins/images/homepage/no-list.png"
        )),
    );
    row.insert(
        "overview".to_owned(),
        details.get("overview").cloned().unwrap_or(Value::Null),
    );
}

/// The details of several films and series, some at the same time. An entry is
/// nothing when Overseerr gave no details.
async fn details_of(
    http: &Outbound,
    install: &Install,
    wanted: &[(i64, &'static str)],
) -> Vec<Option<Value>> {
    let mut all = Vec::with_capacity(wanted.len());
    for group in wanted.chunks(AT_ONCE) {
        let mut tasks = tokio::task::JoinSet::new();
        for (place, (id, kind)) in group.iter().enumerate() {
            let (http, install, path) = (http.clone(), install.clone(), format!("{kind}/{id}"));
            tasks.spawn(async move { (place, install.get(&http, &path).await) });
        }
        let mut found: Vec<Option<Value>> = vec![None; group.len()];
        while let Some(done) = tasks.join_next().await {
            if let Ok((place, Ok(reply))) = done
                && reply.is_success
                && let Some(slot) = found.get_mut(place)
            {
                *slot = Some(reply.body);
            }
        }
        all.extend(found);
    }
    all
}

/// The paging of the list: the limit and the offset of the path, or the defaults.
fn paging(settings: &Settings, limit: Option<&str>, offset: Option<&str>) -> (i64, i64) {
    let whole = |text: Option<&str>| text.and_then(|text| text.trim().parse::<i64>().ok());
    (
        whole(limit).unwrap_or_else(|| i64::from(settings.overseerr_limit)),
        whole(offset).unwrap_or(0),
    )
}

async fn list(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    headers: &HeaderMap,
    limit: Option<&str>,
    offset: Option<&str>,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, caller)?;
    let http = &outbound(state, &settings)?;
    let install = install(&settings, headers, client, true);
    let (limit, offset) = paging(&settings, limit, offset);
    let (mut films, mut series) = (0_u32, 0_u32);
    let mut rows: Vec<(i64, &'static str, Map<String, Value>)> = Vec::new();

    let reply = install
        .get(http, &format!("request?take={limit}&skip={offset}"))
        .await
        .map_err(|error| unreachable(&error))?;
    if reply.is_success {
        // A person sees all requests when the list is not limited, and an Admin or a
        // Co-Admin always.
        let sees_all = !settings.overseerr_limit_user || caller.level() <= GroupLevel::CO_ADMIN;
        let name = caller
            .user()
            .map_or("", |user| user.authenticated.user.username.as_str());
        let entries = reply.body.get("results").and_then(Value::as_array);
        for entry in entries.into_iter().flatten() {
            let Some((id, kind, row)) =
                request_row(entry, sees_all, name, &settings.overseerr_fallback_user)
            else {
                continue;
            };
            if kind == "movie" {
                films += 1;
            } else {
                series += 1;
            }
            // A second request for the same film or series takes the place of the first.
            match rows.iter_mut().find(|(known, _, _)| *known == id) {
                Some(known) => *known = (id, kind, row),
                None => rows.push((id, kind, row)),
            }
        }
    }

    let wanted: Vec<(i64, &'static str)> = rows.iter().map(|(id, kind, _)| (*id, *kind)).collect();
    let details = details_of(http, &install, &wanted).await;
    let mut content: Vec<Map<String, Value>> = rows
        .into_iter()
        .zip(details)
        .filter_map(|((_, _, mut row), details)| {
            add_details(&mut row, &details?);
            Some(row)
        })
        .collect();
    // The newest request first.
    content.sort_by(|a, b| {
        let date = |row: &Map<String, Value>| {
            row.get("request_date")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        date(b).cmp(&date(a))
    });
    Ok(Envelope::success(json!({
        "count": { "movie": films, "tv": series, "limit": limit, "offset": offset },
        "content": content,
    })))
}

/// `GET /homepage/overseerr/requests`.
pub async fn requests(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    list(&state, &caller, &client, &headers, None, None).await
}

/// `GET /homepage/overseerr/requests/{type}`: as without the type, which says nothing.
pub async fn requests_of_type(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path(_kind): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    list(&state, &caller, &client, &headers, None, None).await
}

/// `GET /homepage/overseerr/requests/{type}/{limit}`.
pub async fn requests_limited(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path((_kind, limit)): Path<(String, String)>,
) -> Result<Envelope<Value>, Refusal> {
    list(&state, &caller, &client, &headers, Some(&limit), None).await
}

/// `GET /homepage/overseerr/requests/{type}/{limit}/{offset}`.
pub async fn requests_paged(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path((_kind, limit, offset)): Path<(String, String, String)>,
) -> Result<Envelope<Value>, Refusal> {
    list(
        &state,
        &caller,
        &client,
        &headers,
        Some(&limit),
        Some(&offset),
    )
    .await
}

/// `GET /homepage/overseerr/metadata/{type}/{id}`: what Overseerr knows about a film
/// or a series, as Overseerr gives it.
pub async fn metadata(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path((kind_text, id)): Path<(String, String)>,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    request_permissions().check(&settings, &caller)?;
    let (Some(kind), Some(id)) = (kind(&kind_text), number(&id)) else {
        return Err(Refusal::unprocessable("Type was not supplied"));
    };
    let http = &outbound(&state, &settings)?;
    let reply = install(&settings, &headers, &client, false)
        .get(http, &format!("{kind}/{id}"))
        .await
        .map_err(|error| unreachable(&error))?;
    if reply.is_success {
        Ok(Envelope::success(reply.body))
    } else {
        Err(failed("Error getting series information"))
    }
}

/// What an Admin or a Co-Admin can do with a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Approve,
    Pending,
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
    id: &str,
    action: Action,
) -> Result<Envelope<NoData>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, caller)?;
    // The PHP application lets everybody who sees the requests change them.
    caller.require(GroupLevel::CO_ADMIN)?;
    let Some(id) = number(id) else {
        return Err(Refusal::unprocessable("Id was not supplied"));
    };
    let http = &outbound(state, &settings)?;
    let install = install(&settings, headers, client, false);
    let send = async |method: Method, path: String| {
        install
            .send(http, method, &path, None, As::Application)
            .await
            .map_err(|error| unreachable(&error))
    };
    // "Available" and "unavailable" change the film or series of the request.
    let media = async || {
        let reply = send(Method::GET, format!("request/{id}")).await?;
        number_of(&reply.body, &["media", "id"])
            .filter(|_| reply.is_success)
            .ok_or_else(|| failed("Error getting request information"))
    };
    let (reply, message) = match action {
        Action::Approve => (
            send(Method::POST, format!("request/{id}/approve")).await?,
            "Overseerr Request has been approved",
        ),
        // The PHP application has this text for "pending" too.
        Action::Pending => (
            send(Method::POST, format!("request/{id}/pending")).await?,
            "Overseerr Request has been approved",
        ),
        Action::Available => (
            send(Method::POST, format!("media/{}/available", media().await?)).await?,
            "Overseerr Request has been marked available",
        ),
        Action::Unavailable => (
            send(Method::POST, format!("media/{}/pending", media().await?)).await?,
            "Overseerr Request has been marked unavailable",
        ),
        Action::Deny => (
            send(Method::POST, format!("request/{id}/decline")).await?,
            "Overseerr Request has been denied",
        ),
        Action::Delete => (
            send(Method::DELETE, format!("request/{id}")).await?,
            "Overseerr Request has been deleted",
        ),
    };
    if reply.is_success {
        Ok(Envelope::success(NoData).with_message(message))
    } else {
        Err(refused(&reply))
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
            Path((_kind, id)): Path<(String, String)>,
        ) -> Result<Envelope<NoData>, Refusal> {
            act(&state, &caller, &client, &headers, &id, $action).await
        }
    };
}

action_route!(
    /// `POST /homepage/overseerr/requests/{type}/{id}/approve`.
    approve,
    Action::Approve
);
action_route!(
    /// `POST /homepage/overseerr/requests/{type}/{id}/pending`.
    pending,
    Action::Pending
);
action_route!(
    /// `POST /homepage/overseerr/requests/{type}/{id}/available`.
    available,
    Action::Available
);
action_route!(
    /// `POST /homepage/overseerr/requests/{type}/{id}/unavailable`.
    unavailable,
    Action::Unavailable
);
action_route!(
    /// `PUT /homepage/overseerr/requests/{type}/{id}/deny`.
    deny,
    Action::Deny
);
action_route!(
    /// `DELETE /homepage/overseerr/requests/{type}/{id}`.
    delete,
    Action::Delete
);

/// The seasons of a new request for a series, from the setting `overseerrTvDefault`.
fn seasons_for(setting: &str, asked: Option<&str>, series: &Value) -> Result<Value, Refusal> {
    let numbers = || {
        series
            .get("seasons")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|season| season.get("seasonNumber").and_then(Value::as_i64))
    };
    match setting {
        "first" => Ok(json!([1])),
        "last" => Ok(json!([numbers().next_back()])),
        "all" => Ok(json!(
            numbers().filter(|number| *number != 0).collect::<Vec<_>>()
        )),
        "user" => {
            let asked = asked.filter(|asked| !asked.is_empty());
            let asked = asked.ok_or_else(|| failed("Seasons requested was not supplied"))?;
            let wanted: Vec<i64> = asked
                .split(',')
                .map(|season| season.trim().parse().unwrap_or(0))
                .collect();
            Ok(json!(wanted))
        }
        // Another value of the setting: what the caller named, or nothing.
        _ => Ok(asked.map_or(Value::Null, |asked| json!(asked))),
    }
}

async fn add(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    headers: &HeaderMap,
    (kind_text, id, seasons): (&str, &str, Option<&str>),
) -> Result<Envelope<NoData>, Refusal> {
    let settings = state.settings();
    request_permissions().check(&settings, caller)?;
    let Some(id) = number(id) else {
        return Err(Refusal::unprocessable("Id was not supplied"));
    };
    let is_series = match kind_text {
        "tv" | "season" => true,
        "movie" => false,
        _ => return Err(Refusal::unprocessable("Type was not supplied")),
    };
    // The request is made as the user, with the session that single sign-on gave.
    let Some(session) = cookie_value(headers, "connect.sid").filter(|session| !session.is_empty())
    else {
        return Err(failed("User does not have Auth Cookie"));
    };
    let http = &outbound(state, &settings)?;
    let install = install(&settings, headers, client, false);
    let read = async |path: &str, problem: &'static str| {
        let reply = install
            .get(http, path)
            .await
            .map_err(|error| unreachable(&error))?;
        if reply.is_success {
            Ok(reply.body)
        } else {
            Err(failed(problem))
        }
    };

    let known = read("request/", "Overseerr Connection Error Occurred").await?;
    let id_number: i64 = id.parse().unwrap_or(0);
    for entry in known
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if number_of(entry, &["media", "tmdbId"]) == Some(id_number) {
            let is_there = number_of(entry, &["media", "status"]) == Some(5);
            return Err(Refusal::conflict(if is_there {
                "Request is already available"
            } else {
                "Request is already requested"
            }));
        }
    }
    let me = install
        .send(http, Method::GET, "auth/me", None, As::User(session))
        .await
        .map_err(|error| unreachable(&error))?;
    if !me.is_success {
        return Err(failed("Error getting user information"));
    }

    let whole = |service: &Value, field: &str| number_of(service, &[field]).unwrap_or(0);
    let body = if is_series {
        let series = read(&format!("tv/{id}"), "Error getting series information").await?;
        let seasons = seasons_for(&settings.overseerr_tv_default, seasons, &series)?;
        let services = read("service/sonarr", "Error getting service information").await?;
        let service = default_service(&services, settings.overseerr_prefer4_k)
            .ok_or_else(|| Refusal::not_found("No Sonarr service was found in Overseerr"))?;
        json!({
            "mediaId": id_number,
            "tvdbId": series.get("externalIds").and_then(|ids| ids.get("tvdbId")),
            "mediaType": "tv",
            "is4k": service.get("is4k").and_then(Value::as_bool).unwrap_or(false),
            "seasons": seasons,
            "serverId": whole(service, "id"),
            "profileId": whole(service, "activeProfileId"),
            "rootFolder": service.get("activeDirectory"),
            "languageProfileId": whole(service, "activeLanguageProfileId"),
            "tags": [],
        })
    } else {
        let services = read("service/radarr", "Error getting service information").await?;
        let service = default_service(&services, settings.overseerr_prefer4_k)
            .ok_or_else(|| Refusal::not_found("No Radarr service was found in Overseerr"))?;
        json!({
            "mediaId": id_number,
            "mediaType": "movie",
            "is4k": service.get("is4k").and_then(Value::as_bool).unwrap_or(false),
            "serverId": whole(service, "id"),
            "profileId": whole(service, "activeProfileId"),
            "tags": [],
        })
    };
    let reply = install
        .send(
            http,
            Method::POST,
            "request",
            Some(&body),
            As::User(session),
        )
        .await
        .map_err(|error| unreachable(&error))?;
    if reply.is_success {
        Ok(Envelope::success(NoData).with_message("Overseerr Request submitted"))
    } else {
        Err(refused(&reply))
    }
}

/// `POST /homepage/overseerr/requests/{type}/{id}`: a new request for a film, or for
/// a series with the seasons of the settings.
pub async fn add_request(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, String)>,
) -> Result<Envelope<NoData>, Refusal> {
    add(&state, &caller, &client, &headers, (&kind, &id, None)).await
}

/// `POST /homepage/overseerr/requests/{type}/{id}/{seasons}`: a new request for a
/// series with the seasons that the path names, with commas between them.
pub async fn add_request_for_seasons(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path((kind, id, seasons)): Path<(String, String, String)>,
) -> Result<Envelope<NoData>, Refusal> {
    add(
        &state,
        &caller,
        &client,
        &headers,
        (&kind, &id, Some(&seasons)),
    )
    .await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{kind, number, request_row, seasons_for};

    #[test]
    fn seasons_of_a_new_request_follow_the_setting() {
        let series =
            json!({"seasons": [{"seasonNumber": 0}, {"seasonNumber": 1}, {"seasonNumber": 2}]});
        let seasons =
            |setting: &str, asked: Option<&str>| seasons_for(setting, asked, &series).ok();

        assert_eq!(seasons("first", None), Some(json!([1])));
        assert_eq!(seasons("last", None), Some(json!([2])));
        // All seasons, without the specials.
        assert_eq!(seasons("all", Some("7")), Some(json!([1, 2])));
        assert_eq!(seasons("user", Some("2")), Some(json!([2])));
        assert_eq!(seasons("user", Some("1,2")), Some(json!([1, 2])));
        assert_eq!(seasons("user", None), None);
        assert_eq!(seasons("user", Some("")), None);
    }

    #[test]
    fn parts_of_a_path_are_a_number_and_a_known_kind_only() {
        assert_eq!(number("815"), Some("815"));
        for wrong in ["", "8 15", "815/..", "-1", "1234567890123"] {
            assert_eq!(number(wrong), None, "{wrong}");
        }
        assert_eq!(kind("movie"), Some("movie"));
        assert_eq!(kind("tv"), Some("tv"));
        assert_eq!(kind("user"), None);
    }

    #[test]
    fn request_of_another_user_is_seen_only_without_the_limit() {
        let entry = json!({
            "id": 5, "status": 1, "type": "movie", "createdAt": "2020-01-01T00:00:00.000Z",
            "media": {"tmdbId": 815, "status": 2},
            "requestedBy": {"username": "Pat", "plexUsername": "pat-at-plex", "displayName": "Pat"},
        });
        let seen = |sees_all, name: &str, fallback: &str| {
            request_row(&entry, sees_all, name, fallback).is_some()
        };

        assert!(seen(true, "", ""));
        assert!(seen(false, "pat", ""));
        assert!(seen(false, "zed", "PAT"));
        assert!(!seen(false, "zed", ""));
        // A guest has no name, and the name of nobody is nobody.
        assert!(!seen(false, "", ""));
        let nameless = json!({
            "id": 6, "status": 1, "type": "movie",
            "media": {"tmdbId": 816, "status": 2},
            "requestedBy": {"username": null, "plexUsername": null},
        });
        assert!(request_row(&nameless, false, "", "").is_none());
        assert!(request_row(&nameless, true, "", "").is_some());
        // The user name of Overseerr comes before the one of Plex.
        let row = request_row(&entry, true, "", "").map(|(_, _, row)| row);
        assert_eq!(
            row.and_then(|row| row.get("user").cloned()),
            Some(json!("Pat"))
        );
    }
}
