//! More download clients of the homepage: the queues of Deluge, rTorrent, uTorrent and
//! JDownloader.

use std::time::Duration;

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_integrations::downloaders::{Client as Downloader, Login};
use organizr_integrations::downloaders_more::{
    self as clients, ClientError, DELUGE_TIME_LIMIT, DelugeView, RtorrentView,
};
use organizr_integrations::http::{Outbound, time_limit};
use organizr_settings::Settings;
use serde_json::{Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// What must hold before a caller gets the Deluge item.
pub fn deluge_permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageDelugeEnabled".to_owned()],
        auth: vec!["homepageDelugeAuth".to_owned()],
        not_empty: vec!["delugeURL".to_owned()],
    }
}

/// What must hold before a caller gets the rTorrent item. The PHP application does not
/// ask for an address here: the route of the queue asks for it.
pub fn rtorrent_permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepagerTorrentEnabled".to_owned()],
        auth: vec!["homepagerTorrentAuth".to_owned()],
        not_empty: Vec::new(),
    }
}

/// What must hold before a caller gets the uTorrent item.
pub fn utorrent_permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageuTorrentEnabled".to_owned()],
        auth: vec!["homepageuTorrentAuth".to_owned()],
        not_empty: vec!["uTorrentURL".to_owned()],
    }
}

/// What must hold before a caller gets the JDownloader item.
pub fn jdownloader_permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageJdownloaderEnabled".to_owned()],
        auth: vec!["homepageJdownloaderAuth".to_owned()],
        not_empty: vec!["jdownloaderURL".to_owned()],
    }
}

/// The answer when rTorrent has no address, as in the PHP application.
pub fn no_rtorrent_address(settings: &Settings) -> Result<(), Refusal> {
    if settings.rtorrent_url.is_empty() && settings.rtorrent_url_override.is_empty() {
        Err(Refusal::unprocessable("rTorrent URL is not defined"))
    } else {
        Ok(())
    }
}

/// The answer when a download client cannot be reached, or refused the request.
pub fn failed(error: &ClientError) -> Refusal {
    tracing::warn!(%error, "a download client gave no usable answer");
    Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

/// An address without its query and its fragment. The PHP application leaves both out
/// of the addresses of rTorrent and of uTorrent.
fn without_query(mut address: String) -> String {
    if let Some(at) = address.find(['?', '#']) {
        address.truncate(at);
    }
    address
}

/// What a request to a download client needs: the settings, and the request of the
/// caller for an address that is a path on the host of Organizr.
pub struct Asked<'a> {
    /// The settings.
    pub settings: &'a Settings,
    /// The client for requests to other services.
    pub http: &'a Outbound,
    /// The headers of the request of the caller.
    pub headers: &'a HeaderMap,
    /// How the caller reached Organizr.
    pub client: &'a Client,
    /// How long the download client has to answer. Without it, the time between two
    /// loads of the item is the limit, as in the PHP application.
    pub timeout: Option<Duration>,
}

impl Asked<'_> {
    fn limit(&self, refresh: u64) -> Option<Duration> {
        self.timeout.or_else(|| time_limit(&refresh.to_string()))
    }

    /// The torrents of Deluge.
    pub async fn deluge(&self) -> Result<Vec<Value>, ClientError> {
        let settings = self.settings;
        let url = service_url(&settings.deluge_url, self.headers, self.client);
        let install = Downloader {
            url: &url,
            timeout: Some(DELUGE_TIME_LIMIT),
            never_check_certificate: settings.deluge_disable_cert_check,
        };
        clients::deluge(self.http, install, &settings.deluge_password).await
    }

    /// The address of the XML-RPC endpoint of rTorrent. Without the second address of
    /// the settings, it is the address with `/RPC2`, or the address alone when it
    /// names a PHP file. The second address stands in place of the address when it
    /// holds the address, and else after it.
    fn rtorrent_endpoint(&self) -> String {
        let address = &self.settings.rtorrent_url;
        let other = &self.settings.rtorrent_url_override;
        let complete =
            |address: &str| without_query(service_url(address, self.headers, self.client));
        if other.is_empty() {
            let endpoint = complete(address);
            if address.contains(".php") {
                endpoint
            } else {
                format!("{endpoint}/RPC2")
            }
        } else if other.contains(address.as_str()) {
            complete(other)
        } else {
            complete(&format!("{address}{other}"))
        }
    }

    fn rtorrent_login(&self) -> Login<'_> {
        Login {
            username: &self.settings.rtorrent_username,
            password: &self.settings.rtorrent_password,
        }
    }

    fn rtorrent_install<'a>(&self, url: &'a str) -> Downloader<'a> {
        Downloader {
            url,
            timeout: self.limit(self.settings.rtorrent_refresh),
            never_check_certificate: self.settings.rtorrent_disable_cert_check,
        }
    }

    /// The torrents of rTorrent, or nothing when rTorrent gave no list.
    pub async fn rtorrent(&self) -> Result<Option<Vec<Value>>, ClientError> {
        let url = self.rtorrent_endpoint();
        clients::rtorrent(
            self.http,
            self.rtorrent_install(&url),
            self.rtorrent_login(),
        )
        .await
    }

    /// Whether rTorrent names its methods.
    pub async fn rtorrent_answers(&self) -> Result<bool, ClientError> {
        let url = self.rtorrent_endpoint();
        clients::rtorrent_answers(
            self.http,
            self.rtorrent_install(&url),
            self.rtorrent_login(),
        )
        .await
    }

    /// The torrents of uTorrent, or nothing when uTorrent refused the request.
    pub async fn utorrent(&self) -> Result<Option<Vec<Value>>, ClientError> {
        let settings = self.settings;
        let url = without_query(service_url(
            &settings.utorrent_url,
            self.headers,
            self.client,
        ));
        let install = Downloader {
            url: &url,
            timeout: self.limit(settings.utorrent_refresh),
            never_check_certificate: settings.utorrent_disable_cert_check,
        };
        let login = Login {
            username: &settings.utorrent_username,
            password: &settings.utorrent_password,
        };
        clients::utorrent(self.http, install, login).await
    }

    /// The answer of the service of JDownloader, or nothing when it refused the request.
    pub async fn jdownloader(&self) -> Result<Option<Value>, ClientError> {
        let settings = self.settings;
        let url = service_url(&settings.jdownloader_url, self.headers, self.client);
        let install = Downloader {
            url: &url,
            timeout: self.limit(settings.jdownloader_refresh),
            never_check_certificate: settings.jdownloader_disable_cert_check,
        };
        let login = Login {
            username: &settings.jdownloader_username,
            password: &settings.jdownloader_password,
        };
        clients::jdownloader(self.http, install, login).await
    }
}

