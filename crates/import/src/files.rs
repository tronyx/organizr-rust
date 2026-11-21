//! The files of a PHP install: which ones an import copies, and where to.
//!
//! - The pictures that people uploaded (`data/userTabs`) go to the folder for uploaded
//!   pictures of the new install.
//! - People also put pictures of their own into the picture folders of the PHP
//!   application (`plugins/images/...`). The new application ships only the pictures of
//!   Organizr itself. So each other picture that a record uses, and each other picture
//!   for tabs, is copied to the uploaded pictures, and the records point there.
//! - The custom favicon files (`data/favicon`) are copied as they are.
//! - The custom certificate of an authority (`data/cert/custom.pem`) is copied to
//!   `cert/custom.pem`, where the server reads it, unless it holds a private key.
//! - PHP files that a person added (custom pages, routes and plugins) are listed in the
//!   report. They are never copied and never run.

use std::collections::{BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

use organizr_db::ImportData;
use serde_json::Value;

use crate::report::Report;

/// The pictures that come with the application, by the path that PHP uses, one per
/// line. `web/tools/import-organizr-assets.py` writes the file.
const BUNDLED: &str = include_str!("bundled_images.txt");

/// Where the pictures of the PHP application are, from its folder.
const PICTURES: &str = "plugins/images/";

/// The pictures that the picture chooser of the PHP application offers for a tab.
const TAB_PICTURES: &str = "plugins/images/tabs";

/// The folder of the uploaded pictures, in the PHP install and in the data folder.
const UPLOADED: &str = "userTabs";

/// The folder of the custom favicon files, in the PHP install and in the data folder.
const FAVICON: &str = "favicon";

/// The folder of the custom certificate, in the PHP install and in the data folder.
const CERTIFICATE_FOLDER: &str = "cert";

/// The name of the custom certificate in its folder.
const CERTIFICATE_FILE: &str = "custom.pem";

/// The folders of `data` that an import reads or leaves out on purpose.
const KNOWN_DATA_FOLDERS: [&str; 8] = [
    "cache",
    CERTIFICATE_FOLDER,
    "config",
    FAVICON,
    "pages",
    "plugins",
    "routes",
    UPLOADED,
];

/// One file to copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCopy {
    /// The file of the PHP install.
    pub from: PathBuf,
    /// Where it goes, from the data folder of the new install.
    pub to: PathBuf,
}

/// Whether the application can show a file as a picture: by the ending of its name.
fn is_picture(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ending)| {
        matches!(
            ending.to_ascii_lowercase().as_str(),
            "gif" | "png" | "jpg" | "jpeg" | "svg"
        )
    })
}

/// The regular files of a folder, sorted by name. A folder that is missing has none.
fn files_in(folder: &Path) -> Vec<(String, PathBuf)> {
    let mut files: Vec<(String, PathBuf)> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| Some((entry.file_name().to_str()?.to_owned(), entry.path())))
        .collect();
    files.sort();
    files
}

/// Moves pictures of the PHP install to the uploaded pictures.
struct Relocator<'a> {
    web_root: &'a Path,
    /// The names in the folder of the uploaded pictures, in lower case.
    taken: BTreeSet<String>,
    /// The new path of each picture that was moved, by its old path.
    moved: HashMap<String, String>,
    copies: Vec<FileCopy>,
    /// Pictures that a record names and the PHP install does not have.
    missing: BTreeSet<String>,
    /// Pictures of a kind that the application cannot show.
    not_shown: BTreeSet<String>,
}

