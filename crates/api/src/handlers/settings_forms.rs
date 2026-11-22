//! The forms of the settings pages of PHP Organizr: the `/settings/*` routes and the
//! `settings` routes of the plugins.
//!
//! PHP builds each form from `settingsOption()` calls: groups of fields, each with its
//! type, its label and the stored value. `settings_forms.json` holds those forms as PHP
//! 2.1.5000 writes them, made once from the PHP source by a script that reads it. The
//! script left out the raw HTML and the `attr` of each field, gave the notices, the
//! "about" texts, the API SOCKS panels and the code editors typed kinds, and left out
//! every field of a setting that this application does not have (D-22). The parts that
//! PHP works out for each request are placeholders, which this module fills:
//!
//! - `{"$config": key}`: the stored value, masked as `GET /config` masks it;
//! - `{"$source": name, "args": [...]}`: a list of the install: its groups, its users,
//!   its tabs, its themes, or the parts of a stored value;
//! - `{"$license": text}`: whether `text` holds the setting `license`, as PHP's
//!   `strpos(text, license) !== false`.
//!
//! `docs/deviations.md` lists the differences as D-22 and D-96.

use std::sync::LazyLock;

use axum::Extension;
use axum::extract::{Path, State};
use organizr_domain::{GroupLevel, TabKind};
use organizr_settings::Settings;
use serde_json::{Map, Value, json};

use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::config::shown;
use crate::handlers::fields::Refusal;
use crate::handlers::themes::all_themes;
use crate::state::AppState;

/// The level of every route of this module: Admins and Co-Admins, as `qualifyRequest(1)`.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// What the debug route shows in place of a value that may be secret, as PHP does.
const REDACTED: &str = "***redacted***";

/// The forms, as `settings_forms.json` holds them.
static TABLE: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("settings_forms.json"))
        .inspect_err(|error| tracing::error!(%error, "the settings forms cannot be read"))
        .unwrap_or_default()
});

/// What a missing part of the table reads as.
static NOTHING: Value = Value::Null;

/// A part of an object of the table, or `null`.
fn part<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or(&NOTHING)
}

/// The forms of the `/settings/*` routes, by the last part of the path.
fn route_form(route: &str) -> Value {
    part(part(&TABLE, "routes"), route).clone()
}

/// The homepage items, in the order of PHP.
fn homepage_items() -> &'static [Value] {
    part(&TABLE, "homepage")
        .as_array()
        .map_or(&[], Vec::as_slice)
}

/// What a form needs from the install to fill its placeholders.
struct Fill<'a> {
    settings: &'a Settings,
    level: GroupLevel,
    groups: Vec<Value>,
    users: Vec<Value>,
    tabs: Vec<organizr_domain::Tab>,
    /// The libraries of Komga for the invites plugin, when Komga was asked.
    komga_libraries: Option<Vec<Value>>,
}

