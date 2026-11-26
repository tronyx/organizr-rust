//! Backups: an archive of the data folder, with a copy of the `SQLite` database.
//!
//! A backup never holds the key for the stored secrets. The secrets in the database
//! are sealed with that key, so a backup alone does not give them away. An install
//! on `PostgreSQL` gets an archive of the files only: the database is saved with the
//! tools of `PostgreSQL`.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// The name of the folder of the backups in the data folder.
pub const FOLDER: &str = "backups";

/// The name of the database copy in an archive, and of the database in the data folder.
pub const DATABASE_FILE: &str = "organizr.db";

/// The name of the file in an archive that says what the archive is.
pub const MANIFEST_FILE: &str = "backup.json";

/// The file of the key for the stored secrets. It is never put into an archive, and
/// never taken out of one.
const KEY_FILE: &str = "secret.key";

/// The version of the layout of an archive.
const FORMAT: u32 = 1;

/// What an archive says about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// The version of the layout of the archive.
    pub format: u32,
    /// The version of the application that made the archive.
    pub version: String,
    /// When the archive was made, as `2026-11-20T03:15:00Z`.
    pub created: String,
    /// The database engine of the install: `sqlite` or `postgres`.
    pub engine: String,
    /// Whether the archive holds a copy of the database.
    pub has_database: bool,
}

impl Manifest {
    /// The manifest of an archive that is made now.
    pub fn new(version: &str, created: String, is_sqlite: bool, has_database: bool) -> Self {
        Self {
            format: FORMAT,
            version: version.to_owned(),
            created,
            engine: if is_sqlite { "sqlite" } else { "postgres" }.to_owned(),
            has_database,
        }
    }
}

/// A backup in the folder of the backups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupFile {
    /// The name of the file.
    pub name: String,
    /// The size in bytes.
    pub size: u64,
    /// When the file was written.
    pub modified: SystemTime,
}

/// Whether a text can be the name of a backup: a zip file in the folder itself, with
/// the characters that the application gives its backups.
pub fn is_backup_name(name: &str) -> bool {
    let fits = |letter: char| letter.is_ascii_alphanumeric() || " ._-[]".contains(letter);
    name.len() <= 120
        && name.to_ascii_lowercase().ends_with(".zip")
        && !name.starts_with('.')
        && name.chars().all(fits)
}

/// The name of a new backup, in the form of the PHP application:
/// `backup[2026-11-20_03-15 - ab][0.1.0].zip`. `minute` is the time as
/// `2026-11-20_03-15`, and `mark` is two characters that tell backups of one minute
/// apart.
pub fn name_of(minute: &str, mark: &str, version: &str) -> String {
    format!("backup[{minute} - {mark}][{version}].zip")
}

/// The backups of a folder, the newest first. A folder that does not exist has none.
pub fn list(folder: &Path) -> io::Result<Vec<BackupFile>> {
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let data = entry.metadata()?;
        if data.is_file() && is_backup_name(&name) {
            files.push(BackupFile {
                name,
                size: data.len(),
                modified: data.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            });
        }
    }
    files.sort_by(|a, b| b.modified.cmp(&a.modified).then(b.name.cmp(&a.name)));
    Ok(files)
}

/// A size as the PHP application writes it: `512 B`, `2 kB`, `1.50 MB`. The unit goes
/// up with each three digits of the number of bytes, as in PHP.
#[expect(
    clippy::cast_precision_loss,
    reason = "a size is shown with two decimals at most"
)]
pub fn human_size(bytes: u64, decimals: usize) -> String {
    const UNITS: [&str; 7] = ["B", "kB", "MB", "GB", "TB", "PB", "EB"];
    let digits = bytes.to_string().len();
    let factor = ((digits - 1) / 3).min(UNITS.len() - 1);
    let mut value = bytes as f64;
    for _ in 0..factor {
        value /= 1024.0;
    }
    format!(
        "{value:.decimals$} {}",
        UNITS.get(factor).copied().unwrap_or("B")
    )
}

