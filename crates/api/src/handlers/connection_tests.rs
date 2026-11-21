//! `POST /test/{service}`: the connection tests of the settings pages. A test asks the
//! service with the settings that are stored, and says whether the service answered.
//!
//! Only an Admin or a Co-Admin can run a test.

use std::time::Duration;

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_domain::GroupLevel;
use organizr_integrations::adguard;
use organizr_integrations::downloaders::{
    self, Client as Downloader, DownloaderError, Login, TorrentQuery,
};
use organizr_integrations::downloaders_more::ClientError;
use organizr_integrations::emby::Kind as MediaServer;
use organizr_integrations::emby_live_tv;
use organizr_integrations::http::Outbound;
use organizr_integrations::indexers::Kind as Indexer;
use organizr_integrations::jellystat::Install as JellyStat;
use organizr_integrations::pihole;
use organizr_integrations::probe::{self, Probe};
use organizr_integrations::sickrage;
use organizr_settings::Settings;
use serde_json::Value;

use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::downloaders_more::{self as more, Asked};
use crate::handlers::homepage::{
    Permissions, emby, emby_live_tv as live_tv, indexers, jellystat, no_requests, service_url,
    unifi,
};
use crate::state::AppState;

/// How long a download client has to answer a test.
const TIME_LIMIT: Duration = Duration::from_secs(15);

/// The most characters of an answer of a service that a test shows.
const SHOWN: usize = 300;

/// What a test answers.
type Outcome = Result<Envelope<Value>, Refusal>;

const SUCCEEDED: &str = "API Connection succeeded";

fn succeeded() -> Envelope<Value> {
    Envelope::success(Value::Null).with_message(SUCCEEDED)
}

fn refused(status: StatusCode, message: impl Into<String>) -> Refusal {
    Refusal::Request(status, message.into())
}

/// The answer for a service that cannot be reached. The text names no address.
fn unreachable(error: String) -> Refusal {
    refused(StatusCode::INTERNAL_SERVER_ERROR, error)
}

fn downloader_failed(error: &DownloaderError) -> Refusal {
    refused(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

/// What every test needs from the request.
struct Test<'a> {
    state: &'a AppState,
    headers: &'a HeaderMap,
    client: &'a Client,
    http: &'a Outbound,
}

impl Test<'_> {
    /// The address of a setting, complete.
    fn url(&self, address: &str) -> String {
        service_url(address, self.headers, self.client)
            .trim_end_matches('/')
            .to_owned()
    }

    async fn get(&self, probe: Probe<'_>) -> Result<probe::Answer, String> {
        probe::get(self.http, probe).await
    }
}

/// The start of a text of a service, for an answer.
fn shown(text: &str) -> String {
    text.chars().take(SHOWN).collect()
}

async fn plex(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.plex_url.is_empty() || settings.plex_token.is_empty() {
        return Err(Refusal::unprocessable("URL and/or Token not setup"));
    }
    let url = format!("{}/servers", test.url(&settings.plex_url));
    let probe = Probe {
        url: &url,
        query: &[("X-Plex-Token", &settings.plex_token)],
        never_check_certificate: settings.plex_disable_cert_check,
        ..Probe::default()
    };
    if test.get(probe).await.map_err(unreachable)?.is_success() {
        Ok(succeeded())
    } else {
        Err(Refusal::unprocessable(
            "URL and/or Token not setup correctly",
        ))
    }
}

/// One of the three applications that share an API.
struct Arr<'a> {
    name: &'a str,
    api: &'a str,
    urls: &'a str,
    tokens: &'a str,
    never_check: bool,
}

/// What is wrong with the answer of an install to the question for its root folders,
/// or nothing.
fn arr_fault(answer: &probe::Answer) -> Option<String> {
    let Ok(body) = serde_json::from_str::<Value>(&answer.body) else {
        return Some("Response was not JSON".to_owned());
    };
    match body.get("error") {
        Some(Value::String(text)) => Some(text.clone()),
        Some(error) => Some(
            error
                .get("msg")
                .and_then(Value::as_str)
                .unwrap_or("An error occurred")
                .to_owned(),
        ),
        None if !answer.is_success() => {
            Some(format!("The answer has the status {}", answer.status))
        }
        None if body.is_array() || body.is_object() => None,
        None => Some("Response was not JSON".to_owned()),
    }
}

