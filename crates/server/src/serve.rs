//! The `serve` command: opens the database, loads the settings and serves the API.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::Context;
use organizr_api::client::TrustedProxies;
use organizr_api::{AppState, router};
use organizr_auth::secrets::SecretBox;
use organizr_auth::setup_code::SetupCode;
use organizr_db::{Database, Engine};
use organizr_settings::Settings;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

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

    /// What seals the stored secrets. The key file is made when it is missing.
    pub fn secrets(&self) -> anyhow::Result<SecretBox> {
        let key_file = self
            .secret_key_file
            .clone()
            .unwrap_or_else(|| self.data_dir.join(crate::secret_key::FILE_NAME));
        let key = crate::secret_key::load_or_create(self.secret_key.as_deref(), &key_file)?;
        SecretBox::new(&key).context("cannot use the key for the stored secrets")
    }
}

/// Runs the server until the process is asked to stop.
pub async fn run(args: Args) -> anyhow::Result<()> {
    start_logging();

    let database = open_database(&args.storage).await?;
    let secrets = args.storage.secrets()?;
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
        .with_data_folder(args.storage.data_dir.clone());
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
    // A request for a file of the web interface is answered before it reaches the API.
    let app = router(state)
        .context("the route table is not valid")?
        .layer(axum::middleware::from_fn(crate::web::serve_files));

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
    .with_graceful_shutdown(stop_requested())
    .await
    .context("the server stopped with an error")?;

    tracing::info!("Organizr is stopping");
    scheduler.abort();
    database
        .close()
        .await
        .context("the database did not close cleanly")
}

/// Writes log lines to the standard output. `ORGANIZR_LOG` sets the level, for example
/// `debug` or `organizr_api=debug,info`. The default is `info`.
fn start_logging() {
    let filter = EnvFilter::try_from_env("ORGANIZR_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
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

    use super::default_database_url;

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
    fn windows_paths_are_written_with_forward_slashes() {
        assert_eq!(
            default_database_url(Path::new(r"C:\Organizr\data")),
            "sqlite:C:/Organizr/data/organizr.db"
        );
    }
}
