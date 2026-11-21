//! The themes: `GET /themes`, the theme marketplace and its install and removal, and the
//! style sheet of the Admin.
//!
//! PHP Organizr downloads the CSS files of a marketplace theme from GitHub into
//! `data/themes` and links them into the page. Here the 11 themes of the marketplace are
//! rebuilt in the style sheet of the interface, and no file is downloaded. The marketplace
//! lists them as the marketplace file of PHP Organizr does, and an install only marks a
//! theme as installed: the interface then offers it. `docs/deviations.md` lists the
//! differences as D-42, D-92 and D-93.

use std::sync::LazyLock;

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use organizr_domain::GroupLevel;
use organizr_settings::Settings;
use serde_json::{Map, Value, json};

use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The level of the theme routes: Admins and Co-Admins, as `qualifyRequest(1)` in PHP.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// The theme that the page has when the setting names no theme that it can show.
pub const DEFAULT_THEME: &str = "Organizr";

/// The two themes that PHP Organizr brings along, in the order of PHP.
const BUILT_IN: [&str; 2] = ["Blue", DEFAULT_THEME];

/// The marketplace file of PHP Organizr: the file `themes.json` of the repository
/// `Organizr/Organizr-Themes`, at the commit `50138d6`. Each of its themes has a native
/// version in the style sheet of the interface.
///
/// The API keeps the order of the keys of JSON objects (`serde_json` with
/// `preserve_order`, which the crate gets through `organizr-integrations`), so the list
/// has the order of the file, as in PHP.
static MARKETPLACE: LazyLock<Map<String, Value>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("themes.json"))
        .inspect_err(|error| tracing::error!(%error, "the theme list cannot be read"))
        .unwrap_or_default()
});

/// What PHP stores for a built-in theme.
fn built_in(name: &str) -> Value {
    json!({ "name": name, "repo": null, "version": "1.0.0", "path": "css/themes" })
}

/// The themes that an install has marked as installed: the setting `installedThemes`,
/// an object from the name of a theme to its details. PHP stores the empty text while
/// none is installed.
fn installed(settings: &Settings) -> Map<String, Value> {
    match settings.get("installedThemes") {
        Some(Value::Object(themes)) => themes,
        _ => Map::new(),
    }
}

/// The name as PHP's `cleanClassName($name, '_')` writes it: in lower case, with `_` for
/// a space and a `-`.
fn clean_name(name: &str) -> String {
    name.to_lowercase()
        .replace("%20", "_")
        .replace([' ', '-'], "_")
}

/// The themes of the marketplace, each with what the install knows about it, as PHP's
/// `getThemesMarketplace()` sends them.
fn marketplace(settings: &Settings) -> Map<String, Value> {
    let installed = installed(settings);
    MARKETPLACE
        .iter()
        .map(|(name, details)| {
            let mut details = details.clone();
            let version = details.get("version").cloned().unwrap_or(Value::Null);
            let installed_version = installed
                .get(name)
                .map(|theme| theme.get("version").cloned().unwrap_or(Value::Null));
            let is_installed = installed_version.is_some();
            let needs_update = installed_version.as_ref().is_some_and(|it| *it != version);
            let status = if needs_update {
                "Update Available"
            } else if is_installed {
                "Up to date"
            } else {
                "Not Installed"
            };
            if let Some(fields) = details.as_object_mut() {
                fields.insert("installed".to_owned(), Value::Bool(is_installed));
                fields.insert(
                    "installed_version".to_owned(),
                    installed_version.unwrap_or(Value::Null),
                );
                fields.insert("needs_update".to_owned(), Value::Bool(needs_update));
                fields.insert("status".to_owned(), Value::from(status));
            }
            (name.clone(), details)
        })
        .collect()
}

/// The marketplace theme that a path names, in any letter case, with its name as the
/// marketplace writes it.
fn find<'a>(themes: &'a Map<String, Value>, cleaned: &str) -> Option<(&'a String, &'a Value)> {
    themes
        .iter()
        .find(|(name, _)| name.to_lowercase() == cleaned)
}

