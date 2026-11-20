//! `GET /page`, `GET /page/{page}` and `GET /organizr/{page}[/{var1}[/{var2}]]`: the
//! pages of the interface (D-22).
//!
//! The PHP application builds each page as HTML on the server. Here the interface
//! builds its pages, so a page name answers with the place of the matching page in the
//! interface, after the checks that the PHP page makes: whether the install is set up,
//! and for a settings page, whether the caller is an Admin or a Co-Admin.

use axum::Extension;
use axum::extract::{Path, RawPathParams, State};
use organizr_domain::GroupLevel;
use serde_json::Value;

use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The page names of the PHP application, in the order of `GET /page`. The name
/// `settings_wizard` is on the list of PHP, but its page is named `wizard`.
pub const PAGE_NAMES: [&str; 26] = [
    "dependencies",
    "error",
    "homepage",
    "lockscreen",
    "login",
    "settings_customize_appearance",
    "settings_customize_settings",
    "settings_image_manager",
    "settings_plugins_disabled",
    "settings_plugins_enabled",
    "settings_plugins_settings",
    "settings_plugins",
    "settings_settings_backup",
    "settings_settings_logs",
    "settings_settings_main",
    "settings_settings_sso",
    "settings_tab_editor_categories",
    "settings_tab_editor_homepage_order",
    "settings_tab_editor_homepage",
    "settings_tab_editor_tabs",
    "settings_template",
    "settings_user_manage_groups",
    "settings_user_manage_users",
    "settings",
    "tabs",
    "settings_wizard",
];

/// What a PHP page needs before it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Needs {
    /// Nothing.
    Nothing,
    /// An install that is set up.
    SetUp,
    /// An install that is set up, and an Admin or a Co-Admin.
    SetUpAndCoAdmin,
}

/// The place in the interface of a PHP page, and what the page needs.
fn page(name: &str) -> Option<(&'static str, Needs)> {
    use Needs::{Nothing, SetUp, SetUpAndCoAdmin};
    Some(match name {
        "dependencies" | "tabs" | "wizard" => ("#/", Nothing),
        "error" | "homepage" | "lockscreen" => ("#/", SetUp),
        "login" => ("#/login", SetUp),
        "settings"
        | "settings_customize_appearance"
        | "settings_customize_settings"
        | "settings_settings_main"
        | "settings_template" => ("#/settings/general", SetUpAndCoAdmin),
        "settings_image_manager" | "settings_tab_editor_tabs" => {
            ("#/settings/tabs", SetUpAndCoAdmin)
        }
        "settings_plugins"
        | "settings_plugins_disabled"
        | "settings_plugins_enabled"
        | "settings_plugins_settings" => ("#/settings/plugins", SetUpAndCoAdmin),
        "settings_settings_backup" => ("#/settings/backups", SetUpAndCoAdmin),
        "settings_settings_logs" => ("#/settings/logs", SetUpAndCoAdmin),
        "settings_settings_sso" => ("#/settings/login", SetUpAndCoAdmin),
        "settings_tab_editor_categories" => ("#/settings/categories", SetUpAndCoAdmin),
        "settings_tab_editor_homepage" | "settings_tab_editor_homepage_order" => {
            ("#/settings/homepage", SetUpAndCoAdmin)
        }
        "settings_user_manage_groups" => ("#/settings/groups", SetUpAndCoAdmin),
        "settings_user_manage_users" => ("#/settings/users", SetUpAndCoAdmin),
        _ => return None,
    })
}

/// A page name as `cleanPageName` of PHP writes it: in small letters, with `_` in the
/// place of each space, `-` and `%20`.
fn clean_name(name: &str) -> String {
    name.replace("%20", "_")
        .replace([' ', '-'], "_")
        .to_ascii_lowercase()
}

/// `GET /page`: the names of the pages.
pub async fn names() -> Envelope<[&'static str; 26]> {
    Envelope::success(PAGE_NAMES)
}

/// The answer for a page name. A page that the install or the caller does not meet
/// answers as PHP answers then: `data` is `null`, or the caller gets `401`.
async fn answer(state: &AppState, caller: &Caller, name: &str) -> Result<Envelope<Value>, Refusal> {
    let Some((route, needs)) = page(&clean_name(name)) else {
        return Err(Refusal::conflict("Page not setup"));
    };
    if needs != Needs::Nothing && !state.database().install().is_done().await? {
        return Ok(Envelope::success(Value::Null));
    }
    if needs == Needs::SetUpAndCoAdmin {
        caller.require(GroupLevel::CO_ADMIN)?;
    }
    Ok(Envelope::success(Value::from(route)))
}

/// `GET /page/{page}`: the place of a page in the interface.
pub async fn one(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(name): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    answer(&state, &caller, &name).await
}

/// `GET /organizr/{page}` with up to two more parts, which are left alone: the same
/// answer as `GET /page/{page}`. PHP sends the HTML of the page here, not the envelope.
pub async fn raw(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    parts: RawPathParams,
) -> Result<Envelope<Value>, Refusal> {
    // The first part is the page.
    let name = parts.iter().next().map_or("", |(_, value)| value);
    answer(&state, &caller, name).await
}

#[cfg(test)]
mod tests {
    use super::{PAGE_NAMES, clean_name, page};

    #[test]
    fn every_page_of_the_list_but_the_wizard_has_a_place() {
        for name in PAGE_NAMES {
            assert_eq!(page(name).is_some(), name != "settings_wizard", "{name}");
        }
        assert!(page("wizard").is_some());
        assert!(page("nothing").is_none());
    }

    #[test]
    fn name_is_cleaned_as_php_cleans_it() {
        assert_eq!(
            clean_name("Settings-Tab Editor%20Tabs"),
            "settings_tab_editor_tabs"
        );
    }
}