impl Relocator<'_> {
    /// The path for a record that named `path`: the path in the uploaded pictures when
    /// the picture had to move, else `None`.
    fn relocate(&mut self, path: &str) -> Option<String> {
        if !path.starts_with(PICTURES) || BUNDLED.lines().any(|bundled| bundled == path) {
            return None;
        }
        if let Some(new) = self.moved.get(path) {
            return Some(new.clone());
        }
        // The path comes from a record. It must stay inside the install.
        let relative = Path::new(path);
        if !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        {
            return None;
        }
        let name = relative.file_name()?.to_str()?;
        let from = self.web_root.join(relative);
        if !from.is_file() {
            self.missing.insert(path.to_owned());
            return None;
        }
        if !is_picture(name) {
            self.not_shown.insert(path.to_owned());
            return None;
        }

        let folder = relative
            .parent()
            .and_then(Path::file_name)
            .and_then(|folder| folder.to_str())
            .unwrap_or("picture");
        let mut candidate = name.to_owned();
        let mut number = 1_u32;
        while !self.taken.insert(candidate.to_lowercase()) {
            candidate = if number == 1 {
                format!("{folder}-{name}")
            } else {
                format!("{folder}-{number}-{name}")
            };
            number = number.saturating_add(1);
        }
        let new = format!("data/{UPLOADED}/{candidate}");
        self.copies.push(FileCopy {
            from,
            to: Path::new(UPLOADED).join(&candidate),
        });
        self.moved.insert(path.to_owned(), new.clone());
        Some(new)
    }

    fn relocate_field(&mut self, field: &mut Option<String>) {
        if let Some(new) = field.as_deref().and_then(|path| self.relocate(path)) {
            *field = Some(new);
        }
    }
}

/// Decides which files to copy, and points the records at the pictures that move.
/// `database_file` is the database of the PHP install, which is not a file to copy.
pub fn plan_files(
    web_root: &Path,
    database_file: &Path,
    data: &mut ImportData,
    report: &mut Report,
) -> Vec<FileCopy> {
    let mut copies = Vec::new();
    let mut taken = BTreeSet::new();
    for (name, from) in files_in(&web_root.join("data").join(UPLOADED)) {
        taken.insert(name.to_lowercase());
        copies.push(FileCopy {
            from,
            to: Path::new(UPLOADED).join(name),
        });
    }
    let uploaded = copies.len();

    let mut relocator = Relocator {
        web_root,
        taken,
        moved: HashMap::new(),
        copies: Vec::new(),
        missing: BTreeSet::new(),
        not_shown: BTreeSet::new(),
    };
    for group in &mut data.groups {
        relocator.relocate_field(&mut group.image);
    }
    for user in &mut data.users {
        relocator.relocate_field(&mut user.user.image);
    }
    for category in &mut data.categories {
        relocator.relocate_field(&mut category.image);
    }
    for tab in &mut data.tabs {
        relocator.relocate_field(&mut tab.image);
    }
    // A setting whose whole value is the path of a picture, for example a logo.
    for setting in &mut data.settings {
        if let Ok(Value::String(path)) = serde_json::from_str::<Value>(&setting.value)
            && let Some(new) = relocator.relocate(&path)
        {
            setting.value = Value::String(new).to_string();
        }
    }
    let used = relocator.copies.len();
    // The other pictures for tabs, so that the picture chooser still offers them.
    for (name, _) in files_in(&web_root.join(TAB_PICTURES)) {
        if is_picture(&name) {
            relocator.relocate(&format!("{TAB_PICTURES}/{name}"));
        }
    }

    let moved = relocator.copies.len();
    if moved > 0 {
        report.notes.push(format!(
            "{moved} pictures that a person added to the picture folders of the PHP application \
             are now with the uploaded pictures. {used} of them are used by a record, which \
             points to the new place."
        ));
    }
    for path in &relocator.missing {
        report.notes.push(format!(
            "A record uses the picture {path}, which the PHP install does not have. \
             Choose another picture for it."
        ));
    }
    for path in &relocator.not_shown {
        report.notes.push(format!(
            "A record uses the picture {path}. The application shows only GIF, PNG, JPEG \
             and SVG pictures that were added by a person. Choose another picture for it."
        ));
    }
    copies.append(&mut relocator.copies);
    report.pictures = uploaded + moved;

    let favicons = files_in(&web_root.join("data").join(FAVICON));
    report.other_files = favicons.len();
    copies.extend(favicons.into_iter().map(|(name, from)| FileCopy {
        from,
        to: Path::new(FAVICON).join(name),
    }));

    copies.extend(custom_certificate(web_root, report));

    for folder in ["pages", "plugins", "routes"] {
        php_files_in(
            web_root,
            &Path::new("data").join(folder),
            &mut report.php_files,
        );
    }
    report.php_files.sort();
    other_data_folders(web_root, database_file, report);
    copies
}

