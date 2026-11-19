//! What every request handler can reach. It is built once, at start-up.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use organizr_auth::login_limit::LoginLimiter;
use organizr_auth::secrets::{SecretBox, SecretError};
use organizr_auth::setup_code::SetupCode;
use organizr_db::{Database, DbError, StoredSetting};
use organizr_settings::{Settings, SettingsError};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::client::TrustedProxies;
use crate::ping::{Latency, PingTarget, Prober};

/// The shared state of the application.
#[derive(Debug, Clone)]
pub struct AppState {
    database: Database,
    settings: Arc<ArcSwap<Settings>>,
    trusted_proxies: Arc<TrustedProxies>,
    login_limiter: Arc<LoginLimiter>,
    setup_code: Option<Arc<SetupCode>>,
    secrets: Arc<SecretBox>,
    /// The folder of the uploaded pictures. Without it, no picture can be uploaded.
    picture_folder: Option<Arc<PathBuf>>,
    /// What probes the address to check of a tab.
    prober: Prober,
    /// Held while settings are changed, so that two changes do not overwrite each other.
    settings_change: Arc<Mutex<()>>,
}

/// Why a change of settings failed.
#[derive(Debug, Error)]
pub enum ChangeError {
    /// A key is no setting, or a value does not fit its setting.
    #[error(transparent)]
    Settings(#[from] SettingsError),

    /// A secret could not be sealed for storing.
    #[error(transparent)]
    Secret(#[from] SecretError),

    /// The database failed.
    #[error(transparent)]
    Database(#[from] DbError),
}

impl AppState {
    /// Creates the state around an open, migrated database and the settings read from it.
    /// No proxy is trusted until [`AppState::with_trusted_proxies`] names some.
    pub fn new(database: Database, settings: Settings, secrets: SecretBox) -> Self {
        Self {
            database,
            settings: Arc::new(ArcSwap::from_pointee(settings)),
            trusted_proxies: Arc::new(TrustedProxies::default()),
            login_limiter: Arc::new(LoginLimiter::new()),
            setup_code: None,
            secrets: Arc::new(secrets),
            picture_folder: None,
            prober: Prober::network(),
            settings_change: Arc::new(Mutex::new(())),
        }
    }

    /// Sets the reverse proxies whose forwarding headers are believed.
    #[must_use]
    pub fn with_trusted_proxies(mut self, trusted_proxies: TrustedProxies) -> Self {
        self.trusted_proxies = Arc::new(trusted_proxies);
        self
    }

    /// Sets the code that the set-up wizard asks for. Without a code, the wizard takes
    /// no set-up at all.
    /// The same state, with a folder for the pictures that people upload. The folder is
    /// made when the first picture is stored.
    #[must_use]
    pub fn with_picture_folder(mut self, folder: PathBuf) -> Self {
        self.picture_folder = Some(Arc::new(folder));
        self
    }

    #[must_use]
    pub fn with_setup_code(mut self, setup_code: SetupCode) -> Self {
        self.setup_code = Some(Arc::new(setup_code));
        self
    }

    /// The application database.
    pub const fn database(&self) -> &Database {
        &self.database
    }

    /// The settings as they are now.
    ///
    /// The settings can change while the application runs. A handler asks once and uses
    /// what it got, so one request works with one set of settings from start to end.
    pub fn settings(&self) -> Arc<Settings> {
        self.settings.load_full()
    }

    /// Puts new settings in place of the current ones. Requests that start after this
    /// call use them. A request that is under way keeps the settings it started with.
    pub fn replace_settings(&self, settings: Settings) {
        self.settings.store(Arc::new(settings));
    }

    /// Reads the settings from the database again and puts them in place of the current
    /// ones. Returns the stored entries that cannot be used; each of those settings has
    /// its default.
    pub async fn reload_settings(&self) -> Result<Vec<SettingsError>, DbError> {
        let (settings, problems) = stored_settings(&self.database, &self.secrets).await?;
        self.replace_settings(settings);
        Ok(problems)
    }

    /// The reverse proxies whose forwarding headers are believed.
    pub fn trusted_proxies(&self) -> &TrustedProxies {
        &self.trusted_proxies
    }

    /// The code that the set-up wizard asks for, when there is one.
    pub fn setup_code(&self) -> Option<&SetupCode> {
        self.setup_code.as_deref()
    }

    /// The count of failed logins. It starts empty when the application starts.
    pub fn login_limiter(&self) -> &LoginLimiter {
        &self.login_limiter
    }

    /// Changes settings: each key gets the value in its stored form. The new values are
    /// stored in the database, and requests from now on use them.
    ///
    /// All settings are changed, or none is. A setting that gets its default has no row
    /// in the database afterwards. A secret is sealed before it is stored.
    pub async fn change_settings(&self, changes: Vec<(String, Value)>) -> Result<(), ChangeError> {
        let _turn = self.settings_change.lock().await;
        // The settings are large, so the copies do not stand in this future itself.
        let mut settings = Box::new((*self.settings()).clone());
        let defaults = Box::new(Settings::defaults());
        let mut puts = Vec::new();
        let mut removes = Vec::new();
        for (key, value) in changes {
            settings.set(&key, &value)?;
            // The stored form as the registry writes it, for example a number as text.
            let stored = settings.get(&key).unwrap_or(value);
            if defaults.get(&key).as_ref() == Some(&stored) {
                removes.push(key);
            } else {
                let value = self.stored_form(&key, &stored)?;
                puts.push(StoredSetting { key, value });
            }
        }
        self.database.settings().apply(&puts, &removes).await?;
        self.replace_settings(*settings);
        Ok(())
    }

    /// The text to store for a setting: JSON, with a secret sealed.
    fn stored_form(&self, key: &str, value: &Value) -> Result<String, SecretError> {
        match value {
            Value::String(secret) if Settings::is_secret(key) => {
                Ok(Value::String(self.secrets.seal(key, secret)?).to_string())
            }
            other => Ok(other.to_string()),
        }
    }

    /// The same state, with another way to probe the address to check of a tab. A test
    /// uses it to stand in for the network.
    #[must_use]
    pub fn with_prober(mut self, prober: Prober) -> Self {
        self.prober = prober;
        self
    }

    /// Probes a target once.
    pub async fn probe(&self, target: PingTarget) -> Latency {
        self.prober.probe(target).await
    }

    /// The folder of the uploaded pictures, when the application has one.
    pub fn picture_folder(&self) -> Option<&Path> {
        self.picture_folder.as_deref().map(PathBuf::as_path)
    }

    /// What seals the secrets that the install stores, and opens them again.
    pub fn secrets(&self) -> &SecretBox {
        &self.secrets
    }
}

/// Reads the settings from the database: the defaults, with each stored value on top.
/// Stored secrets are opened with `secrets`.
///
/// A stored entry that cannot be used is left out and reported, and its setting keeps
/// the default. That includes a secret that the key of the install does not open, for
/// example after the key file was lost.
pub async fn stored_settings(
    database: &Database,
    secrets: &SecretBox,
) -> Result<(Settings, Vec<SettingsError>), DbError> {
    let mut problems = Vec::new();
    let mut readable = Vec::new();
    for setting in database.settings().all().await? {
        if !Settings::is_secret(&setting.key) {
            readable.push(setting);
        } else if let Some(value) = opened(secrets, &setting) {
            readable.push(StoredSetting {
                key: setting.key,
                value,
            });
        } else {
            problems.push(SettingsError::UnreadableSecret { key: setting.key });
        }
    }
    let (settings, more) = Settings::from_stored(
        readable
            .iter()
            .map(|setting| (setting.key.as_str(), setting.value.as_str())),
    );
    problems.extend(more);
    Ok((settings, problems))
}

/// The stored form of a secret with the secret in plain text, or `None` when the secret
/// cannot be opened. A stored value that is no JSON text comes back as it is, and the
/// settings registry reports it.
fn opened(secrets: &SecretBox, setting: &StoredSetting) -> Option<String> {
    let Ok(Value::String(sealed)) = serde_json::from_str::<Value>(&setting.value) else {
        return Some(setting.value.clone());
    };
    let secret = secrets.open(&setting.key, &sealed).ok()?;
    Some(Value::String(secret).to_string())
}