/// Every theme that the install has: the built-in ones, and the installed ones after
/// them, as PHP's `getAllThemesInformation()` sends them.
pub(crate) fn all_themes(settings: &Settings) -> Map<String, Value> {
    let mut themes: Map<String, Value> = BUILT_IN
        .iter()
        .map(|name| ((*name).to_owned(), built_in(name)))
        .collect();
    themes.extend(installed(settings));
    themes
}

/// The theme that the page shows: the setting `theme` when the install has that theme
/// and the interface has a native version of it, and else `Organizr`. PHP falls back to
/// `Organizr` in the same way when the theme is not installed.
pub fn applied_theme(settings: &Settings) -> String {
    let theme = settings.theme.as_str();
    let is_built_in = BUILT_IN.contains(&theme);
    let is_installed_native =
        MARKETPLACE.contains_key(theme) && installed(settings).contains_key(theme);
    if is_built_in || is_installed_native {
        theme.to_owned()
    } else {
        DEFAULT_THEME.to_owned()
    }
}

/// `GET /themes`: the themes that the install has.
pub async fn list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Map<String, Value>>, Refusal> {
    caller.require(MANAGER)?;
    Ok(Envelope::success(all_themes(&state.settings())))
}

/// `GET /themes/marketplace`: the themes of the marketplace.
pub async fn marketplace_list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Map<String, Value>>, Refusal> {
    caller.require(MANAGER)?;
    Ok(Envelope::success(marketplace(&state.settings())))
}

/// Stores the new value of `installedThemes`.
async fn store(state: &AppState, themes: Map<String, Value>) -> Result<(), Refusal> {
    // PHP stores an empty array, which its JSON writes as `[]`. The empty text is the
    // default of the setting, and means the same.
    let value = if themes.is_empty() {
        Value::String(String::new())
    } else {
        Value::Object(themes)
    };
    state
        .change_settings(vec![("installedThemes".to_owned(), value)])
        .await
        .map_err(|error| Refusal::Api(error.into()))
}

/// `POST /themes/manage/{theme}`: marks a theme of the marketplace as installed.
///
/// Nothing is downloaded: the interface has the theme already. The answer holds the
/// theme as the marketplace listed it before the install, as in PHP.
pub async fn install(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(theme): Path<String>,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    let cleaned = clean_name(&theme);
    let settings = state.settings();
    let themes = marketplace(&settings);
    let Some((name, details)) = find(&themes, &cleaned) else {
        let by_lower_name: Map<String, Value> = themes
            .iter()
            .map(|(name, details)| (name.to_lowercase(), details.clone()))
            .collect();
        return Err(Refusal::RequestWithData(
            StatusCode::NOT_FOUND,
            format!("Theme [{cleaned}] does not exist in marketplace"),
            Value::Object(by_lower_name),
        ));
    };
    let mut installed = installed(&settings);
    let folder = details
        .get("project_folder")
        .and_then(Value::as_str)
        .unwrap_or_default();
    installed.insert(
        name.clone(),
        json!({
            "name": name,
            "version": details.get("version").cloned().unwrap_or(Value::Null),
            "repo": details.get("repo").cloned().unwrap_or(Value::Null),
            "path": format!("data/themes/{folder}"),
        }),
    );
    store(&state, installed).await?;
    tracing::info!(theme = %name, "a theme was installed");
    Ok(Envelope::success(details.clone())
        .with_message("Theme installed")
        .into_response())
}

/// `DELETE /themes/manage/{theme}`: takes the mark "installed" from a theme.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(theme): Path<String>,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    let cleaned = clean_name(&theme);
    let settings = state.settings();
    let themes = marketplace(&settings);
    let Some((name, details)) = find(&themes, &cleaned) else {
        return Err(Refusal::not_found("Theme does not exist in marketplace"));
    };
    let mut installed = installed(&settings);
    if installed.shift_remove(name).is_none() {
        return Err(Refusal::not_found("Theme is not installed"));
    }
    store(&state, installed).await?;
    tracing::info!(theme = %name, "a theme was removed");
    Ok(Envelope::success(details.clone())
        .with_message("Theme removed")
        .into_response())
}

/// What a browser may do with the style sheet when it opens it as a page: nothing.
const STYLE_SHEET_POLICY: &str = "default-src 'none'; frame-ancestors 'none'";

