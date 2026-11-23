//! The `move-to-postgres` command: copies the SQLite database of an install into an empty
//! PostgreSQL database.

use std::fmt::Write as _;
use std::path::PathBuf;

use anyhow::{Context, bail};
use organizr_db::{Database, DbError, TableCount};

use crate::serve::StorageArgs;

/// The options of `organizr move-to-postgres`.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// The SQLite file to copy. Without it, the file `organizr.db` in the data folder.
    #[arg(long)]
    from: Option<PathBuf>,

    /// Print what the copy would do, and write nothing.
    #[arg(long)]
    dry_run: bool,

    #[command(flatten)]
    storage: StorageArgs,
}

const TARGET_NOT_EMPTY: &str = "the PostgreSQL database already holds records. Point ORGANIZR_DATABASE_URL at an empty database";

/// Copies the database and prints its report.
pub async fn run(args: Args) -> anyhow::Result<()> {
    let file = args
        .from
        .clone()
        .unwrap_or_else(|| args.storage.data_dir().join("organizr.db"));
    if !file.is_file() {
        bail!("the SQLite file {} does not exist", file.display());
    }
    let source = Database::open_sqlite_read_only(&file)
        .await
        .context("cannot open the SQLite file")?;
    if source.has_pending_migrations().await? {
        bail!(
            "the SQLite file was last opened by an older version of Organizr. Start Organizr once on SQLite with this version, stop it, and run this command again"
        );
    }

    let url = match args.storage.database_url() {
        Some(url) if url.starts_with("postgres://") || url.starts_with("postgresql://") => url,
        _ => bail!("ORGANIZR_DATABASE_URL must name the PostgreSQL database to copy into"),
    };
    // The URL can hold a password, so it is not put into an error.
    let target = Database::connect(url)
        .await
        .context("cannot open the PostgreSQL database")?;

    let counts = if args.dry_run {
        if target.holds_records().await? {
            bail!(TARGET_NOT_EMPTY);
        }
        source.row_counts().await?
    } else {
        match source.copy_into(&target).await {
            Err(DbError::NotEmpty) => bail!(TARGET_NOT_EMPTY),
            other => other?,
        }
    };
    println!("{}", report(&counts, args.dry_run));

    source
        .close()
        .await
        .context("the SQLite file did not close cleanly")?;
    target
        .close()
        .await
        .context("the PostgreSQL database did not close cleanly")?;
    Ok(())
}

