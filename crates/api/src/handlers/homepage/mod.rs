//! The homepage: a page of Organizr that shows items, each one with data of another
//! service or with HTML of the Admin.
//!
//! The PHP application builds the homepage as HTML on the server. Here the interface
//! builds it. It asks `GET /homepage/items` which items the caller sees and in which
//! order, and then asks the route of each item for its data.

pub mod adguard;
pub mod arr;
pub mod calendar_more;
pub mod downloaders;
pub mod downloaders_more;
pub mod emby;
pub mod health_checks;
pub mod html;
pub mod indexers;
pub mod monitorr;
pub mod netdata;
pub mod octoprint;
pub mod ombi;
pub mod overseerr;
pub mod pihole;
pub mod plex;
pub mod speedtest_tracker;
pub mod tautulli;
pub mod unifi;
pub mod uptime_kuma;
pub mod weather;

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use organizr_integrations::arr::Kind;
use organizr_integrations::emby::Kind as EmbyKind;
use organizr_integrations::indexers::Kind as IndexerKind;
use organizr_settings::Settings;
use serde::Serialize;
use serde_json::{Value, json};

use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::php_url;
use crate::state::AppState;

/// What must hold before a caller gets an item: the `...HomepagePermissions` arrays
/// of the PHP application. The checks run in the order of the fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Permissions {
    /// Switches that must all be on.
    pub enabled: Vec<String>,
    /// Settings that each hold a level. The caller needs each level, or more access.
    pub auth: Vec<String>,
    /// Settings that must all hold a value.
    pub not_empty: Vec<String>,
}

/// Whether PHP takes a stored value for empty: `false`, `0`, an empty text, the text
/// `0`, an empty list, or nothing.
fn is_empty(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::Bool(on)) => !on,
        Some(Value::Number(number)) => number.as_f64() == Some(0.0),
        Some(Value::String(text)) => text.is_empty() || text == "0",
        Some(Value::Array(list)) => list.is_empty(),
        Some(Value::Object(map)) => map.is_empty(),
    }
}

/// The level that a setting holds. A value that is no level lets nobody in but the
/// Admin, whose level is 0.
fn level_of(value: Option<&Value>) -> u32 {
    match value {
        Some(Value::String(text)) => text.trim().parse().unwrap_or(0),
        Some(Value::Number(number)) => number
            .as_u64()
            .and_then(|level| u32::try_from(level).ok())
            .unwrap_or(0),
        _ => 0,
    }
}

impl Permissions {
    /// The first reason why the caller does not get the item, with the status and the
    /// text of the PHP application.
    pub fn check(&self, settings: &Settings, caller: &Caller) -> Result<(), Refusal> {
        for key in &self.enabled {
            if is_empty(settings.get(key).as_ref()) {
                return Err(Refusal::conflict(format!("{key} module is not enabled")));
            }
        }
        for key in &self.auth {
            if u32::from(caller.level().get()) > level_of(settings.get(key).as_ref()) {
                return Err(Refusal::Request(
                    StatusCode::UNAUTHORIZED,
                    "User not approved to view this homepage item".to_owned(),
                ));
            }
        }
        for key in &self.not_empty {
            if is_empty(settings.get(key).as_ref()) {
                return Err(Refusal::unprocessable(&format!("{key} was not supplied")));
            }
        }
        Ok(())
    }

    /// Whether the caller gets the item.
    pub fn allow(&self, settings: &Settings, caller: &Caller) -> bool {
        self.check(settings, caller).is_ok()
    }
}

/// One item of the homepage that the caller sees.
#[derive(Debug, Serialize, PartialEq)]
pub struct Item {
    /// The key of the setting that holds the place of the item, for example
    /// `homepageOrdercustomhtml01`.
    key: &'static str,
    /// The kind of the item, which says how the interface shows it.
    kind: &'static str,
    /// Which one of several items of a kind, for example `01` for custom HTML.
    id: String,
    /// Settings that say how the interface shows the item. Most items have none.
    #[serde(skip_serializing_if = "Value::is_null")]
    options: Value,
}

