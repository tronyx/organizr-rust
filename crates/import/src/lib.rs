//! One-time importer that moves a PHP Organizr install into the Rust application.
//!
//! An import has three steps. [`read`] finds the configuration file and the database of
//! the PHP install and reads them; no PHP code runs. [`plan::plan`] decides what to
//! store, and repairs records that the new application could not hold. [`run`] does
//! both and stores the result in one step, so the new database holds the whole install
//! or nothing.

pub mod legacy_aes;
pub mod legacy_db;
pub mod php_config;
pub mod plan;
pub mod report;

use std::path::{Path, PathBuf};

use organizr_auth::secrets::SecretBox;
use organizr_db::{Database, DbError};
use thiserror::Error;

use crate::legacy_db::{LegacyDatabase, LegacyDbError};
use crate::php_config::PhpConfigError;
use crate::plan::{LegacyInstall, Plan, PlanError};
use crate::report::Report;

/// Where the configuration file is, from the folder of the PHP application.
const CONFIG_FILE: &str = "data/config/config.php";

/// Where the folder of the PHP application can be, from the folder that was given: the
/// folder itself, or the places inside the `/config` volume of a container image.
const WEB_ROOTS: [&str; 5] = ["", "www/organizr", "www/Dashboard", "www", "html"];

/// The tables that the importer reads.
const IMPORTED_TABLES: [&str; 4] = ["groups", "users", "categories", "tabs"];

/// Why an import cannot run.
#[derive(Debug, Error)]
pub enum ImportError {
    /// The folder holds no PHP install.
    #[error(
        "no Organizr install was found in {0}: there is no file data/config/config.php in it, \
         or in its folders www/organizr, www/Dashboard, www or html"
    )]
    NoInstall(String),
    /// The configuration file cannot be read from the disk.
    #[error("cannot read {file}: {source}")]
    Unreadable {
        /// The file.
        file: String,
        /// What the system reported.
        source: std::io::Error,
    },
    /// The configuration file is not what the PHP application writes.
    #[error("the configuration file {file} cannot be understood: {source}")]
    Config {
        /// The file.
        file: String,
        /// What is wrong with it.
        source: PhpConfigError,
    },
    /// The install uses a database server.
    #[error(
        "the PHP install keeps its data in a {0} database; this version imports SQLite databases only"
    )]
    UnsupportedDriver(String),
    /// The database file was not found.
    #[error(
        "the database file of the PHP install was not found (looked at: {}); \
         name it with the option --database",
        .0.join(", ")
    )]
    NoDatabase(Vec<String>),
    /// The database of the PHP install cannot be read.
    #[error(transparent)]
    LegacyDatabase(#[from] LegacyDbError),
    /// No plan can be made.
    #[error(transparent)]
    Plan(#[from] PlanError),
    /// The new database holds records already.
    #[error(
        "the new database is not empty; an import needs a new, empty database, \
         so nothing was changed"
    )]
    NotEmpty,
    /// The new database reported an error.
    #[error("cannot store the import: {0}")]
    Database(#[source] DbError),
}

/// What to import.
#[derive(Debug, Clone, Copy)]
pub struct ImportRequest<'a> {
    /// The folder of the PHP install: the folder of the PHP application, or the
    /// `/config` volume of a container image that holds it.
    pub from: &'a Path,
    /// The database file of the PHP install, when it is not where its configuration
    /// says.
    pub database_file: Option<&'a Path>,
}

/// A PHP install that was found and read.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    /// The folder of the PHP application.
    pub web_root: PathBuf,
    /// The database file.
    pub database_file: PathBuf,
    /// What the install holds.
    pub install: LegacyInstall,
}

