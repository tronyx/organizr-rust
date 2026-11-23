//! Extensions: WebAssembly components in the folder `plugins` of the data folder.
//!
//! An extension answers no request until an Admin approves it. The approval is for
//! one file, named by its SHA-256, with the permissions that the Admin saw. When the
//! file changes, the extension stops until an Admin approves it again.

use std::collections::BTreeMap;
use std::path::{Path as FilePath, PathBuf};

use axum::Extension;
use axum::body::Bytes;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use organizr_db::ApprovedExtension;
use organizr_domain::GroupLevel;
use organizr_integrations::probe;
use organizr_plugins::extension::{Call, Caller as ExtensionCaller, ExtensionError, Request};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::handlers::extension_http;
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::html::POLICY;
use crate::state::AppState;

/// The folder of the extensions in the data folder.
const FOLDER: &str = "plugins";

/// The file of an extension that holds the component.
const COMPONENT_FILE: &str = "plugin.wasm";

/// The file of an extension that describes it.
const MANIFEST_FILE: &str = "plugin.json";

/// The level that sees the extensions.
const VIEWER: GroupLevel = GroupLevel::CO_ADMIN;

/// The level that approves, switches off and deletes extensions.
const MANAGER: GroupLevel = GroupLevel::ADMIN;

/// The permission to keep values.
const STORAGE: &str = "storage";

/// Whether this version knows a permission: `storage`, a site of `http:<host>`, or
/// `http-local` (D-122).
fn is_known_permission(permission: &str) -> bool {
    permission == STORAGE || extension_http::is_permission(permission)
}

/// The most bytes of the file of a component.
const COMPONENT_LIMIT: u64 = 32 * 1024 * 1024;

/// The most bytes of the body of a request to an extension.
const BODY_LIMIT: usize = 1024 * 1024;

/// The headers of a request that an extension sees. The cookies and the API key of the
/// caller are never among them.
const REQUEST_HEADERS: [&str; 4] = ["accept", "accept-language", "content-type", "user-agent"];

/// The headers of an answer that an extension can set.
const RESPONSE_HEADERS: [&str; 3] = ["cache-control", "content-disposition", "content-type"];

/// What the file `plugin.json` of an extension says.
#[derive(Debug, Clone, Deserialize)]
struct Manifest {
    name: String,
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: String,
    /// The permissions that the extension asks for.
    #[serde(default)]
    permissions: Vec<String>,
    /// The level of the group with the least access that can call the extension.
    /// Without it, only an Admin and a Co-Admin can.
    #[serde(default = "default_level")]
    least_access: u16,
}

const fn default_level() -> u16 {
    1
}

/// Whether a text can be the id of an extension: the name of its folder.
fn is_id(text: &str) -> bool {
    (1..=40).contains(&text.len())
        && text
            .chars()
            .all(|letter| letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '-')
        && !text.starts_with('-')
        // This name is a part of the address of the marketplace routes.
        && text != "marketplace"
}

/// An extension as the list shows it.
#[derive(Debug, Serialize)]
pub struct ExtensionRow {
    id: String,
    name: String,
    version: String,
    description: String,
    author: String,
    permissions: Vec<String>,
    least_access: u16,
    /// The SHA-256 of the file of the component, as hex text.
    sha256: String,
    /// Whether the extension answers requests.
    enabled: bool,
    /// Whether an Admin approved the file as it is now.
    approved: bool,
    /// Whether the file changed after an Admin approved it.
    changed: bool,
    /// What is wrong with the extension, when it cannot be approved.
    problem: Option<String>,
}

/// What the disk holds for an extension.
struct OnDisk {
    manifest: Result<Manifest, String>,
    /// The bytes of the component and their SHA-256.
    component: Result<(Vec<u8>, String), String>,
}