async fn arr(test: &Test<'_>, app: Arr<'_>) -> Outcome {
    if app.urls.is_empty() {
        return Err(Refusal::unprocessable(&format!(
            "{} URL is not defined",
            app.name
        )));
    }
    if app.tokens.is_empty() {
        return Err(Refusal::unprocessable(&format!(
            "{} Token is not defined",
            app.name
        )));
    }
    let mut errors = String::new();
    for (address, token) in pihole::installs(app.urls, app.tokens) {
        let base = test.url(address);
        let url = format!("{base}/api/{}/rootfolder", app.api);
        let probe = Probe {
            url: &url,
            headers: &[("X-Api-Key", token)],
            never_check_certificate: app.never_check,
            ..Probe::default()
        };
        let fault = match test.get(probe).await {
            Ok(answer) => arr_fault(&answer),
            Err(error) => Some(error),
        };
        if let Some(fault) = fault {
            errors.push_str(&base);
            errors.push_str(": ");
            errors.push_str(&shown(&fault));
        }
    }
    if errors.is_empty() {
        Ok(Envelope::success(Value::Null))
    } else {
        Err(refused(StatusCode::INTERNAL_SERVER_ERROR, errors))
    }
}

async fn sabnzbd(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.sabnzbd_url.is_empty() || settings.sabnzbd_token.is_empty() {
        return Err(Refusal::unprocessable("URL and/or Token not setup"));
    }
    let url = format!("{}/api", test.url(&settings.sabnzbd_url));
    let probe = Probe {
        url: &url,
        query: &[
            ("mode", "queue"),
            ("output", "json"),
            ("apikey", &settings.sabnzbd_token),
        ],
        never_check_certificate: settings.sabnzbd_disable_cert_check,
        ..Probe::default()
    };
    let answer = test.get(probe).await.map_err(unreachable)?;
    if !answer.is_success() {
        return Err(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            shown(&answer.body),
        ));
    }
    let data: Value = serde_json::from_str(&answer.body).unwrap_or(Value::Null);
    // SABnzbd answers a wrong key with the status 200 and an error in the answer.
    if let Some(error) = data.get("error") {
        let message = error
            .as_str()
            .map_or_else(|| error.to_string(), str::to_owned);
        return Err(Refusal::RequestWithData(
            StatusCode::INTERNAL_SERVER_ERROR,
            shown(&message),
            data,
        ));
    }
    Ok(Envelope::success(data).with_message(SUCCEEDED))
}

async fn tautulli(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.tautulli_url.is_empty() {
        return Err(Refusal::unprocessable("Tautulli URL is not defined"));
    }
    if settings.tautulli_apikey.is_empty() {
        return Err(Refusal::unprocessable("Tautulli Token is not defined"));
    }
    let url = format!("{}/api/v2", test.url(&settings.tautulli_url));
    let probe = Probe {
        url: &url,
        query: &[
            ("apikey", &settings.tautulli_apikey),
            ("cmd", "get_home_stats"),
            ("grouping", "1"),
        ],
        never_check_certificate: settings.tautulli_disable_cert_check,
        ..Probe::default()
    };
    if test.get(probe).await.map_err(unreachable)?.is_success() {
        Ok(succeeded())
    } else {
        Err(Refusal::conflict(
            "Tautulli Error Occurred - Check URL or Credentials",
        ))
    }
}

async fn overseerr(test: &Test<'_>, caller: &Caller) -> Outcome {
    let settings = test.state.settings();
    Permissions {
        enabled: Vec::new(),
        auth: Vec::new(),
        not_empty: vec!["overseerrURL".to_owned(), "overseerrToken".to_owned()],
    }
    .check(&settings, caller)?;
    let url = format!("{}/api/v1/settings/main", test.url(&settings.overseerr_url));
    let probe = Probe {
        url: &url,
        headers: &[
            ("Accept", "application/json"),
            ("X-Api-Key", &settings.overseerr_token),
        ],
        never_check_certificate: settings.overseerr_disable_cert_check,
        ..Probe::default()
    };
    if test.get(probe).await.map_err(unreachable)?.is_success() {
        Ok(succeeded())
    } else {
        Err(refused(StatusCode::UNAUTHORIZED, "API Connection failed"))
    }
}

