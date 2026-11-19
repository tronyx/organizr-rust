//! The picture cache: pictures of other applications that the server fetches and
//! keeps, and sends from `data/cache/`.
//!
//! A browser cannot load a picture from Sonarr or Radarr without their API key. The
//! PHP application fetches such a picture while it answers the request for the
//! calendar. Here the server fetches it in the background, so the calendar does not
//! wait. Until the picture is there, its address answers `404`.

use std::collections::HashSet;
use std::path::{Path as FilePath, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, header};
use axum::response::Response;
use organizr_integrations::http::{Outbound, without_address};

use crate::error::ApiError;
use crate::state::AppState;

/// The largest picture that is kept, in bytes.
const SIZE_LIMIT: usize = 10 * 1024 * 1024;

/// How long the source of a picture has to send it.
const TIME_LIMIT: Duration = Duration::from_secs(15);

/// Where a picture comes from.
#[derive(Debug, Clone)]
pub struct Source {
    /// The address of the picture.
    pub url: String,
    /// The API key of the application, when the picture is a file of the application
    /// itself.
    pub key: Option<String>,
    /// Whether the certificate of the application is left unchecked.
    pub never_check_certificate: bool,
}

/// The folder of the cache, and the pictures that are on their way.
#[derive(Debug, Clone)]
pub struct PictureCache {
    folder: Arc<PathBuf>,
    busy: Arc<Mutex<HashSet<String>>>,
}

/// Whether the text can be the name of a cached picture, without its ending: letters,
/// digits and hyphens only, so the name cannot leave the folder.
pub fn is_cache_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

/// The content type of a picture, read from its first bytes. Nothing for a file that
/// is no picture of a kind that a browser shows.
pub fn picture_type(content: &[u8]) -> Option<&'static str> {
    if content.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if content.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if content.starts_with(b"GIF8") {
        Some("image/gif")
    } else if content.starts_with(b"RIFF") && content.get(8..12) == Some(b"WEBP".as_slice()) {
        Some("image/webp")
    } else {
        None
    }
}

impl PictureCache {
    /// A cache in the given folder. The folder is made when the first picture comes.
    pub fn new(folder: PathBuf) -> Self {
        Self {
            folder: Arc::new(folder),
            busy: Arc::default(),
        }
    }

    fn file(&self, name: &str) -> PathBuf {
        self.folder.join(format!("{name}.jpg"))
    }

    /// Fetches a picture in the background, unless the cache has it or it is on its
    /// way. A picture that cannot be fetched is tried again at the next call.
    pub fn fetch_later(&self, http: &Outbound, name: &str, source: Source) {
        if !is_cache_name(name) || self.file(name).exists() {
            return;
        }
        let is_web_address = ["http://", "https://"]
            .iter()
            .any(|scheme| source.url.starts_with(scheme));
        if !is_web_address {
            return;
        }
        {
            let Ok(mut busy) = self.busy.lock() else {
                return;
            };
            if !busy.insert(name.to_owned()) {
                return;
            }
        }
        let (cache, http, name) = (self.clone(), http.clone(), name.to_owned());
        tokio::spawn(async move {
            if let Err(error) = cache.fetch(&http, &name, &source).await {
                tracing::warn!(%error, name, "a picture for the cache could not be fetched");
            }
            if let Ok(mut busy) = cache.busy.lock() {
                busy.remove(&name);
            }
        });
    }

    async fn fetch(&self, http: &Outbound, name: &str, source: &Source) -> Result<(), String> {
        let mut request = http
            .client(&source.url, source.never_check_certificate)
            .get(&source.url)
            .timeout(TIME_LIMIT);
        if let Some(key) = &source.key {
            request = request.header("X-Api-Key", key);
        }
        // The address can hold an API key, so the error names no address.
        let mut response = request.send().await.map_err(without_address)?;
        if !response.status().is_success() {
            return Err(format!("the source answered {}", response.status()));
        }
        let mut content = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(without_address)? {
            if content.len() + chunk.len() > SIZE_LIMIT {
                return Err("the picture is too large".to_owned());
            }
            content.extend_from_slice(&chunk);
        }
        if picture_type(&content).is_none() {
            return Err("the source did not send a picture".to_owned());
        }
        store(&self.folder, &self.file(name), &content)
            .await
            .map_err(|error| error.to_string())
    }
}

/// Writes the file under another name first, so a request never gets half a picture.
async fn store(folder: &FilePath, file: &FilePath, content: &[u8]) -> std::io::Result<()> {
    tokio::fs::create_dir_all(folder).await?;
    let unfinished = file.with_extension("part");
    tokio::fs::write(&unfinished, content).await?;
    tokio::fs::rename(&unfinished, file).await
}

/// `GET /data/cache/{name}`: a cached picture. Everybody can ask for it, as in the PHP
/// application.
pub async fn cached_picture(
    State(state): State<AppState>,
    Path(name): Path<String>,
    uri: axum::http::Uri,
) -> Result<Response, ApiError> {
    let not_found = || ApiError::NotFound {
        endpoint: uri.path().to_owned(),
        method: "GET".to_owned(),
    };
    let file = name
        .strip_suffix(".jpg")
        .filter(|name| is_cache_name(name))
        .zip(state.picture_cache())
        .map(|(name, cache)| cache.file(name))
        .ok_or_else(not_found)?;
    let content = match tokio::fs::read(&file).await {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(not_found()),
        Err(error) => return Err(ApiError::PictureFolder(error)),
    };
    let content_type = picture_type(&content).ok_or_else(not_found)?;

    let mut response = Response::new(Body::from(content));
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    // The file is shown as a picture, and as nothing else.
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; sandbox"),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::{is_cache_name, picture_type};

    #[test]
    fn cache_name_cannot_leave_the_folder() {
        for name in ["tv-4711", "movie-815", "A1"] {
            assert!(is_cache_name(name), "{name}");
        }
        for name in [
            "",
            "..",
            "a/b",
            "a\\b",
            "a.jpg",
            "tv 1",
            "tv_1",
            &"a".repeat(101),
        ] {
            assert!(!is_cache_name(name), "{name}");
        }
    }

    #[test]
    fn picture_is_known_by_its_first_bytes() {
        assert_eq!(picture_type(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(picture_type(b"\x89PNG\r\n\x1a\n"), Some("image/png"));
        assert_eq!(picture_type(b"GIF89a"), Some("image/gif"));
        assert_eq!(
            picture_type(b"RIFF\x00\x00\x00\x00WEBPVP8 "),
            Some("image/webp")
        );
        assert_eq!(picture_type(b"<svg xmlns"), None);
        assert_eq!(picture_type(b"<!doctype html>"), None);
        assert_eq!(picture_type(b"RIFF\x00\x00\x00\x00WAVE"), None);
        assert_eq!(picture_type(b""), None);
    }
}
