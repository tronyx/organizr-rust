//! The `restore` command: puts the files and the database of a backup back.

use std::path::{Path, PathBuf};

use anyhow::Context;
use organizr_api::backup::{self, RestoreOptions};

use crate::serve::StorageArgs;

/// The options of `organizr restore`.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// The backup: a zip file that Organizr made.
    #[arg(long)]
    from: PathBuf,

    /// Replace the database when one exists. Stop the server first.
    #[arg(long)]
    overwrite: bool,

    #[command(flatten)]
    storage: StorageArgs,
}

/// The file of a database URL for SQLite: `sqlite:/data/organizr.db?mode=rwc` names
/// `/data/organizr.db`. Another URL names no file.
fn sqlite_file(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("sqlite:")?;
    let path = rest.split('?').next().unwrap_or(rest);
    let path = path.strip_prefix("//").unwrap_or(path);
    (!path.is_empty() && path != ":memory:").then(|| PathBuf::from(path))
}

/// The file that the install keeps its database in, or nothing for an install on a
/// database server.
fn database_file(storage: &StorageArgs) -> Option<PathBuf> {
    match storage.database_url() {
        Some(url) => sqlite_file(url),
        None => Some(storage.data_dir().join(backup::DATABASE_FILE)),
    }
}

/// What the command prints after a restore.
fn report(restored: &backup::Restored, data_dir: &Path, keeps_a_file: bool) -> String {
    let mut lines = vec![
        "The restore is complete.".to_owned(),
        String::new(),
        format!(
            "The backup was made on {} by Organizr {}.",
            restored.manifest.created, restored.manifest.version
        ),
        format!(
            "{} files were written to {}.",
            restored.files,
            data_dir.display()
        ),
    ];
    lines.push(if restored.database {
        "The database was restored.".to_owned()
    } else if !restored.manifest.has_database {
        "The backup holds no database: its install uses PostgreSQL. Restore the database \
         with the tools of PostgreSQL."
            .to_owned()
    } else if keeps_a_file {
        "The database was not restored.".to_owned()
    } else {
        "The database of the backup was not restored, because this install does not keep \
         its database in a file."
            .to_owned()
    });
    lines.push(String::new());
    lines.push(
        "A backup never holds the key for the stored secrets. Make sure that the key of the \
         install that made the backup is in place before you start the server. Without it, \
         the stored passwords and tokens of other services cannot be read."
            .to_owned(),
    );
    lines.join("\n")
}

/// Runs the restore and prints what it did.
pub fn run(args: &Args) -> anyhow::Result<()> {
    let data_dir = args.storage.data_dir();
    let database_file = database_file(&args.storage);
    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("cannot create the data folder {}", data_dir.display()))?;
    let options = RestoreOptions {
        data_dir,
        database_file: database_file.as_deref(),
        overwrite: args.overwrite,
    };
    let restored = backup::restore(&args.from, options)?;
    println!("{}", report(&restored, data_dir, database_file.is_some()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::sqlite_file;
    use std::path::PathBuf;

    #[test]
    fn only_a_url_for_a_sqlite_file_names_a_file() {
        let file = |url: &str| sqlite_file(url);
        assert_eq!(
            file("sqlite:/data/organizr.db"),
            Some(PathBuf::from("/data/organizr.db"))
        );
        assert_eq!(
            file("sqlite:///data/organizr.db?mode=rwc"),
            Some(PathBuf::from("/data/organizr.db"))
        );
        assert_eq!(
            file("sqlite:C:/data/organizr.db"),
            Some(PathBuf::from("C:/data/organizr.db"))
        );
        assert_eq!(file("sqlite::memory:"), None);
        assert_eq!(file("sqlite:"), None);
        assert_eq!(file("postgres://user:secret@host/organizr"), None);
    }
}
