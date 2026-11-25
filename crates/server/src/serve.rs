//! The `serve` command: opens the database, loads the settings and serves the API.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use organizr_api::client::TrustedProxies;
use organizr_api::log_files;
use organizr_api::{AppState, router};
use organizr_auth::secrets::SecretBox;
use organizr_auth::setup_code::SetupCode;
use organizr_db::{Database, Engine};
use organizr_settings::Settings;
use tokio::net::TcpListener;
use tracing_subscriber::filter::{FilterExt as _, filter_fn};
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{EnvFilter, Layer as _};

use crate::log_file::FileLog;

/// The options of `organizr serve`. Each one can also be set in the environment.
///
/// These are the settings the application needs before it can read its database, so they
/// cannot live in the database.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// The address and port to listen on.
    #[arg(long, env = "ORGANIZR_LISTEN", default_value = "0.0.0.0:8080")]
    listen: SocketAddr,

    /// The reverse proxies in front of the application: addresses and networks, separated
    /// by commas, for example `172.18.0.0/16`. The headers `X-Forwarded-For` and
    /// `X-Forwarded-Proto` are believed only from these. Without it, no proxy is trusted.
    #[arg(long, env = "ORGANIZR_TRUSTED_PROXIES", default_value = "")]
    trusted_proxies: String,

    /// The address of the list of a marketplace for extensions: a JSON file. Without
    /// it, extensions are installed by hand only.
    #[arg(long, env = "ORGANIZR_EXTENSION_MARKETPLACE", default_value = "")]
    extension_marketplace: String,

    /// Whether every page runs the scripts that an Admin stores in the settings "Custom
    /// Javascript" and "Theme Javascript". Such a script runs in the session of every
    /// person who opens the page, guests on the login page too. Off unless set to `1`.
    /// The variable takes `1`, `true`, `yes` or `on`, and `0`, `false`, `no` or `off`.
    #[arg(
        long,
        env = "ORGANIZR_CUSTOM_JS",
        action = clap::ArgAction::SetTrue,
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    custom_js: bool,

    #[command(flatten)]
    storage: StorageArgs,
}

/// Where the application keeps its data. `serve` and `import` share these options.
#[derive(Debug, clap::Args)]
pub struct StorageArgs {
    /// The folder for the data of the application.
    #[arg(long, env = "ORGANIZR_DATA_DIR", default_value = "data")]
    data_dir: PathBuf,

    /// The database: `sqlite:<file>` or `postgres://user:password@host/database`.
    /// Without it, the application uses the SQLite file `organizr.db` in the data folder.
    #[arg(long, env = "ORGANIZR_DATABASE_URL", hide_env_values = true)]
    database_url: Option<String>,

    /// The key that protects the stored secrets: 64 hex digits. Without it, the
    /// application uses the key file.
    #[arg(long, env = "ORGANIZR_SECRET_KEY", hide_env_values = true)]
    secret_key: Option<String>,

    /// The file that holds the key for the stored secrets. The application makes
    /// the file on its first start. Without this option, the file is `secret.key` in
    /// the data folder.
    #[arg(long, env = "ORGANIZR_SECRET_KEY_FILE")]
    secret_key_file: Option<PathBuf>,
}

impl StorageArgs {
    /// The folder for the data of the application.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// The database URL, when one was given.
    pub fn database_url(&self) -> Option<&str> {
        self.database_url.as_deref()
    }

    /// The file that holds the key for the stored secrets.
    fn key_file(&self) -> PathBuf {
        self.secret_key_file
            .clone()
            .unwrap_or_else(|| self.data_dir.join(crate::secret_key::FILE_NAME))
    }

    /// What seals the stored secrets. The key file is made when it is missing.
    pub fn secrets(&self) -> anyhow::Result<SecretBox> {
        let key = crate::secret_key::load_or_create(self.secret_key.as_deref(), &self.key_file())?;
        SecretBox::new(&key).context("cannot use the key for the stored secrets")
    }

    /// A reminder to keep a copy of the key, which names where the key is.
    pub fn key_reminder(&self) -> String {
        let place = if self.secret_key.is_some() {
            "the key in ORGANIZR_SECRET_KEY".to_owned()
        } else {
            format!("the key file {}", self.key_file().display())
        };
        format!(
            "Keep a copy of {place} in a safe place apart from your backups: \
             the secrets in the database cannot be read without it."
        )
    }
}

