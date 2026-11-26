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

/// How long a picture of a Plex server counts as fresh.
const KEPT_FOR: Duration = Duration::from_hours(7 * 24);

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

/// The width of a JPEG or PNG picture, read from its header.
pub fn picture_width(content: &[u8]) -> Option<u32> {
    let pair = |at: usize| -> Option<u16> {
        let bytes = content.get(at..at + 2)?;
        Some(u16::from_be_bytes([*bytes.first()?, *bytes.get(1)?]))
    };
    if content.starts_with(&[0x89, b'P', b'N', b'G']) {
        if content.get(12..16) != Some(b"IHDR".as_slice()) {
            return None;
        }
        let bytes: [u8; 4] = content.get(16..20)?.try_into().ok()?;
        return Some(u32::from_be_bytes(bytes));
    }
    if !content.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    // The segments of a JPEG file: a marker 0xFF and its kind, then a length. The width
    // is in the segment that starts the frame.
    let mut at = 2;
    loop {
        if *content.get(at)? != 0xFF {
            return None;
        }
        let kind = *content.get(at + 1)?;
        match kind {
            0xFF => at += 1,
            0x01 | 0xD0..=0xD7 => at += 2,
            0xC0..=0xCF if !matches!(kind, 0xC4 | 0xC8 | 0xCC) => {
                return pair(at + 7).map(u32::from);
            }
            _ => at += 2 + usize::from(pair(at + 2)?),
        }
    }
}

/// How many pictures [`PictureCache::random_wide_picture`] reads at most.
const RANDOM_TRIES: usize = 20;

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

    /// Whether the cache holds the picture, and got it less than a week ago.
    pub fn is_fresh(&self, name: &str) -> bool {
        is_cache_name(name)
            && std::fs::metadata(self.file(name))
                .and_then(|file| file.modified())
                .ok()
                .and_then(|modified| modified.elapsed().ok())
                .is_some_and(|age| age < KEPT_FOR)
    }

    /// The bytes of a picture, when the cache holds it.
    pub async fn read(&self, name: &str) -> Option<Vec<u8>> {
        if !is_cache_name(name) {
            return None;
        }
        tokio::fs::read(self.file(name)).await.ok()
    }

    /// The address of a picture of something that plays, chosen at random, that is at
    /// least `least_width` pixels wide: `data/cache/<name>.jpg`. As `getRandomMediaImage`
    /// of PHP, for the login page. The pictures of something that plays have names that
    /// end with `-np`.
    pub async fn random_wide_picture(&self, least_width: u32) -> Option<String> {
        let mut names = Vec::new();
        let mut entries = tokio::fs::read_dir(&*self.folder).await.ok()?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let file_name = entry.file_name();
            if let Some(name) = file_name
                .to_str()
                .and_then(|name| name.strip_suffix(".jpg"))
                && is_cache_name(name)
                && name.ends_with("-np")
            {
                names.push(name.to_owned());
            }
        }
        for _ in 0..RANDOM_TRIES {
            let count = u64::try_from(names.len()).ok().filter(|count| *count > 0)?;
            let index = usize::try_from(getrandom::u64().ok()? % count).ok()?;
            let name = names.swap_remove(index);
            if let Some(content) = self.read(&name).await
                && picture_width(&content).is_some_and(|width| width >= least_width)
            {
                return Some(format!("data/cache/{name}.jpg"));
            }
        }
        None
    }

    /// Puts a picture into the cache, in the place of the one with the same name.
    pub async fn keep(&self, name: &str, content: &[u8]) -> std::io::Result<()> {
        if !is_cache_name(name) {
            return Err(std::io::Error::other("the name is no name of the cache"));
        }
        store(&self.folder, &self.file(name), content).await
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
        // A redirect must not take the key to another place.
        let client = if source.key.is_some() {
            http.keyed_client(&source.url, source.never_check_certificate)
        } else {
            http.client(&source.url, source.never_check_certificate)
        };
        let mut request = client.get(&source.url).timeout(TIME_LIMIT);
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
    picture_response(content).ok_or_else(not_found)
}

/// The answer that sends the bytes of a picture, or nothing when the bytes are no
/// picture of a kind that a browser shows.
pub fn picture_response(content: Vec<u8>) -> Option<Response> {
    let content_type = picture_type(&content)?;
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
    Some(response)
}

#[cfg(test)]
mod tests {
    use super::{is_cache_name, picture_type, picture_width};

    #[test]
    fn width_is_read_from_the_header_of_a_jpeg_or_png() {
        // A JPEG with a segment before the frame, which is skipped.
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00];
        jpeg.extend_from_slice(&[0xFF, 0xC2, 0x00, 0x11, 0x08, 0x02, 0xA3, 0x04, 0xB0]);
        assert_eq!(picture_width(&jpeg), Some(1200));
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&640_u32.to_be_bytes());
        png.extend_from_slice(&480_u32.to_be_bytes());
        assert_eq!(picture_width(&png), Some(640));
        for broken in [
            b"".as_slice(),
            b"GIF89a".as_slice(),
            &[0xFF, 0xD8, 0xFF, 0xE0, 0x00],
            &[0xFF, 0xD8, 0x00, 0x00],
        ] {
            assert_eq!(picture_width(broken), None, "{broken:?}");
        }
    }

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
