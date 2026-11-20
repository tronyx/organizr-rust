//! The download clients of the homepage: SABnzbd, qBittorrent, NZBGet and Transmission.
//! Deluge, rTorrent, uTorrent and JDownloader are in `downloaders_more`.

use std::time::Duration;

use reqwest::header;
use serde_json::Value;

use crate::http::Outbound;

/// How long a download client has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// Why a download client gave no usable answer.
#[derive(Debug, thiserror::Error)]
pub enum DownloaderError {
    /// The client cannot be reached, or it did not answer in time. The text names no
    /// address, because the address of SABnzbd holds its API key.
    #[error("the download client cannot be reached: {0}")]
    Unreachable(String),
    /// The client did not start a session: the name or the password is wrong.
    #[error("the download client gave no session")]
    NoSession,
}

fn unreachable(error: reqwest::Error) -> DownloaderError {
    DownloaderError::Unreachable(error.without_url().to_string())
}

/// An install of a download client, and how it is asked.
#[derive(Debug, Clone, Copy)]
pub struct Client<'a> {
    /// The address, complete.
    pub url: &'a str,
    /// How long the client has to answer.
    pub timeout: Option<Duration>,
    /// Whether the certificate of the client is left unchecked.
    pub never_check_certificate: bool,
}

impl Client<'_> {
    pub(crate) fn base(&self) -> &str {
        self.url.trim_end_matches('/')
    }

    pub(crate) fn limit(&self) -> Duration {
        self.timeout.unwrap_or(TIME_LIMIT)
    }
}

/// Asks SABnzbd with the given fields of its API, for example `mode=queue`.
///
/// Gives the answer as SABnzbd sent it, or nothing when SABnzbd answered with an
/// error or with something that is no JSON.
pub async fn sabnzbd(
    http: &Outbound,
    client: Client<'_>,
    key: &str,
    fields: &[(&str, &str)],
) -> Result<Option<Value>, DownloaderError> {
    let url = format!("{}/api", client.base());
    let mut query = fields.to_vec();
    query.push(("output", "json"));
    query.push(("apikey", key));
    let response = http
        .client(&url, client.never_check_certificate)
        .get(&url)
        .query(&query)
        .timeout(client.limit())
        .send()
        .await
        .map_err(unreachable)?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "SABnzbd refused the request");
        return Ok(None);
    }
    Ok(response.json().await.ok())
}

/// The name and the password of an account of a download client.
#[derive(Clone, Copy)]
pub struct Login<'a> {
    /// The user name.
    pub username: &'a str,
    /// The password.
    pub password: &'a str,
}

impl Login<'_> {
    /// The request with the name and the password, when both are set.
    pub(crate) fn on(self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.username.is_empty() || self.password.is_empty() {
            request
        } else {
            request.basic_auth(self.username, Some(self.password))
        }
    }
}

/// The queue and the history of NZBGet, each as NZBGet sent it, or nothing when
/// NZBGet refused that request.
pub async fn nzbget(
    http: &Outbound,
    client: Client<'_>,
    login: Login<'_>,
) -> Result<(Option<Value>, Option<Value>), DownloaderError> {
    let mut answers = [None, None];
    for (answer, method) in answers.iter_mut().zip(["listgroups", "history"]) {
        let url = format!("{}/jsonrpc/{method}", client.base());
        let request = http
            .client(&url, client.never_check_certificate)
            .get(&url)
            .timeout(client.limit());
        let response = login.on(request).send().await.map_err(unreachable)?;
        if response.status().is_success() {
            *answer = response.json::<Value>().await.ok();
        } else {
            tracing::warn!(status = %response.status(), method, "NZBGet refused the request");
        }
    }
    let [queue, history] = answers;
    Ok((queue, history))
}

/// The header in which Transmission names its session.
const TRANSMISSION_SESSION: &str = "x-transmission-session-id";

/// The fields of a torrent that Transmission is asked for.
const TRANSMISSION_FIELDS: [&str; 12] = [
    "id",
    "name",
    "totalSize",
    "eta",
    "isFinished",
    "isStalled",
    "percentDone",
    "rateDownload",
    "status",
    "downloadDir",
    "errorString",
    "addedDate",
];