/// Runs the server until the process is asked to stop.
pub async fn run(args: Args) -> anyhow::Result<()> {
    // Before the log files, the database and the key go into the folder.
    prepare_data_dir(&args.storage.data_dir)?;
    let log_file = start_logging(&args.storage.data_dir);

    let database = open_database(&args.storage).await?;
    let secrets = args.storage.secrets()?;
    // Settings that PHP's defaults name differently from the key that PHP reads move to
    // that key before the settings are read (D-112).
    let notes = organizr_api::adopt_renamed_settings(&database, &secrets)
        .await
        .context("cannot move the values of renamed settings")?;
    for note in &notes {
        tracing::info!("{note}");
    }
    let settings = load_settings(&database, &secrets).await?;
    let trusted_proxies: TrustedProxies = args
        .trusted_proxies
        .parse()
        .context("the list of trusted proxies is not valid")?;
    if trusted_proxies == TrustedProxies::default() {
        tracing::info!("no reverse proxy is trusted; forwarding headers are ignored");
    }
    let mut state = AppState::new(database.clone(), settings, secrets)
        .with_trusted_proxies(trusted_proxies)
        // The pictures that people upload, at the place of the PHP application.
        .with_picture_folder(args.storage.data_dir.join("userTabs"))
        // The pictures of other applications that the server fetches and keeps.
        .with_picture_cache(args.storage.data_dir.join("cache"))
        .with_data_folder(args.storage.data_dir.clone())
        .with_custom_js(args.custom_js);
    if args.custom_js {
        tracing::warn!(
            "ORGANIZR_CUSTOM_JS is on: every page runs the custom JavaScript of the Admin"
        );
    }
    // The log file follows the `logLevel` and `maxLogFiles` settings from now on.
    log_file.follow(state.settings_watch());
    #[cfg(feature = "wasm-plugins")]
    {
        state = state.with_extension_marketplace(&args.extension_marketplace);
    }
    #[cfg(not(feature = "wasm-plugins"))]
    if !args.extension_marketplace.is_empty() {
        tracing::warn!("this server was built without extensions, so the marketplace is not used");
    }
    let is_set_up = database
        .install()
        .is_done()
        .await
        .context("cannot read whether the install is set up")?;
    if !is_set_up {
        // The code goes to the log on purpose: only the person who runs the server can
        // read it there, and the set-up wizard asks for it. Each start makes a new one.
        let code = SetupCode::generate().context("cannot make the set-up code")?;
        tracing::warn!(
            "this install is not set up; the set-up wizard asks for the set-up code {code}"
        );
        state = state.with_setup_code(code);
    }
    // The jobs of the application run in a task of their own, which ends with the server.
    let scheduler = tokio::spawn(crate::scheduler::run(state.clone()));
    // The streams of the chat end when the server stops, so that they do not hold it.
    let chat_hub = std::sync::Arc::clone(state.chat_hub());
    // A request for a file of the web interface is answered before it reaches the API.
    let app = router(state)
        .context("the route table is not valid")?
        .layer(axum::middleware::from_fn(crate::web::serve_files))
        // The files of the web interface get a request id too. The router keeps this one.
        .layer(axum::middleware::from_fn(organizr_api::request_id::assign));

    let listener = TcpListener::bind(args.listen)
        .await
        .with_context(|| format!("cannot listen on {}", args.listen))?;
    tracing::info!(address = %args.listen, version = env!("CARGO_PKG_VERSION"), "Organizr is listening");

    // The address of each connection goes to the application, which needs it to decide
    // whether a forwarding header can be believed.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        stop_requested().await;
        chat_hub.close();
    })
    .await
    .context("the server stopped with an error")?;

    tracing::info!("Organizr is stopping");
    scheduler.abort();
    let closed = database
        .close()
        .await
        .context("the database did not close cleanly");
    log_file.flush(LAST_LINES);
    closed
}

/// How long the server waits at the end for the last lines of the log file.
const LAST_LINES: Duration = Duration::from_secs(2);

/// Writes log lines to the standard output, and to the log file of the day in the folder
/// `logs` of the data folder. `ORGANIZR_LOG` sets the level of the standard output, for
/// example `debug` or `organizr_api=debug,info`. The default is `info`. The `logLevel`
/// setting sets the level of the file.
fn start_logging(data_dir: &Path) -> FileLog {
    let filter = EnvFilter::try_from_env("ORGANIZR_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let (log_file, file_layer) = FileLog::start(log_files::folder(data_dir));
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_filter(filter.and(filter_fn(crate::log_file::may_reach_the_output))),
        )
        .with(file_layer.with_filter(filter_fn(crate::log_file::may_reach_the_file)))
        .init();
    log_file
}

/// The user that runs the server in the container image (`USER` of docker/Dockerfile).
const IMAGE_USER: u32 = 65_532;

/// Makes the data folder when it is missing, and makes sure that the server can write to
/// it. Without this check, a folder that belongs to another user stops the server later
/// with an error of SQLite that does not name the cause.
pub fn prepare_data_dir(data_dir: &Path) -> anyhow::Result<()> {
    let hint = || {
        format!(
            "The user that runs Organizr must be able to write to it. In the container \
             image that is the user {IMAGE_USER}: give it the folder on the host, for \
             example with `chown -R {IMAGE_USER}:{IMAGE_USER} /path/on/the/host`."
        )
    };
    std::fs::create_dir_all(data_dir).with_context(|| {
        format!(
            "cannot make the data folder {}. {}",
            data_dir.display(),
            hint()
        )
    })?;
    let probe = data_dir.join(format!(".organizr-write-test-{}", std::process::id()));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .and_then(|_| std::fs::remove_file(&probe))
        .with_context(|| {
            format!(
                "cannot write to the data folder {}. {}",
                data_dir.display(),
                hint()
            )
        })
}