/// The kind and the permissions of the item behind an order key, or nothing for an
/// item that this application does not show.
fn item_of(key: &'static str) -> Option<(Item, Permissions)> {
    if let Some(number) = key.strip_prefix("homepageOrdercustomhtml") {
        let permissions = html::permissions(number)?;
        let item = Item {
            key,
            kind: "customhtml",
            id: number.to_owned(),
            options: Value::Null,
        };
        return Some((item, permissions));
    }
    let (kind, permissions) = match key {
        "homepageOrderhealthchecks" => ("healthchecks", health_checks::permissions()),
        "homepageOrderUptimeKuma" => ("uptimekuma", uptime_kuma::permissions()),
        // The calendar has rules of its own, see `is_seen`.
        "homepageOrdercalendar" => ("calendar", Permissions::default()),
        "homepageOrderSonarrQueue" => ("sonarrqueue", arr::queue_permissions(Kind::Sonarr)),
        "homepageOrderRadarrQueue" => ("radarrqueue", arr::queue_permissions(Kind::Radarr)),
        "homepageOrdersabnzbd" => ("sabnzbd", downloaders::sabnzbd_permissions()),
        "homepageOrderqBittorrent" => ("qbittorrent", downloaders::qbittorrent_permissions()),
        "homepageOrderPihole" => ("pihole", pihole::permissions()),
        "homepageOrdernzbget" => ("nzbget", downloaders::nzbget_permissions()),
        "homepageOrdertransmission" => ("transmission", downloaders::transmission_permissions()),
        "homepageOrderdeluge" => ("deluge", downloaders_more::deluge_permissions()),
        "homepageOrderrTorrent" => ("rtorrent", downloaders_more::rtorrent_permissions()),
        "homepageOrderuTorrent" => ("utorrent", downloaders_more::utorrent_permissions()),
        "homepageOrderjdownloader" => ("jdownloader", downloaders_more::jdownloader_permissions()),
        "homepageOrderWeatherAndAir" => ("weather", weather::permissions()),
        "homepageOrdertautulli" => ("tautulli", tautulli::permissions()),
        "homepageOrderoverseerr" => ("overseerr", overseerr::permissions()),
        "homepageOrderombi" => ("ombi", ombi::permissions()),
        "homepageOrderJackett" => ("jackett", indexers::permissions(IndexerKind::Jackett)),
        "homepageOrderProwlarr" => ("prowlarr", indexers::permissions(IndexerKind::Prowlarr)),
        "homepageOrderAdguard" => ("adguard", adguard::permissions()),
        "homepageOrderNetdata" => ("netdata", netdata::permissions()),
        "homepageOrderMonitorr" => ("monitorr", monitorr::permissions()),
        "homepageOrderSpeedtest" => ("speedtest", speedtest_tracker::permissions()),
        "homepageOrderOctoprint" => ("octoprint", octoprint::permissions()),
        "homepageOrderunifi" => ("unifi", unifi::permissions()),
        "homepageOrderplexnowplaying" => ("plexstreams", plex::permissions(plex::Part::Streams)),
        "homepageOrderplexrecent" => ("plexrecent", plex::permissions(plex::Part::Recent)),
        "homepageOrderplexplaylist" => ("plexplaylists", plex::permissions(plex::Part::Playlists)),
        "homepageOrderembynowplaying" => (
            "embystreams",
            emby::permissions(EmbyKind::Emby, emby::Part::Streams),
        ),
        "homepageOrderembyrecent" => (
            "embyrecent",
            emby::permissions(EmbyKind::Emby, emby::Part::Recent),
        ),
        "homepageOrderjellyfinnowplaying" => (
            "jellyfinstreams",
            emby::permissions(EmbyKind::Jellyfin, emby::Part::Streams),
        ),
        "homepageOrderjellyfinrecent" => (
            "jellyfinrecent",
            emby::permissions(EmbyKind::Jellyfin, emby::Part::Recent),
        ),
        _ => return None,
    };
    let item = Item {
        key,
        kind,
        id: String::new(),
        options: Value::Null,
    };
    Some((item, permissions))
}

/// The address of a service as the settings hold it, completed as the PHP application
/// completes it. An address that starts with a slash is a path on the host of
/// Organizr.
pub fn service_url(address: &str, headers: &HeaderMap, client: &Client) -> String {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    service_address(address, client.https, host)
}

/// The address of a service, as `qualifyURL` of PHP gives it without its option
/// `includeTrailing`: white space is taken out, and slashes at the end are cut off.
/// So a path that is put after the address starts with exactly one slash.
fn service_address(address: &str, https: bool, host: &str) -> String {
    let mut address: String = address.chars().filter(|c| !c.is_whitespace()).collect();
    if address.starts_with('/') {
        let scheme = if https { "https" } else { "http" };
        address = format!("{scheme}://{host}{address}");
    }
    php_url::qualify(address.trim_end_matches('/'), https, host)
}

/// The answer when this server cannot send requests to other services.
pub fn no_requests() -> Refusal {
    Refusal::Request(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Requests to other services cannot be sent".to_owned(),
    )
}

