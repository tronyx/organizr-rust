//! The `/image` routes: the pictures that a tab, a category or a group can have.
//!
//! There are two kinds of pictures. The pictures that come with the application are
//! files of the web interface, under `plugins/images/tabs/`. The pictures that an Admin
//! or a Co-Admin uploads live in the data folder and are sent from `data/userTabs/`.
//! Both paths are the ones of the PHP application, so the pictures of an imported
//! install are found. `docs/deviations.md` lists the differences from the PHP application
//! as D-10 and D-54.

use std::collections::BTreeMap;
use std::path::{Path as FilePath, PathBuf};

use axum::Extension;
use axum::body::Body;
use axum::extract::multipart::{Multipart, MultipartRejection};
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use organizr_domain::GroupLevel;
use serde::Serialize;
use serde_json::{Value, json};

use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The level a caller needs to list, upload and delete pictures.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// The names of the pictures for tabs that come with the application, one per line.
/// `web/tools/import-organizr-assets.py` writes the file.
const BUNDLED: &str = include_str!("bundled_tab_images.txt");

/// Where the web interface sends the pictures that come with the application.
const BUNDLED_PATH: &str = "plugins/images/tabs/";

/// Where the application sends the pictures that were uploaded.
const UPLOADED_PATH: &str = "data/userTabs/";

/// The largest picture that can be uploaded, in bytes.
pub const LARGEST_UPLOAD: usize = 10 * 1024 * 1024;

/// How many pictures a page of the chooser list holds when the request does not say.
const PAGE_LENGTH: u64 = 20;

/// The content type of a picture by the ending of its name, for the endings that a
/// picture can have: `gif`, `png`, `jpeg` and `jpg`, in any letter case.
fn picture_type(name: &str) -> Option<&'static str> {
    let (_, ending) = name.rsplit_once('.')?;
    match ending.to_ascii_lowercase().as_str() {
        "gif" => Some("image/gif"),
        "png" => Some("image/png"),
        "jpeg" | "jpg" => Some("image/jpeg"),
        _ => None,
    }
}

/// Whether the content starts as a GIF, a PNG or a JPEG picture does.
fn is_picture(content: &[u8]) -> bool {
    content.starts_with(b"GIF87a")
        || content.starts_with(b"GIF89a")
        || content.starts_with(b"\x89PNG\r\n\x1a\n")
        || content.starts_with(b"\xff\xd8\xff")
}

/// The last part of a path, as a browser on Windows or on another system writes one.
fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Whether a name is safe as the name of a file in the upload folder, on every system:
/// not empty, not hidden, at most 255 bytes, and without a character that a file system
/// reads as something else than a part of a name.
fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && !name.starts_with('.')
        && !name.ends_with([' ', '.'])
        && !name
            .chars()
            .any(|character| character.is_control() || "/\\:*?\"<>|".contains(character))
}