/// `GET /appearance/custom.css`: the style sheets of the Admin, `customThemeCss` and then
/// `customCss`, as one style sheet.
///
/// PHP writes the two texts into `<style>` elements of the page. The page of this
/// interface allows no style that is written into it, so it links this file. Every
/// caller gets it, as every caller gets the two texts from `GET /launch` in PHP. The
/// router sends no answer to a cache, so a change applies on the next load.
pub async fn custom_css(State(state): State<AppState>) -> Response {
    let settings = state.settings();
    let mut sheet = String::new();
    for part in [&settings.custom_theme_css, &settings.custom_css] {
        if !part.is_empty() {
            sheet.push_str(part);
            sheet.push('\n');
        }
    }
    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/css; charset=utf-8"),
            ),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
            (
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static(STYLE_SHEET_POLICY),
            ),
        ],
        sheet,
    )
        .into_response()
}

/// The scripts of the Admin, `customThemeJava` and then `customJava`, as one text. Empty
/// when the switch of the operator is off.
pub fn custom_script(state: &AppState) -> String {
    if !state.custom_js() {
        return String::new();
    }
    let settings = state.settings();
    let mut script = String::new();
    for part in [&settings.custom_theme_java, &settings.custom_java] {
        if !part.trim().is_empty() {
            script.push_str(part);
            script.push('\n');
        }
    }
    script
}

/// `GET /appearance/custom.js`: the scripts of the Admin as one file, which the page loads
/// from its own site, so that its policy stays `script-src 'self'`.
///
/// It exists only when the operator turned the switch `ORGANIZR_CUSTOM_JS` on and an Admin
/// stored a script. Else the answer is "not found". Every caller gets it, as PHP runs the
/// scripts for every caller of a set-up install, guests on the login page too (D-93).
pub async fn custom_js(
    State(state): State<AppState>,
    uri: axum::http::Uri,
) -> Result<Response, crate::error::ApiError> {
    let script = custom_script(&state);
    if script.is_empty() {
        return Err(crate::error::ApiError::NotFound {
            endpoint: uri.path().to_owned(),
            method: "GET".to_owned(),
        });
    }
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/javascript; charset=utf-8"),
            ),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
        ],
        script,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use organizr_settings::Settings;
    use serde_json::{Value, json};

    use super::{MARKETPLACE, applied_theme, clean_name};

    #[test]
    fn marketplace_holds_the_eleven_themes_in_the_order_of_the_file() {
        let names: Vec<&str> = MARKETPLACE.keys().map(String::as_str).collect();
        assert_eq!(
            names,
            [
                "Nord",
                "Dracula",
                "Overseerr",
                "Onedark",
                "Aquamarine",
                "Space_Gray",
                "Plex_theme_park",
                "Dark_theme_park",
                "Hotline",
                "Hotpink",
                "Plex"
            ]
        );
    }

    #[test]
    fn name_is_cleaned_as_php_cleans_it() {
        assert_eq!(clean_name("Space Gray"), "space_gray");
        assert_eq!(clean_name("Plex-Theme-Park"), "plex_theme_park");
        assert_eq!(clean_name("Nord%20"), "nord_");
    }

    fn with(theme: &str, installed: &Value) -> Settings {
        let mut settings = Settings::defaults();
        settings
            .set("theme", &Value::from(theme))
            .unwrap_or_else(|error| panic!("{error}"));
        settings
            .set("installedThemes", installed)
            .unwrap_or_else(|error| panic!("{error}"));
        settings
    }

    #[test]
    fn page_shows_a_theme_that_is_built_in_or_installed_and_rebuilt() {
        assert_eq!(applied_theme(&with("Blue", &json!(""))), "Blue");
        assert_eq!(applied_theme(&with("Nord", &json!(""))), "Organizr");
        assert_eq!(
            applied_theme(&with("Nord", &json!({ "Nord": { "version": "1.0.2" } }))),
            "Nord"
        );
        // A theme of another marketplace has no native version.
        assert_eq!(
            applied_theme(&with("Other", &json!({ "Other": { "version": "1" } }))),
            "Organizr"
        );
    }
}