/// The items that the caller sees, in the order of the homepage.
pub fn visible_items(settings: &Settings, caller: &Caller) -> Vec<Item> {
    let mut placed: Vec<(u64, &'static str)> = Settings::KEYS
        .iter()
        .filter(|key| key.contains("homepageOrder"))
        .map(|key| {
            let place = settings
                .get(key)
                .as_ref()
                .and_then(Value::as_str)
                .and_then(|text| text.parse().ok())
                .unwrap_or(0);
            (place, *key)
        })
        .collect();
    // Two items on one place stand in the order of their keys.
    placed.sort_unstable();
    let downloader_place = placed
        .iter()
        .find(|(_, key)| *key == DOWNLOADER_KEY)
        .map_or(0, |(place, _)| *place);

    let mut shown: Vec<(u64, Item)> = Vec::new();
    // The downloaders that show together in one item, as tabs.
    let mut combined: Vec<&'static str> = Vec::new();
    for (place, key) in placed {
        let Some((mut item, permissions)) = item_of(key) else {
            continue;
        };
        let seen = if item.kind == "calendar" {
            // The calendar shows when the caller gets one source of it at least.
            arr::sees_a_calendar(settings, caller)
        } else {
            permissions.allow(settings, caller)
        };
        if !seen {
            continue;
        }
        if is_combined(settings, item.kind) {
            combined.push(item.kind);
            continue;
        }
        if item.kind == "customhtml" && html::is_trusted(settings, &item.id) {
            // The interface then shows the item in a frame without a sandbox.
            item.options = json!({ "trusted": true });
        }
        if item.kind == "calendar" {
            item.options = json!({
                "defaultView": settings.calendar_default,
                "firstDay": settings.calendar_first_day,
                "limit": settings.calendar_limit,
            });
        }
        if item.kind == "ombi" {
            item.options = ombi::options(settings);
        }
        if item.kind == "jackett" {
            item.options = indexers::options(IndexerKind::Jackett, settings);
        }
        if item.kind == "prowlarr" {
            item.options = indexers::options(IndexerKind::Prowlarr, settings);
        }
        if item.kind == "weather" {
            // The title shows only when its switch is on.
            let title = if settings.homepage_weather_and_air_weather_header_toggle {
                settings.homepage_weather_and_air_weather_header.as_str()
            } else {
                ""
            };
            item.options = json!({ "title": title });
        }
        shown.push((place, item));
    }
    if !combined.is_empty() {
        // The tabs stand in the order of the places of the members.
        let item = Item {
            key: DOWNLOADER_KEY,
            kind: "downloader",
            id: String::new(),
            options: json!({ "members": combined }),
        };
        let at = shown.partition_point(|(place, _)| *place <= downloader_place);
        shown.insert(at, (downloader_place, item));
    }
    shown.into_iter().map(|(_, item)| item).collect()
}

/// The key of the place of the item that shows several downloaders as tabs.
const DOWNLOADER_KEY: &str = "homepageOrderdownloader";

/// Whether the setting of an item says that it shows in the downloader item.
fn is_combined(settings: &Settings, kind: &str) -> bool {
    match kind {
        "sabnzbd" => settings.sabnzbd_combine,
        "nzbget" => settings.nzbget_combine,
        "transmission" => settings.transmission_combine,
        "qbittorrent" => settings.qbittorrent_combine,
        "deluge" => settings.deluge_combine,
        "rtorrent" => settings.rtorrent_combine,
        "utorrent" => settings.utorrent_combine,
        "jdownloader" => settings.jdownloader_combine,
        "sonarrqueue" => settings.homepage_sonarr_queue_combine,
        "radarrqueue" => settings.homepage_radarr_queue_combine,
        _ => false,
    }
}

/// `GET /homepage/items`: the items of the homepage that the caller sees, in order.
pub async fn items(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Envelope<Vec<Item>> {
    Envelope::success(visible_items(&state.settings(), &caller))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{is_empty, level_of, service_address};

    #[test]
    fn slashes_at_the_end_of_a_service_address_are_cut_off() {
        let address = |text| service_address(text, true, "organizr.example.com");

        assert_eq!(
            address("https://checks.example.com/api/v1/checks/"),
            "https://checks.example.com/api/v1/checks"
        );
        assert_eq!(
            address(" https://sonarr.example.com:8989// "),
            "https://sonarr.example.com:8989"
        );
        assert_eq!(address("sonarr:8989/"), "http://sonarr:8989");
        assert_eq!(address("/sonarr/"), "https://organizr.example.com/sonarr");
        assert_eq!(address("/"), "https://organizr.example.com");
    }

    #[test]
    fn empty_is_what_php_takes_for_empty() {
        for value in [
            json!(null),
            json!(false),
            json!(0),
            json!(""),
            json!("0"),
            json!([]),
        ] {
            assert!(is_empty(Some(&value)), "{value}");
        }
        assert!(is_empty(None));
        for value in [json!(true), json!(1), json!("00"), json!(" "), json!(["a"])] {
            assert!(!is_empty(Some(&value)), "{value}");
        }
    }

    #[test]
    fn level_that_cannot_be_read_is_the_level_of_the_admin() {
        assert_eq!(level_of(Some(&json!("4"))), 4);
        assert_eq!(level_of(Some(&json!(999))), 999);
        assert_eq!(level_of(Some(&json!("guest"))), 0);
        assert_eq!(level_of(None), 0);
    }
}
