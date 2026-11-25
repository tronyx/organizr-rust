//! The plugin manager of PHP Organizr: the lists of the built-in plugins, and the plugin
//! marketplace.
//!
//! The built-in plugins are compiled into this application, so the lists hold their
//! details as the `plugin.php` files of PHP Organizr 2.1.5000 give them. The marketplace
//! of PHP lists plugins that are PHP code, which this application cannot run, so it is
//! empty here: the extensions of D-75 have their own marketplace. `docs/deviations.md`
//! lists the differences as D-94.

use axum::Extension;
use axum::extract::{Path, State};
use organizr_domain::GroupLevel;
use organizr_settings::Settings;
use serde_json::{Map, Value, json};

use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The level of the marketplace routes: Admins and Co-Admins, as `qualifyRequest(1)`.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// A built-in plugin, with the fields of its `$GLOBALS['plugins']` entry in PHP.
struct BuiltIn {
    /// The key of the plugin in the list.
    key: &'static str,
    author: &'static str,
    category: &'static str,
    link: &'static str,
    license: &'static str,
    /// The prefix of the keys of its settings, `<prefix>-enabled` among them.
    prefix: &'static str,
    /// Only the bookmark plugin has a prefix for its tables.
    db_prefix: Option<&'static str>,
    version: &'static str,
    /// The folder of the plugin in PHP, for the path of its picture.
    folder: &'static str,
    bind: bool,
    /// The route of its settings form in PHP, after `api/v2/`, when it has one. The list
    /// names it as PHP does, though this application does not build it (D-96).
    settings_route: Option<&'static str>,
}

/// The built-in plugins of PHP Organizr 2.1.5000, in the order of PHP's `asort()`: PHP
/// compares the entries as arrays, so the entry with one more field, the bookmark
/// plugin, comes last, and the others go by name. The shuck-stop plugin of PHP is not
/// part of this application (D-94).
const BUILT_IN: [BuiltIn; 7] = [
    BuiltIn {
        key: "Chat",
        author: "CauseFX",
        category: "Utilities",
        link: "",
        license: "personal,business",
        prefix: "CHAT",
        db_prefix: None,
        version: "1.0.0",
        folder: "chat",
        bind: true,
        settings_route: Some("plugins/chat/settings"),
    },
    BuiltIn {
        key: "HealthChecks",
        author: "CauseFX",
        category: "Utilities",
        link: "",
        license: "personal,business",
        prefix: "HEALTHCHECKS",
        db_prefix: None,
        version: "1.0.0",
        folder: "healthChecks",
        bind: false,
        settings_route: None,
    },
    BuiltIn {
        key: "Invites",
        author: "CauseFX",
        category: "Management",
        link: "",
        license: "personal",
        prefix: "INVITES",
        db_prefix: None,
        version: "1.1.0",
        folder: "invites",
        bind: true,
        settings_route: Some("plugins/invites/settings"),
    },
    BuiltIn {
        key: "PHP Mailer",
        author: "PHP Mailer",
        category: "Mail",
        link: "https://github.com/PHPMailer/PHPMailer",
        license: "personal,business",
        prefix: "PHPMAILER",
        db_prefix: None,
        version: "1.0.0",
        folder: "php-mailer",
        bind: true,
        settings_route: Some("plugins/php-mailer/settings"),
    },
    BuiltIn {
        key: "SpeedTest",
        author: "CauseFX",
        category: "Utilities",
        link: "",
        license: "personal,business",
        prefix: "SPEEDTEST",
        db_prefix: None,
        version: "1.0.0",
        folder: "speedTest",
        bind: true,
        settings_route: Some("plugins/speedtest/settings"),
    },
    BuiltIn {
        key: "Bookmark",
        author: "leet1994",
        category: "Utilities",
        link: "",
        license: "personal,business",
        prefix: "BOOKMARK",
        db_prefix: Some("BOOKMARK"),
        version: "0.1.0",
        folder: "bookmark",
        bind: true,
        settings_route: Some("plugins/bookmark/settings"),
    },
    // The Plex Libraries plugin of this application, which PHP has not built in (D-121).
    BuiltIn {
        key: "Plex Libraries",
        author: "Organizr Rust",
        category: "Management",
        link: "",
        license: "personal,business",
        prefix: "PLEXLIBRARIES",
        db_prefix: None,
        version: "1.0.0",
        folder: "plexLibraries",
        bind: true,
        // PHP has no form for it, and this application builds no form route (D-96).
        settings_route: None,
    },
];

/// Which plugins a list holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Which {
    All,
    Enabled,
    Disabled,
}

