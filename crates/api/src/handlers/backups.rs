//! The backup routes: list, make, download and delete backups.
//!
//! Only an Admin can use them. A backup holds the database with the password hashes
//! of every user, so it is for the person with the most access only.

use std::path::{Path as FilePath, PathBuf};

use axum::Extension;
use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use organizr_db::Engine;
use organizr_domain::GroupLevel;
use organizr_settings::Settings;
use serde::Serialize;
use tokio::io::AsyncReadExt;

use crate::backup::{self, Manifest, Source};
use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The level that the backup routes ask for: Admins and Co-Admins, as in PHP (D-74).
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// The version of the application, for the name of a backup.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The size of the pieces in which a backup is sent.
const PIECE: usize = 64 * 1024;

/// The folder of the backups: the folder of the `backupLocation` setting when it
/// exists, and else the folder `backups` in the data folder.
fn folder_of(settings: &Settings, data_dir: &FilePath) -> PathBuf {
    let chosen = FilePath::new(&settings.backup_location);
    if !settings.backup_location.is_empty() && chosen.is_dir() {
        chosen.to_owned()
    } else {
        data_dir.join(backup::FOLDER)
    }
}

/// The folder of the backups of this install, after the checks of the routes.
fn folder_for(state: &AppState, caller: &Caller) -> Result<PathBuf, Refusal> {
    caller.require(MANAGER)?;
    let data_dir = state
        .data_folder()
        .ok_or_else(|| Refusal::conflict("Backups are not set up"))?;
    Ok(folder_of(&state.settings(), data_dir))
}

