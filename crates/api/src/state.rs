//! What every request handler can reach. It is built once, at start-up.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use arc_swap::ArcSwap;
use organizr_auth::login_limit::LoginLimiter;
use organizr_auth::secrets::{SecretBox, SecretError};
use organizr_auth::setup_code::SetupCode;
use organizr_db::{Database, DbError, StoredSetting};
use organizr_integrations::custom_certificate::{self, CustomCertificate};
use organizr_integrations::http::Outbound;
use organizr_integrations::ipinfo::IPINFO;
use organizr_integrations::media_login::EMBY_CONNECT;
use organizr_integrations::organizr_app::ORGANIZR_API;
use organizr_integrations::plex::{PLEX_TV, PlexTv};
use organizr_integrations::releases::latest_release_api;
use organizr_integrations::sponsors::{GITHUB_SPONSORS, OPEN_COLLECTIVE};
use organizr_integrations::stripe::STRIPE_API;
use organizr_integrations::trakt::{TRAKT_API, TRAKT_SITE};
use organizr_integrations::youtube::YOUTUBE_API;
use organizr_plugins::mail::Mailer;
use organizr_settings::{Settings, SettingsError};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::allowed_hosts::AllowedHosts;
use crate::client::TrustedProxies;
use crate::handlers::chat::ChatHub;
use crate::picture_cache::PictureCache;
use crate::ping::{Latency, PingTarget, Prober};
use crate::update_check::UpdateStatus;
use organizr_integrations::weather::BREEZOMETER;

/// The shared state of the application.
#[derive(Debug, Clone)]
pub struct AppState {
    database: Database,
    settings: Arc<ArcSwap<Settings>>,
    trusted_proxies: Arc<TrustedProxies>,
    /// The hosts that a path of a service address may be completed with (D-154).
    allowed_hosts: Arc<AllowedHosts>,
    login_limiter: Arc<LoginLimiter>,
    setup_code: Option<Arc<SetupCode>>,
    secrets: Arc<SecretBox>,
    /// The folder of the uploaded pictures. Without it, no picture can be uploaded.
    picture_folder: Option<Arc<PathBuf>>,
    /// The pictures of other applications that the server keeps. Without it, no such
    /// picture is fetched.
    picture_cache: Option<PictureCache>,
    /// The data folder, for the backups.
    data_folder: Option<Arc<PathBuf>>,
    /// The file of the master key, which a backup leaves out (D-147).
    key_file: Option<Arc<PathBuf>>,
    /// What lets one backup be made at a time.
    backup_lock: Arc<Mutex<()>>,
    /// The runtime for extensions. `None` when it could not start.
    #[cfg(feature = "wasm-plugins")]
    extension_runtime: Option<organizr_plugins::extension::Extensions>,
    /// The address of the list of the marketplace for extensions, when one is set.
    #[cfg(feature = "wasm-plugins")]
    extension_marketplace: Option<Arc<str>>,
    /// What lets one call of an extension run at a time, by the id of the extension.
    #[cfg(feature = "wasm-plugins")]
    extension_locks: Arc<std::sync::Mutex<std::collections::HashMap<String, Arc<Mutex<()>>>>>,
    /// What probes the address to check of a tab.
    prober: Prober,
    /// What sends the e-mails.
    mailer: Mailer,
    /// The client for requests to other services. `None` when it could not be built.
    outbound: Option<Outbound>,
    /// The address of the account service of Plex.
    plex_tv: Arc<str>,
    /// The address of the API of Emby Connect.
    emby_connect: Arc<str>,
    /// The address of the weather API.
    breezometer: Arc<str>,
    /// The addresses of the API and of the pages of Trakt.
    trakt: Arc<(String, String)>,
    /// The address of the API of Stripe.
    stripe: Arc<str>,
    /// The address of the API of YouTube.
    youtube: Arc<str>,
    /// The addresses of the list of members at Open Collective and of the sponsor page
    /// at GitHub.
    sponsors: Arc<(String, String)>,
    /// The address of ipinfo.io.
    ipinfo: Arc<str>,
    /// The address of the API of the PHP project Organizr.
    organizr_api: Arc<str>,
    /// Whether `POST /homepage/scrape` also fetches a page of this computer or of a
    /// private network. Only a test says yes.
    scrapes_local_pages: bool,
    /// The second of the last time that the scheduler of the server woke, or 0.
    scheduler_beat: Arc<AtomicI64>,
    /// Held while settings are changed, so that two changes do not overwrite each other.
    settings_change: Arc<Mutex<()>>,
    /// Whether the page runs the scripts of the Admin (`customJava`): the switch
    /// `ORGANIZR_CUSTOM_JS` of the operator. Off unless the operator turns it on.
    custom_js: bool,
    /// The new messages of the chat plugin, and how many each sender sent lately.
    chat_hub: Arc<ChatHub>,
    /// The address of the newest release in the releases API of GitHub.
    releases_api: Arc<str>,
    /// What the last version check found.
    update_status: Arc<ArcSwap<UpdateStatus>>,
}