impl<'a> Fill<'a> {
    async fn new(
        state: &AppState,
        settings: &'a Settings,
        caller: &Caller,
    ) -> Result<Fill<'a>, Refusal> {
        let database = state.database();
        // PHP's `groupSelect()`: the name and the level of each group.
        let groups = database
            .groups()
            .list()
            .await?
            .into_iter()
            .map(|group| json!({ "name": group.name, "value": group.level.get() }))
            .collect();
        // PHP's `userSelect()`: the name and the id of each user.
        let users = database
            .users()
            .list()
            .await?
            .into_iter()
            .map(|user| json!({ "name": user.username, "value": user.id.get() }))
            .collect();
        let tabs = database.tabs().list().await?;
        Ok(Self {
            settings,
            level: caller.level(),
            groups,
            users,
            tabs,
            komga_libraries: None,
        })
    }

    /// A value with every placeholder filled.
    fn fill(&self, value: &Value) -> Value {
        match value {
            Value::Object(fields) => {
                if let Some(key) = fields.get("$config") {
                    return self.config(key);
                }
                if let Some(Value::String(name)) = fields.get("$source") {
                    let args: Vec<Value> = fields
                        .get("args")
                        .and_then(Value::as_array)
                        .map_or_else(Vec::new, |args| {
                            args.iter().map(|arg| self.fill(arg)).collect()
                        });
                    return self.source(name, &args);
                }
                if let Some(Value::String(text)) = fields.get("$license") {
                    return Value::Bool(text.contains(self.settings.license.as_str()));
                }
                Value::Object(
                    fields
                        .iter()
                        .map(|(key, value)| (key.clone(), self.fill(value)))
                        .collect(),
                )
            }
            Value::Array(entries) => Value::Array(entries.iter().map(|v| self.fill(v)).collect()),
            other => other.clone(),
        }
    }

    /// The stored value of a setting as `GET /config` shows it to the caller, or the
    /// empty text for a name that is no setting, as PHP's `$this->config[$name] ?? ''`.
    fn config(&self, key: &Value) -> Value {
        key.as_str()
            .and_then(|key| {
                self.settings
                    .get(key)
                    .map(|value| shown(key, value, self.level))
            })
            .unwrap_or_else(|| Value::String(String::new()))
    }

    /// A list that PHP makes for each request.
    fn source(&self, name: &str, args: &[Value]) -> Value {
        match name {
            "groupOptions" => Value::Array(self.groups.clone()),
            "userOptions" => Value::Array(self.users.clone()),
            "makeOptionsFromValues" => options_from_values(
                args.first().unwrap_or(&Value::Null),
                args.get(1) == Some(&Value::Bool(true)),
                args.get(2).and_then(Value::as_str),
            ),
            "getIframeTabs" => self.frame_tabs(args.first().and_then(Value::as_str)),
            // PHP's `_getKomgaLibraries()`, with its one choice when Komga gives none.
            "komgaLibraries" => match &self.komga_libraries {
                Some(libraries) if !libraries.is_empty() => Value::Array(libraries.clone()),
                _ => {
                    json!([{"name": "Refresh page to update List", "value": "", "disabled": true}])
                }
            },
            "getAllThemes" => Value::Array(
                all_themes(self.settings)
                    .keys()
                    .map(|name| json!({ "name": name, "value": name }))
                    .collect(),
            ),
            _ => Value::Array(Vec::new()),
        }
    }

    /// PHP's `getIframeTabs()`: "Open in New Window", then the tabs that are on, show in
    /// a frame and are for the caller, with the given address when there is one.
    fn frame_tabs(&self, address: Option<&str>) -> Value {
        let address = address.filter(|address| !address.is_empty());
        let mut tabs = vec![json!({ "name": "Open in New Window", "value": "" })];
        tabs.extend(
            self.tabs
                .iter()
                .filter(|tab| tab.enabled && tab.kind == TabKind::Iframe)
                .filter(|tab| tab.access.least_access() >= self.level)
                .filter(|tab| {
                    address.is_none_or(|address| {
                        tab.url.as_deref() == Some(address)
                            || tab.url_local.as_deref() == Some(address)
                    })
                })
                .map(|tab| json!({ "name": tab.name, "value": tab.id.get() })),
        );
        Value::Array(tabs)
    }
}

/// PHP's `makeOptionsFromValues()`: one choice for each part of a text with commas, and
/// a first choice for "nothing" when `blank` is set. An empty text gives the empty text,
/// as in PHP.
fn options_from_values(values: &Value, blank: bool, blank_label: Option<&str>) -> Value {
    let text = match values {
        Value::String(text) => text.clone(),
        Value::Bool(true) => "1".to_owned(),
        Value::Number(number) => number.to_string(),
        _ => String::new(),
    };
    if text.is_empty() {
        return Value::String(String::new());
    }
    let mut options = Vec::new();
    if blank {
        let label = blank_label
            .filter(|label| !label.is_empty())
            .unwrap_or("Select option...");
        options.push(json!({ "name": label, "value": "" }));
    }
    options.extend(
        text.split(',')
            .map(|part| json!({ "name": part, "value": part })),
    );
    Value::Array(options)
}