async fn ombi(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.ombi_url.is_empty() {
        return Err(Refusal::unprocessable("Ombi URL is not defined"));
    }
    if settings.ombi_token.is_empty() {
        return Err(Refusal::unprocessable("Ombi Token is not defined"));
    }
    let url = format!("{}/api/v1/Settings/about", test.url(&settings.ombi_url));
    let probe = Probe {
        url: &url,
        headers: &[
            ("Accept", "application/json"),
            ("ApiKey", &settings.ombi_token),
        ],
        never_check_certificate: settings.ombi_disable_cert_check,
        ..Probe::default()
    };
    let answer = test.get(probe).await.map_err(unreachable)?;
    if answer.is_success() {
        Ok(succeeded())
    } else if answer.body.trim().is_empty() {
        Err(refused(StatusCode::UNAUTHORIZED, "API Connection failed"))
    } else {
        // What Ombi answered says why.
        Err(refused(StatusCode::UNAUTHORIZED, shown(&answer.body)))
    }
}

async fn sick_rage(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.sickrage_url.is_empty() {
        return Err(Refusal::unprocessable("SickRage URL is not defined"));
    }
    if settings.sickrage_token.is_empty() {
        return Err(Refusal::unprocessable("SickRage Token is not defined"));
    }
    let mut errors = String::new();
    for (address, token) in pihole::installs(&settings.sickrage_url, &settings.sickrage_token) {
        let base = test.url(address);
        let install = sickrage::Install {
            url: &base,
            key: token,
            never_check_certificate: settings.sickrage_disable_cert_check,
        };
        let fault = match install.about(test.http, TIME_LIMIT).await {
            Ok(answer) => sickrage::fault(&answer),
            Err(error) => Some(error.to_string()),
        };
        if let Some(fault) = fault {
            errors.push_str(&base);
            errors.push_str(": ");
            errors.push_str(&shown(&fault));
        }
    }
    if errors.is_empty() {
        Ok(Envelope::success(Value::Null))
    } else {
        Err(refused(StatusCode::INTERNAL_SERVER_ERROR, errors))
    }
}

async fn nzbget(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.nzbget_url.is_empty() {
        return Err(Refusal::unprocessable("NZBGet URL is not defined"));
    }
    let url = test.url(&settings.nzbget_url);
    let install = Downloader {
        url: &url,
        timeout: Some(TIME_LIMIT),
        never_check_certificate: settings.nzbget_disable_cert_check,
    };
    let login = Login {
        username: &settings.nzbget_username,
        password: &settings.nzbget_password,
    };
    let (queue, _) = downloaders::nzbget(test.http, install, login)
        .await
        .map_err(|error| downloader_failed(&error))?;
    if queue.is_some() {
        Ok(succeeded())
    } else {
        Err(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "NZBGet: An Error Occurred",
        ))
    }
}

async fn transmission(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.transmission_url.is_empty() {
        return Err(Refusal::unprocessable("Transmission URL is not defined"));
    }
    let url = test.url(&settings.transmission_url);
    let install = Downloader {
        url: &url,
        timeout: Some(TIME_LIMIT),
        never_check_certificate: settings.transmission_disable_cert_check,
    };
    let login = Login {
        username: &settings.transmission_username,
        password: &settings.transmission_password,
    };
    let fault = match downloaders::transmission(test.http, install, login).await {
        Ok(Some(_)) => return Ok(succeeded()),
        Ok(None) => "Unknown",
        Err(DownloaderError::NoSession) => "Could not get session ID",
        Err(error) => return Err(downloader_failed(&error)),
    };
    Err(refused(
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Transmission Connect Function - Error: {fault}"),
    ))
}

async fn qbittorrent(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.qbittorrent_url.is_empty() {
        return Err(Refusal::unprocessable("qBittorrent URL is not defined"));
    }
    let url = test.url(&settings.qbittorrent_url);
    let install = Downloader {
        url: &url,
        timeout: Some(TIME_LIMIT),
        never_check_certificate: settings.qbittorrent_disable_cert_check,
    };
    let query = TorrentQuery {
        username: &settings.qbittorrent_username,
        password: &settings.qbittorrent_password,
        first_api: settings.qbittorrent_api_version == 1,
        sort: &settings.qbittorrent_sort_order,
        reverse: settings.qbittorrent_reverse_sorting,
    };
    match downloaders::qbittorrent(test.http, install, query).await {
        Ok(_) => Ok(succeeded()),
        Err(DownloaderError::NoSession) => Err(Refusal::conflict(
            "qBittorrent Connect Function - Error: Could not get session ID",
        )),
        Err(error) => Err(downloader_failed(&error)),
    }
}