/// `GET /homepage/deluge/queue`: the torrents of Deluge, without the ones that the
/// settings hide.
pub async fn deluge_queue(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    deluge_permissions().check(&settings, &caller)?;
    let asked = Asked {
        settings: &settings,
        http: state.outbound().ok_or_else(no_requests)?,
        headers: &headers,
        client: &client,
        timeout: None,
    };
    let torrents = asked.deluge().await.map_err(|error| failed(&error))?;
    let view = DelugeView {
        hide_seeding: settings.deluge_hide_seeding,
        hide_completed: settings.deluge_hide_completed,
        hide_status: settings.deluge_hide_status,
    };
    Ok(Envelope::success(json!({
        "content": {
            "queueItems": clients::shown_deluge(torrents, view),
            "historyItems": false,
        },
    })))
}

/// `GET /homepage/rtorrent/queue`: the torrents of rTorrent, in the order of the
/// settings and without the ones that the settings hide.
pub async fn rtorrent_queue(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    // The PHP application looks for the address before everything else.
    no_rtorrent_address(&settings)?;
    rtorrent_permissions().check(&settings, &caller)?;
    let asked = Asked {
        settings: &settings,
        http: state.outbound().ok_or_else(no_requests)?,
        headers: &headers,
        client: &client,
        timeout: None,
    };
    let rows = asked.rtorrent().await.map_err(|error| failed(&error))?;
    let content = rows.map_or(Value::Bool(false), |rows| {
        let view = RtorrentView {
            hide_seeding: settings.rtorrent_hide_seeding,
            hide_completed: settings.rtorrent_hide_completed,
            ignored_labels: &settings.rtorrent_ignore_label,
            sort_order: &settings.rtorrent_sort_order,
            limit: settings.rtorrent_limit,
        };
        json!({
            "queueItems": clients::rtorrent_items(&rows, &view),
            "historyItems": false,
        })
    });
    Ok(Envelope::success(json!({ "content": content })))
}

/// `GET /homepage/utorrent/queue`: the torrents of uTorrent, without the ones that the
/// settings hide.
pub async fn utorrent_queue(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    utorrent_permissions().check(&settings, &caller)?;
    let asked = Asked {
        settings: &settings,
        http: state.outbound().ok_or_else(no_requests)?,
        headers: &headers,
        client: &client,
        timeout: None,
    };
    let rows = asked.utorrent().await.map_err(|error| failed(&error))?;
    let content = rows.map_or(Value::Bool(false), |rows| {
        let items = clients::utorrent_items(
            &rows,
            settings.utorrent_hide_seeding,
            settings.utorrent_hide_completed,
        );
        json!({ "queueItems": items })
    });
    Ok(Envelope::success(json!({ "content": content })))
}

/// `GET /homepage/jdownloader/queue`: the packages of JDownloader, as its service
/// gives them.
pub async fn jdownloader_queue(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    jdownloader_permissions().check(&settings, &caller)?;
    let asked = Asked {
        settings: &settings,
        http: state.outbound().ok_or_else(no_requests)?,
        headers: &headers,
        client: &client,
        timeout: None,
    };
    let answer = asked.jdownloader().await.map_err(|error| failed(&error))?;
    let content = answer.map_or(Value::Bool(false), |answer| {
        clients::jdownloader_content(&answer)
    });
    Ok(Envelope::success(json!({ "content": content })))
}