fn sha256_of(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Reads the two files of an extension. This blocks.
fn read_extension(folder: &FilePath) -> OnDisk {
    let manifest = std::fs::read_to_string(folder.join(MANIFEST_FILE))
        .map_err(|error| format!("The file {MANIFEST_FILE} cannot be read: {error}"))
        .and_then(|text| {
            serde_json::from_str::<Manifest>(&text)
                .map_err(|error| format!("The file {MANIFEST_FILE} is not valid: {error}"))
        })
        .and_then(|manifest| {
            match manifest
                .permissions
                .iter()
                .find(|permission| !is_known_permission(permission))
            {
                Some(unknown) => Err(format!(
                    "The extension asks for the permission \"{unknown}\", which this version \
                     of Organizr does not know"
                )),
                None if manifest.least_access > 999 => {
                    Err("The level of the extension is over 999".to_owned())
                }
                None => Ok(manifest),
            }
        });
    let file = folder.join(COMPONENT_FILE);
    let component = std::fs::metadata(&file)
        .map_err(|error| format!("The file {COMPONENT_FILE} cannot be read: {error}"))
        .and_then(|data| {
            if data.len() > COMPONENT_LIMIT {
                Err(format!(
                    "The file {COMPONENT_FILE} is larger than {COMPONENT_LIMIT} bytes"
                ))
            } else {
                std::fs::read(&file)
                    .map_err(|error| format!("The file {COMPONENT_FILE} cannot be read: {error}"))
            }
        })
        .map(|bytes| {
            let sha256 = sha256_of(&bytes);
            (bytes, sha256)
        });
    OnDisk {
        manifest,
        component,
    }
}

/// The permissions of a manifest as the database keeps them: sorted, with commas.
fn permission_text(permissions: &[String]) -> String {
    let mut sorted: Vec<&str> = permissions.iter().map(String::as_str).collect();
    sorted.sort_unstable();
    sorted.dedup();
    sorted.join(",")
}

fn row_of(id: &str, disk: &OnDisk, approved: Option<&ApprovedExtension>) -> ExtensionRow {
    let sha256 = disk
        .component
        .as_ref()
        .map_or_else(|_| String::new(), |(_, sha256)| sha256.clone());
    let same_file = approved.is_some_and(|approved| approved.sha256 == sha256);
    let problem = match (&disk.manifest, &disk.component) {
        (Err(problem), _) | (_, Err(problem)) => Some(problem.clone()),
        _ => None,
    };
    let manifest = disk.manifest.as_ref().ok();
    ExtensionRow {
        id: id.to_owned(),
        name: manifest.map_or_else(|| id.to_owned(), |manifest| manifest.name.clone()),
        version: manifest.map_or_else(String::new, |manifest| manifest.version.clone()),
        description: manifest.map_or_else(String::new, |manifest| manifest.description.clone()),
        author: manifest.map_or_else(String::new, |manifest| manifest.author.clone()),
        permissions: manifest.map_or_else(Vec::new, |manifest| manifest.permissions.clone()),
        least_access: manifest.map_or(1, |manifest| manifest.least_access),
        sha256,
        enabled: approved.is_some_and(|approved| approved.enabled) && same_file,
        approved: same_file,
        changed: approved.is_some() && !same_file,
        problem,
    }
}

/// The folder of the extensions of this install.
fn folder_of(state: &AppState) -> Result<PathBuf, Refusal> {
    state
        .data_folder()
        .map(|data| data.join(FOLDER))
        .ok_or_else(|| Refusal::conflict("Extensions are not set up"))
}

fn failed(error: impl std::fmt::Display) -> Refusal {
    tracing::error!(%error, "a task for an extension failed");
    Refusal::Request(
        StatusCode::INTERNAL_SERVER_ERROR,
        "The extension could not be read".to_owned(),
    )
}

/// The ids of the folders of extensions, sorted. This blocks.
fn ids_in(folder: &FilePath) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| is_id(name))
        .collect();
    ids.sort_unstable();
    ids
}