fn client_failed(error: &ClientError) -> Refusal {
    refused(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

impl Test<'_> {
    /// What the tests of Deluge, rTorrent, uTorrent and JDownloader ask with.
    fn asked<'a>(&'a self, settings: &'a Settings) -> Asked<'a> {
        Asked {
            settings,
            http: self.http,
            headers: self.headers,
            client: self.client,
            timeout: Some(TIME_LIMIT),
        }
    }
}

/// Deluge has the test among its homepage permissions in the PHP application.
async fn deluge(test: &Test<'_>, caller: &Caller) -> Outcome {
    let settings = test.state.settings();
    more::deluge_permissions().check(&settings, caller)?;
    test.asked(&settings)
        .deluge()
        .await
        .map_err(|error| client_failed(&error))?;
    Ok(succeeded())
}

async fn rtorrent(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    more::no_rtorrent_address(&settings)?;
    let answers = test
        .asked(&settings)
        .rtorrent_answers()
        .await
        .map_err(|error| client_failed(&error))?;
    if answers {
        Ok(succeeded())
    } else {
        Err(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "rTorrent error occurred",
        ))
    }
}

async fn utorrent(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.utorrent_url.is_empty() {
        return Err(Refusal::unprocessable("uTorrent URL is not defined"));
    }
    let torrents = test
        .asked(&settings)
        .utorrent()
        .await
        .map_err(|error| client_failed(&error))?;
    if torrents.is_some() {
        Ok(succeeded())
    } else {
        Err(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "uTorrent error occurred",
        ))
    }
}

async fn jdownloader(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.jdownloader_url.is_empty() {
        return Err(Refusal::unprocessable("JDownloader URL is not defined"));
    }
    let answer = test
        .asked(&settings)
        .jdownloader()
        .await
        .map_err(|error| client_failed(&error))?;
    if answer.is_some() {
        Ok(succeeded())
    } else {
        Err(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "JDownloader Error Occurred",
        ))
    }
}

async fn pi_hole(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.pihole_url.is_empty() {
        return Err(Refusal::unprocessable("Pihole URL is not defined"));
    }
    let mut errors = String::new();
    let http = test.state.outbound().ok_or_else(no_requests)?;
    for (address, password) in pihole::installs(&settings.pihole_url, &settings.pihole_token) {
        let base = test.url(address);
        let base = base.trim_end_matches('/');
        // PHP checks only that a password is there and that the install answers. A
        // password that Pi-hole refuses fails the test here too (D-101).
        let fault = if password.is_empty() {
            Some("Missing API Token".to_owned())
        } else {
            match pihole::log_in(http, base, password).await {
                Ok(Some(_)) => None,
                Ok(None) => Some("Login failed".to_owned()),
                Err(error) => Some(error),
            }
        };
        if let Some(fault) = fault {
            errors.push_str(&probe::host_of(base).unwrap_or_default());
            errors.push_str(": ");
            errors.push_str(&fault);
        }
    }
    if errors.is_empty() {
        Ok(Envelope::success(Value::Null))
    } else {
        Err(refused(StatusCode::INTERNAL_SERVER_ERROR, errors))
    }
}

/// What is wrong with the answer of AdGuard Home to the question for its statistics,
/// or nothing.
fn adguard_fault(is_success: bool, body: &str) -> Option<&'static str> {
    if !is_success {
        return Some("Unknown Failure");
    }
    match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(_) | Value::Array(_)) => None,
        _ => Some("Response was not JSON"),
    }
}