/// What an archive is made from.
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    /// The data folder.
    pub data_dir: &'a Path,
    /// A copy of the database that nothing writes to, when the install has one.
    pub database_copy: Option<&'a Path>,
    /// Folders that are left out, next to the cache and the backups.
    pub left_out: &'a [PathBuf],
    /// The file of the master key that the server uses. It never goes into an archive,
    /// wherever in the data folder it is (D-147).
    pub key_file: Option<&'a Path>,
    /// The live database file of the install, when it keeps one. It and the files that
    /// `SQLite` keeps next to it never go into an archive, wherever in the data folder
    /// they are: the archive holds the copy instead.
    pub database_file: Option<&'a Path>,
}

fn other(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}

/// Whether a file or a folder of the data folder stays out of an archive. `relative`
/// is its path below the data folder, with `/` between the parts.
fn stays_out(relative: &str) -> bool {
    let first = relative.split('/').next().unwrap_or_default();
    let last = relative.rsplit('/').next().unwrap_or_default();
    first == "cache"
        || relative.split('/').any(|part| part == FOLDER)
        // The key, and the database with the files that SQLite keeps next to it.
        || relative == KEY_FILE
        || first.starts_with(DATABASE_FILE)
        || last.starts_with(".snapshot-")
}

/// A path in one spelling for each file or folder, to compare two paths: with the links,
/// `.` and `..` resolved, and in lower case on Windows, which does not tell the case of
/// names apart. A file that does not exist is named in the resolved folder it would be
/// in. `None` when its folder does not exist either.
fn path_key(path: &Path) -> Option<String> {
    let resolved = path.canonicalize().ok().or_else(|| {
        let folder = path.parent()?.canonicalize().ok()?;
        Some(folder.join(path.file_name()?))
    })?;
    let text = resolved.to_string_lossy().into_owned();
    Some(if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    })
}

/// The files of a folder and of the folders in it, sorted. Links are not followed. A
/// folder whose [`path_key`] is in `left_out` is left out.
fn files_below(folder: &Path, left_out: &[String], found: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(folder)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            if !path_key(&path).is_some_and(|key| left_out.contains(&key)) {
                files_below(&path, left_out, found)?;
            }
        } else if kind.is_file() {
            found.push(path);
        }
    }
    Ok(())
}

/// The size from which an entry is written as a ZIP64 entry. ZIP64 is needed from 4 GiB;
/// the margin leaves room for a file that grows while it is copied. Smaller entries stay
/// plain, so that simple unzip tools list their real sizes.
const LARGE_ENTRY: u64 = 2 << 30;

/// The options of one entry: compressed, with the time of its last change, and ZIP64 only
/// for a large file.
fn entry_options(size: u64, changed: SystemTime) -> SimpleFileOptions {
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .large_file(size >= LARGE_ENTRY);
    match zip_time(changed) {
        Some(time) => options.last_modified_time(time),
        None => options,
    }
}

/// A time as a ZIP file writes it: the date and time in UTC, from 1980 to 2107.
fn zip_time(time: SystemTime) -> Option<zip::DateTime> {
    let moment = jiff::Timestamp::try_from(time)
        .ok()?
        .to_zoned_datetime(jiff::tz::TimeZone::UTC)
        .datetime();
    zip::DateTime::from_date_and_time(
        u16::try_from(moment.year()).ok()?,
        u8::try_from(moment.month()).ok()?,
        u8::try_from(moment.day()).ok()?,
        u8::try_from(moment.hour()).ok()?,
        u8::try_from(moment.minute()).ok()?,
        u8::try_from(moment.second()).ok()?,
    )
    .ok()
}