/// The entry of a plugin, with the fields in the order of PHP and `enabled` last.
fn entry(plugin: &BuiltIn, enabled: bool) -> Value {
    let mut fields = Map::new();
    fields.insert("name".to_owned(), json!(plugin.key));
    fields.insert("author".to_owned(), json!(plugin.author));
    fields.insert("category".to_owned(), json!(plugin.category));
    fields.insert("link".to_owned(), json!(plugin.link));
    fields.insert("license".to_owned(), json!(plugin.license));
    fields.insert("idPrefix".to_owned(), json!(plugin.prefix));
    fields.insert("configPrefix".to_owned(), json!(plugin.prefix));
    if let Some(db_prefix) = plugin.db_prefix {
        fields.insert("dbPrefix".to_owned(), json!(db_prefix));
    }
    fields.insert("version".to_owned(), json!(plugin.version));
    fields.insert(
        "image".to_owned(),
        json!(format!("api/plugins/{}/logo.png", plugin.folder)),
    );
    fields.insert("settings".to_owned(), Value::Bool(true));
    fields.insert("bind".to_owned(), Value::Bool(plugin.bind));
    fields.insert(
        "api".to_owned(),
        plugin
            .settings_route
            .map_or(Value::Bool(false), |route| json!(format!("api/v2/{route}"))),
    );
    fields.insert("homepage".to_owned(), Value::Bool(false));
    fields.insert("enabled".to_owned(), Value::Bool(enabled));
    Value::Object(fields)
}

/// The plugins of a list, as PHP's `getPlugins()` sends them: only those whose license
/// holds the setting `license` (every plugin while the setting is empty), keyed by name.
fn plugins(settings: &Settings, which: Which) -> Map<String, Value> {
    BUILT_IN
        .iter()
        .filter(|plugin| plugin.license.contains(settings.license.as_str()))
        .filter_map(|plugin| {
            let enabled = matches!(
                settings.get(&format!("{}-enabled", plugin.prefix)),
                Some(Value::Bool(true))
            );
            let wanted = match which {
                Which::All => true,
                Which::Enabled => enabled,
                Which::Disabled => !enabled,
            };
            wanted.then(|| (plugin.key.to_owned(), entry(plugin, enabled)))
        })
        .collect()
}

/// `GET /plugins`: every built-in plugin. PHP checks no level.
pub async fn all(State(state): State<AppState>) -> Envelope<Map<String, Value>> {
    Envelope::success(plugins(&state.settings(), Which::All))
}

/// `GET /plugins/enabled`: the built-in plugins that are switched on.
pub async fn enabled(State(state): State<AppState>) -> Envelope<Map<String, Value>> {
    Envelope::success(plugins(&state.settings(), Which::Enabled))
}

/// `GET /plugins/disabled`: the built-in plugins that are switched off.
pub async fn disabled(State(state): State<AppState>) -> Envelope<Map<String, Value>> {
    Envelope::success(plugins(&state.settings(), Which::Disabled))
}

/// `GET /plugins/marketplace`: the plugins of the marketplace of PHP Organizr that this
/// application can install. None is: they are PHP code.
pub async fn marketplace(
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Map<String, Value>>, Refusal> {
    caller.require(MANAGER)?;
    Ok(Envelope::success(Map::new()))
}

/// `POST /plugins/manage/{plugin}` and `DELETE /plugins/manage/{plugin}`: the install
/// and the removal of a marketplace plugin. The marketplace is empty, so every name gets
/// the answer of PHP for a plugin that it does not list.
pub async fn manage(
    Extension(caller): Extension<Caller>,
    Path(_plugin): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(MANAGER)?;
    Err(Refusal::not_found("Plugin does not exist in marketplace"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_order_is_that_of_php_asort() {
        let names: Vec<&str> = BUILT_IN.iter().map(|plugin| plugin.key).collect();
        assert_eq!(
            names,
            [
                "Chat",
                "HealthChecks",
                "Invites",
                "PHP Mailer",
                "SpeedTest",
                "Bookmark",
                // Not a plugin of PHP: it comes after them (D-121).
                "Plex Libraries"
            ]
        );
    }

    #[test]
    fn every_enabled_key_is_a_setting() {
        for plugin in &BUILT_IN {
            let key = format!("{}-enabled", plugin.prefix);
            assert!(Settings::KEYS.contains(&key.as_str()), "{key}");
        }
    }

    #[test]
    fn a_business_license_leaves_out_the_invites() {
        let mut settings = Settings::defaults();
        assert_eq!(plugins(&settings, Which::All).len(), 7);
        settings.license = "business".to_owned();
        let names: Vec<String> = plugins(&settings, Which::All).keys().cloned().collect();
        assert!(!names.contains(&"Invites".to_owned()), "{names:?}");
        assert_eq!(names.len(), 6);
    }
}