/// The settings of a running application, as they are at each moment. Reading them is
/// cheap and never waits for a change.
#[derive(Debug, Clone)]
pub struct SettingsWatch(Arc<ArcSwap<Settings>>);

impl SettingsWatch {
    /// Runs `read` with the settings as they are now.
    pub fn read<R>(&self, read: impl FnOnce(&Settings) -> R) -> R {
        read(&self.0.load())
    }
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
            allowed_hosts: Arc::new(AllowedHosts::default()),
            login_limiter: Arc::new(LoginLimiter::new()),
            setup_code: None,
            secrets: Arc::new(secrets),
            picture_folder: None,
            picture_cache: None,
            data_folder: None,
            key_file: None,
            backup_lock: Arc::new(Mutex::new(())),
            #[cfg(feature = "wasm-plugins")]
            extension_runtime: organizr_plugins::extension::Extensions::new()
                .inspect_err(|error| {
                    tracing::error!(%error, "the runtime for extensions could not start");
                })
                .ok(),
            #[cfg(feature = "wasm-plugins")]
            extension_marketplace: None,
            #[cfg(feature = "wasm-plugins")]
            extension_locks: Arc::default(),
            prober: Prober::network(),
            mailer: Mailer::smtp(),
            outbound: Outbound::new()
                .inspect_err(|error| {
                    tracing::error!(%error, "the client for other services could not be built");
                })
                .ok(),
            plex_tv: Arc::from(PLEX_TV),
            emby_connect: Arc::from(EMBY_CONNECT),
            breezometer: Arc::from(BREEZOMETER),
            trakt: Arc::new((TRAKT_API.to_owned(), TRAKT_SITE.to_owned())),
            stripe: Arc::from(STRIPE_API),
            youtube: Arc::from(YOUTUBE_API),
            sponsors: Arc::new((OPEN_COLLECTIVE.to_owned(), GITHUB_SPONSORS.to_owned())),
            ipinfo: Arc::from(IPINFO),
            organizr_api: Arc::from(ORGANIZR_API),
            scrapes_local_pages: false,
            scheduler_beat: Arc::default(),
            settings_change: Arc::new(Mutex::new(())),
            custom_js: false,
            chat_hub: Arc::default(),
            releases_api: Arc::from(latest_release_api()),
            update_status: Arc::new(ArcSwap::from_pointee(UpdateStatus::unchecked())),
        }
    }

    /// Sets the hosts of `ORGANIZR_ALLOWED_HOSTS` (D-154).
    #[must_use]
    pub fn with_allowed_hosts(mut self, allowed_hosts: AllowedHosts) -> Self {
        self.allowed_hosts = Arc::new(allowed_hosts);
        self
    }

    /// The hosts of `ORGANIZR_ALLOWED_HOSTS`, which a path of a service address may be
    /// completed with (D-154).
    pub fn allowed_hosts(&self) -> &AllowedHosts {
        &self.allowed_hosts
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

    /// The same state, with a folder for the pictures of other applications that the
    /// server fetches and keeps.
    #[must_use]
    pub fn with_picture_cache(mut self, folder: PathBuf) -> Self {
        self.picture_cache = Some(PictureCache::new(folder));
        self
    }

    /// The same state, with the switch for the scripts of the Admin on or off. With it
    /// on, every page runs `customThemeJava` and `customJava` (D-93).
    #[must_use]
    pub const fn with_custom_js(mut self, enabled: bool) -> Self {
        self.custom_js = enabled;
        self
    }

    /// What the routes of the chat plugin share.
    pub fn chat_hub(&self) -> &Arc<ChatHub> {
        &self.chat_hub
    }

    /// Whether the page runs the scripts of the Admin.
    pub const fn custom_js(&self) -> bool {
        self.custom_js
    }

    /// The same state, with the data folder of the install. A backup holds its files.
    /// The certificate of an authority that an Admin uploaded is read from it, and the
    /// requests that ask for it trust that authority.
    #[must_use]
    pub fn with_data_folder(mut self, folder: PathBuf) -> Self {
        let file = folder.join(custom_certificate::FILE);
        self.data_folder = Some(Arc::new(folder));
        if let (Ok(text), Some(outbound)) = (std::fs::read(&file), &self.outbound) {
            let trusted = CustomCertificate::parse(&text)
                .map_err(|error| error.to_string())
                .and_then(|certificate| {
                    outbound
                        .set_custom_certificate(Some(&certificate))
                        .map_err(|error| error.without_url().to_string())
                });
            if let Err(error) = trusted {
                tracing::warn!(%error, "the uploaded certificate of an authority is not used");
            }
        }
        self
    }

    /// The file of the certificate of an authority that an Admin uploads, when the
    /// application has a data folder.
    pub fn custom_certificate_file(&self) -> Option<PathBuf> {
        self.data_folder()
            .map(|folder| folder.join(custom_certificate::FILE))
    }

    /// The same state, with another address for the newest release in the releases
    /// API of GitHub. Only a test changes it.
    #[must_use]
    pub fn with_releases_api(mut self, address: &str) -> Self {
        self.releases_api = Arc::from(address);
        self
    }

    /// The address of the newest release in the releases API of GitHub.
    pub fn releases_api(&self) -> &str {
        &self.releases_api
    }

    /// What the last version check found.
    pub fn update_status(&self) -> Arc<UpdateStatus> {
        self.update_status.load_full()
    }

    /// Keeps what a version check found.
    pub fn set_update_status(&self, status: UpdateStatus) {
        self.update_status.store(Arc::new(status));
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

    /// A view of the settings that follows each change, for a part of the server that
    /// lives outside the requests: the writer of the log file.
    pub fn settings_watch(&self) -> SettingsWatch {
        SettingsWatch(Arc::clone(&self.settings))
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
    /// The data folder of the install, when the server named one.
    pub fn data_folder(&self) -> Option<&Path> {
        self.data_folder.as_deref().map(PathBuf::as_path)
    }

    /// The same state, with the file of the master key that the server uses. A backup
    /// leaves that file out, wherever it is (D-147).
    #[must_use]
    pub fn with_key_file(mut self, file: PathBuf) -> Self {
        self.key_file = Some(Arc::new(file));
        self
    }

    /// The file of the master key, when the server reads the key from a file.
    pub fn key_file(&self) -> Option<PathBuf> {
        self.key_file.as_deref().cloned()
    }

    /// The same state, with the address of the list of a marketplace for extensions.
    /// An empty address means no marketplace.
    #[cfg(feature = "wasm-plugins")]
    #[must_use]
    pub fn with_extension_marketplace(mut self, address: &str) -> Self {
        self.extension_marketplace =
            (!address.trim().is_empty()).then(|| Arc::from(address.trim()));
        self
    }

    /// The address of the list of the marketplace for extensions, when one is set.
    #[cfg(feature = "wasm-plugins")]
    pub fn extension_marketplace(&self) -> Option<&str> {
        self.extension_marketplace.as_deref()
    }

    /// The runtime for extensions, when it started.
    #[cfg(feature = "wasm-plugins")]
    pub fn extension_runtime(&self) -> Option<&organizr_plugins::extension::Extensions> {
        self.extension_runtime.as_ref()
    }

    /// The lock that a call of the extension with the id holds.
    #[cfg(feature = "wasm-plugins")]
    pub fn extension_lock(&self, id: &str) -> Arc<Mutex<()>> {
        let mut locks = self
            .extension_locks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        locks.entry(id.to_owned()).or_default().clone()
    }

    /// The lock that a backup holds while it is made.
    pub fn backup_lock(&self) -> &Mutex<()> {
        &self.backup_lock
    }

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

    /// The same state, with another way to send e-mails. A test uses it to keep the
    /// e-mails.
    #[must_use]
    pub fn with_mailer(mut self, mailer: Mailer) -> Self {
        self.mailer = mailer;
        self
    }

    /// The same state, with another address for the account service of Plex. A test
    /// uses it to put a stand-in in the place of `plex.tv`.
    #[must_use]
    pub fn with_plex_tv(mut self, address: &str) -> Self {
        self.plex_tv = Arc::from(address);
        self
    }

    /// The same state, with another address for the API of Emby Connect. A test puts a
    /// stand-in there.
    #[must_use]
    pub fn with_emby_connect(mut self, address: &str) -> Self {
        self.emby_connect = Arc::from(address);
        self
    }

    /// The address of the API of Emby Connect.
    pub fn emby_connect(&self) -> &str {
        &self.emby_connect
    }

    /// The same state, with another address for the weather API. A test puts a
    /// stand-in there.
    #[must_use]
    pub fn with_breezometer(mut self, address: &str) -> Self {
        self.breezometer = Arc::from(address);
        self
    }

    /// The address of the weather API.
    pub fn breezometer(&self) -> &str {
        &self.breezometer
    }

    /// The same state, with other addresses for the API and for the pages of Trakt. A
    /// test puts a stand-in there.
    #[must_use]
    pub fn with_trakt(mut self, api: &str, site: &str) -> Self {
        self.trakt = Arc::new((api.to_owned(), site.to_owned()));
        self
    }

    /// The address of the API of Trakt.
    pub fn trakt_api(&self) -> &str {
        &self.trakt.0
    }

    /// The address of the pages of Trakt.
    pub fn trakt_site(&self) -> &str {
        &self.trakt.1
    }

    /// The same state, with another address for the API of Stripe. A test puts a
    /// stand-in there.
    #[must_use]
    pub fn with_stripe(mut self, address: &str) -> Self {
        self.stripe = Arc::from(address);
        self
    }

    /// The address of the API of Stripe.
    pub fn stripe(&self) -> &str {
        &self.stripe
    }

    /// The same state, with another address for the API of YouTube. A test puts a
    /// stand-in there.
    #[must_use]
    pub fn with_youtube(mut self, address: &str) -> Self {
        self.youtube = Arc::from(address);
        self
    }

    /// The address of the API of YouTube.
    pub fn youtube(&self) -> &str {
        &self.youtube
    }

    /// The same state, with other addresses for the list of members at Open Collective
    /// and for the sponsor page at GitHub. A test puts a stand-in there.
    #[must_use]
    pub fn with_sponsors(mut self, open_collective: &str, github: &str) -> Self {
        self.sponsors = Arc::new((open_collective.to_owned(), github.to_owned()));
        self
    }

    /// The address of the list of members at Open Collective.
    pub fn open_collective(&self) -> &str {
        &self.sponsors.0
    }

    /// The address of the sponsor page at GitHub.
    pub fn github_sponsors(&self) -> &str {
        &self.sponsors.1
    }

    /// The same state, with another address for ipinfo.io. A test puts a stand-in
    /// there.
    #[must_use]
    pub fn with_ipinfo(mut self, address: &str) -> Self {
        self.ipinfo = Arc::from(address);
        self
    }

    /// The address of ipinfo.io.
    pub fn ipinfo(&self) -> &str {
        &self.ipinfo
    }

    /// The same state, with another address for the API of the PHP project Organizr.
    /// A test puts a stand-in there.
    #[must_use]
    pub fn with_organizr_api(mut self, address: &str) -> Self {
        self.organizr_api = Arc::from(address);
        self
    }

    /// The address of the API of the PHP project Organizr.
    pub fn organizr_api(&self) -> &str {
        &self.organizr_api
    }

    /// The same state, but `POST /homepage/scrape` also fetches a page of this
    /// computer. A test uses it to fetch a page of a stand-in.
    #[must_use]
    pub const fn with_local_pages_scraped(mut self) -> Self {
        self.scrapes_local_pages = true;
        self
    }

    /// Whether `POST /homepage/scrape` also fetches a page of this computer or of a
    /// private network.
    pub const fn scrapes_local_pages(&self) -> bool {
        self.scrapes_local_pages
    }

    /// Notes that the scheduler of the server woke at the given second.
    pub fn scheduler_woke(&self, second: i64) {
        self.scheduler_beat.store(second, Ordering::Relaxed);
    }

    /// The second of the last time that the scheduler of the server woke, or 0 when
    /// it never did.
    pub fn scheduler_beat(&self) -> i64 {
        self.scheduler_beat.load(Ordering::Relaxed)
    }

    /// The client for requests to other services.
    pub const fn outbound(&self) -> Option<&Outbound> {
        self.outbound.as_ref()
    }

    /// The client for the requests to a service, with its setting "use the custom
    /// certificate": when it is on, those requests also trust the authority that an
    /// Admin uploaded.
    pub fn outbound_for(&self, custom_certificate: bool) -> Option<Outbound> {
        self.outbound
            .as_ref()
            .map(|outbound| outbound.with_custom_certificate(custom_certificate))
    }

    /// The client for the account service of Plex.
    pub fn plex(&self) -> Option<PlexTv> {
        Some(PlexTv::new(self.outbound.clone()?, &self.plex_tv))
    }

    /// What sends the e-mails.
    pub const fn mailer(&self) -> &Mailer {
        &self.mailer
    }

    /// Probes a target once.
    pub async fn probe(&self, target: PingTarget) -> Latency {
        self.prober.probe(target).await
    }

    /// The pictures of other applications that the server keeps, when it has a folder
    /// for them.
    pub const fn picture_cache(&self) -> Option<&PictureCache> {
        self.picture_cache.as_ref()
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