/// The text the command prints: what is, or would be, copied, and what to do next.
fn report(counts: &[TableCount], dry_run: bool) -> String {
    let mut text = if dry_run {
        String::from(
            "Dry run: nothing was written.\n\nThese records would be copied to PostgreSQL:\n",
        )
    } else {
        String::from("These records were copied to PostgreSQL:\n")
    };
    for count in counts {
        let _ = writeln!(text, "{:>8}  {}", count.rows, count.table);
    }
    text.push_str(
        "\nThe SQLite file is not changed. Set ORGANIZR_DATABASE_URL to the PostgreSQL database and start Organizr with the same data folder.",
    );
    text
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, FromArgMatches, Parser};
    use organizr_db::{COPIED_TABLES, Database, TableCount};
    use organizr_testkit::engines::fresh_postgres;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Statement};

    use super::{Args, report, run};

    #[derive(Debug, Parser)]
    struct Cli {
        #[command(flatten)]
        args: Args,
    }

    fn args(arguments: &[&str]) -> Args {
        let mut all = vec!["organizr"];
        all.extend_from_slice(arguments);
        Cli::try_parse_from(all).expect("the arguments parse").args
    }

    /// Like `args`, but the database URL is only what the arguments give, whatever the
    /// environment of the developer holds.
    fn args_without_environment(arguments: &[&str]) -> Args {
        let mut all = vec!["organizr"];
        all.extend_from_slice(arguments);
        let matches = Cli::command()
            .mut_arg("database_url", |arg| arg.env(None::<&str>))
            .try_get_matches_from(all)
            .expect("the arguments parse");
        Cli::from_arg_matches(&matches)
            .expect("the arguments make the options")
            .args
    }

    fn url_of(file: &std::path::Path) -> String {
        format!("sqlite:{}", file.display().to_string().replace('\\', "/"))
    }

    /// A migrated SQLite file with a group and a user.
    async fn seeded_source(directory: &std::path::Path) -> std::path::PathBuf {
        let file = directory.join("organizr.db");
        let database = Database::connect(&format!("{}?mode=rwc", url_of(&file)))
            .await
            .expect("the file opens");
        database.migrate().await.expect("the migrations apply");
        organizr_db::entity::group::ActiveModel {
            id: Set(1),
            level: Set(0),
            name: Set("Administrator".to_owned()),
            name_key: Set("administrator".to_owned()),
            image: Set(None),
            is_default: Set(false),
        }
        .insert(database.connection())
        .await
        .expect("a group is stored");
        organizr_db::entity::user::ActiveModel {
            id: Set(2),
            username: Set("admin".to_owned()),
            username_key: Set("admin".to_owned()),
            email: Set(None),
            email_key: Set(None),
            password_hash: Set("hash".to_owned()),
            group_id: Set(1),
            locked: Set(false),
            image: Set(None),
            created_at: Set(1_700_000_000),
            auth_service: Set("internal".to_owned()),
            auth_provider: Set(None),
            auth_subject: Set(None),
            two_factor_kind: Set(None),
            two_factor_secret: Set(None),
            plex_token: Set(None),
            recovery_hash: Set(None),
            recovery_expires_at: Set(None),
        }
        .insert(database.connection())
        .await
        .expect("a user is stored");
        database.close().await.expect("the file closes");
        file
    }

    fn two_counts() -> Vec<TableCount> {
        vec![
            TableCount {
                table: "groups",
                rows: 6,
            },
            TableCount {
                table: "users",
                rows: 12345,
            },
        ]
    }

    const FOOTER: &str = "\nThe SQLite file is not changed. Set ORGANIZR_DATABASE_URL to the PostgreSQL database and start Organizr with the same data folder.";

    #[test]
    fn report_of_a_dry_run_says_nothing_was_written() {
        let expected = format!(
            "Dry run: nothing was written.\n\nThese records would be copied to PostgreSQL:\n       6  groups\n   12345  users\n{FOOTER}"
        );
        assert_eq!(report(&two_counts(), true), expected);
    }

    #[test]
    fn report_of_a_copy_says_were_copied() {
        let expected = format!(
            "These records were copied to PostgreSQL:\n       6  groups\n   12345  users\n{FOOTER}"
        );
        assert_eq!(report(&two_counts(), false), expected);
    }

    #[tokio::test]
    async fn target_that_is_not_postgres_is_refused() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let source = seeded_source(directory.path()).await;
        let from = source.display().to_string();
        for arguments in [
            vec!["--from", from.as_str()],
            vec!["--from", from.as_str(), "--database-url", "sqlite:x.db"],
        ] {
            let error = run(args_without_environment(&arguments))
                .await
                .expect_err("the target is refused");
            assert_eq!(
                error.to_string(),
                "ORGANIZR_DATABASE_URL must name the PostgreSQL database to copy into"
            );
        }
    }

    #[tokio::test]
    async fn missing_sqlite_file_is_refused() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let file = directory.path().join("none.db");
        let error = run(args(&[
            "--from",
            &file.display().to_string(),
            "--database-url",
            "postgres://u:p@127.0.0.1:1/x",
        ]))
        .await
        .expect_err("the file is missing");
        assert_eq!(
            error.to_string(),
            format!("the SQLite file {} does not exist", file.display())
        );
    }

    #[tokio::test]
    async fn dry_run_writes_nothing() {
        let Some(fresh) = fresh_postgres().await else {
            return;
        };
        let directory = tempfile::tempdir().expect("a temporary directory");
        let source = seeded_source(directory.path()).await;
        run(args(&[
            "--from",
            &source.display().to_string(),
            "--database-url",
            &fresh.url,
            "--dry-run",
        ]))
        .await
        .expect("the dry run succeeds");
        let tables = fresh
            .database
            .connection()
            .query_all_raw(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT 1 FROM information_schema.tables WHERE table_schema = 'public'",
            ))
            .await
            .expect("the tables are listed");
        assert!(tables.is_empty());
        fresh.remove().await;
    }

    #[tokio::test]
    async fn dry_run_against_a_database_with_records_is_refused() {
        let Some(fresh) = fresh_postgres().await else {
            return;
        };
        fresh.database.migrate().await.expect("the schema is made");
        organizr_db::entity::group::ActiveModel {
            id: Set(1),
            level: Set(0),
            name: Set("Existing".to_owned()),
            name_key: Set("existing".to_owned()),
            image: Set(None),
            is_default: Set(false),
        }
        .insert(fresh.database.connection())
        .await
        .expect("a group is stored");
        let directory = tempfile::tempdir().expect("a temporary directory");
        let source = seeded_source(directory.path()).await;
        let error = run(args(&[
            "--from",
            &source.display().to_string(),
            "--database-url",
            &fresh.url,
            "--dry-run",
        ]))
        .await
        .expect_err("the database holds records");
        assert_eq!(
            error.to_string(),
            "the PostgreSQL database already holds records. Point ORGANIZR_DATABASE_URL at an empty database"
        );
        fresh.remove().await;
    }

    #[tokio::test]
    async fn copy_runs_end_to_end() {
        let Some(fresh) = fresh_postgres().await else {
            return;
        };
        let directory = tempfile::tempdir().expect("a temporary directory");
        let source = seeded_source(directory.path()).await;
        run(args(&[
            "--from",
            &source.display().to_string(),
            "--database-url",
            &fresh.url,
        ]))
        .await
        .expect("the copy succeeds");
        let counts = fresh.database.row_counts().await.expect("rows are counted");
        assert_eq!(counts.len(), COPIED_TABLES.len());
        let rows_of = |name: &str| counts.iter().find(|c| c.table == name).map(|c| c.rows);
        assert_eq!(rows_of("groups"), Some(1));
        assert_eq!(rows_of("users"), Some(1));
        fresh.remove().await;
    }
}