fn failed(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Makes a backup, and deletes the oldest backups over the number of the
/// `keepBackupsCountCron` setting. Returns the name of the new backup.
///
/// Two backups are never made at the same time.
pub async fn create_backup(state: &AppState) -> Result<String, String> {
    let data_dir = state
        .data_folder()
        .ok_or("the data folder is not known")?
        .to_owned();
    let _one_at_a_time = state.backup_lock().lock().await;
    let settings = state.settings();
    let folder = folder_of(&settings, &data_dir);
    tokio::fs::create_dir_all(&folder).await.map_err(failed)?;

    let now = jiff::Timestamp::now();
    let minute = now.strftime("%Y-%m-%d_%H-%M").to_string();
    // Two characters tell the backups of one minute apart. A name that is taken is not
    // used again.
    let (id, name, target) = loop {
        let id = uuid::Uuid::new_v4().simple().to_string();
        let mark: String = id.chars().take(2).collect();
        let name = backup::name_of(&minute, &mark, VERSION);
        let target = folder.join(&name);
        if !tokio::fs::try_exists(&target).await.map_err(failed)? {
            break (id, name, target);
        }
    };
    // The parts of a backup have names that no list shows until the backup is complete.
    let snapshot = folder.join(format!(".snapshot-{id}.db"));
    let partial = folder.join(format!(".snapshot-{id}.zip"));

    let database = state.database();
    let has_database = database.snapshot(&snapshot).await.map_err(failed)?;
    let manifest = Manifest::new(
        VERSION,
        now.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
        database.engine() == Engine::Sqlite,
        has_database,
    );
    let written = {
        let (partial, snapshot, folder) = (partial.clone(), snapshot.clone(), folder.clone());
        tokio::task::spawn_blocking(move || {
            let source = Source {
                data_dir: &data_dir,
                database_copy: has_database.then_some(snapshot.as_path()),
                left_out: std::slice::from_ref(&folder),
            };
            backup::write_archive(&partial, source, &manifest)
        })
        .await
        .map_err(failed)
        .and_then(|result| result.map_err(failed))
    };
    if has_database {
        let _ = tokio::fs::remove_file(&snapshot).await;
    }
    let files = match written {
        Ok(files) => files,
        Err(error) => {
            let _ = tokio::fs::remove_file(&partial).await;
            return Err(error);
        }
    };
    tokio::fs::rename(&partial, &target).await.map_err(failed)?;
    tracing::info!(name, files, has_database, "a backup was made");

    // The number 0 keeps every backup.
    let keep = usize::try_from(settings.keep_backups_count_cron).unwrap_or(usize::MAX);
    if keep > 0 {
        let all =
            tokio::task::spawn_blocking(move || backup::list(&folder).map(|all| (all, folder)))
                .await
                .map_err(failed)?
                .map_err(failed)?;
        let (all, folder) = all;
        for old in all.into_iter().skip(keep) {
            tracing::info!(
                name = old.name,
                "an old backup is deleted: it is over the limit"
            );
            let _ = tokio::fs::remove_file(folder.join(&old.name)).await;
        }
    }
    Ok(name)
}

/// A backup in the list.
#[derive(Debug, Serialize)]
struct BackupRow {
    name: String,
    size: String,
    /// When the backup was written, as `2026-11-20T03:15:00Z`.
    date: String,
}

/// The `data` of `GET /backup`, with the field names of the PHP application.
#[derive(Debug, Serialize)]
pub struct BackupList {
    total_size: String,
    total_files: usize,
    /// The backups, the newest first, or `null` when there is none.
    files: Option<Vec<BackupRow>>,
}

/// `GET /backup`: the backups, the newest first.
pub async fn list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<BackupList>, Refusal> {
    let folder = folder_for(&state, &caller)?;
    let all = tokio::task::spawn_blocking(move || backup::list(&folder))
        .await
        .map_err(|error| Refusal::conflict(format!("Backup list failed: {error}")))?
        .map_err(|error| {
            tracing::warn!(%error, "the backups cannot be listed");
            Refusal::conflict("Backup list failed")
        })?;
    let total: u64 = all.iter().map(|file| file.size).sum();
    let rows: Vec<BackupRow> = all
        .into_iter()
        .map(|file| BackupRow {
            size: backup::human_size(file.size, 0),
            date: jiff::Timestamp::try_from(file.modified).map_or_else(
                |_| String::new(),
                |time| time.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
            ),
            name: file.name,
        })
        .collect();
    Ok(Envelope::success(BackupList {
        total_size: backup::human_size(total, 2),
        total_files: rows.len(),
        files: (!rows.is_empty()).then_some(rows),
    }))
}

/// `POST /backup`: makes a backup.
pub async fn create(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<NoData>, Refusal> {
    folder_for(&state, &caller)?;
    create_backup(&state).await.map_err(|error| {
        tracing::error!(error, "a backup could not be made");
        Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Backup could not be created".to_owned(),
        )
    })?;
    Ok(Envelope::success(NoData).with_message("Backup has been created"))
}

/// The last part of a path that a caller sent, for a text of an answer.
fn last_part(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

/// `GET /backup/{filename}`: sends a backup as a file to save.
pub async fn download(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(name): Path<String>,
) -> Result<Response, Refusal> {
    let folder = folder_for(&state, &caller)?;
    let missing = || Refusal::not_found("File does not exist");
    if !backup::is_backup_name(&name) {
        return Err(missing());
    }
    let file = tokio::fs::File::open(folder.join(&name))
        .await
        .map_err(|_| missing())?;
    let length = file.metadata().await.map_err(|_| missing())?.len();
    tracing::info!(name, "a backup is downloaded");

    // The file is sent in pieces, so a large backup does not fill the memory.
    let pieces = futures_util::stream::unfold(Some(file), |file| async move {
        let mut file = file?;
        let mut piece = vec![0_u8; PIECE];
        match file.read(&mut piece).await {
            Ok(0) => None,
            Ok(size) => {
                piece.truncate(size);
                Some((Ok(Bytes::from(piece)), Some(file)))
            }
            Err(error) => Some((Err(error), None)),
        }
    });
    let disposition = format!("attachment; filename=\"{name}\"");
    Ok((
        [
            (header::CONTENT_TYPE, "application/zip".to_owned()),
            (header::CONTENT_DISPOSITION, disposition),
            (header::CONTENT_LENGTH, length.to_string()),
        ],
        Body::from_stream(pieces),
    )
        .into_response())
}

/// `DELETE /backup/{filename}`: deletes a backup.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(name): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    let folder = folder_for(&state, &caller)?;
    if !backup::is_backup_name(&name) {
        return Err(Refusal::conflict(format!(
            "{} is not approved to be deleted",
            last_part(&name)
        )));
    }
    match tokio::fs::remove_file(folder.join(&name)).await {
        Ok(()) => {
            tracing::info!(name, "a backup was deleted");
            Ok(Envelope::success(NoData).with_message(format!("{name} has been deleted")))
        }
        Err(_) => Err(Refusal::not_found("File does not exist")),
    }
}