/// AdGuard Home: the statistics of each install. The requests carry no user name and
/// no password, as the requests of the homepage item.
async fn adguard(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.adguard_url.is_empty() {
        return Err(Refusal::unprocessable("AdGuard URL is not defined"));
    }
    let mut errors = String::new();
    let login = adguard::authorization(&settings.adguard_username, &settings.adguard_password);
    let headers: Vec<(&str, &str)> = login
        .as_deref()
        .map(|login| ("Authorization", login))
        .into_iter()
        .collect();
    for address in settings.adguard_url.split(',') {
        let base = test.url(address);
        let url = format!("{base}/control/stats");
        let probe = Probe {
            url: &url,
            headers: &headers,
            ..Probe::default()
        };
        let fault = match test.get(probe).await {
            Ok(answer) => adguard_fault(answer.is_success(), &answer.body).map(str::to_owned),
            Err(error) => Some(error),
        };
        if let Some(fault) = fault {
            errors.push_str(&probe::host_of(&base).unwrap_or_default());
            errors.push_str(": ");
            errors.push_str(&fault);
        }
    }
    if errors.is_empty() {
        Ok(Envelope::success(Value::Null))
    } else {
        Err(refused(StatusCode::INTERNAL_SERVER_ERROR, errors))
    }
}

/// Jackett and Prowlarr: a search for a text that no release has. The answer holds
/// what the service found, without the key of the service.
async fn indexer(test: &Test<'_>, caller: &Caller, kind: Indexer) -> Outcome {
    let settings = test.state.settings();
    indexers::check(kind, &settings, caller, true)?;
    let url = indexers::address(kind, &settings, test.headers, test.client);
    let install = indexers::install(kind, &settings, &url);
    let reply = install
        .search(test.http, kind, "this-is-just-a-test-for-organizr")
        .await
        .map_err(unreachable)?;
    if !reply.is_success {
        return Err(refused(
            StatusCode::FORBIDDEN,
            format!("Error connecting to {}", kind.name()),
        ));
    }
    let found = reply.json(install.key).unwrap_or(Value::Bool(false));
    Ok(Envelope::success(serde_json::json!({ "content": found })))
}

/// The login at the controller, and the health of the site.
async fn unifi_health(test: &Test<'_>, caller: &Caller) -> Outcome {
    let settings = test.state.settings();
    unifi::test_permissions().check(&settings, caller)?;
    unifi::health(&settings, test.http, test.headers, test.client)
        .await
        .map_err(|error| unifi::refusal(&error))?;
    Ok(succeeded())
}

/// The sites of a classic controller, for the choice of the site of the settings.
async fn unifi_sites(test: &Test<'_>, caller: &Caller) -> Outcome {
    let settings = test.state.settings();
    unifi::test_permissions().check(&settings, caller)?;
    let url = test.url(&settings.unifi_url);
    let sites = unifi::controller(&settings, &url)
        .sites(test.http)
        .await
        .map_err(|error| unifi::refusal(&error))?;
    Ok(Envelope::success(sites))
}

/// Emby and Jellyfin: the list of the users of the server. The key goes in a header.
async fn media_server(test: &Test<'_>, caller: &Caller, kind: MediaServer) -> Outcome {
    let settings = test.state.settings();
    let name = kind.name();
    let (address, token, never_check) = match kind {
        MediaServer::Emby => {
            // Emby has the test among its homepage permissions in the PHP application.
            emby::permissions(kind, emby::Part::Metadata).check(&settings, caller)?;
            (
                &settings.emby_url,
                &settings.emby_token,
                settings.emby_disable_cert_check,
            )
        }
        MediaServer::Jellyfin => {
            if settings.jellyfin_url.is_empty() {
                return Err(Refusal::unprocessable("Jellyfin URL is not defined"));
            }
            if settings.jellyfin_token.is_empty() {
                return Err(Refusal::unprocessable("Jellyfin Token is not defined"));
            }
            (
                &settings.jellyfin_url,
                &settings.jellyfin_token,
                settings.jellyfin_disable_cert_check,
            )
        }
    };
    let url = format!("{}/Users", test.url(address));
    let (header, key) = kind.key_header(token);
    let probe = Probe {
        url: &url,
        headers: &[(header, &key)],
        never_check_certificate: never_check,
        ..Probe::default()
    };
    let answer = test.get(probe).await.map_err(unreachable)?;
    if !answer.is_success() {
        return Err(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{name} Connection Error"),
        ));
    }
    // The PHP application reads the answer of Jellyfin, and not the one of Emby.
    let is_json = matches!(
        serde_json::from_str::<Value>(&answer.body),
        Ok(Value::Array(_) | Value::Object(_))
    );
    if kind == MediaServer::Jellyfin && !is_json {
        return Err(Refusal::conflict("URL or token incorrect"));
    }
    Ok(succeeded())
}

