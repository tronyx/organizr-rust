//! Helpers of the homepage that ask the wide web: the search for the coordinates of
//! a place, the search for a trailer at YouTube, and the fetch of one page.

use std::net::{IpAddr, SocketAddr};

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use organizr_domain::GroupLevel;
use organizr_integrations::scrape::{self, ScrapeError};
use organizr_integrations::youtube;
use serde_json::{Value, json};

use super::{no_requests, service_url};
use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::{Refusal, is_nothing_in_php};
use crate::state::AppState;

/// `POST /homepage/weather/coordinates`: the search for the latitude and the longitude
/// of a place, for the settings of the weather item.
///
/// The PHP application asks Mapbox, with a key of the author of PHP Organizr that
/// every install shares. This application has no such key, and the settings of PHP
/// have no place for a key of the Admin. So the search finds nothing: the answer is
/// the one of the PHP application for a search that Mapbox refuses.
pub async fn coordinates(
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Envelope<Value>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    if data.field("query").is_none_or(is_nothing_in_php) {
        return Err(Refusal::unprocessable("Query was not supplied"));
    }
    Ok(Envelope::success(Value::Null))
}

fn no_video() -> Refusal {
    Refusal::Request(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Bad response from YouTube".to_owned(),
    )
}

/// `GET /homepage/youtube/{query}`: the first short video that YouTube finds for the
/// title and the words "official trailer", as YouTube sends it.
///
/// The search uses the API key of the settings. Without one, the answer is the one
/// for a search that YouTube refuses: the keys of the author of PHP Organizr, which
/// the PHP application falls back on, are not used.
pub async fn trailer(
    State(state): State<AppState>,
    Path(query): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    if query.is_empty() || query == "0" {
        return Err(Refusal::unprocessable("No query supplied"));
    }
    let settings = state.settings();
    if settings.youtube_api.is_empty() {
        return Err(no_video());
    }
    let http = state.outbound().ok_or_else(no_requests)?;
    let found = youtube::trailer(http, state.youtube(), &settings.youtube_api, &query)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "YouTube cannot be reached");
            no_video()
        })?
        .ok_or_else(no_video)?;
    Ok(Envelope::success(found))
}

fn not_allowed() -> Refusal {
    Refusal::Request(
        StatusCode::FORBIDDEN,
        "Access to local or internal URLs is not allowed".to_owned(),
    )
}

/// Where the host of a page is reached, when every address of the host is one of the
/// wide web. With `local`, an address of this computer or of a private network
/// counts too.
async fn checked_address(host: &str, port: u16, local: bool) -> Option<SocketAddr> {
    let addresses: Vec<SocketAddr> = match host.parse::<IpAddr>() {
        Ok(address) => vec![SocketAddr::new(address, port)],
        Err(_) => tokio::net::lookup_host((host, port)).await.ok()?.collect(),
    };
    // A name with one private address among its addresses is refused: the request
    // could go to that one.
    if !local
        && addresses
            .iter()
            .any(|address| !scrape::is_public(address.ip()))
    {
        return None;
    }
    addresses.first().copied()
}

/// `POST /homepage/scrape`: fetches the page at the address that the body names, and
/// answers with its text. With the `type` `json`, the text is read as JSON. With
/// `html`, the entities of the text are written as characters.
///
/// The address must be one of the wide web. Only an Admin or a Co-Admin can use the
/// route: the server fetches an address that the caller names.
pub async fn scrape(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Envelope<Value>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let named = data.text("url").unwrap_or_default();
    if named.is_empty() || named == "0" {
        return Err(Refusal::unprocessable("URL was not supplied"));
    }
    let url = service_url(&named, &headers, &client);
    let (host, port) = scrape::target(&url).ok_or_else(not_allowed)?;
    let address = checked_address(&host, port, state.scrapes_local_pages())
        .await
        .ok_or_else(not_allowed)?;
    let http = state.outbound().ok_or_else(no_requests)?;

    let page = scrape::fetch(http, &url, &host, address)
        .await
        .map_err(|error| {
            if let ScrapeError::Unreachable(reason) = &error {
                tracing::warn!(reason, "a page could not be fetched");
            }
            Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
        })?;
    if !(200..300).contains(&page.status) {
        return Err(Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error getting successful response".to_owned(),
        ));
    }
    let content = match data.text("type").as_deref() {
        Some("html") => json!(scrape::without_entities(&page.body)),
        Some("json") => serde_json::from_str(&page.body).unwrap_or(Value::Null),
        _ => json!(page.body),
    };
    Ok(Envelope::success(json!({
        "full_url": url,
        "drill_url": scrape::parts(&url),
        "response_code": page.status,
        "result": "Success",
        "data": content,
    })))
}

#[cfg(test)]
mod tests {
    use super::checked_address;

    #[tokio::test]
    async fn address_of_this_computer_is_refused_unless_a_test_allows_it() {
        assert_eq!(checked_address("127.0.0.1", 80, false).await, None);
        assert_eq!(checked_address("192.168.1.10", 443, false).await, None);
        assert_eq!(checked_address("::1", 80, false).await, None);
        assert_eq!(checked_address("localhost", 80, false).await, None);
        assert_eq!(
            checked_address("127.0.0.1", 8080, true).await,
            Some("127.0.0.1:8080".parse().expect("an address"))
        );
        assert_eq!(
            checked_address("93.184.216.34", 443, false).await,
            Some("93.184.216.34:443".parse().expect("an address"))
        );
    }
}