/// `GET /extensions`: the extensions of the folder, with what an Admin decided.
pub async fn list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Vec<ExtensionRow>>, Refusal> {
    caller.require(VIEWER)?;
    let folder = folder_of(&state)?;
    let approved = state.database().extensions().list().await?;
    let rows = tokio::task::spawn_blocking(move || {
        ids_in(&folder)
            .into_iter()
            .map(|id| {
                let disk = read_extension(&folder.join(&id));
                let known = approved.iter().find(|approved| approved.id == id);
                row_of(&id, &disk, known)
            })
            .collect()
    })
    .await
    .map_err(failed)?;
    Ok(Envelope::success(rows))
}

/// Reads the extension with the id from the disk.
async fn on_disk(state: &AppState, id: &str) -> Result<OnDisk, Refusal> {
    let not_found = || Refusal::not_found("Extension not found");
    if !is_id(id) {
        return Err(not_found());
    }
    let folder = folder_of(state)?.join(id);
    tokio::task::spawn_blocking(move || folder.is_dir().then(|| read_extension(&folder)))
        .await
        .map_err(failed)?
        .ok_or_else(not_found)
}

/// `POST /extensions/{id}/enable`: approves the file of an extension with its
/// permissions, and switches the extension on.
///
/// The body names what the Admin saw: `sha256`, and `permissions` as a list. When the
/// file or its permissions are not the same any more, nothing is approved.
pub async fn enable(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(MANAGER)?;
    let runtime = state
        .extension_runtime()
        .ok_or_else(|| Refusal::conflict("The runtime for extensions did not start"))?
        .clone();
    let disk = on_disk(&state, &id).await?;
    let manifest = disk.manifest.map_err(Refusal::conflict)?;
    let (bytes, sha256) = disk.component.map_err(Refusal::conflict)?;

    let seen_file = data.text("sha256").unwrap_or_default();
    if seen_file != sha256 {
        return Err(Refusal::conflict(
            "The file of the extension changed. Load the list again.",
        ));
    }
    let seen_permissions: Vec<String> = data
        .field("permissions")
        .and_then(|value| value.as_array())
        .map_or_else(Vec::new, |list| {
            list.iter()
                .filter_map(|entry| entry.as_str().map(str::to_owned))
                .collect()
        });
    let permissions = permission_text(&manifest.permissions);
    if permission_text(&seen_permissions) != permissions {
        return Err(Refusal::conflict(
            "The permissions of the extension changed. Load the list again.",
        ));
    }

    // A file that is no extension for this interface is not approved.
    let checked = {
        let sha256 = sha256.clone();
        tokio::task::spawn_blocking(move || runtime.check(&bytes, &sha256))
            .await
            .map_err(failed)?
    };
    if let Err(error) = checked {
        tracing::warn!(extension = id, %error, "an extension was not approved");
        return Err(Refusal::conflict(shown(&error.to_string())));
    }
    state
        .database()
        .extensions()
        .approve(&id, &sha256, &permissions, i32::from(manifest.least_access))
        .await?;
    tracing::info!(
        extension = id,
        sha256,
        permissions,
        "an extension was approved"
    );
    Ok(Envelope::success(NoData).with_message("Extension enabled"))
}

/// `POST /extensions/{id}/disable`: switches an extension off. Its values stay.
pub async fn disable(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(MANAGER)?;
    if !state
        .database()
        .extensions()
        .set_enabled(&id, false)
        .await?
    {
        return Err(Refusal::not_found("Extension not found"));
    }
    tracing::info!(extension = id, "an extension was switched off");
    Ok(Envelope::success(NoData).with_message("Extension disabled"))
}

/// `DELETE /extensions/{id}`: deletes the folder of an extension, its approval and its
/// values.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(MANAGER)?;
    let not_found = || Refusal::not_found("Extension not found");
    if !is_id(&id) {
        return Err(not_found());
    }
    let folder = folder_of(&state)?.join(&id);
    let approved = state.database().extensions().find(&id).await?;
    let had_folder = tokio::task::spawn_blocking(move || {
        folder.is_dir() && std::fs::remove_dir_all(&folder).is_ok()
    })
    .await
    .map_err(failed)?;
    let had_row = state.database().extensions().delete(&id).await?;
    if let (Some(approved), Some(runtime)) = (approved, state.extension_runtime()) {
        runtime.forget(&approved.sha256);
    }
    if !had_folder && !had_row {
        return Err(not_found());
    }
    tracing::info!(extension = id, "an extension was deleted");
    Ok(Envelope::success(NoData).with_message("Extension deleted"))
}

