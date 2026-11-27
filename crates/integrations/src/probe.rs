//! One request to a service, for a connection test: the status and the text of the
//! answer.

use std::net::SocketAddr;
use std::time::Duration;

use crate::http::{Outbound, without_address};

/// How long a service has to answer a test.
const TIME_LIMIT: Duration = Duration::from_secs(15);

/// A request of a test.
#[derive(Debug, Clone, Copy, Default)]
pub struct Probe<'a> {
    /// The address, without a query.
    pub url: &'a str,
    /// The fields of the query.
    pub query: &'a [(&'a str, &'a str)],
    /// Headers of the request.
    pub headers: &'a [(&'a str, &'a str)],
    /// Whether the certificate of the service is left unchecked.
    pub never_check_certificate: bool,
}

/// What a service answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The HTTP status.
    pub status: u16,
    /// The body as text.
    pub body: String,
}

impl Answer {
    /// Whether the status says that the request worked.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Sends the request. Fails when the service cannot be reached. The text of the error
/// names no address, because the address of some services holds a key.
pub async fn get(http: &Outbound, probe: Probe<'_>) -> Result<Answer, String> {
    // A header of the probe can hold a key, which a redirect must not take elsewhere.
    let client = if probe.headers.is_empty() {
        http.client(probe.url, probe.never_check_certificate)
    } else {
        http.keyed_client(probe.url, probe.never_check_certificate)
    };
    let mut request = client.get(probe.url).timeout(TIME_LIMIT);
    if !probe.query.is_empty() {
        request = request.query(probe.query);
    }
    for (name, value) in probe.headers {
        request = request.header(*name, *value);
    }
    let response = request.send().await.map_err(without_address)?;
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    Ok(Answer { status, body })
}

/// The host of an address.
pub fn host_of(address: &str) -> Option<String> {
    reqwest::Url::parse(address)
        .ok()?
        .host_str()
        .map(str::to_owned)
}

/// How long a download has.
const DOWNLOAD_TIME_LIMIT: Duration = Duration::from_secs(120);

/// Downloads a file of at most `limit` bytes. Fails when the server cannot be reached,
/// when it answers with an error, and when the file is larger than the limit.
pub async fn download(http: &Outbound, url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let response = http
        .client_for(url)
        .get(url)
        .timeout(DOWNLOAD_TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
    body_of(response, limit).await
}

/// What one request of a download gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// The file.
    File(Vec<u8>),
    /// A redirect to this whole address.
    Redirect(String),
}

/// As [`download`], with a client that follows no redirect. A redirect gives its
/// address, so that the caller can check it before it follows. A redirect without an
/// address that can be read fails.
///
/// With `pinned`, the request reaches the host of `url` at that address and nowhere
/// else, without a new lookup of the name, so a name that answers with another address
/// the second time cannot lead the request elsewhere (see [`Outbound::pinned`]).
pub async fn download_step(
    http: &Outbound,
    url: &str,
    limit: usize,
    pinned: Option<SocketAddr>,
) -> Result<Step, String> {
    let pinned_client;
    let client = match pinned {
        Some(address) => {
            let parsed = reqwest::Url::parse(url).map_err(|error| error.to_string())?;
            let host = parsed
                .host_str()
                .ok_or_else(|| "the address has no host".to_owned())?;
            pinned_client = http.pinned(host, address).map_err(without_address)?;
            &pinned_client
        }
        None => http.direct_client_for(url),
    };
    let response = client
        .get(url)
        .timeout(DOWNLOAD_TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
    if response.status().is_redirection() {
        let next = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|location| response.url().join(location).ok())
            .ok_or_else(|| "the server answered with a redirect without an address".to_owned())?;
        return Ok(Step::Redirect(next.into()));
    }
    body_of(response, limit).await.map(Step::File)
}

/// The body of an answer of at most `limit` bytes. Fails when the answer is an error
/// and when the body is larger than the limit.
async fn body_of(mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>, String> {
    let too_large = || format!("the file is larger than {limit} bytes");
    if !response.status().is_success() {
        return Err(format!(
            "the server answered with the status {}",
            response.status().as_u16()
        ));
    }
    let announced = response.content_length().unwrap_or(0);
    if usize::try_from(announced).unwrap_or(usize::MAX) > limit {
        return Err(too_large());
    }
    // The size is counted while the file comes, because a server can name no size.
    let mut bytes = Vec::new();
    while let Some(piece) = response.chunk().await.map_err(without_address)? {
        if bytes.len() + piece.len() > limit {
            return Err(too_large());
        }
        bytes.extend_from_slice(&piece);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use organizr_testkit::services::StandIn;

    /// A step with a pinned address reaches that address, not one that a new lookup of
    /// the name would give. The name here never resolves, so only the pin reaches it.
    #[tokio::test]
    async fn pinned_step_reaches_the_checked_address_without_a_lookup() {
        let site = StandIn::start(Router::new().route("/file", get(|| async { "the file" }))).await;
        let address: std::net::SocketAddr = site
            .url()
            .trim_start_matches("http://")
            .parse()
            .expect("the address of the site");
        let http = Outbound::new().expect("the client can be built");
        let url = format!("http://download.invalid:{}/file", address.port());

        let pinned = download_step(&http, &url, 100, Some(address)).await;
        assert_eq!(pinned, Ok(Step::File(b"the file".to_vec())));
        let looked_up = download_step(&http, &url, 100, None).await;
        assert!(looked_up.is_err(), "{looked_up:?}");
    }
}
