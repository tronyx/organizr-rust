//! The newest release of this application, from the releases API of GitHub for the
//! repository of the project. The version check of the server asks it.
//!
//! The request is small: it sends no token, its `User-Agent` names only the
//! application, it has a time limit, and an answer larger than [`LARGEST_ANSWER`] is
//! not read.

use std::time::Duration;

use semver::Version;
use serde_json::Value;
use thiserror::Error;

use crate::http::{Outbound, escaped};

/// How long GitHub has to answer.
pub const TIME_LIMIT: Duration = Duration::from_secs(10);

/// The largest answer that is read, in bytes. The answer for one release is a few
/// kilobytes.
pub const LARGEST_ANSWER: usize = 1024 * 1024;

/// The `User-Agent` of the request: the name of the application, nothing more.
pub const USER_AGENT: &str = "organizr";

/// The address of the repository of this project, from its manifest.
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// The address of the newest release in the releases API of GitHub. GitHub leaves out
/// drafts and pre-releases there.
pub fn latest_release_api() -> String {
    let path = REPOSITORY
        .trim_end_matches('/')
        .trim_start_matches("https://github.com/");
    format!("https://api.github.com/repos/{path}/releases/latest")
}

/// The page of a release on GitHub.
pub fn release_page(tag: &str) -> String {
    format!(
        "{}/releases/tag/{}",
        REPOSITORY.trim_end_matches('/'),
        escaped(tag)
    )
}

/// The newest release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatestRelease {
    /// The version of the release.
    pub version: Version,
    /// The name of its tag, for example `v1.2.0`.
    pub tag: String,
}

/// Why the newest release is not known. The texts name no address.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReleaseError {
    /// GitHub could not be reached, or did not answer in time.
    #[error("GitHub could not be reached: {0}")]
    Unreachable(String),
    /// GitHub answered with a status other than 200.
    #[error("GitHub answered with the status {0}")]
    Refused(u16),
    /// The answer is larger than [`LARGEST_ANSWER`].
    #[error("the answer of GitHub is too large")]
    TooLarge,
    /// The answer is no JSON, or has no tag that is a version.
    #[error("the answer of GitHub names no version")]
    NoVersion,
}

/// Asks the releases API at `url` for the newest release.
pub async fn latest(http: &Outbound, url: &str) -> Result<LatestRelease, ReleaseError> {
    let unreachable =
        |error: reqwest::Error| ReleaseError::Unreachable(error.without_url().to_string());
    let mut response = http
        .client_for(url)
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(unreachable)?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(ReleaseError::Refused(response.status().as_u16()));
    }
    if response
        .content_length()
        .is_some_and(|length| length > LARGEST_ANSWER as u64)
    {
        return Err(ReleaseError::TooLarge);
    }
    let mut body = Vec::new();
    while let Some(piece) = response.chunk().await.map_err(unreachable)? {
        if body.len() + piece.len() > LARGEST_ANSWER {
            return Err(ReleaseError::TooLarge);
        }
        body.extend_from_slice(&piece);
    }
    let answer: Value = serde_json::from_slice(&body).map_err(|_| ReleaseError::NoVersion)?;
    let tag = answer
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or(ReleaseError::NoVersion)?;
    let version = version_of(tag).ok_or(ReleaseError::NoVersion)?;
    Ok(LatestRelease {
        version,
        tag: tag.to_owned(),
    })
}

/// The version that a tag names: `v1.2.0` and `1.2.0` name 1.2.0.
pub fn version_of(tag: &str) -> Option<Version> {
    let text = tag.trim();
    let text = text
        .strip_prefix('v')
        .or_else(|| text.strip_prefix('V'))
        .unwrap_or(text);
    Version::parse(text).ok()
}

#[cfg(test)]
mod tests {
    use super::{latest_release_api, release_page, version_of};

    #[test]
    fn addresses_come_from_the_repository_of_the_project() {
        let api = latest_release_api();
        assert!(api.starts_with("https://api.github.com/repos/"), "{api}");
        assert!(api.ends_with("/releases/latest"), "{api}");
        assert!(!api.contains("causefx"), "{api}");
        let page = release_page("v1.2.0");
        assert!(page.starts_with("https://github.com/"), "{page}");
        assert!(page.ends_with("/releases/tag/v1.2.0"), "{page}");
        assert!(release_page("a/../b").ends_with("/releases/tag/a%2F..%2Fb"));
    }

    #[test]
    fn tag_names_a_version_with_or_without_a_v() {
        assert_eq!(
            version_of("v1.2.0").map(|v| v.to_string()).as_deref(),
            Some("1.2.0")
        );
        assert_eq!(
            version_of("0.3.1").map(|v| v.to_string()).as_deref(),
            Some("0.3.1")
        );
        assert_eq!(
            version_of("v2.0.0-rc.1").map(|v| v.to_string()).as_deref(),
            Some("2.0.0-rc.1")
        );
        for wrong in ["", "latest", "v1.2", "2.1.5000.1"] {
            assert_eq!(version_of(wrong), None, "{wrong}");
        }
    }
}
