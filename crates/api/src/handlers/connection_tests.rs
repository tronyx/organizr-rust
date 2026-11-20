//! `POST /test/{service}`: the connection tests of the settings pages. A test asks the
//! service with the settings that are stored, and says whether the service answered.
//!
//! Only an Admin or a Co-Admin can run a test.

use std::time::Duration;

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_domain::GroupLevel;
use organizr_integrations::downloaders::{
    self, Client as Downloader, DownloaderError, Login, TorrentQuery,
};
use organizr_integrations::emby::Kind as MediaServer;
use organizr_integrations::http::Outbound;
use organizr_integrations::pihole;
use organizr_integrations::probe::{self, Probe};
use serde_json::Value;

use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::{Permissions, emby, no_requests, service_url};
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

/// What is wrong with the answer of Pi-hole to the question for its status, or nothing.
fn pihole_fault(is_success: bool, body: &str) -> Option<&'static str> {
    if !is_success {
        return Some("Unknown Failure");
    }
    match serde_json::from_str::<Value>(body) {
        Ok(answer) if answer.get("status").is_some() => None,
        Ok(Value::Object(_) | Value::Array(_)) => Some("Missing API Token"),
        _ => Some("Response was not JSON"),
    }
}

async fn pi_hole(test: &Test<'_>) -> Outcome {
    let settings = test.state.settings();
    if settings.pihole_url.is_empty() {
        return Err(Refusal::unprocessable("Pihole URL is not defined"));
    }
    let mut errors = String::new();
    for (address, token) in pihole::installs(&settings.pihole_url, &settings.pihole_token) {
        let base = test.url(address);
        let url = format!("{base}/api.php?status");
        let with_token = [("auth", token)];
        let probe = Probe {
            url: &url,
            query: if token.is_empty() { &[] } else { &with_token },
            ..Probe::default()
        };
        let fault = match test.get(probe).await {
            Ok(answer) => pihole_fault(answer.is_success(), &answer.body).map(str::to_owned),
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
    Nzbget,
    Transmission,
    Pihole,
    Emby,
    Jellyfin,
}

async fn run(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    headers: &HeaderMap,
    service: Service,
) -> Outcome {
    caller.require(GroupLevel::CO_ADMIN)?;
    let http = state.outbound().ok_or_else(no_requests)?;
    let test = Test {
        state,
        headers,
        client,
        http,
    };
    let settings = state.settings();
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
        Service::Nzbget => nzbget(&test).await,
        Service::Transmission => transmission(&test).await,
        Service::Pihole => pi_hole(&test).await,
        Service::Emby => media_server(&test, caller, MediaServer::Emby).await,
        Service::Jellyfin => media_server(&test, caller, MediaServer::Jellyfin).await,
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
    fn answer_of_pihole_is_read_for_its_fault() {
        assert_eq!(pihole_fault(true, r#"{"status": "enabled"}"#), None);
        assert_eq!(pihole_fault(true, "[]"), Some("Missing API Token"));
        assert_eq!(pihole_fault(true, "{}"), Some("Missing API Token"));
        assert_eq!(pihole_fault(true, "<html>"), Some("Response was not JSON"));
        assert_eq!(pihole_fault(false, "{}"), Some("Unknown Failure"));
    }

    #[test]
    fn long_text_of_a_service_is_cut() {
        assert_eq!(shown(&"a".repeat(SHOWN + 5)).len(), SHOWN);
        assert_eq!(shown("short"), "short");
    }
}
