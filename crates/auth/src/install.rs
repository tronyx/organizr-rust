//! Setting up a new install: the first Admin account and the settings of the install.

use md5::{Digest, Md5};
use organizr_db::{Database, DbError, NewInstall, StoredSetting};
use organizr_domain::{Timestamp, User};
use thiserror::Error;
use uuid::Uuid;

use crate::password::{self, HashError};
use crate::secrets::{SecretBox, SecretError};

/// What the person who sets an install up enters.
#[derive(Clone, PartialEq, Eq)]
pub struct SetUp {
    /// The name of the first account, which is an Admin.
    pub username: String,
    /// The password of that account.
    pub password: String,
    /// The e-mail address of that account.
    pub email: String,
    /// The API key of the install.
    pub api_key: String,
    /// The licence the install is used under: `personal` or `business`.
    pub license: String,
    /// The password that a new user must know to register an account.
    pub registration_password: String,
}

/// The passwords and the API key are secrets, so `Debug` shows the name only.
impl std::fmt::Debug for SetUp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SetUp")
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

/// Why a set-up failed.
#[derive(Debug, Error)]
pub enum InstallError {
    /// The install has been set up already.
    #[error("the install has been set up already")]
    AlreadySetUp,

    /// The database failed.
    #[error(transparent)]
    Database(DbError),

    /// A secret could not be sealed for storing.
    #[error(transparent)]
    Secret(#[from] SecretError),

    /// A password could not be hashed.
    #[error(transparent)]
    Hash(#[from] HashError),

    /// The task that hashes the passwords did not finish.
    #[error("the password task did not finish: {0}")]
    Task(#[source] tokio::task::JoinError),
}

impl From<DbError> for InstallError {
    fn from(error: DbError) -> Self {
        match error {
            DbError::AlreadySetUp => Self::AlreadySetUp,
            other => Self::Database(other),
        }
    }
}

/// The address of the Gravatar picture for an e-mail address, as the PHP application
/// makes it. Gravatar finds the picture by the MD5 hash of the address in lower case.
pub fn gravatar(email: &str) -> String {
    let hash = hex::encode(Md5::digest(email.trim().to_lowercase().as_bytes()));
    format!("https://www.gravatar.com/avatar/{hash}?s=100&d=mm")
}

/// A value of a setting in its stored form: JSON text.
fn stored(key: &str, value: &str) -> StoredSetting {
    StoredSetting {
        key: key.to_owned(),
        value: serde_json::Value::from(value).to_string(),
    }
}

/// Sets up a new install and returns its first Admin account.
///
/// The install gets a new random id. The password of the account and the registration
/// password are stored as hashes, and the API key is sealed with `secrets`. Everything
/// is stored, or nothing is. Fails with
/// [`InstallError::AlreadySetUp`] when the install has a user.
pub async fn set_up(
    database: &Database,
    request: SetUp,
    secrets: &SecretBox,
    now: Timestamp,
) -> Result<User, InstallError> {
    // Hashing takes a while. Do not start it for an install that is set up.
    if database.install().is_done().await? {
        return Err(InstallError::AlreadySetUp);
    }

    let SetUp {
        username,
        password,
        email,
        api_key,
        license,
        registration_password,
    } = request;
    let (password_hash, registration_hash) = tokio::task::spawn_blocking(move || {
        Ok::<_, HashError>((
            password::hash(&password)?,
            password::hash(&registration_password)?,
        ))
    })
    .await
    .map_err(InstallError::Task)??;

    let install_id = Uuid::new_v4().to_string();
    let admin = database
        .install()
        .run(NewInstall {
            admin_image: Some(gravatar(&email)),
            admin_username: username,
            admin_email: email,
            admin_password_hash: password_hash,
            created_at: now,
            settings: vec![
                stored("uuid", &install_id),
                stored("organizrAPI", &secrets.seal("organizrAPI", &api_key)?),
                stored("license", &license),
                // A new install starts on the modern layout. An install that is updated or
                // imported from PHP has no stored value and keeps the default, OG (D-125).
                stored("layout", "modern"),
                stored(
                    "registrationPassword",
                    &secrets.seal("registrationPassword", registration_hash.expose())?,
                ),
            ],
        })
        .await?;
    Ok(admin)
}

#[cfg(test)]
mod tests {
    use super::{SetUp, gravatar};

    #[test]
    fn gravatar_address_is_the_one_php_makes() {
        // The address the set-up wizard of the PHP application stored for this account.
        let expected =
            "https://www.gravatar.com/avatar/e64c7d89f26bd1972efa854d13d7dd61?s=100&d=mm";

        assert_eq!(gravatar("admin@example.com"), expected);
        assert_eq!(gravatar("  Admin@Example.COM "), expected);
    }

    #[test]
    fn debug_output_shows_no_secret() {
        let request = SetUp {
            username: "robin".to_owned(),
            password: "the-password".to_owned(),
            email: "robin@example.com".to_owned(),
            api_key: "the-api-key".to_owned(),
            license: "personal".to_owned(),
            registration_password: "the-registration-password".to_owned(),
        };

        let shown = format!("{request:?}");

        assert!(shown.contains("robin"), "{shown}");
        for secret in ["the-password", "the-api-key", "the-registration-password"] {
            assert!(!shown.contains(secret), "{shown}");
        }
    }
}
