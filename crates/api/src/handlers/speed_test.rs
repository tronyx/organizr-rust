//! The speed test plugin: the routes that the browser measures the connection to
//! Organizr with.
//!
//! PHP Organizr has three PHP files of the LibreSpeed project for this. Here three
//! routes do the same: one sends bytes, one takes bytes, and one names the address
//! of the caller.

use std::convert::Infallible;
use std::sync::OnceLock;

use axum::Extension;
use axum::body::{Body, Bytes};
use axum::extract::{RawQuery, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use http_body_util::BodyExt;
use organizr_domain::GroupLevel;
use serde_json::{Value, json};

use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The size of one chunk of a download: one mebibyte.
const CHUNK: usize = 1024 * 1024;

/// The largest number of chunks that one download request gets.
pub const MOST_CHUNKS: usize = 100;

/// The largest upload that one request can send, in bytes.
pub const LARGEST_UPLOAD: usize = 64 * 1024 * 1024;

/// The plugin is for callers of the level of its setting, and only when it is on.
fn check(state: &AppState, caller: &Caller) -> Result<(), Refusal> {
    let settings = state.settings();
    if !settings.speed_test_enabled {
        return Err(Refusal::conflict("SPEEDTEST-enabled module is not enabled"));
    }
    let level = u16::try_from(settings.speed_test_auth_include).unwrap_or(0);
    caller.require(GroupLevel::new(level).unwrap_or(GroupLevel::ADMIN))?;
    Ok(())
}

/// One chunk of bytes that a proxy cannot compress. The bytes are the same for each
/// request: they carry nothing, they only fill the line.
fn chunk() -> Bytes {
    static CHUNK_BYTES: OnceLock<Bytes> = OnceLock::new();
    CHUNK_BYTES
        .get_or_init(|| {
            // A small generator of numbers that look random. Nothing here is secret.
            let mut state = 0x9E37_79B9_7F4A_7C15_u64;
            let mut bytes = Vec::with_capacity(CHUNK);
            while bytes.len() < CHUNK {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                bytes.extend_from_slice(&state.to_le_bytes());
            }
            bytes.truncate(CHUNK);
            Bytes::from(bytes)
        })
        .clone()
}

/// The number of chunks that the query asks for with `ckSize`: 4 without it, and at
/// most [`MOST_CHUNKS`].
fn chunks_asked(query: Option<&str>) -> usize {
    query
        .into_iter()
        .flat_map(|query| query.split('&'))
        .find_map(|pair| pair.strip_prefix("ckSize="))
        .and_then(|size| size.parse::<usize>().ok())
        .map_or(4, |size| size.clamp(1, MOST_CHUNKS))
}

fn no_store(response: &mut Response) {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
    );
}

/// `GET /plugins/speedtest/garbage`: bytes to measure the download with. The query
/// field `ckSize` is the number of mebibytes.
pub async fn garbage(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    RawQuery(query): RawQuery,
) -> Result<Response, Refusal> {
    check(&state, &caller)?;
    let count = chunks_asked(query.as_deref());
    let stream = futures_util::stream::iter((0..count).map(|_| Ok::<Bytes, Infallible>(chunk())));
    let mut response = Response::new(Body::from_stream(stream));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    // A proxy that compresses the bytes would make the line look faster than it is.
    headers.insert(
        header::CONTENT_ENCODING,
        HeaderValue::from_static("identity"),
    );
    if let Ok(length) = HeaderValue::from_str(&(count * CHUNK).to_string()) {
        headers.insert(header::CONTENT_LENGTH, length);
    }
    no_store(&mut response);
    Ok(response)
}

/// `GET /plugins/speedtest/empty`: an empty answer, to measure the time of a request.
pub async fn ping(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Response, Refusal> {
    check(&state, &caller)?;
    let mut response = Response::new(Body::empty());
    no_store(&mut response);
    Ok(response)
}

/// `POST /plugins/speedtest/empty`: takes the bytes of the body and keeps none of
/// them, to measure the upload. The answer says how many bytes came.
pub async fn upload(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    mut body: Body,
) -> Result<Envelope<Value>, Refusal> {
    check(&state, &caller)?;
    let mut received = 0_usize;
    while let Some(frame) = body.frame().await {
        let Ok(frame) = frame else {
            return Err(Refusal::Request(
                StatusCode::BAD_REQUEST,
                "The upload was cut off".to_owned(),
            ));
        };
        if let Some(data) = frame.data_ref() {
            received += data.len();
            if received > LARGEST_UPLOAD {
                return Err(Refusal::Request(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "The upload is too large".to_owned(),
                ));
            }
        }
    }
    Ok(Envelope::success(json!({ "received": received })))
}

/// `GET /plugins/speedtest/ip`: the address that the server sees the caller at.
pub async fn ip(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
) -> Result<Envelope<Value>, Refusal> {
    check(&state, &caller)?;
    Ok(Envelope::success(
        json!({ "ip": client.address.to_string() }),
    ))
}

#[cfg(test)]
mod tests {
    use super::{CHUNK, MOST_CHUNKS, chunk, chunks_asked};

    #[test]
    fn download_size_comes_from_the_query_within_limits() {
        assert_eq!(chunks_asked(None), 4);
        assert_eq!(chunks_asked(Some("ckSize=20")), 20);
        assert_eq!(chunks_asked(Some("r=0.5&ckSize=7")), 7);
        assert_eq!(chunks_asked(Some("ckSize=0")), 1);
        assert_eq!(chunks_asked(Some("ckSize=100000")), MOST_CHUNKS);
        assert_eq!(chunks_asked(Some("ckSize=many")), 4);
    }

    #[test]
    fn chunk_is_one_mebibyte_that_does_not_repeat_at_once() {
        let bytes = chunk();
        assert_eq!(bytes.len(), CHUNK);
        assert_ne!(bytes[..64], bytes[64..128]);
        assert!(bytes.iter().any(|byte| *byte != bytes[0]));
    }
}