/// A form with its placeholders filled. PHP writes an empty form as `[]`.
fn filled_form(fill: &Fill<'_>, form: &Value) -> Value {
    match fill.fill(form) {
        Value::Object(groups) if groups.is_empty() => Value::Array(Vec::new()),
        other => other,
    }
}

/// Answers with one of the five forms of the `/settings/*` routes.
async fn answer_route(
    state: &AppState,
    caller: &Caller,
    route: &str,
) -> Result<Envelope<Value>, Refusal> {
    caller.require(MANAGER)?;
    let settings = state.settings();
    let fill = Fill::new(state, &settings, caller).await?;
    Ok(Envelope::success(filled_form(&fill, &route_form(route))))
}

/// `GET /settings/appearance`: the form "Customize - Appearance".
pub async fn appearance(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    answer_route(&state, &caller, "appearance").await
}

/// `GET /settings/plugin`: the settings of the plugin marketplace.
pub async fn plugin(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    answer_route(&state, &caller, "plugin").await
}

/// `GET /settings/theme`: the settings of the theme marketplace.
pub async fn theme(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    answer_route(&state, &caller, "theme").await
}

/// `GET /settings/main`: the main settings.
pub async fn main(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    answer_route(&state, &caller, "main").await
}

/// `GET /settings/sso`: the settings of the single sign-on.
pub async fn sso(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    answer_route(&state, &caller, "sso").await
}

/// The fields of a homepage item that `GET /settings/homepage` lists.
const ITEM_INFO: [&str; 5] = ["name", "enabled", "image", "category", "settingsArray"];

/// `GET /settings/homepage`: the homepage items, each with its name, whether the license
/// has it, its picture, its category and the name of its form.
pub async fn homepage(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    caller.require(MANAGER)?;
    let settings = state.settings();
    let fill = Fill::new(&state, &settings, &caller).await?;
    let items = homepage_items()
        .iter()
        .map(|item| {
            let info: Map<String, Value> = ITEM_INFO
                .iter()
                .filter_map(|key| {
                    item.get(*key)
                        .map(|value| ((*key).to_owned(), fill.fill(value)))
                })
                .collect();
            Value::Object(info)
        })
        .collect();
    Ok(Envelope::success(Value::Array(items)))
}

/// The homepage item of a path, by its name in any letter case, filled.
async fn homepage_item(state: &AppState, caller: &Caller, item: &str) -> Result<Value, Refusal> {
    caller.require(MANAGER)?;
    let wanted = item.to_lowercase();
    let found = homepage_items()
        .iter()
        .find(|candidate| {
            part(candidate, "name")
                .as_str()
                .is_some_and(|name| name.to_lowercase() == wanted)
        })
        .ok_or_else(|| Refusal::not_found("Homepage item was not found"))?;
    let settings = state.settings();
    let fill = Fill::new(state, &settings, caller).await?;
    Ok(fill.fill(found))
}

/// `GET /settings/homepage/{item}`: the form of a homepage item.
pub async fn homepage_item_form(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(item): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    Ok(Envelope::success(
        homepage_item(&state, &caller, &item).await?,
    ))
}

/// The kinds of fields that the debug route leaves out: PHP leaves out its `html`,
/// `blank` and `button` fields, and the typed kinds here stand for its `html` fields.
const NOT_DEBUGGED: [&str; 8] = [
    "html",
    "blank",
    "button",
    "notice",
    "about",
    "socks",
    "cronfile",
    "code-editor",
];

/// Whether the debug route hides the value of a field. PHP hides a field whose name
/// holds `token`, `key` or `password`, but misses `password` at the start of the name.
/// Here every secret setting, every field of a password kind, every name with one of the
/// three words, and every address that holds a user, a password or a query, which can
/// hold a key, are hidden.
fn is_hidden_in_debug(name: &str, kind: &str, value: &Value) -> bool {
    let lower = name.to_ascii_lowercase();
    Settings::is_secret(name)
        || kind.starts_with("password")
        || ["token", "key", "password", "secret"]
            .iter()
            .any(|word| lower.contains(word))
        || value
            .as_str()
            .is_some_and(|text| text.split(',').any(may_hold_a_key))
}

