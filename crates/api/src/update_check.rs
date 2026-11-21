//! The version check: the server asks the releases API of GitHub for the newest release
//! of this project and compares its version with its own.
//!
//! PHP Organizr updates itself by a script. This application runs from a container
//! image, so an update means pulling the new image (D-23). The scheduler runs the check
//! at the schedule `autoUpdateCronSchedule` when `autoUpdateCronEnabled` is on, and
//! `GET /update/check` runs it at once. The last result is kept while the server runs.
//! `GET /launch` gives it to Admins and Co-Admins, and the routes `/update/*` send it
//! with their answer `409`.

use organizr_integrations::releases::{self, ReleaseError};
use semver::Version;
use serde::Serialize;

use crate::state::AppState;

/// The version of this application.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// What the last version check found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    /// The version of this application.
    pub current: String,
    /// The version of the newest release, or `None` before a check found one.
    pub latest: Option<String>,
    /// Whether the newest release is newer than this application.
    pub update_available: bool,
    /// The page of the newest release on GitHub.
    pub release_url: Option<String>,
}

impl UpdateStatus {
    /// The status before any check.
    pub fn unchecked() -> Self {
        Self {
            current: CURRENT.to_owned(),
            latest: None,
            update_available: false,
            release_url: None,
        }
    }

    /// The status for the newest release.
    pub fn for_release(release: &releases::LatestRelease) -> Self {
        let update_available =
            Version::parse(CURRENT).is_ok_and(|current| release.version > current);
        Self {
            current: CURRENT.to_owned(),
            latest: Some(release.version.to_string()),
            update_available,
            release_url: Some(releases::release_page(&release.tag)),
        }
    }
}

/// Asks GitHub for the newest release now, and keeps the result. A failed check keeps
/// the result of the last check that worked.
pub async fn check_for_update(state: &AppState) -> Result<UpdateStatus, ReleaseError> {
    let http = state
        .outbound()
        .ok_or_else(|| ReleaseError::Unreachable("no requests can be sent".to_owned()))?;
    match releases::latest(http, state.releases_api()).await {
        Ok(release) => {
            let status = UpdateStatus::for_release(&release);
            if status.update_available {
                tracing::info!(latest = %release.version, current = CURRENT, "a new version of Organizr is out");
            } else {
                tracing::debug!(latest = %release.version, "Organizr is up to date");
            }
            state.set_update_status(status.clone());
            Ok(status)
        }
        Err(error) => {
            tracing::warn!(%error, "the version check failed");
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use organizr_integrations::releases::{LatestRelease, version_of};

    use super::{CURRENT, UpdateStatus};

    fn release(tag: &str) -> LatestRelease {
        LatestRelease {
            version: version_of(tag).expect("the tag is a version"),
            tag: tag.to_owned(),
        }
    }

    #[test]
    fn only_a_newer_release_is_an_update() {
        let newer = UpdateStatus::for_release(&release("v999.0.0"));
        assert!(newer.update_available);
        assert_eq!(newer.latest.as_deref(), Some("999.0.0"));
        assert!(
            newer
                .release_url
                .as_deref()
                .is_some_and(|url| url.ends_with("/releases/tag/v999.0.0"))
        );
        assert!(!UpdateStatus::for_release(&release(CURRENT)).update_available);
        assert!(!UpdateStatus::for_release(&release("0.0.1")).update_available);
        // A pre-release of the current version is older than the version.
        assert!(!UpdateStatus::for_release(&release(&format!("{CURRENT}-rc.1"))).update_available);
    }
}