/// Writes an archive to `target`. Returns the number of files of the data folder in it.
///
/// This blocks while it reads and writes files, so a server calls it on a thread for
/// blocking work.
pub fn write_archive(target: &Path, source: Source<'_>, manifest: &Manifest) -> io::Result<usize> {
    let mut archive = ZipWriter::new(File::create(target)?);
    archive
        .start_file(MANIFEST_FILE, entry_options(0, SystemTime::now()))
        .map_err(other)?;
    archive.write_all(&serde_json::to_vec_pretty(manifest).map_err(other)?)?;
    if let Some(copy) = source.database_copy {
        let mut file = File::open(copy)?;
        let metadata = file.metadata()?;
        archive
            .start_file(
                DATABASE_FILE,
                entry_options(metadata.len(), SystemTime::now()),
            )
            .map_err(other)?;
        io::copy(&mut file, &mut archive)?;
    }

    let mut files = Vec::new();
    if source.data_dir.is_dir() {
        let left_out: Vec<String> = source
            .left_out
            .iter()
            .filter_map(|folder| path_key(folder))
            .collect();
        files_below(source.data_dir, &left_out, &mut files)?;
    }
    // The key file, and the live database with the files that SQLite keeps next to it.
    let mut never: Vec<String> = source.key_file.and_then(path_key).into_iter().collect();
    if let Some(live) = source.database_file.and_then(path_key) {
        for suffix in ["-wal", "-shm", "-journal"] {
            never.push(format!("{live}{suffix}"));
        }
        never.push(live);
    }
    let mut count = 0;
    for file in files {
        if !never.is_empty() && path_key(&file).is_some_and(|key| never.contains(&key)) {
            continue;
        }
        let Ok(below) = file.strip_prefix(source.data_dir) else {
            continue;
        };
        let relative = below
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        // The two names at the top of an archive belong to the archive itself.
        if stays_out(&relative) || relative == MANIFEST_FILE {
            continue;
        }
        let mut opened = File::open(&file)?;
        let metadata = opened.metadata()?;
        let changed = metadata.modified().unwrap_or_else(|_| SystemTime::now());
        archive
            .start_file(relative, entry_options(metadata.len(), changed))
            .map_err(other)?;
        io::copy(&mut opened, &mut archive)?;
        count += 1;
    }
    archive.finish().map_err(other)?.sync_all()?;
    Ok(count)
}

/// Why an archive cannot be restored.
#[derive(Debug, thiserror::Error)]
pub enum RestoreError {
    /// A file cannot be read or written.
    #[error("cannot restore the backup: {0}")]
    Io(#[from] io::Error),
    /// The file is no backup of this application.
    #[error("the file is no backup of Organizr: {0}")]
    NotABackup(String),
    /// A newer version of the application made the archive.
    #[error(
        "the backup has the layout {0}, and this version of Organizr reads the layout {FORMAT}; \
         use a newer version of Organizr"
    )]
    NewerFormat(u32),
    /// The data folder holds a database, and nobody said that it can go.
    #[error(
        "{0} exists already; a restore replaces it, so stop the server and run the command \
         again with the option --overwrite"
    )]
    WouldReplaceDatabase(String),
    /// A part of the archive is larger than a backup of this application can be.
    #[error("the backup is not restored: {0}")]
    NotSafe(String),
}

/// The largest size of the manifest of an archive. The manifest of the application is a
/// few hundred bytes.
const MANIFEST_LIMIT: u64 = 1 << 20;

/// The largest sizes of the entries of an archive that a restore writes. An archive with
/// a larger entry is refused, so that a damaged or crafted archive cannot fill the disk.
#[derive(Debug, Clone, Copy)]
struct Limits {
    /// The largest database.
    database: u64,
    /// The largest other file.
    file: u64,
}

/// The limits of a restore: far above what an install holds.
const LIMITS: Limits = Limits {
    database: 64 << 30,
    file: 4 << 30,
};

/// Writes an entry to `target`, and refuses an entry that holds more than its size says.
/// The entry goes to a file next to the target first, so a refused entry leaves the
/// target as it was.
fn write_entry(
    entry: &mut impl Read,
    size: u64,
    name: &str,
    target: &Path,
) -> Result<(), RestoreError> {
    let mut partial = target.as_os_str().to_owned();
    partial.push(".restoring");
    let partial = PathBuf::from(partial);
    let written = File::create(&partial)
        .and_then(|mut file| io::copy(&mut entry.take(size.saturating_add(1)), &mut file));
    let result = match written {
        Ok(written) if written > size => Err(RestoreError::NotSafe(format!(
            "{name} holds more than the size that the archive gives for it"
        ))),
        Ok(_) => std::fs::rename(&partial, target).map_err(RestoreError::from),
        // The zip reader also finds an entry that is larger than it says, or damaged.
        Err(error) if error.kind() == io::ErrorKind::InvalidData => Err(RestoreError::NotSafe(
            format!("{name} cannot be read: {error}"),
        )),
        Err(error) => Err(error.into()),
    };
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result
}