/// The most bytes of the list of a marketplace.
const LIST_LIMIT: usize = 1024 * 1024;

/// The list of a marketplace.
#[derive(Debug, Deserialize)]
struct MarketList {
    extensions: Vec<MarketEntry>,
}

/// An extension that a marketplace offers.
#[derive(Debug, Clone, Deserialize)]
struct MarketEntry {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    permissions: Vec<String>,
    #[serde(default = "default_level")]
    least_access: u16,
    /// The address of the file of the component.
    download: String,
    /// The SHA-256 of that file, as hex text.
    sha256: String,
}

impl MarketEntry {
    /// What keeps the entry from being installed, when something does.
    fn problem(&self) -> Option<String> {
        let is_hash = self.sha256.len() == 64
            && self
                .sha256
                .chars()
                .all(|letter| letter.is_ascii_digit() || ('a'..='f').contains(&letter));
        if !is_hash {
            return Some("The entry has no valid SHA-256".to_owned());
        }
        if !(self.download.starts_with("https://") || self.download.starts_with("http://")) {
            return Some("The entry has no address to download from".to_owned());
        }
        if self.least_access > 999 {
            return Some("The level of the extension is over 999".to_owned());
        }
        self.permissions
            .iter()
            .find(|permission| !is_known_permission(permission))
            .map(|unknown| {
                format!(
                    "The extension asks for the permission \"{unknown}\", which this version \
                     of Organizr does not know"
                )
            })
    }
}

/// An extension of the marketplace as the list shows it.
#[derive(Debug, Serialize)]
pub struct MarketRow {
    id: String,
    name: String,
    version: String,
    description: String,
    author: String,
    permissions: Vec<String>,
    least_access: u16,
    sha256: String,
    /// Whether the folder of the extensions holds an extension with this id.
    installed: bool,
    /// Whether the installed file is the file of the marketplace.
    up_to_date: bool,
    problem: Option<String>,
}

/// Loads the list of the marketplace. An entry whose id is no id is left out.
async fn market_entries(state: &AppState) -> Result<Vec<MarketEntry>, Refusal> {
    let url = state
        .extension_marketplace()
        .ok_or_else(|| Refusal::conflict("No marketplace is set up"))?;
    let http = state
        .outbound()
        .ok_or_else(|| Refusal::conflict("No requests can be sent"))?;
    let unreadable = |error: String| {
        tracing::warn!(error, "the marketplace for extensions cannot be read");
        Refusal::conflict("Could not access the marketplace")
    };
    let bytes = probe::download(http, url, LIST_LIMIT)
        .await
        .map_err(unreadable)?;
    let list: MarketList =
        serde_json::from_slice(&bytes).map_err(|error| unreadable(error.to_string()))?;
    Ok(list
        .extensions
        .into_iter()
        .filter(|entry| is_id(&entry.id))
        .collect())
}