/// The live television of Emby: the name of the server, and whether it has live
/// television.
async fn emby_live_television(test: &Test<'_>, caller: &Caller) -> Outcome {
    let settings = test.state.settings();
    live_tv::permissions().check(&settings, caller)?;
    let url = live_tv::address(&settings, test.headers, test.client);
    match emby_live_tv::test(test.http, &live_tv::server(&settings, &url)).await {
        Ok(message) => Ok(Envelope::success(Value::Null).with_message(message)),
        Err(error) => Err(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )),
    }
}

/// JellyStat: the page of JellyStat for an item that shows it in a frame, and the
/// libraries for an item that shows the statistics.
async fn jelly_stat(test: &Test<'_>, caller: &Caller) -> Outcome {
    let settings = test.state.settings();
    jellystat::permissions(&settings).check(&settings, caller)?;
    let failed = |error: String| {
        refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Connection test failed: {error}"),
        )
    };
    let never_check = settings.jelly_stat_disable_cert_check;
    if settings.homepage_jelly_stat_display_mode == "iframe" {
        let url = test.url(&settings.jelly_stat_url);
        let probe = Probe {
            url: &url,
            never_check_certificate: never_check,
            ..Probe::default()
        };
        return if test.get(probe).await.map_err(failed)?.is_success() {
            Ok(Envelope::success(Value::Null).with_message("Successfully connected to JellyStat"))
        } else {
            Err(refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to connect to JellyStat URL",
            ))
        };
    }
    let url = jellystat::api_address(&settings, test.headers, test.client);
    let install = JellyStat {
        url: &url,
        key: &settings.jelly_stat_apikey,
        never_check_certificate: never_check,
    };
    let answer = install
        .get(test.http, "/api/getLibraries", &[])
        .await
        .map_err(failed)?;
    if answer.is_success() {
        match serde_json::from_str::<Value>(&answer.body) {
            Ok(Value::Object(fields)) if fields.contains_key("error") => {
                let error = fields
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                return Err(refused(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("JellyStat API error: {}", shown(error)),
                ));
            }
            Ok(Value::Array(_) | Value::Object(_)) => {
                return Ok(Envelope::success(Value::Null)
                    .with_message("Successfully connected to JellyStat API"));
            }
            _ => {}
        }
    }
    // The libraries did not come. The page of JellyStat says whether it runs at all.
    let page = install.get(test.http, "", &[]).await.map_err(failed)?;
    let message = if page.is_success() {
        "JellyStat is reachable but API key is invalid or API endpoints are not responding correctly."
    } else {
        "Cannot connect to JellyStat URL. Check URL and network connectivity."
    };
    Err(refused(StatusCode::INTERNAL_SERVER_ERROR, message))
}

/// The services that have a test.
#[derive(Debug, Clone, Copy)]
enum Service {
    Plex,
    Sonarr,
    Radarr,
    Lidarr,
    Sabnzbd,
    Qbittorrent,
    Tautulli,
    Overseerr,
    Ombi,
    Sickrage,
    Nzbget,
    Transmission,
    Deluge,
    Rtorrent,
    Utorrent,
    Jdownloader,
    Pihole,
    Emby,
    Jellyfin,
    Jackett,
    Prowlarr,
    Adguard,
    Unifi,
    UnifiSites,
    EmbyLiveTv,
    JellyStat,
}