/// Finds the PHP install and reads its configuration file and its database.
pub async fn read(request: ImportRequest<'_>) -> Result<Source, ImportError> {
    let web_root = WEB_ROOTS
        .iter()
        .map(|inside| request.from.join(inside))
        .find(|root| root.join(CONFIG_FILE).is_file())
        .ok_or_else(|| ImportError::NoInstall(request.from.display().to_string()))?;
    let config_file = web_root.join(CONFIG_FILE);
    let shown = config_file.display().to_string();
    let text = std::fs::read_to_string(&config_file).map_err(|source| ImportError::Unreadable {
        file: shown.clone(),
        source,
    })?;
    let config = php_config::parse(&text).map_err(|source| ImportError::Config {
        file: shown,
        source,
    })?;

    let setting = |key: &str| {
        config
            .iter()
            .find(|(name, _)| name == key)
            .and_then(|(_, value)| value.as_str())
            .unwrap_or("")
    };
    let driver = setting("driver");
    if !(driver.is_empty() || driver.starts_with("sqlite")) {
        return Err(ImportError::UnsupportedDriver(driver.to_owned()));
    }
    let database_file = match request.database_file {
        Some(file) => file.to_owned(),
        None => find_database(
            &[request.from, &web_root],
            setting("dbLocation"),
            setting("dbName"),
        )?,
    };

    let database = LegacyDatabase::open(&database_file).await?;
    let mut install = LegacyInstall {
        config,
        groups: database.rows("groups").await?,
        users: database.rows("users").await?,
        categories: database.rows("categories").await?,
        tabs: database.rows("tabs").await?,
        other_tables: Vec::new(),
    };
    for table in database.tables().await? {
        if !IMPORTED_TABLES.contains(&table.as_str()) {
            let rows = database.count(&table).await?;
            install.other_tables.push((table, rows));
        }
    }
    database.close().await?;

    Ok(Source {
        web_root,
        database_file,
        install,
    })
}

/// Finds the database file. The configuration names it by the path inside the old
/// server or container, which is often another path on the machine of the import. So
/// the file is also looked for under the given folders, by the end of that path.
fn find_database(folders: &[&Path], location: &str, name: &str) -> Result<PathBuf, ImportError> {
    if name.is_empty() {
        return Err(ImportError::NoDatabase(Vec::new()));
    }
    let names = [name.to_owned(), format!("{name}.db")];
    let parts: Vec<&str> = location
        .split(['/', '\\'])
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .collect();

    let mut candidates: Vec<PathBuf> = names
        .iter()
        .map(|name| Path::new(location).join(name))
        .collect();
    for folder in folders {
        for skipped in 0..=parts.len() {
            let mut inside = folder.to_path_buf();
            inside.extend(parts.iter().skip(skipped));
            candidates.extend(names.iter().map(|name| inside.join(name)));
        }
    }
    candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .cloned()
        .ok_or_else(|| {
            let mut tried: Vec<String> = candidates
                .iter()
                .map(|candidate| candidate.display().to_string())
                .collect();
            tried.dedup();
            ImportError::NoDatabase(tried)
        })
}

/// Reads the PHP install and makes the plan, and writes nothing. The secrets of the plan
/// are sealed with `secrets`.
pub async fn dry_run(
    request: ImportRequest<'_>,
    secrets: &SecretBox,
) -> Result<Report, ImportError> {
    let source = read(request).await?;
    let Plan { mut report, .. } = plan::plan(&source.install, secrets)?;
    report.dry_run = true;
    Ok(report)
}

/// Imports the PHP install into `target`, which must be empty and have its schema.
/// Everything is stored, or nothing is.
pub async fn run(
    request: ImportRequest<'_>,
    target: &Database,
    secrets: &SecretBox,
) -> Result<Report, ImportError> {
    let source = read(request).await?;
    store(&source, target, secrets).await
}

/// Stores a PHP install that was read in `target`, which must be empty and have its
/// schema. Everything is stored, or nothing is.
pub async fn store(
    source: &Source,
    target: &Database,
    secrets: &SecretBox,
) -> Result<Report, ImportError> {
    let is_empty = target
        .import()
        .is_possible()
        .await
        .map_err(ImportError::Database)?;
    if !is_empty {
        return Err(ImportError::NotEmpty);
    }
    let Plan { data, report } = plan::plan(&source.install, secrets)?;
    target
        .import()
        .run(data)
        .await
        .map_err(|error| match error {
            DbError::NotEmpty => ImportError::NotEmpty,
            other => ImportError::Database(other),
        })?;
    Ok(report)
}