/// `GET /extensions/marketplace`: the extensions that the marketplace offers.
pub async fn marketplace(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Vec<MarketRow>>, Refusal> {
    caller.require(VIEWER)?;
    let folder = folder_of(&state)?;
    let entries = market_entries(&state).await?;
    let rows = tokio::task::spawn_blocking(move || {
        entries
            .into_iter()
            .map(|entry| {
                let file = folder.join(&entry.id).join(COMPONENT_FILE);
                let installed = std::fs::read(file).ok().map(|bytes| sha256_of(&bytes));
                MarketRow {
                    problem: entry.problem(),
                    installed: installed.is_some(),
                    up_to_date: installed.as_deref() == Some(entry.sha256.as_str()),
                    id: entry.id,
                    name: entry.name,
                    version: entry.version,
                    description: entry.description,
                    author: entry.author,
                    permissions: entry.permissions,
                    least_access: entry.least_access,
                    sha256: entry.sha256,
                }
            })
            .collect()
    })
    .await
    .map_err(failed)?;
    Ok(Envelope::success(rows))
}

/// `POST /extensions/marketplace/{id}`: downloads an extension of the marketplace into
/// the folder of the extensions.
///
/// The file must have the SHA-256 that the marketplace names. The extension does not
/// answer until an Admin approves it, as an extension that was copied by hand.
pub async fn install(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(MANAGER)?;
    let folder = folder_of(&state)?;
    let entry = market_entries(&state)
        .await?
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| Refusal::not_found("Extension does not exist in marketplace"))?;
    if let Some(problem) = entry.problem() {
        return Err(Refusal::conflict(problem));
    }
    let http = state
        .outbound()
        .ok_or_else(|| Refusal::conflict("No requests can be sent"))?;
    let limit = usize::try_from(COMPONENT_LIMIT).unwrap_or(usize::MAX);
    let bytes = probe::download(http, &entry.download, limit)
        .await
        .map_err(|error| {
            tracing::warn!(
                extension = id,
                error,
                "an extension could not be downloaded"
            );
            Refusal::conflict("The extension could not be downloaded")
        })?;
    if sha256_of(&bytes) != entry.sha256 {
        tracing::warn!(
            extension = id,
            "an extension was not installed: the file is not the file of the marketplace"
        );
        return Err(Refusal::conflict(
            "The file does not have the SHA-256 that the marketplace names. Nothing was installed.",
        ));
    }

    let manifest = serde_json::json!({
        "name": entry.name,
        "version": entry.version,
        "description": entry.description,
        "author": entry.author,
        "permissions": entry.permissions,
        "least_access": entry.least_access,
    });
    let target = folder.join(&id);
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&target)?;
        // The component gets its name when it is complete.
        let partial = target.join(format!("{COMPONENT_FILE}.part"));
        std::fs::write(&partial, &bytes)?;
        std::fs::rename(&partial, target.join(COMPONENT_FILE))?;
        std::fs::write(target.join(MANIFEST_FILE), manifest.to_string())
    })
    .await
    .map_err(failed)?
    .map_err(failed)?;
    tracing::info!(
        extension = id,
        version = entry.version,
        "an extension was installed"
    );
    Ok(Envelope::success(NoData)
        .with_message("Extension installed. It runs after an Admin approves it."))
}

/// The start of a text of the runtime, for an answer.
fn shown(text: &str) -> String {
    text.chars().take(300).collect()
}