/// The torrents of Transmission, as it sent them, or nothing when Transmission
/// refused the request for them.
pub async fn transmission(
    http: &Outbound,
    client: Client<'_>,
    login: Login<'_>,
) -> Result<Option<Vec<Value>>, DownloaderError> {
    let url = format!("{}/rpc", client.base());
    let web = http.client(&url, client.never_check_certificate);
    // Transmission refuses the first request, and names the session in its answer.
    let response = login
        .on(web.get(&url).timeout(client.limit()))
        .send()
        .await
        .map_err(unreachable)?;
    let session = response
        .headers()
        .get(TRANSMISSION_SESSION)
        .and_then(|value| value.to_str().ok())
        .filter(|session| !session.is_empty())
        .ok_or(DownloaderError::NoSession)?
        .to_owned();

    let body = serde_json::json!({
        "method": "torrent-get",
        "arguments": { "fields": TRANSMISSION_FIELDS },
        "tags": "",
    });
    let request = web
        .post(&url)
        .header(TRANSMISSION_SESSION, session)
        .json(&body)
        .timeout(client.limit());
    let response = login.on(request).send().await.map_err(unreachable)?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "Transmission refused the request");
        return Ok(None);
    }
    let answer = response.json::<Value>().await.unwrap_or(Value::Null);
    let torrents = answer
        .pointer("/arguments/torrents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(Some(torrents))
}

/// The torrents of Transmission without the ones that the settings hide, the oldest
/// first. Transmission gives each torrent a status: 0 for stopped, 5 for waiting to
/// upload, and 6 for uploading.
pub fn shown_transfers(
    mut torrents: Vec<Value>,
    hide_seeding: bool,
    hide_completed: bool,
) -> Vec<Value> {
    torrents.retain(|torrent| {
        let status = torrent.get("status").and_then(Value::as_i64);
        let seeding = matches!(status, Some(5 | 6));
        let stopped = status == Some(0);
        !(hide_seeding && seeding || hide_completed && stopped)
    });
    torrents.sort_by_key(|torrent| torrent.get("addedDate").and_then(Value::as_i64));
    torrents
}

/// How qBittorrent is asked for its torrents.
#[derive(Debug, Clone, Copy)]
pub struct TorrentQuery<'a> {
    /// The user name.
    pub username: &'a str,
    /// The password.
    pub password: &'a str,
    /// Whether the install has the first version of the API, which qBittorrent had
    /// before version 4.1.
    pub first_api: bool,
    /// The field that the torrents are sorted by, for example `eta`.
    pub sort: &'a str,
    /// Whether the order is turned around.
    pub reverse: bool,
}

/// The value of the session cookie in the answer to a login. Its name is `SID`; a
/// newer qBittorrent can add something to the name.
fn session_cookie(headers: &header::HeaderMap) -> Option<String> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|cookie| cookie.split(';').next()?.split_once('='))
        .find(|(name, value)| name.trim().contains("SID") && !value.is_empty())
        .map(|(name, value)| format!("{}={value}", name.trim()))
}

/// The torrents of qBittorrent, as it sent them.
pub async fn qbittorrent(
    http: &Outbound,
    client: Client<'_>,
    query: TorrentQuery<'_>,
) -> Result<Vec<Value>, DownloaderError> {
    let (login, list) = if query.first_api {
        ("/login", "/query/torrents")
    } else {
        ("/api/v2/auth/login", "/api/v2/torrents/info")
    };
    let url = format!("{}{login}", client.base());
    let web = http.client(&url, client.never_check_certificate);
    let response = web
        .post(&url)
        .form(&[("username", query.username), ("password", query.password)])
        .timeout(client.limit())
        .send()
        .await
        .map_err(unreachable)?;
    let cookie = session_cookie(response.headers()).ok_or(DownloaderError::NoSession)?;

    let url = format!("{}{list}", client.base());
    let reverse = if query.reverse { "true" } else { "false" };
    let response = web
        .get(&url)
        .query(&[("sort", query.sort), ("reverse", reverse)])
        .header(header::COOKIE, cookie)
        .timeout(client.limit())
        .send()
        .await
        .map_err(unreachable)?;
    match response.json::<Value>().await {
        Ok(Value::Array(torrents)) => Ok(torrents),
        _ => Ok(Vec::new()),
    }
}