/// Whether the requests of the test of a service also trust the authority that an
/// Admin uploaded: the setting "use the custom certificate" of the service.
const fn uses_custom_certificate(service: Service, settings: &Settings) -> bool {
    match service {
        Service::Plex => settings.plex_use_custom_certificate,
        Service::Sonarr => settings.sonarr_use_custom_certificate,
        Service::Radarr => settings.radarr_use_custom_certificate,
        Service::Lidarr => settings.lidarr_use_custom_certificate,
        Service::Sabnzbd => settings.sabnzbd_use_custom_certificate,
        Service::Qbittorrent => settings.qbittorrent_use_custom_certificate,
        Service::Tautulli => settings.tautulli_use_custom_certificate,
        Service::Overseerr => settings.overseerr_use_custom_certificate,
        Service::Ombi => settings.ombi_use_custom_certificate,
        Service::Sickrage => settings.sickrage_use_custom_certificate,
        Service::Nzbget => settings.nzbget_use_custom_certificate,
        Service::Transmission => settings.transmission_use_custom_certificate,
        Service::Deluge => settings.deluge_use_custom_certificate,
        Service::Rtorrent => settings.rtorrent_use_custom_certificate,
        Service::Utorrent => settings.utorrent_use_custom_certificate,
        Service::Jdownloader => settings.jdownloader_use_custom_certificate,
        Service::Emby | Service::EmbyLiveTv => settings.emby_use_custom_certificate,
        Service::Jellyfin => settings.jellyfin_use_custom_certificate,
        Service::Jackett => settings.jackett_use_custom_certificate,
        Service::Prowlarr => settings.prowlarr_use_custom_certificate,
        Service::Unifi | Service::UnifiSites => settings.unifi_use_custom_certificate,
        Service::JellyStat => settings.jelly_stat_use_custom_certificate,
        // Pi-hole and AdGuard Home have no such setting.
        Service::Pihole | Service::Adguard => false,
    }
}

async fn run(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    headers: &HeaderMap,
    service: Service,
) -> Outcome {
    caller.require(GroupLevel::CO_ADMIN)?;
    let settings = state.settings();
    let http = &state
        .outbound_for(uses_custom_certificate(service, &settings))
        .ok_or_else(no_requests)?;
    let test = Test {
        state,
        headers,
        client,
        http,
    };
    match service {
        Service::Plex => plex(&test).await,
        Service::Sonarr => {
            let app = Arr {
                name: "Sonarr",
                api: "v3",
                urls: &settings.sonarr_url,
                tokens: &settings.sonarr_token,
                never_check: settings.sonarr_disable_cert_check,
            };
            arr(&test, app).await
        }
        Service::Radarr => {
            let app = Arr {
                name: "Radarr",
                api: "v3",
                urls: &settings.radarr_url,
                tokens: &settings.radarr_token,
                never_check: settings.radarr_disable_cert_check,
            };
            arr(&test, app).await
        }
        Service::Lidarr => {
            let app = Arr {
                name: "Lidarr",
                api: "v1",
                urls: &settings.lidarr_url,
                tokens: &settings.lidarr_token,
                never_check: settings.lidarr_disable_cert_check,
            };
            arr(&test, app).await
        }
        Service::Sabnzbd => sabnzbd(&test).await,
        Service::Qbittorrent => qbittorrent(&test).await,
        Service::Tautulli => tautulli(&test).await,
        Service::Overseerr => overseerr(&test, caller).await,
        Service::Ombi => ombi(&test).await,
        Service::Sickrage => sick_rage(&test).await,
        Service::Nzbget => nzbget(&test).await,
        Service::Transmission => transmission(&test).await,
        Service::Deluge => deluge(&test, caller).await,
        Service::Rtorrent => rtorrent(&test).await,
        Service::Utorrent => utorrent(&test).await,
        Service::Jdownloader => jdownloader(&test).await,
        Service::Pihole => pi_hole(&test).await,
        Service::Emby => media_server(&test, caller, MediaServer::Emby).await,
        Service::Jellyfin => media_server(&test, caller, MediaServer::Jellyfin).await,
        Service::Jackett => indexer(&test, caller, Indexer::Jackett).await,
        Service::Prowlarr => indexer(&test, caller, Indexer::Prowlarr).await,
        Service::Adguard => adguard(&test).await,
        Service::Unifi => unifi_health(&test, caller).await,
        Service::UnifiSites => unifi_sites(&test, caller).await,
        Service::EmbyLiveTv => emby_live_television(&test, caller).await,
        Service::JellyStat => jelly_stat(&test, caller).await,
    }
}