/// `ANY /extension/{id}`: a request to the first route of an extension.
pub async fn call_root(
    state: State<AppState>,
    caller: Extension<Caller>,
    Path(id): Path<String>,
    method: Method,
    query: RawQuery,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Refusal> {
    call(
        state,
        caller,
        Path((id, String::new())),
        method,
        query,
        headers,
        body,
    )
    .await
}

/// `ANY /extension/{id}/{path}`: hands a request to an extension and sends its answer.
///
/// The document of an answer is kept apart from Organizr in the browser, as the
/// custom HTML of the homepage is: it cannot read the cookies of Organizr.
pub async fn call(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path((id, path)): Path<(String, String)>,
    method: Method,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Refusal> {
    // One answer for an extension that is not there and for one that is switched off.
    let not_found = || Refusal::not_found("Extension not found");
    let extensions = state.database().extensions();
    let approved = extensions
        .find(&id)
        .await?
        .filter(|approved| approved.enabled)
        .ok_or_else(not_found)?;
    let level = u16::try_from(approved.least_access).unwrap_or(0);
    caller.require(GroupLevel::new(level).unwrap_or(GroupLevel::ADMIN))?;
    let runtime = state
        .extension_runtime()
        .ok_or_else(|| Refusal::conflict("The runtime for extensions did not start"))?
        .clone();
    if body.len() > BODY_LIMIT {
        return Err(Refusal::Request(
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("The body of a request to an extension has {BODY_LIMIT} bytes at most"),
        ));
    }

    let disk = on_disk(&state, &id).await?;
    let (bytes, sha256) = disk.component.map_err(|_| not_found())?;
    if sha256 != approved.sha256 {
        tracing::warn!(
            extension = id,
            "an extension did not run: its file changed after the approval"
        );
        return Err(Refusal::conflict(
            "The extension changed after it was approved. An Admin must approve it again.",
        ));
    }

    // One call of an extension at a time, so that two calls do not lose a value.
    let lock = state.extension_lock(&id);
    let _one_at_a_time = lock.lock().await;
    let can_store = approved.permissions.split(',').any(|one| one == STORAGE);
    let values: BTreeMap<String, String> = if can_store {
        let stored = extensions.find(&id).await?.map(|now| now.values);
        stored
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    let request = Request {
        method: method.as_str().to_owned(),
        path: format!("/{path}"),
        query: query.unwrap_or_default(),
        headers: REQUEST_HEADERS
            .iter()
            .filter_map(|name| {
                let value = headers.get(*name)?.to_str().ok()?;
                Some(((*name).to_owned(), value.to_owned()))
            })
            .collect(),
        body: body.to_vec(),
        caller: ExtensionCaller {
            username: caller
                .user()
                .map(|user| user.authenticated.user.username.clone()),
            level: caller.level().get(),
        },
    };
    let http = state.outbound().and_then(|outbound| {
        extension_http::Gate::for_permissions(
            outbound,
            &approved.permissions,
            &id,
            tokio::runtime::Handle::current(),
        )
    });
    let call = Call {
        extension: id.clone(),
        can_store,
        values,
        http: http.map(|gate| {
            std::sync::Arc::new(gate) as std::sync::Arc<dyn organizr_plugins::extension::HttpGate>
        }),
    };
    let outcome =
        tokio::task::spawn_blocking(move || runtime.handle(&bytes, &sha256, call, request))
            .await
            .map_err(failed)?;
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => {
            tracing::warn!(extension = id, %error, "an extension gave no answer");
            let status = match error {
                ExtensionError::Stopped(_) | ExtensionError::AnswerTooLarge => {
                    StatusCode::BAD_GATEWAY
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            return Err(Refusal::Request(status, shown(&error.to_string())));
        }
    };
    if let Some(values) = outcome.values {
        let text = serde_json::to_string(&values).unwrap_or_else(|_| "{}".to_owned());
        extensions.set_values(&id, &text).await?;
    }

    Ok(answer_of(outcome.response))
}

/// The answer of an extension as the browser gets it. The extension sets the status,
/// the body and a few headers. The headers that keep its document apart from Organizr
/// are always added.
fn answer_of(answer: organizr_plugins::extension::Response) -> Response {
    let status = StatusCode::from_u16(answer.status)
        .ok()
        .filter(|status| !status.is_informational())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let mut response = (status, answer.body).into_response();
    let headers = response.headers_mut();
    headers.remove(header::CONTENT_TYPE);
    for (name, value) in answer.headers {
        let name = name.to_ascii_lowercase();
        if !RESPONSE_HEADERS.contains(&name.as_str()) {
            continue;
        }
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(&value),
        ) {
            headers.insert(name, value);
        }
    }
    if !headers.contains_key(header::CONTENT_TYPE) {
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
    }
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(POLICY),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_a_folder_name_of_small_letters_digits_and_hyphens() {
        for id in ["hello", "my-extension-2", "a"] {
            assert!(is_id(id), "{id}");
        }
        for id in [
            "",
            "-a",
            "Hello",
            "a b",
            "a/b",
            "..",
            "a.b",
            "marketplace",
            &"a".repeat(41),
        ] {
            assert!(!is_id(id), "{id}");
        }
    }

    #[test]
    fn permissions_are_compared_whatever_their_order() {
        let list =
            |items: &[&str]| -> Vec<String> { items.iter().map(|&one| one.to_owned()).collect() };
        assert_eq!(permission_text(&list(&["storage"])), "storage");
        assert_eq!(permission_text(&list(&["b", "a", "b"])), "a,b");
        assert_eq!(permission_text(&[]), "");
    }
}
