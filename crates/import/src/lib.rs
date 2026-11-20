//! One-time importer that moves a PHP Organizr install into the Rust application.
//!
//! An import has three steps. [`read`] finds the configuration file and the database of
//! the PHP install and reads them; no PHP code runs. [`plan::plan`] decides what to
//! store, and repairs records that the new application could not hold.
//! [`files::plan_files`] decides which files to copy. [`run`] does all of it and stores
//! the records in one step, so the new database holds the whole install or nothing.

pub mod files;
pub mod legacy_aes;
pub mod legacy_db;
pub mod php_config;
pub mod plan;
pub mod report;

use std::io::ErrorKind;
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
const IMPORTED_TABLES: [&str; 7] = [
    "groups",
    "users",
    "categories",
    "tabs",
    "BOOKMARK-categories",
    "BOOKMARK-tabs",
    "invites",
];

/// Why an import cannot run.
#[derive(Debug, Error)]
pub enum ImportError {
    /// The folder holds no PHP install.
    #[error(
        "no Organizr install was found in {0}: there is no file data/config/config.php in it, \
         or in its folders www/organizr, www/Dashboard, www or html"
    )]
    NoInstall(String),
    /// A place where a file of the PHP install can be could not be looked at, and the
    /// file was found in no other place.
    #[error(
        "cannot look at {path}: {source}; the user that runs the import must be able to          enter each folder of that path (in the container image, that user has the UID 65532)"
    )]
    Inaccessible {
        /// The path that could not be looked at.
        path: String,
        /// What the system reported.
        source: std::io::Error,
    },
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
    /// A file of the PHP install cannot be copied.
    #[error(transparent)]
    Files(#[from] files::CopyError),
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
    let config_files: Vec<PathBuf> = WEB_ROOTS
        .iter()
        .map(|inside| request.from.join(inside).join(CONFIG_FILE))
        .collect();
    let config_file = first_file(&config_files)?
        .ok_or_else(|| ImportError::NoInstall(request.from.display().to_string()))?;
    // The configuration file is two folders below the folder of the application.
    let web_root = config_file
        .ancestors()
        .nth(3)
        .unwrap_or(request.from)
        .to_owned();
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
        bookmark_categories: database.rows("BOOKMARK-categories").await?,
        bookmarks: database.rows("BOOKMARK-tabs").await?,
        invites: database.rows("invites").await?,
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
    first_file(&candidates)?.ok_or_else(|| {
        let mut tried: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.display().to_string())
            .collect();
        tried.dedup();
        ImportError::NoDatabase(tried)
    })
}

/// Whether the system says with this kind of error that nothing is at a path.
const fn means_absent(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::NotFound | ErrorKind::NotADirectory)
}

/// The first of the paths that is a file.
///
/// A path that cannot be looked at, for example in a folder that this user cannot
/// enter, does not stop the search: the file can be at a later path. When no path is a
/// file, the first path that could not be looked at is the error, because "not found"
/// would send the person to the wrong repair.
fn first_file(candidates: &[PathBuf]) -> Result<Option<PathBuf>, ImportError> {
    let mut inaccessible = None;
    for candidate in candidates {
        match std::fs::metadata(candidate) {
            Ok(found) if found.is_file() => return Ok(Some(candidate.clone())),
            Ok(_) => {}
            Err(error) if means_absent(error.kind()) => {}
            Err(source) => {
                inaccessible.get_or_insert(ImportError::Inaccessible {
                    path: candidate.display().to_string(),
                    source,
                });
            }
        }
    }
    inaccessible.map_or(Ok(None), Err)
}

/// Reads the PHP install and makes the plan, and writes nothing. The secrets of the plan
/// are sealed with `secrets`.
pub async fn dry_run(
    request: ImportRequest<'_>,
    secrets: &SecretBox,
) -> Result<Report, ImportError> {
    let source = read(request).await?;
    let Plan {
        mut data,
        mut report,
    } = plan::plan(&source.install, secrets)?;
    files::plan_files(
        &source.web_root,
        &source.database_file,
        &mut data,
        &mut report,
    );
    report.dry_run = true;
    Ok(report)
}

/// Imports the PHP install into `target`, which must be empty and have its schema, and
/// copies its files into `data_dir`. All records are stored, or none is.
pub async fn run(
    request: ImportRequest<'_>,
    target: &Database,
    secrets: &SecretBox,
    data_dir: &Path,
) -> Result<Report, ImportError> {
    let source = read(request).await?;
    store(&source, target, secrets, data_dir).await
}

/// Stores a PHP install that was read in `target`, which must be empty and have its
/// schema, and copies its files into `data_dir`. All records are stored, or none is.
///
/// The files are copied first. When the records cannot be stored after that, the copied
/// files stay, and the next import writes over them.
pub async fn store(
    source: &Source,
    target: &Database,
    secrets: &SecretBox,
    data_dir: &Path,
) -> Result<Report, ImportError> {
    let is_empty = target
        .import()
        .is_possible()
        .await
        .map_err(ImportError::Database)?;
    if !is_empty {
        return Err(ImportError::NotEmpty);
    }
    let Plan {
        mut data,
        mut report,
    } = plan::plan(&source.install, secrets)?;
    let copies = files::plan_files(
        &source.web_root,
        &source.database_file,
        &mut data,
        &mut report,
    );
    files::copy_files(&copies, data_dir)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_missing_path_means_that_nothing_is_there() {
        assert!(means_absent(ErrorKind::NotFound));
        assert!(means_absent(ErrorKind::NotADirectory));
        assert!(!means_absent(ErrorKind::PermissionDenied));
        assert!(!means_absent(ErrorKind::Other));
    }

    #[test]
    fn first_file_passes_folders_and_missing_paths() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let file = folder.path().join("config.php");
        std::fs::write(&file, "text").expect("the file can be written");
        let candidates = [
            folder.path().join("missing"),
            // A folder is no file.
            folder.path().to_owned(),
            // A path through a file names nothing.
            file.join("below"),
            file.clone(),
        ];

        let found = first_file(&candidates).expect("every path can be looked at");
        assert_eq!(found, Some(file));
        let found = first_file(&candidates[..3]).expect("every path can be looked at");
        assert_eq!(found, None);
    }
}