/// Makes the handler of the test of one service.
macro_rules! test_route {
    ($(#[$doc:meta])* $name:ident, $service:expr) => {
        $(#[$doc])*
        pub async fn $name(
            State(state): State<AppState>,
            Extension(caller): Extension<Caller>,
            Extension(client): Extension<Client>,
            headers: HeaderMap,
        ) -> Result<Envelope<Value>, Refusal> {
            run(&state, &caller, &client, &headers, $service).await
        }
    };
}

test_route!(
    /// `POST /test/plex`.
    test_plex,
    Service::Plex
);
test_route!(
    /// `POST /test/sonarr`.
    test_sonarr,
    Service::Sonarr
);
test_route!(
    /// `POST /test/radarr`.
    test_radarr,
    Service::Radarr
);
test_route!(
    /// `POST /test/lidarr`.
    test_lidarr,
    Service::Lidarr
);
test_route!(
    /// `POST /test/sabnzbd`.
    test_sabnzbd,
    Service::Sabnzbd
);
test_route!(
    /// `POST /test/qbittorrent`.
    test_qbittorrent,
    Service::Qbittorrent
);
test_route!(
    /// `POST /test/tautulli`.
    test_tautulli,
    Service::Tautulli
);
test_route!(
    /// `POST /test/overseerr`.
    test_overseerr,
    Service::Overseerr
);
test_route!(
    /// `POST /test/ombi`.
    test_ombi,
    Service::Ombi
);
test_route!(
    /// `POST /test/sickrage`.
    test_sickrage,
    Service::Sickrage
);
test_route!(
    /// `POST /test/nzbget`.
    test_nzbget,
    Service::Nzbget
);
test_route!(
    /// `POST /test/transmission`.
    test_transmission,
    Service::Transmission
);
test_route!(
    /// `POST /test/deluge`.
    test_deluge,
    Service::Deluge
);
test_route!(
    /// `POST /test/rtorrent`.
    test_rtorrent,
    Service::Rtorrent
);
test_route!(
    /// `POST /test/utorrent`.
    test_utorrent,
    Service::Utorrent
);
test_route!(
    /// `POST /test/jdownloader`.
    test_jdownloader,
    Service::Jdownloader
);
test_route!(
    /// `POST /test/pihole`.
    test_pihole,
    Service::Pihole
);
test_route!(
    /// `POST /test/emby`.
    test_emby,
    Service::Emby
);
test_route!(
    /// `POST /test/jellyfin`.
    test_jellyfin,
    Service::Jellyfin
);
test_route!(
    /// `POST /test/jackett`.
    test_jackett,
    Service::Jackett
);
test_route!(
    /// `POST /test/prowlarr`.
    test_prowlarr,
    Service::Prowlarr
);
test_route!(
    /// `POST /test/adguard`.
    test_adguard,
    Service::Adguard
);
test_route!(
    /// `POST /test/unifi`.
    test_unifi,
    Service::Unifi
);
test_route!(
    /// `POST /test/unifi/site`.
    test_unifi_site,
    Service::UnifiSites
);
test_route!(
    /// `POST /test/embyLiveTVTracker`.
    test_emby_live_tv,
    Service::EmbyLiveTv
);
test_route!(
    /// `POST /test/jellystat`.
    test_jellystat,
    Service::JellyStat
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answer_of_an_arr_application_is_read_for_its_fault() {
        let fault = |status: u16, body: &str| {
            arr_fault(&probe::Answer {
                status,
                body: body.to_owned(),
            })
        };
        assert_eq!(fault(200, r#"[{"path": "/tv"}]"#), None);
        assert_eq!(fault(200, "{}"), None);
        assert_eq!(
            fault(200, "<html>").as_deref(),
            Some("Response was not JSON")
        );
        assert_eq!(fault(200, "5").as_deref(), Some("Response was not JSON"));
        assert_eq!(
            fault(401, r#"{"error": "Unauthorized"}"#).as_deref(),
            Some("Unauthorized")
        );
        assert_eq!(
            fault(200, r#"{"error": {"code": 500, "msg": "Broken"}}"#).as_deref(),
            Some("Broken")
        );
        assert_eq!(
            fault(404, "[]").as_deref(),
            Some("The answer has the status 404")
        );
    }

    #[test]
    fn answer_of_adguard_is_read_for_its_fault() {
        assert_eq!(adguard_fault(true, r#"{"num_dns_queries": 5}"#), None);
        assert_eq!(adguard_fault(true, "<html>"), Some("Response was not JSON"));
        assert_eq!(adguard_fault(true, "5"), Some("Response was not JSON"));
        assert_eq!(adguard_fault(false, "{}"), Some("Unknown Failure"));
    }

    #[test]
    fn long_text_of_a_service_is_cut() {
        assert_eq!(shown(&"a".repeat(SHOWN + 5)).len(), SHOWN);
        assert_eq!(shown("short"), "short");
    }
}
