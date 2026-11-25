//! The `import` command: moves an install of PHP Organizr into this application.

use std::path::PathBuf;

use anyhow::Context;
use organizr_auth::secrets::SecretBox;
use organizr_import::ImportRequest;

use crate::serve::{StorageArgs, open_database, prepare_data_dir};

/// The options of `organizr import`.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// The folder of the PHP install: the folder of the PHP application (the one with
    /// `index.php` and `data`), or the `/config` volume of a container that holds it.
    /// The command only reads there.
    #[arg(long)]
    from: PathBuf,

    /// The SQLite file of the PHP install. Needed only when the file is not found from
    /// the path in the configuration of the PHP install.
    #[arg(long)]
    database: Option<PathBuf>,

    /// Read the PHP install and print what an import would do, and write nothing.
    #[arg(long)]
    dry_run: bool,

    #[command(flatten)]
    storage: StorageArgs,
}

/// Runs the import and prints its report.
pub async fn run(args: Args) -> anyhow::Result<()> {
    let request = ImportRequest {
        from: &args.from,
        database_file: args.database.as_deref(),
    };

    if args.dry_run {
        // The sealed values are thrown away, so any key does.
        let secrets = SecretBox::with_random_key().context("cannot make a key")?;
        let report = organizr_import::dry_run(request, &secrets).await?;
        println!("{report}");
        return Ok(());
    }

    // The PHP install is read before anything is created for the new one.
    let source = organizr_import::read(request).await?;
    prepare_data_dir(args.storage.data_dir())?;
    let database = open_database(&args.storage).await?;
    let secrets = args.storage.secrets()?;
    let report =
        organizr_import::store(&source, &database, &secrets, args.storage.data_dir()).await?;
    database
        .close()
        .await
        .context("the database did not close cleanly")?;
    println!("{report}");
    println!("{}", args.storage.key_reminder());
    Ok(())
}