/// Opens the database and brings its schema up to date.
pub async fn open_database(args: &StorageArgs) -> anyhow::Result<Database> {
    let url = if let Some(url) = &args.database_url {
        url.clone()
    } else {
        std::fs::create_dir_all(&args.data_dir).with_context(|| {
            format!("cannot create the data folder {}", args.data_dir.display())
        })?;
        default_database_url(&args.data_dir)
    };

    // The URL can hold a password, so it is not put into an error or a log line.
    let database = Database::connect(&url)
        .await
        .context("cannot open the database")?;
    database
        .migrate()
        .await
        .context("cannot bring the database schema up to date")?;

    let engine = match database.engine() {
        Engine::Sqlite => "SQLite",
        Engine::Postgres => "PostgreSQL",
    };
    tracing::info!(engine, "the database is ready");
    Ok(database)
}

/// The URL of the SQLite file in the data folder.
fn default_database_url(data_dir: &Path) -> String {
    let file = data_dir.join("organizr.db");
    format!("sqlite:{}", file.display().to_string().replace('\\', "/"))
}

/// Reads the settings. A stored entry that cannot be used is logged and left at its default.
pub async fn load_settings(database: &Database, secrets: &SecretBox) -> anyhow::Result<Settings> {
    let (settings, problems) = organizr_api::stored_settings(database, secrets)
        .await
        .context("cannot read the settings")?;
    for problem in &problems {
        tracing::warn!(%problem, "a stored setting is ignored");
    }
    Ok(settings)
}

/// Completes when the process is asked to stop: Ctrl-C, or the terminate signal that
/// `docker stop` sends.
async fn stop_requested() {
    let interrupt = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(%error, "cannot wait for Ctrl-C");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => {
                tracing::warn!(%error, "cannot wait for the terminate signal");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = interrupt => {}
        () = terminate => {}
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use clap::Parser;

    use super::{Args, default_database_url, prepare_data_dir};

    /// The options of `organizr serve`, as a command of their own.
    #[derive(Debug, Parser)]
    struct Serve {
        #[command(flatten)]
        args: Args,
    }

    #[test]
    fn scripts_of_the_admin_are_off_unless_the_operator_turns_them_on() {
        let parsed = |words: &[&str]| {
            Serve::try_parse_from(words)
                .unwrap_or_else(|error| panic!("{error}"))
                .args
                .custom_js
        };
        // The test does not read the environment: no `ORGANIZR_CUSTOM_JS` is set there.
        assert!(!parsed(&["serve"]));
        assert!(parsed(&["serve", "--custom-js"]));
    }

    #[test]
    fn key_reminder_names_where_the_key_is() {
        let reminder = |words: &[&str]| {
            Serve::try_parse_from(words)
                .unwrap_or_else(|error| panic!("{error}"))
                .args
                .storage
                .key_reminder()
        };
        let file = Path::new("/config").join("secret.key");

        assert!(
            reminder(&["serve", "--data-dir", "/config"])
                .starts_with(&format!("Keep a copy of the key file {} ", file.display()))
        );
        assert!(
            reminder(&["serve", "--secret-key-file", "/run/secrets/organizr_key"])
                .contains("the key file /run/secrets/organizr_key in a safe place")
        );
        assert!(
            reminder(&["serve", "--secret-key", &"0".repeat(64)])
                .starts_with("Keep a copy of the key in ORGANIZR_SECRET_KEY ")
        );
    }

    #[test]
    fn default_database_is_a_file_in_the_data_folder() {
        assert_eq!(
            default_database_url(Path::new("/config")),
            "sqlite:/config/organizr.db"
        );
        assert_eq!(
            default_database_url(Path::new("data")),
            "sqlite:data/organizr.db"
        );
    }

    #[test]
    fn data_folder_is_made_and_left_without_the_test_file() {
        let parent = tempfile::tempdir().expect("a folder");
        let data = parent.path().join("data");

        prepare_data_dir(&data).expect("the folder can be used");

        assert!(data.is_dir());
        let left: Vec<_> = std::fs::read_dir(&data).expect("readable").collect();
        assert_eq!(left.len(), 0, "the test file is removed");
    }

    #[cfg(unix)]
    #[test]
    fn data_folder_that_the_server_cannot_write_to_stops_it_with_the_reason() {
        use std::os::unix::fs::PermissionsExt;

        let data = tempfile::tempdir().expect("a folder");
        let read_only = std::fs::Permissions::from_mode(0o555);
        std::fs::set_permissions(data.path(), read_only).expect("permissions can be set");

        let error = prepare_data_dir(data.path()).expect_err("the folder is read-only");
        let text = format!("{error:#}");

        std::fs::set_permissions(data.path(), std::fs::Permissions::from_mode(0o755))
            .expect("permissions can be set back");
        assert!(text.contains("cannot write to the data folder"), "{text}");
        assert!(text.contains("65532"), "{text}");
    }

    #[test]
    fn windows_paths_are_written_with_forward_slashes() {
        assert_eq!(
            default_database_url(Path::new(r"C:\Organizr\data")),
            "sqlite:C:/Organizr/data/organizr.db"
        );
    }
}