/// The copy of the custom certificate of the PHP install, if it has one. The server
/// checks the certificate when it starts, and its log says when it does not use it. A
/// file that holds a private key is not copied, and the report says so.
fn custom_certificate(web_root: &Path, report: &mut Report) -> Option<FileCopy> {
    let from = web_root
        .join("data")
        .join(CERTIFICATE_FOLDER)
        .join(CERTIFICATE_FILE);
    if !from.is_file() {
        return None;
    }
    let name = format!("data/{CERTIFICATE_FOLDER}/{CERTIFICATE_FILE}");
    let Ok(text) = std::fs::read(&from) else {
        report.notes.push(format!(
            "The custom certificate {name} of the PHP install cannot be read, so it is not copied."
        ));
        return None;
    };
    if text
        .windows(b"PRIVATE KEY-----".len())
        .any(|part| part == b"PRIVATE KEY-----")
    {
        report.notes.push(format!(
            "The custom certificate {name} of the PHP install holds a private key, so it is \
             not copied. Upload the certificate of the authority alone in Settings, General."
        ));
        return None;
    }
    report.other_files += 1;
    Some(FileCopy {
        from,
        to: Path::new(CERTIFICATE_FOLDER).join(CERTIFICATE_FILE),
    })
}

/// Adds the PHP files of a folder and of the folders in it, by their path from the
/// folder of the PHP application.
fn php_files_in(web_root: &Path, folder: &Path, found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(web_root.join(folder)) else {
        return;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let inside = folder.join(&name);
        if entry.path().is_dir() {
            php_files_in(web_root, &inside, found);
        } else if name.to_ascii_lowercase().ends_with(".php") {
            found.push(inside.display().to_string().replace('\\', "/"));
        }
    }
}

/// Reports the folders of `data` that an import does not know. The folder of the
/// database is none of them.
fn other_data_folders(web_root: &Path, database_file: &Path, report: &mut Report) {
    let Ok(entries) = std::fs::read_dir(web_root.join("data")) else {
        return;
    };
    let mut others: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir() && !database_file.starts_with(entry.path()))
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter(|name| !KNOWN_DATA_FOLDERS.contains(&name.as_str()))
        .collect();
    others.sort();
    for name in others {
        report.notes.push(format!(
            "The folder data/{name} of the PHP install is not copied. Look at what it holds."
        ));
    }
}

/// Why a file was not copied.
#[derive(Debug, thiserror::Error)]
#[error("cannot copy {from} to {to}: {source}")]
pub struct CopyError {
    from: String,
    to: String,
    source: std::io::Error,
}

/// Copies the files into the data folder of the new install. A file that is there
/// already is written over, so an import that failed later can run again.
pub fn copy_files(copies: &[FileCopy], data_dir: &Path) -> Result<(), CopyError> {
    for copy in copies {
        let to = data_dir.join(&copy.to);
        let failed = |source| CopyError {
            from: copy.from.display().to_string(),
            to: to.display().to_string(),
            source,
        };
        if let Some(folder) = to.parent() {
            std::fs::create_dir_all(folder).map_err(failed)?;
        }
        std::fs::copy(&copy.from, &to).map_err(failed)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{BUNDLED, is_picture};

    fn pictures_below(folder: &Path, from: &Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(folder)
            .expect("the folder can be listed")
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                pictures_below(&path, from, found);
            } else if let Ok(inside) = path.strip_prefix(from) {
                found.push(inside.display().to_string().replace('\\', "/"));
            }
        }
    }

    #[test]
    fn list_of_shipped_pictures_is_the_folder_of_the_web_interface() {
        let public = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/public");
        let mut shipped = Vec::new();
        pictures_below(&public.join("plugins/images"), &public, &mut shipped);
        shipped.sort();

        let listed: Vec<&str> = BUNDLED.lines().collect();

        assert_eq!(
            listed, shipped,
            "run web/tools/import-organizr-assets.py, or correct crates/import/src/bundled_images.txt"
        );
        assert!(listed.contains(&"plugins/images/tabs/plex.png"));
    }

    #[test]
    fn picture_is_known_by_the_ending_of_its_name() {
        for name in [
            "a.png", "a.PNG", "a.gif", "a.jpg", "a.jpeg", "a.b.JPeG", "a.svg",
        ] {
            assert!(is_picture(name), "{name}");
        }
        for name in ["a.webp", "a.ico", "png", "a.png.txt", "a.", ""] {
            assert!(!is_picture(name), "{name}");
        }
    }
}