/// The pictures that were uploaded: the names of the files in the folder that the
/// application sends as pictures. Another file that somebody put there is left out.
async fn uploaded(folder: Option<&FilePath>) -> std::io::Result<Vec<String>> {
    let Some(folder) = folder else {
        return Ok(Vec::new());
    };
    let mut entries = match tokio::fs::read_dir(folder).await {
        Ok(entries) => entries,
        // Nothing was uploaded yet.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut names = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let is_file = entry.file_type().await.is_ok_and(|kind| kind.is_file());
        if let Some(name) = entry.file_name().to_str()
            && is_file
            && is_safe_name(name)
            && picture_type(name).is_some()
        {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}

/// Every picture as the path that a tab stores for it, sorted by the name of the file
/// without regard to letter case. An uploaded picture stands in for a picture of the
/// application with the same name.
async fn all_paths(state: &AppState) -> Result<Vec<String>, ApiError> {
    let mut paths: BTreeMap<String, &'static str> = BUNDLED
        .lines()
        .filter(|name| !name.is_empty())
        .map(|name| (name.to_owned(), BUNDLED_PATH))
        .collect();
    let names = uploaded(state.picture_folder())
        .await
        .map_err(ApiError::PictureFolder)?;
    for name in names {
        paths.insert(name, UPLOADED_PATH);
    }
    let mut sorted: Vec<(String, &'static str)> = paths.into_iter().collect();
    sorted.sort_by_cached_key(|(name, _)| name.to_ascii_lowercase());
    Ok(sorted
        .into_iter()
        .map(|(name, path)| format!("{path}{name}"))
        .collect())
}

/// `GET /image`: the path of every picture.
pub async fn list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Vec<String>>, Refusal> {
    caller.require(MANAGER)?;
    Ok(Envelope::success(all_paths(&state).await?))
}

/// One entry of the chooser list.
#[derive(Debug, Serialize)]
struct Choice {
    /// The path that a tab stores.
    id: String,
    /// The name of the file.
    text: String,
}

/// A page number or a page length of the chooser list: a whole number from 1 on. The
/// answer repeats it as the request wrote it.
fn count(given: Option<&str>, fallback: u64, name: &str) -> Result<(u64, Value), Refusal> {
    match given {
        None => Ok((fallback, json!(fallback))),
        Some(text) => text
            .parse::<u64>()
            .ok()
            .filter(|number| *number > 0)
            .map(|number| (number, json!(text)))
            .ok_or_else(|| Refusal::unprocessable(&format!("{name} is not valid"))),
    }
}

/// `GET /image/select`: the pictures for a chooser, one page at a time.
///
/// The query can name `search` (a part of a file name, in any letter case), `page`
/// (from 1) and `limit` (the length of a page).
pub async fn select(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    RawQuery(query): RawQuery,
) -> Result<Envelope<Value>, Refusal> {
    caller.require(MANAGER)?;
    let query: Vec<(String, String)> = form_urlencoded::parse(query.unwrap_or_default().as_bytes())
        .into_owned()
        .collect();
    let given = |name: &str| {
        query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    let term = given("search");
    let (page, page_shown) = count(given("page"), 1, "page")?;
    let (limit, limit_shown) = count(given("limit"), PAGE_LENGTH, "limit")?;

    // PHP takes the empty text and the text `0` for "no search".
    let wanted = term
        .filter(|term| !term.is_empty() && *term != "0")
        .map(str::to_ascii_lowercase);
    let found: Vec<Choice> = all_paths(&state)
        .await?
        .into_iter()
        .map(|path| Choice {
            text: base_name(&path).to_owned(),
            id: path,
        })
        .filter(|choice| {
            wanted
                .as_ref()
                .is_none_or(|wanted| choice.text.to_ascii_lowercase().contains(wanted))
        })
        .collect();
    let total = found.len() as u64;
    let skipped =
        usize::try_from(page.saturating_sub(1).saturating_mul(limit)).unwrap_or(usize::MAX);
    let shown: Vec<Choice> = found
        .into_iter()
        .skip(skipped)
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .collect();
    Ok(Envelope::success(json!({
        "results": shown,
        "limit": limit_shown,
        "page": page_shown,
        "term": term,
        "total": total,
        "pagination": { "more": page < total.div_ceil(limit) },
    })))
}

/// The file of an upload request.
struct Upload {
    name: String,
    content_type: String,
    content: Vec<u8>,
}

/// Reads the parts of an upload request. Returns the file of the field `file`, and
/// whether the request held a file at all.
async fn read_upload(mut parts: Multipart) -> Result<(Option<Upload>, bool), Refusal> {
    let mut upload = None;
    let mut has_file = false;
    loop {
        let part = match parts.next_field().await {
            Ok(Some(part)) => part,
            Err(error) if error.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                return Err(too_large());
            }
            // The end of the request, or a part that cannot be read: what was read counts.
            Ok(None) | Err(_) => break,
        };
        let Some(name) = part.file_name().map(str::to_owned) else {
            continue;
        };
        has_file = true;
        if part.name() != Some("file") || upload.is_some() {
            continue;
        }
        let content_type = part.content_type().unwrap_or_default().to_owned();
        let content = match part.bytes().await {
            Ok(content) => content.to_vec(),
            Err(error) if error.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                return Err(too_large());
            }
            Err(_) => return Err(upload_error()),
        };
        upload = Some(Upload {
            name,
            content_type,
            content,
        });
    }
    Ok((upload, has_file))
}

/// The refusal for an upload that the application could not read or store. The status
/// and the text are those of the PHP application.
fn upload_error() -> Refusal {
    Refusal::Request(
        StatusCode::INTERNAL_SERVER_ERROR,
        "File upload error".to_owned(),
    )
}

fn too_large() -> Refusal {
    Refusal::Request(
        StatusCode::PAYLOAD_TOO_LARGE,
        "File is larger than 10 MB".to_owned(),
    )
}

fn forbidden(message: &str) -> Refusal {
    Refusal::Request(StatusCode::FORBIDDEN, message.to_owned())
}

/// `POST /image`: stores a picture that the request uploads in the field `file`.
///
/// The picture must be a GIF, a PNG or a JPEG of at most 10 MB, by its content and by the
/// ending of its name. A picture with the name of an uploaded picture replaces it.
pub async fn upload(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    parts: Result<Multipart, MultipartRejection>,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    let no_file = || {
        Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No File was uploaded".to_owned(),
        )
    };
    let (upload, has_file) = read_upload(parts.map_err(|_| no_file())?).await?;
    if !has_file {
        return Err(no_file());
    }
    let Some(upload) = upload else {
        return Err(upload_error());
    };

    // The checks of the PHP application, in its order.
    if !upload.content_type.contains("image/") {
        return Err(Refusal::RequestWithData(
            StatusCode::FORBIDDEN,
            "File Type not image".to_owned(),
            json!(upload.content_type),
        ));
    }
    if !is_picture(&upload.content) {
        return Err(forbidden("File Type not approved"));
    }
    let name = base_name(&upload.name).trim();
    if picture_type(name).is_none() {
        return Err(forbidden("File Extension not approved"));
    }
    if !is_safe_name(name) {
        return Err(forbidden("File Name not approved"));
    }

    let folder = state.picture_folder().ok_or_else(upload_error)?;
    let stored = async {
        tokio::fs::create_dir_all(folder).await?;
        tokio::fs::write(folder.join(name), &upload.content).await
    };
    if let Err(error) = stored.await {
        tracing::error!(%error, "an uploaded picture could not be stored");
        return Err(upload_error());
    }
    tracing::info!(picture = name, "a picture was uploaded");
    Ok(Envelope::success(NoData)
        .with_message(format!("{name} has been uploaded"))
        .into_response())
}

/// The file in the upload folder with the given name, when the name is nothing but a
/// name. The caller makes sure that the name ends as a picture does.
fn picture_file(state: &AppState, name: &str) -> Option<PathBuf> {
    state
        .picture_folder()
        .filter(|_| is_safe_name(name))
        .map(|folder| folder.join(name))
}

/// `DELETE /image/{image}`: deletes an uploaded picture. The pictures that come with the
/// application cannot be deleted.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(image): Path<String>,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    // What PHP counts as "no picture": the text `0`.
    if image == "0" {
        return Err(Refusal::unprocessable("No image supplied"));
    }
    let name = base_name(&image);
    let path = format!("{UPLOADED_PATH}{name}");
    if picture_type(name).is_none() {
        return Err(Refusal::conflict(format!(
            "{path} is not approved to be deleted"
        )));
    }
    let not_found = || Refusal::not_found(&format!("{path} does not exist"));
    let file = picture_file(&state, name).ok_or_else(not_found)?;
    match tokio::fs::remove_file(&file).await {
        Ok(()) => {
            tracing::info!(picture = name, "an uploaded picture was deleted");
            Ok(Envelope::success(NoData)
                .with_message(format!("{name} has been deleted"))
                .into_response())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(not_found()),
        Err(error) => Err(ApiError::PictureFolder(error).into()),
    }
}

/// `GET /data/userTabs/{name}`: an uploaded picture. Everybody can ask for it, as for the
/// pictures that come with the application: the menu of a guest shows them too.
pub async fn uploaded_picture(
    State(state): State<AppState>,
    Path(name): Path<String>,
    uri: axum::http::Uri,
) -> Result<Response, ApiError> {
    let not_found = || ApiError::NotFound {
        endpoint: uri.path().to_owned(),
        method: "GET".to_owned(),
    };
    let content_type = picture_type(&name).ok_or_else(not_found)?;
    let file = picture_file(&state, &name).ok_or_else(not_found)?;
    let content = match tokio::fs::read(&file).await {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(not_found()),
        Err(error) => return Err(ApiError::PictureFolder(error)),
    };

    let mut response = Response::new(Body::from(content));
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    // The file is shown as the picture that its name says, and as nothing else. A browser
    // that opens it as a page runs nothing in it.
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; sandbox"),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::{BUNDLED, base_name, is_picture, is_safe_name, picture_type};

    #[test]
    fn picture_has_one_of_four_endings_in_any_letter_case() {
        assert_eq!(picture_type("logo.gif"), Some("image/gif"));
        assert_eq!(picture_type("logo.PNG"), Some("image/png"));
        assert_eq!(picture_type("my.logo.Jpeg"), Some("image/jpeg"));
        assert_eq!(picture_type("logo.jpg"), Some("image/jpeg"));
        for name in [
            "logo.svg",
            "logo.bmp",
            "logo.png.html",
            "png",
            "logo.",
            "logo",
            "",
        ] {
            assert_eq!(picture_type(name), None, "{name}");
        }
    }

    #[test]
    fn content_of_a_picture_starts_as_its_format_says() {
        assert!(is_picture(b"GIF89a\x01\x00\x01\x00\x00\x00\x00;"));
        assert!(is_picture(b"GIF87a..."));
        assert!(is_picture(b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR"));
        assert!(is_picture(b"\xff\xd8\xff\xe0\x00\x10JFIF"));
        for content in [
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>".as_slice(),
            b"<html><script>alert(1)</script></html>",
            b"GIF8",
            b"\x89PNG",
            b"just text",
            b"",
        ] {
            assert!(!is_picture(content), "{content:?}");
        }
    }

    #[test]
    fn base_name_is_the_part_after_the_last_separator() {
        assert_eq!(base_name("logo.png"), "logo.png");
        assert_eq!(base_name("C:\\Users\\me\\logo.png"), "logo.png");
        assert_eq!(base_name("../../etc/passwd"), "passwd");
        assert_eq!(base_name("folder/"), "");
    }

    #[test]
    fn safe_name_holds_nothing_that_a_file_system_reads_as_more_than_a_name() {
        for name in [
            "logo.png",
            "My Logo (2).png",
            "café.jpg",
            "a-b_c.gif",
            "日本.png",
        ] {
            assert!(is_safe_name(name), "{name}");
        }
        for name in [
            "",
            ".",
            "..",
            ".hidden.png",
            "a/b.png",
            "a\\b.png",
            "con:1.png",
            "what?.png",
            "a*.png",
            "a|b.png",
            "\"quoted\".png",
            "<b>.png",
            "line\nbreak.png",
            "zero\0.png",
            "ends with a space.png ",
            "ends with a dot.",
        ] {
            assert!(!is_safe_name(name), "{name:?}");
        }
        assert!(is_safe_name(&format!("{}.png", "x".repeat(251))));
        assert!(!is_safe_name(&format!("{}.png", "x".repeat(252))));
    }

    #[test]
    fn list_of_pictures_of_the_application_names_pictures_only() {
        let names: Vec<&str> = BUNDLED.lines().collect();

        assert!(names.len() > 100);
        for name in &names {
            assert!(is_safe_name(name) && picture_type(name).is_some(), "{name}");
        }
        assert!(names.contains(&"plex.png"));
    }
}