/// The torrents without the ones that the settings hide: the ones that only upload,
/// and the ones that are complete and paused.
pub fn shown_torrents(
    torrents: Vec<Value>,
    hide_seeding: bool,
    hide_completed: bool,
) -> Vec<Value> {
    let mut hidden: Vec<&str> = Vec::new();
    if hide_seeding {
        hidden.extend(["uploading", "stalledUP", "queuedUP"]);
    }
    if hide_completed {
        hidden.push("pausedUP");
    }
    torrents
        .into_iter()
        .filter(|torrent| {
            let state = torrent.get("state").and_then(Value::as_str);
            !state.is_some_and(|state| hidden.contains(&state))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderMap, HeaderValue, SET_COOKIE};
    use serde_json::json;

    use super::{Value, session_cookie, shown_torrents, shown_transfers};

    fn headers(cookies: &[&'static str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for cookie in cookies {
            headers.append(SET_COOKIE, HeaderValue::from_static(cookie));
        }
        headers
    }

    #[test]
    fn session_cookie_is_the_one_with_sid_in_its_name() {
        assert_eq!(
            session_cookie(&headers(&[
                "lang=en; path=/",
                "SID=abc123; HttpOnly; path=/"
            ])),
            Some("SID=abc123".to_owned())
        );
        assert_eq!(
            session_cookie(&headers(&["QBT_SID_8080=xyz; HttpOnly"])),
            Some("QBT_SID_8080=xyz".to_owned())
        );
        assert_eq!(session_cookie(&headers(&["lang=en"])), None);
        assert_eq!(session_cookie(&headers(&["SID=; path=/"])), None);
        assert_eq!(session_cookie(&headers(&[])), None);
    }

    #[test]
    fn transfers_that_the_settings_hide_are_left_out_and_the_oldest_is_first() {
        let torrent = |name: &str, status: i64, added: i64| serde_json::json!({"name": name, "status": status, "addedDate": added});
        let all = vec![
            torrent("new", 4, 30),
            torrent("stopped", 0, 20),
            torrent("waits to upload", 5, 10),
            torrent("uploads", 6, 5),
        ];
        let names = |torrents: Vec<Value>| -> Vec<String> {
            torrents
                .iter()
                .map(|torrent| torrent["name"].as_str().unwrap_or_default().to_owned())
                .collect()
        };
        assert_eq!(
            names(shown_transfers(all.clone(), false, false)),
            ["uploads", "waits to upload", "stopped", "new"]
        );
        assert_eq!(
            names(shown_transfers(all.clone(), true, false)),
            ["stopped", "new"]
        );
        assert_eq!(
            names(shown_transfers(all.clone(), false, true)),
            ["uploads", "waits to upload", "new"]
        );
        assert_eq!(names(shown_transfers(all, true, true)), ["new"]);
    }

    #[test]
    fn torrents_that_the_settings_hide_are_left_out() {
        let torrents = || {
            [
                "downloading",
                "uploading",
                "stalledUP",
                "queuedUP",
                "pausedUP",
                "pausedDL",
            ]
            .iter()
            .map(|state| json!({"name": state, "state": state}))
            .collect::<Vec<_>>()
        };
        let names = |hide_seeding, hide_completed| {
            shown_torrents(torrents(), hide_seeding, hide_completed)
                .iter()
                .map(|torrent| torrent["name"].as_str().unwrap_or("?").to_owned())
                .collect::<Vec<_>>()
        };

        assert_eq!(names(false, false).len(), 6);
        assert_eq!(names(true, false), ["downloading", "pausedUP", "pausedDL"]);
        assert_eq!(
            names(false, true),
            [
                "downloading",
                "uploading",
                "stalledUP",
                "queuedUP",
                "pausedDL"
            ]
        );
        assert_eq!(names(true, true), ["downloading", "pausedDL"]);
    }
}
