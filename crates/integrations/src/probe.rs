//! One request to a service, for a connection test: the status and the text of the
//! answer.

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
    let mut request = http
        .client(probe.url, probe.never_check_certificate)
        .get(probe.url)
        .timeout(TIME_LIMIT);
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
    let too_large = || format!("the file is larger than {limit} bytes");
    let mut response = http
        .client_for(url)
        .get(url)
        .timeout(DOWNLOAD_TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
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
