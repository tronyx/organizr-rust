//! What every request handler can reach. It is built once, at start-up.

use std::sync::Arc;

use arc_swap::ArcSwap;
use organizr_auth::login_limit::LoginLimiter;
use organizr_auth::secrets::SecretBox;
use organizr_auth::setup_code::SetupCode;
use organizr_db::{Database, DbError, StoredSetting};
use organizr_settings::{Settings, SettingsError};
use serde_json::Value;

use crate::client::TrustedProxies;

/// The shared state of the application.
#[derive(Debug, Clone)]
pub struct AppState {
    database: Database,
    settings: Arc<ArcSwap<Settings>>,
    trusted_proxies: Arc<TrustedProxies>,
    login_limiter: Arc<LoginLimiter>,
    setup_code: Option<Arc<SetupCode>>,
    secrets: Arc<SecretBox>,
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
