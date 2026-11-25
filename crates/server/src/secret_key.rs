//! The master key of the install: where it comes from, and how a new one is made.
//!
//! The key protects the secrets that the install stores, such as the tokens of other
//! services. It is not in the database. It comes from the environment, or else from a
//! key file, which the server writes on its first start.

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::Path;

use anyhow::Context;
use organizr_auth::secrets::MasterKey;

/// The name of the key file in the data folder.
pub const FILE_NAME: &str = "secret.key";

/// The master key: the given text when there is one, else the key in the file. When the
/// file does not exist, a new key is made and written to it.
///
/// `from_environment` is the value of `ORGANIZR_SECRET_KEY`. A key from there is never
/// written to a file.
pub fn load_or_create(from_environment: Option<&str>, file: &Path) -> anyhow::Result<MasterKey> {
    if let Some(text) = from_environment {
        return MasterKey::from_hex(text).context("ORGANIZR_SECRET_KEY is not a key");
    }
    match std::fs::read_to_string(file) {
        Ok(text) => MasterKey::from_hex(&text)
            .with_context(|| format!("the key file {} does not hold a key", file.display())),
        Err(error) if error.kind() == ErrorKind::NotFound => create(file),
        Err(error) => {
            Err(error).with_context(|| format!("cannot read the key file {}", file.display()))
        }
    }
}

/// Makes a new key and writes it to a new file that only the owner can read.
fn create(file: &Path) -> anyhow::Result<MasterKey> {
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)
            .with_context(|| format!("cannot create the folder {}", folder.display()))?;
    }
    let key = MasterKey::generate().context("cannot make a key for the secrets")?;
    let mut options = OpenOptions::new();
    // The file must be new: a key that exists is never written over.
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut handle = options
        .open(file)
        .with_context(|| format!("cannot create the key file {}", file.display()))?;
    writeln!(handle, "{}", key.to_hex().as_str())
        .and_then(|()| handle.sync_all())
        .with_context(|| format!("cannot write the key file {}", file.display()))?;
    tracing::warn!(
        file = %file.display(),
        "a new key for the stored secrets was written; keep a copy of this file in a safe place \
         apart from your backups, because the secrets in the database cannot be read without it"
    );
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::{FILE_NAME, load_or_create};

    /// A key of a test. It is no key of any install.
    const KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    #[test]
    fn first_start_writes_a_key_file_and_the_next_start_reads_it() {
        let folder = tempfile::tempdir().expect("a folder can be made");
        let file = folder.path().join("data").join(FILE_NAME);

        let first = load_or_create(None, &file).expect("a key can be made");
        let written = std::fs::read_to_string(&file).expect("the key file exists");
        let second = load_or_create(None, &file).expect("the key can be read");

        assert_eq!(written.trim(), first.to_hex().as_str());
        assert_eq!(written.trim().len(), 64);
        assert_eq!(second.to_hex(), first.to_hex());
    }

    #[cfg(unix)]
    #[test]
    fn key_file_is_for_its_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let folder = tempfile::tempdir().expect("a folder can be made");
        let file = folder.path().join(FILE_NAME);

        load_or_create(None, &file).expect("a key can be made");

        let mode = std::fs::metadata(&file)
            .expect("the key file exists")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn key_from_the_environment_wins_and_writes_no_file() {
        let folder = tempfile::tempdir().expect("a folder can be made");
        let file = folder.path().join(FILE_NAME);
        std::fs::write(&file, "ff".repeat(32)).expect("the file can be written");

        let key = load_or_create(Some(KEY), &file).expect("the key is valid");
        assert_eq!(key.to_hex().as_str(), KEY);

        let other = folder.path().join("none").join(FILE_NAME);
        load_or_create(Some(KEY), &other).expect("the key is valid");
        assert!(!other.exists());
    }

    #[test]
    fn text_that_is_no_key_is_refused_and_the_file_stays() {
        let folder = tempfile::tempdir().expect("a folder can be made");
        let file = folder.path().join(FILE_NAME);
        std::fs::write(&file, "not a key").expect("the file can be written");

        let from_file = load_or_create(None, &file).expect_err("the file holds no key");
        assert!(
            from_file.to_string().contains("does not hold a key"),
            "{from_file}"
        );
        let from_environment =
            load_or_create(Some("too short"), &file).expect_err("the text is no key");
        assert!(
            from_environment.to_string().contains("ORGANIZR_SECRET_KEY"),
            "{from_environment}"
        );
        // The error does not repeat the text, which can be a key with a typing mistake.
        assert!(!format!("{from_environment:#}").contains("too short"));
        assert_eq!(
            std::fs::read_to_string(&file).expect("the file is still there"),
            "not a key"
        );
    }
}