/// Whether an address has a part that can hold a key: a user and a password before the
/// host, or a query.
fn may_hold_a_key(address: &str) -> bool {
    let Some((_, rest)) = address.trim().split_once("://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    authority.contains('@') || rest.contains('?')
}

/// `GET /settings/homepage/{item}/debug`: the values of the form of a homepage item,
/// by group and name, without the groups "About" and "Test Connection", as PHP sends
/// them. A value that may be secret is `***redacted***` when it is set.
pub async fn homepage_item_debug(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(item): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    let item = homepage_item(&state, &caller, &item).await?;
    let mut debug = Map::new();
    if let Some(groups) = part(&item, "settings").as_object() {
        for (title, fields) in groups {
            if title == "About" || title == "Test Connection" {
                continue;
            }
            let mut values = Map::new();
            for field in fields.as_array().map_or(&[][..], Vec::as_slice) {
                let kind = part(field, "type").as_str().unwrap_or_default();
                if NOT_DEBUGGED.contains(&kind) {
                    continue;
                }
                let name = part(field, "name").as_str().unwrap_or_default();
                let value = part(field, "value");
                let is_empty = value.as_str().is_some_and(str::is_empty);
                let shown_value = if is_hidden_in_debug(name, kind, value) && !is_empty {
                    Value::String(REDACTED.to_owned())
                } else {
                    value.clone()
                };
                values.insert(name.to_owned(), shown_value);
            }
            if !values.is_empty() {
                debug.insert(title.clone(), Value::Object(values));
            }
        }
    }
    Ok(Envelope::success(if debug.is_empty() {
        Value::Array(Vec::new())
    } else {
        Value::Object(debug)
    }))
}

/// Answers with the form of a plugin.
async fn answer_plugin(
    state: &AppState,
    caller: &Caller,
    plugin: &str,
) -> Result<Envelope<Value>, Refusal> {
    caller.require(MANAGER)?;
    let settings = state.settings();
    let fill = Fill::new(state, &settings, caller).await?;
    Ok(Envelope::success(filled_form(
        &fill,
        part(part(&TABLE, "plugins"), plugin),
    )))
}

/// `GET /plugins/bookmark/settings`. PHP's form holds only HTML, so it is empty here.
pub async fn bookmark_plugin(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    answer_plugin(&state, &caller, "bookmark").await
}

/// `GET /plugins/healthchecks/settings`.
pub async fn health_checks_plugin(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    answer_plugin(&state, &caller, "healthchecks").await
}

/// `GET /plugins/invites/settings`. The libraries of Komga come from Komga, as PHP asks
/// for them for each form.
pub async fn invites_plugin(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    caller.require(MANAGER)?;
    let settings = state.settings();
    let mut fill = Fill::new(&state, &settings, &caller).await?;
    fill.komga_libraries = komga_libraries(&state, &settings).await;
    Ok(Envelope::success(filled_form(
        &fill,
        part(part(&TABLE, "plugins"), "invites"),
    )))
}

/// The longest wait for the libraries of Komga.
const KOMGA_TIME: std::time::Duration = std::time::Duration::from_secs(10);

/// The libraries of the Komga of the invites plugin as choices, or nothing when it is not
/// set up or does not answer. PHP asks the address of single sign-on, `komgaURL`; the
/// address of the plugin, `INVITES-komga-uri`, comes first here, as its account goes
/// there too (D-118).
async fn komga_libraries(state: &AppState, settings: &Settings) -> Option<Vec<Value>> {
    let url = [&settings.invites_komga_uri, &settings.komga_url]
        .into_iter()
        .find(|url| !url.is_empty())?;
    if settings.invites_komga_api_key.is_empty() {
        return None;
    }
    let http = state.outbound()?;
    match organizr_integrations::komga::libraries(
        http,
        url.trim_end_matches('/'),
        &settings.invites_komga_api_key,
        KOMGA_TIME,
    )
    .await
    {
        Ok(libraries) => Some(
            libraries
                .into_iter()
                .map(|library| json!({ "name": library.name, "value": library.id }))
                .collect(),
        ),
        Err(error) => {
            tracing::warn!(%error, "the libraries of Komga cannot be read");
            None
        }
    }
}

/// `GET /plugins/php-mailer/settings`.
pub async fn mailer_plugin(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    answer_plugin(&state, &caller, "php-mailer").await
}

/// `GET /plugins/speedtest/settings`.
pub async fn speed_test_plugin(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    answer_plugin(&state, &caller, "speedtest").await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every value of the table, with the path to it.
    fn walk<'a>(value: &'a Value, path: &str, out: &mut Vec<(String, &'a Value)>) {
        out.push((path.to_owned(), value));
        match value {
            Value::Object(fields) => {
                for (key, inner) in fields {
                    walk(inner, &format!("{path}/{key}"), out);
                }
            }
            Value::Array(entries) => {
                for (index, inner) in entries.iter().enumerate() {
                    walk(inner, &format!("{path}/{index}"), out);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn the_table_can_be_read() {
        assert!(TABLE["routes"]["main"].is_object());
        assert_eq!(homepage_items().len(), 40);
        assert!(TABLE["plugins"]["invites"].is_object());
    }

    #[test]
    fn every_setting_of_the_table_is_in_the_registry() {
        let mut values = Vec::new();
        walk(&TABLE, "", &mut values);
        let mut checked = 0;
        for (path, value) in values {
            if let Some(Value::String(key)) = value.get("$config") {
                assert!(Settings::KEYS.contains(&key.as_str()), "{path}: {key}");
                checked += 1;
            }
            if let Some(Value::String(name)) = value.get("name")
                && value
                    .get("value")
                    .is_some_and(|v| v.get("$config").is_some())
            {
                assert!(Settings::KEYS.contains(&name.as_str()), "{path}: {name}");
            }
        }
        assert!(checked > 800, "{checked}");
    }

    #[test]
    fn the_table_holds_no_html_and_no_attr() {
        let mut values = Vec::new();
        walk(&TABLE, "", &mut values);
        for (path, value) in values {
            if let Some(fields) = value.as_object() {
                for key in ["html", "attr", "bodyHTML"] {
                    assert!(!fields.contains_key(key), "{path} has {key}");
                }
                assert_ne!(fields.get("type"), Some(&json!("html")), "{path}");
                for key in fields.keys() {
                    assert!(
                        !key.starts_with('$')
                            || ["$config", "$source", "$license"].contains(&key.as_str()),
                        "{path}: {key}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_shuck_stop_plugin_is_not_in_the_table() {
        assert!(
            !include_str!("settings_forms.json")
                .to_lowercase()
                .contains("shuck")
        );
    }

    #[test]
    fn options_from_values_follow_php() {
        assert_eq!(options_from_values(&json!(""), true, None), json!(""));
        assert_eq!(
            options_from_values(&json!("a,b"), false, None),
            json!([{"name": "a", "value": "a"}, {"name": "b", "value": "b"}])
        );
        assert_eq!(
            options_from_values(&json!("a"), true, Some("Use Default")),
            json!([{"name": "Use Default", "value": ""}, {"name": "a", "value": "a"}])
        );
    }

    #[test]
    fn the_debug_route_hides_what_php_misses() {
        for (name, kind, value) in [
            ("passwordOfSomething", "input", "x"),
            ("sonarrToken", "select2", "abc"),
            ("plexID", "password-alt", "abc"),
            ("organizrAPI", "password-alt-copy", "abc"),
            ("sonarrURL", "select2", "http://a:b@host:8989"),
            (
                "sonarrURL",
                "select2",
                "http://host:8989,https://host/api?apikey=1",
            ),
        ] {
            assert!(
                is_hidden_in_debug(name, kind, &json!(value)),
                "{name} {value}"
            );
        }
        assert!(!is_hidden_in_debug(
            "sonarrURL",
            "select2",
            &json!("http://host:8989/sonarr")
        ));
        assert!(!is_hidden_in_debug("calendarStart", "number", &json!("14")));
    }
}