/// How an archive is restored.
#[derive(Debug, Clone, Copy)]
pub struct RestoreOptions<'a> {
    /// The data folder that gets the files.
    pub data_dir: &'a Path,
    /// The file that gets the database, or nothing when the install does not keep its
    /// database in a file.
    pub database_file: Option<&'a Path>,
    /// Whether a database that exists can be replaced.
    pub overwrite: bool,
}

/// What a restore did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored {
    /// What the archive says about itself.
    pub manifest: Manifest,
    /// The number of files that the data folder got.
    pub files: usize,
    /// Whether the database was restored.
    pub database: bool,
}

fn not_a_backup(error: impl std::fmt::Display) -> RestoreError {
    RestoreError::NotABackup(error.to_string())
}

/// Restores an archive: the files go to the data folder, and the database copy goes to
/// the database file. A file that exists is replaced. The key for the stored secrets
/// is never touched.
///
/// The server must not run while a database is restored.
pub fn restore(archive: &Path, options: RestoreOptions<'_>) -> Result<Restored, RestoreError> {
    restore_with(archive, options, LIMITS)
}

fn restore_with(
    archive: &Path,
    options: RestoreOptions<'_>,
    limits: Limits,
) -> Result<Restored, RestoreError> {
    let mut archive = ZipArchive::new(File::open(archive)?).map_err(not_a_backup)?;
    let manifest: Manifest = {
        let mut file = archive
            .by_name(MANIFEST_FILE)
            .map_err(|_| not_a_backup(format!("it holds no file {MANIFEST_FILE}")))?;
        let too_large =
            || RestoreError::NotSafe(format!("{MANIFEST_FILE} is larger than a manifest can be"));
        if file.size() > MANIFEST_LIMIT {
            return Err(too_large());
        }
        let mut text = String::new();
        (&mut file)
            .take(MANIFEST_LIMIT + 1)
            .read_to_string(&mut text)?;
        if text.len() as u64 > MANIFEST_LIMIT {
            return Err(too_large());
        }
        serde_json::from_str(&text).map_err(not_a_backup)?
    };
    if manifest.format > FORMAT {
        return Err(RestoreError::NewerFormat(manifest.format));
    }
    let database_file = options.database_file.filter(|_| manifest.has_database);
    if let Some(file) = database_file
        && file.exists()
        && !options.overwrite
    {
        return Err(RestoreError::WouldReplaceDatabase(
            file.display().to_string(),
        ));
    }

    // Every size is checked before anything is written.
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(not_a_backup)?;
        let limit = if entry.name().ok().as_deref() == Some(DATABASE_FILE) {
            limits.database
        } else {
            limits.file
        };
        if entry.size() > limit {
            return Err(RestoreError::NotSafe(format!(
                "{} is larger than a file of a backup can be",
                entry.name().unwrap_or_default()
            )));
        }
    }

    let mut restored = Restored {
        manifest,
        files: 0,
        database: false,
    };
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(not_a_backup)?;
        // A name that leads out of the folder is not followed.
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let name = relative.to_string_lossy().replace('\\', "/");
        if entry.is_dir() || name == MANIFEST_FILE || name == KEY_FILE {
            continue;
        }
        let target = if name == DATABASE_FILE {
            let Some(file) = database_file else {
                continue;
            };
            // What SQLite kept next to the old database does not fit the new one.
            for suffix in ["-wal", "-shm"] {
                let mut beside = file.as_os_str().to_owned();
                beside.push(suffix);
                match std::fs::remove_file(PathBuf::from(beside)) {
                    Err(error) if error.kind() != io::ErrorKind::NotFound => {
                        return Err(error.into());
                    }
                    _ => {}
                }
            }
            restored.database = true;
            file.to_owned()
        } else if stays_out(&name) {
            continue;
        } else {
            restored.files += 1;
            options.data_dir.join(&relative)
        };
        if let Some(folder) = target.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let size = entry.size();
        write_entry(&mut entry, size, &name, &target)?;
    }
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().expect("the file is in a folder"))
            .expect("the folder can be made");
        std::fs::write(path, text).expect("the file can be written");
    }

    fn read(path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    /// A data folder with files of each kind, and an archive of it.
    fn archive_of_a_data_folder() -> (tempfile::TempDir, PathBuf, usize) {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let data = folder.path().join("data");
        write(&data.join("userTabs/logo.png"), "a picture");
        write(&data.join("favicon/icon.ico"), "an icon");
        write(&data.join("cache/tv-1.jpg"), "a cached picture");
        write(&data.join("backups/old.zip"), "an old backup");
        write(&data.join("secret.key"), "the key");
        write(&data.join("keys/organizr.key"), "the key, moved");
        write(&data.join("organizr.db"), "the live database");
        write(
            &data.join("organizr.db-wal"),
            "the log of the live database",
        );
        let copy = folder.path().join("copy.db");
        write(&copy, "the database copy");
        let other_folder = data.join("elsewhere");
        write(&other_folder.join("more.zip"), "another backup");

        let target = folder.path().join("backup.zip");
        let source = Source {
            data_dir: &data,
            database_copy: Some(&copy),
            left_out: std::slice::from_ref(&other_folder),
            key_file: Some(&data.join("keys/./organizr.key")),
            database_file: None,
        };
        let manifest = Manifest::new("1.2.3", "2026-11-20T03:15:00Z".to_owned(), true, true);
        let count = write_archive(&target, source, &manifest).expect("the archive is written");
        (folder, target, count)
    }

    #[test]
    fn small_entries_are_plain_zip_entries_with_their_dates() {
        let (_folder, target, _) = archive_of_a_data_folder();

        // The first local header holds real sizes, not the 0xFFFFFFFF of ZIP64, so a
        // simple unzip tool lists them as they are.
        let bytes = std::fs::read(&target).expect("the archive can be read");
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        let size_at =
            |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"));
        assert_ne!(size_at(18), u32::MAX);
        assert_ne!(size_at(22), u32::MAX);

        let mut archive = ZipArchive::new(File::open(&target).expect("the archive opens"))
            .expect("the archive is a zip file");
        for index in 0..archive.len() {
            let entry = archive.by_index(index).expect("the entry can be read");
            let date = entry.last_modified().expect("the entry has a date");
            assert!(date.year() >= 2026, "{:?}: {date:?}", entry.name());
        }
    }

    #[test]
    fn archive_holds_the_files_and_the_database_copy_and_never_the_key() {
        let (folder, target, count) = archive_of_a_data_folder();
        assert_eq!(count, 2);

        let new = folder.path().join("new");
        let database = new.join("organizr.db");
        let options = RestoreOptions {
            data_dir: &new,
            database_file: Some(&database),
            overwrite: false,
        };
        let restored = restore(&target, options).expect("the archive is restored");
        assert_eq!((restored.files, restored.database), (2, true));
        assert_eq!(restored.manifest.version, "1.2.3");
        assert_eq!(
            read(&new.join("userTabs/logo.png")).as_deref(),
            Some("a picture")
        );
        assert_eq!(
            read(&new.join("favicon/icon.ico")).as_deref(),
            Some("an icon")
        );
        assert_eq!(read(&database).as_deref(), Some("the database copy"));
        for left_out in [
            "secret.key",
            // The key file that the server uses, wherever it is (D-147).
            "keys/organizr.key",
            "cache/tv-1.jpg",
            "backups/old.zip",
            "elsewhere/more.zip",
            "organizr.db-wal",
            "backup.json",
        ] {
            assert!(!new.join(left_out).exists(), "{left_out}");
        }
    }

    /// The names of the entries of an archive.
    fn entries_of(target: &Path) -> Vec<String> {
        let mut archive = ZipArchive::new(File::open(target).expect("the archive opens"))
            .expect("the archive is a zip file");
        (0..archive.len())
            .map(|index| {
                let entry = archive.by_index(index).expect("the entry can be read");
                entry.name().expect("the entry has a name").into_owned()
            })
            .collect()
    }

    #[test]
    fn folder_left_out_is_found_also_when_its_path_is_written_another_way() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let data = folder.path().join("data");
        write(&data.join("userTabs/logo.png"), "a picture");
        write(&data.join("saved/old.zip"), "an old backup");
        write(&data.join("Kept/old.zip"), "another old backup");
        let mut left_out = vec![
            // A way back up, and a slash at the end.
            data.join("userTabs").join("..").join("saved").join(""),
        ];
        if cfg!(windows) {
            // Windows does not tell the case of names apart.
            left_out.push(data.join("KEPT"));
        } else {
            left_out.push(data.join("Kept"));
        }
        let target = folder.path().join("backup.zip");
        let source = Source {
            data_dir: &data.join("."),
            database_copy: None,
            left_out: &left_out,
            key_file: None,
            database_file: None,
        };
        let manifest = Manifest::new("1.2.3", String::new(), true, false);
        let count = write_archive(&target, source, &manifest).expect("the archive is written");
        assert_eq!(count, 1);
        assert_eq!(entries_of(&target), [MANIFEST_FILE, "userTabs/logo.png"]);
    }

    #[test]
    fn live_database_of_the_url_stays_out_wherever_it_is() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let data = folder.path().join("data");
        write(&data.join("userTabs/logo.png"), "a picture");
        for suffix in ["", "-wal", "-shm", "-journal"] {
            write(
                &data.join(format!("db/live.db{suffix}")),
                "the live database",
            );
        }
        write(&data.join("db/live.db.txt"), "notes");
        let target = folder.path().join("backup.zip");
        let source = Source {
            data_dir: &data,
            database_copy: None,
            left_out: &[],
            key_file: None,
            database_file: Some(&data.join("userTabs/../db/live.db")),
        };
        let manifest = Manifest::new("1.2.3", String::new(), true, false);
        write_archive(&target, source, &manifest).expect("the archive is written");
        assert_eq!(
            entries_of(&target),
            [MANIFEST_FILE, "db/live.db.txt", "userTabs/logo.png"]
        );
    }

    /// An archive with a manifest and the entries `files`, stored without compression.
    fn archive_with(target: &Path, files: &[(&str, &[u8])]) {
        let mut archive = ZipWriter::new(File::create(target).expect("the file can be made"));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        archive
            .start_file(MANIFEST_FILE, options)
            .expect("the entry starts");
        let manifest = Manifest::new("1.0.0", String::new(), true, true);
        archive
            .write_all(&serde_json::to_vec(&manifest).expect("the manifest is JSON"))
            .expect("the entry is written");
        for (name, bytes) in files {
            archive
                .start_file(*name, options)
                .expect("the entry starts");
            archive.write_all(bytes).expect("the entry is written");
        }
        archive.finish().expect("the archive is complete");
    }

    #[test]
    fn manifest_that_is_too_large_is_refused() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let file = folder.path().join("large.zip");
        let mut archive = ZipWriter::new(File::create(&file).expect("the file can be made"));
        archive
            .start_file(MANIFEST_FILE, SimpleFileOptions::default())
            .expect("the entry starts");
        let padding = " ".repeat(2 << 20);
        let text = format!(
            "{{\"format\":1,\"version\":\"1\",\"created\":\"\",\"engine\":\"sqlite\",\
             \"has_database\":false{padding}}}"
        );
        archive.write_all(text.as_bytes()).expect("it is written");
        archive.finish().expect("the archive is complete");
        let options = RestoreOptions {
            data_dir: &folder.path().join("new"),
            database_file: None,
            overwrite: true,
        };
        let refused = restore(&file, options);
        assert!(
            matches!(refused, Err(RestoreError::NotSafe(_))),
            "{refused:?}"
        );
    }

    #[test]
    fn entry_that_is_larger_than_the_limit_is_refused_before_anything_is_written() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let file = folder.path().join("large.zip");
        archive_with(
            &file,
            &[
                ("userTabs/a.png", b"small"),
                ("userTabs/b.png", b"too large"),
            ],
        );
        let new = folder.path().join("new");
        let options = RestoreOptions {
            data_dir: &new,
            database_file: None,
            overwrite: true,
        };
        let limits = Limits {
            database: 100,
            file: 8,
        };
        let refused = restore_with(&file, options, limits);
        assert!(
            matches!(refused, Err(RestoreError::NotSafe(_))),
            "{refused:?}"
        );
        assert!(!new.join("userTabs").exists());
    }

    #[test]
    fn entry_that_holds_more_than_its_size_says_is_refused() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let file = folder.path().join("lying.zip");
        archive_with(&file, &[("userTabs/a.png", b"0123456789")]);
        // Both headers of the entry say that it holds 2 bytes.
        let mut bytes = std::fs::read(&file).expect("the archive can be read");
        let name = b"userTabs/a.png";
        let mut patched = 0;
        for at in 0..bytes.len() - 4 {
            let header = &bytes[at..at + 4];
            let size_at = if header == b"PK\x03\x04" {
                at + 22
            } else if header == b"PK\x01\x02" {
                at + 24
            } else {
                continue;
            };
            let name_at = if header == b"PK\x03\x04" {
                at + 30
            } else {
                at + 46
            };
            if bytes.get(name_at..name_at + name.len()) == Some(name.as_slice()) {
                bytes[size_at..size_at + 4].copy_from_slice(&2_u32.to_le_bytes());
                patched += 1;
            }
        }
        assert_eq!(patched, 2);
        std::fs::write(&file, bytes).expect("the archive can be written");

        let new = folder.path().join("new");
        let options = RestoreOptions {
            data_dir: &new,
            database_file: None,
            overwrite: true,
        };
        let refused = restore(&file, options);
        assert!(
            matches!(refused, Err(RestoreError::NotSafe(_))),
            "{refused:?}"
        );
        assert!(!new.join("userTabs/a.png").exists());
    }

    #[test]
    fn restore_replaces_a_database_only_when_it_is_told_to() {
        let (folder, target, _) = archive_of_a_data_folder();
        let new = folder.path().join("new");
        let database = new.join("organizr.db");
        write(&database, "a database in use");
        write(&new.join("organizr.db-wal"), "its log");
        write(&new.join("secret.key"), "the key of this install");
        let options = |overwrite| RestoreOptions {
            data_dir: &new,
            database_file: Some(&database),
            overwrite,
        };

        let refused = restore(&target, options(false));
        assert!(
            matches!(refused, Err(RestoreError::WouldReplaceDatabase(_))),
            "{refused:?}"
        );
        assert_eq!(read(&database).as_deref(), Some("a database in use"));
        assert!(!new.join("userTabs").exists());

        restore(&target, options(true)).expect("the archive is restored");
        assert_eq!(read(&database).as_deref(), Some("the database copy"));
        assert!(!new.join("organizr.db-wal").exists());
        assert_eq!(
            read(&new.join("secret.key")).as_deref(),
            Some("the key of this install")
        );

        // An install on PostgreSQL gets the files, and no database file.
        let files_only = folder.path().join("files-only");
        let options = RestoreOptions {
            data_dir: &files_only,
            database_file: None,
            overwrite: false,
        };
        let restored = restore(&target, options).expect("the archive is restored");
        assert_eq!((restored.files, restored.database), (2, false));
        assert!(!files_only.join("organizr.db").exists());
    }

    #[test]
    fn file_that_is_no_backup_is_refused() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let options = RestoreOptions {
            data_dir: folder.path(),
            database_file: None,
            overwrite: true,
        };
        let text = folder.path().join("text.zip");
        write(&text, "no zip file");
        assert!(matches!(
            restore(&text, options),
            Err(RestoreError::NotABackup(_))
        ));

        // A zip file without the manifest, with a name that leads out of the folder.
        let plain = folder.path().join("plain.zip");
        let mut archive = ZipWriter::new(File::create(&plain).expect("the file can be made"));
        archive
            .start_file("../outside.txt", SimpleFileOptions::default())
            .expect("the entry starts");
        archive.write_all(b"text").expect("the entry is written");
        archive.finish().expect("the archive is complete");
        assert!(matches!(
            restore(&plain, options),
            Err(RestoreError::NotABackup(_))
        ));

        // An archive of a newer layout.
        let newer = folder.path().join("newer.zip");
        let mut archive = ZipWriter::new(File::create(&newer).expect("the file can be made"));
        archive
            .start_file(MANIFEST_FILE, SimpleFileOptions::default())
            .expect("the entry starts");
        let mut manifest = Manifest::new("9.0.0", String::new(), true, false);
        manifest.format = FORMAT + 1;
        archive
            .write_all(&serde_json::to_vec(&manifest).expect("the manifest is JSON"))
            .expect("the entry is written");
        archive.finish().expect("the archive is complete");
        assert!(matches!(
            restore(&newer, options),
            Err(RestoreError::NewerFormat(2))
        ));
    }

    #[test]
    fn entry_that_leads_out_of_the_folder_is_not_written() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let inside = folder.path().join("inside");
        let file = folder.path().join("evil.zip");
        let mut archive = ZipWriter::new(File::create(&file).expect("the file can be made"));
        let options = SimpleFileOptions::default();
        archive
            .start_file(MANIFEST_FILE, options)
            .expect("the entry starts");
        let manifest = Manifest::new("1.0.0", String::new(), true, false);
        archive
            .write_all(&serde_json::to_vec(&manifest).expect("the manifest is JSON"))
            .expect("the entry is written");
        for name in ["../outside.txt", "userTabs/fine.txt", "secret.key"] {
            archive.start_file(name, options).expect("the entry starts");
            archive.write_all(b"text").expect("the entry is written");
        }
        archive.finish().expect("the archive is complete");

        let options = RestoreOptions {
            data_dir: &inside,
            database_file: None,
            overwrite: true,
        };
        let restored = restore(&file, options).expect("the archive is restored");
        assert_eq!(restored.files, 1);
        assert!(inside.join("userTabs/fine.txt").exists());
        assert!(!folder.path().join("outside.txt").exists());
        assert!(!inside.join("secret.key").exists());
    }

    #[test]
    fn names_and_sizes_are_written_as_php_writes_them() {
        assert_eq!(
            name_of("2026-11-20_03-15", "ab", "0.1.0"),
            "backup[2026-11-20_03-15 - ab][0.1.0].zip"
        );
        assert!(is_backup_name("backup[2026-11-20_03-15 - ab][0.1.0].zip"));
        assert!(is_backup_name("OLD.ZIP"));
        for name in [
            "",
            "a.txt",
            "../a.zip",
            "a/b.zip",
            "a\\b.zip",
            ".hidden.zip",
            "a\u{e4}.zip",
        ] {
            assert!(!is_backup_name(name), "{name}");
        }
        assert!(!is_backup_name(&format!("{}.zip", "a".repeat(120))));

        assert_eq!(human_size(0, 0), "0 B");
        assert_eq!(human_size(512, 0), "512 B");
        // The unit goes up with the number of digits, not at 1024.
        assert_eq!(human_size(1000, 0), "1 kB");
        assert_eq!(human_size(1_572_864, 2), "1.50 MB");
        assert_eq!(human_size(5 * 1024 * 1024 * 1024, 2), "5.00 GB");
    }

    #[test]
    fn backups_are_listed_the_newest_first() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        assert_eq!(list(&folder.path().join("missing")).expect("it works"), []);
        for (name, age) in [("b.zip", 20), ("a.zip", 10), ("c.zip", 30)] {
            let path = folder.path().join(name);
            write(&path, name);
            let time = SystemTime::now() - std::time::Duration::from_secs(age);
            File::options()
                .write(true)
                .open(&path)
                .and_then(|file| file.set_modified(time))
                .expect("the time can be set");
        }
        write(&folder.path().join("notes.txt"), "no backup");
        std::fs::create_dir(folder.path().join("folder.zip")).expect("the folder can be made");

        let names: Vec<String> = list(folder.path())
            .expect("the folder can be read")
            .into_iter()
            .map(|file| file.name)
            .collect();
        assert_eq!(names, ["a.zip", "b.zip", "c.zip"]);
    }
}
