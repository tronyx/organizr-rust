//! `GET /launch`: what the front end loads first.
//!
//! The answer holds the caller, the tabs and categories the caller sees, how the page
//! looks, and the settings that a page needs. The module `launch_settings` builds the
//! parts `settings`, `plugins` and `sso`.

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, header};
use organizr_domain::{Category, GroupLevel, Tab};
use organizr_settings::{Settings, UnsortedTabs};
use serde::Serialize;
use serde_json::Value;

use crate::caller::Caller;
use crate::client::{Client, LocalNetworks};
use crate::envelope::Envelope;
use crate::error::ApiError;
use crate::handlers::launch_settings::Visitor;
use crate::handlers::tabs::{CategoryRow, PageAddress, TabRow, TabVariables};
use crate::handlers::token::{SessionSummary, UserObject, describe_caller};
use crate::state::AppState;

/// The version of this application.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The PHP extensions and functions that the PHP application lists as present. A client
/// that reads the list finds what it found before.
const DEPENDENCIES: [&str; 18] = [
    "PDO_SQLITE",
    "PDO",
    "SQLITE3",
    "zip",
    "cURL",
    "openssl",
    "simplexml",
    "json",
    "session",
    "filter",
    "fileinfo",
    "mbstring",
    "hash",
    "fopen",
    "fsockopen",
    "fwrite",
    "fclose",
    "readfile",
];

/// The `data` of the launch answer. The field names and their order are those of the PHP
/// application.
#[derive(Debug, Serialize)]
pub struct LaunchData {
    categories: ListOrFalse<CategoryObject>,
    tabs: ListOrFalse<TabObject>,
    user: Option<UserObject<SessionSummary>>,
    branch: String,
    theme: String,
    style: String,
    version: &'static str,
    settings: Value,
    plugins: Value,
    appearance: Appearance,
    status: LaunchStatus,
    sso: Value,
    /// Warnings of the server for the admin, for example about a full disk. None yet.
    warnings: Option<Vec<String>>,
}

/// A list, or `false` on an install that is not set up, as PHP sends it.
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ListOrFalse<T> {
    List(Vec<T>),
    NotSetUp(bool),
}

/// A category of the menu: the category as the API shows it, and a count.
#[derive(Debug, Serialize)]
struct CategoryObject {
    #[serde(flatten)]
    category: CategoryRow,
    /// How many of the tabs in the answer are in this category.
    count: usize,
}

/// A tab of the menu: the tab as the API shows it, and the address to open.
#[derive(Debug, Serialize)]
struct TabObject {
    #[serde(flatten)]
    tab: TabRow,
    /// The address the front end opens: the local address of the tab for a caller on
    /// the local network, when the tab has one, and else the address.
    access_url: Option<String>,
}

/// How the page looks. All values are settings.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Appearance {
    logo: String,
    title: String,
    use_logo: bool,
    use_logo_login: bool,
    header_color: String,
    header_text_color: String,
    sidebar_color: String,
    sidebar_text_color: String,
    accent_color: String,
    accent_text_color: String,
    button_color: String,
    button_text_color: String,
    button_text_hover_color: String,
    button_hover_color: String,
    login_wallpaper: String,
    /// A picture from a media server for the login page. Not built yet.
    random_media_image: bool,
    login_logo: String,
    custom_css: String,
    custom_theme_css: String,
    custom_java: String,
    custom_theme_java: String,
}

/// The state of the install. Most values exist for clients of the PHP application and
/// are fixed here.
#[derive(Debug, Serialize)]
struct LaunchStatus {
    /// `ok`, or `wizard` on an install that is not set up.
    status: &'static str,
    writable: &'static str,
    #[serde(rename = "minVersion")]
    min_version: &'static str,
    #[serde(rename = "dependenciesActive")]
    dependencies_active: [&'static str; 18],
    #[serde(rename = "dependenciesInactive")]
    dependencies_inactive: [&'static str; 0],
    version: &'static str,
    os: &'static str,
    php: &'static str,
    php_user: &'static str,
}

/// Whether a caller of the given level sees a tab in the menu.
///
/// A tab is shown to the levels from `group_id_max` to `group_id`. A tab that is marked
/// "add to admin" is also shown to Admins and Co-Admins, whatever its levels.
fn is_shown(tab: &Tab, level: GroupLevel) -> bool {
    tab.enabled
        && (tab.access.contains(level) || (tab.add_to_admin && level <= GroupLevel::CO_ADMIN))
}

/// The tabs a caller of the given level sees, in the order of the menu.
///
/// The PHP application sorts by `order`, downwards when the `unsortedTabs` setting is
/// `top` and upwards when it is `bottom`. Tabs with the same `order` follow their id,
/// in both directions. Among them, the tabs that an admin sees only because they are
/// marked "add to admin" come last. This is the order that the two queries of the PHP
/// application and its sort give.
fn visible_tabs(mut tabs: Vec<Tab>, level: GroupLevel, unsorted: UnsortedTabs) -> Vec<Tab> {
    tabs.retain(|tab| is_shown(tab, level));
    tabs.sort_by_key(|tab| tab.id);
    let (mut shown, for_admins_only): (Vec<Tab>, Vec<Tab>) =
        tabs.into_iter().partition(|tab| tab.access.contains(level));
    shown.extend(for_admins_only);
    // The sort keeps the order above for tabs with the same place.
    match unsorted {
        UnsortedTabs::Top => shown.sort_by_key(|tab| std::cmp::Reverse(tab.position)),
        UnsortedTabs::Bottom => shown.sort_by_key(|tab| tab.position),
    }
    shown
}

fn tab_object(
    tab: Tab,
    categories: &[Category],
    variables: &TabVariables,
    page: &PageAddress<'_>,
) -> TabObject {
    let (tab, access_url) = TabRow::of(tab, categories)
        .filled_in(variables)
        .for_menu(page);
    TabObject { tab, access_url }
}

fn appearance(settings: &Settings) -> Appearance {
    Appearance {
        logo: settings.logo.clone(),
        title: settings.title.clone(),
        use_logo: settings.use_logo,
        use_logo_login: settings.use_logo_login,
        header_color: settings.header_color.clone(),
        header_text_color: settings.header_text_color.clone(),
        sidebar_color: settings.sidebar_color.clone(),
        sidebar_text_color: settings.sidebar_text_color.clone(),
        accent_color: settings.accent_color.clone(),
        accent_text_color: settings.accent_text_color.clone(),
        button_color: settings.button_color.clone(),
        button_text_color: settings.button_text_color.clone(),
        button_text_hover_color: settings.button_text_hover_color.clone(),
        button_hover_color: settings.button_hover_color.clone(),
        login_wallpaper: settings.login_wallpaper.clone(),
        random_media_image: false,
        login_logo: settings.login_logo.clone(),
        custom_css: settings.custom_css.clone(),
        custom_theme_css: settings.custom_theme_css.clone(),
        custom_java: settings.custom_java.clone(),
        custom_theme_java: settings.custom_theme_java.clone(),
    }
}

/// `GET /launch`: the caller, the menu of the caller, and how the page looks.
///
/// Any caller may ask. A guest gets the tabs that guests may see, which on a new install
/// are none. On an install that is not set up, `categories` and `tabs` are `false` and
/// `status.status` is `wizard`.
pub async fn launch(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    Extension(caller): Extension<Caller>,
    headers: HeaderMap,
) -> Result<Envelope<LaunchData>, ApiError> {
    let settings = state.settings();
    let database = state.database();
    let is_set_up = database.install().is_done().await?;
    let variables = TabVariables::of(&caller, &headers);
    let caller_is_local = LocalNetworks::from_setting(&settings.local_ip_list).contains(&client);
    let visitor = Visitor {
        settings: &settings,
        level: caller.level(),
        client: &client,
        is_local: caller_is_local,
        headers: &headers,
    };
    let page = PageAddress {
        https: client.https,
        host: headers
            .get(header::HOST)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default(),
        caller_is_local,
    };

    let (categories, tabs) = if is_set_up {
        let categories = database.categories().list().await?;
        let tabs = visible_tabs(
            database.tabs().list().await?,
            caller.level(),
            settings.unsorted_tabs,
        );
        let category_objects = categories
            .iter()
            .map(|category| CategoryObject {
                category: CategoryRow::from(category),
                count: tabs
                    .iter()
                    .filter(|tab| tab.category == category.id)
                    .count(),
            })
            .collect();
        let tab_objects = tabs
            .into_iter()
            .map(|tab| tab_object(tab, &categories, &variables, &page))
            .collect();
        (
            ListOrFalse::List(category_objects),
            ListOrFalse::List(tab_objects),
        )
    } else {
        (ListOrFalse::NotSetUp(false), ListOrFalse::NotSetUp(false))
    };

    Ok(Envelope::success(LaunchData {
        categories,
        tabs,
        user: describe_caller(&state, &caller, client).await?,
        branch: settings.branch.clone(),
        theme: settings.theme.clone(),
        style: settings.style.clone(),
        version: VERSION,
        settings: visitor.settings(),
        plugins: visitor.plugins(),
        appearance: appearance(&settings),
        status: LaunchStatus {
            status: if is_set_up { "ok" } else { "wizard" },
            writable: "yes",
            min_version: "yes",
            dependencies_active: DEPENDENCIES,
            dependencies_inactive: [],
            version: VERSION,
            os: if cfg!(windows) { "win" } else { "*nix" },
            php: "n/a",
            php_user: "n/a",
        },
        sso: visitor.sso_cookies(),
        warnings: None,
    }))
}

#[cfg(test)]
mod tests {
    use organizr_domain::{CategoryId, GroupLevel, LevelRange, Tab, TabId, TabKind};
    use organizr_settings::UnsortedTabs;

    use super::{is_shown, visible_tabs};

    fn level(value: u16) -> GroupLevel {
        GroupLevel::new(value).expect("the level is valid")
    }

    /// A tab that the levels from `most_access` to `least_access` see.
    fn tab(id: i64, position: i64, least_access: u16, most_access: u16) -> Tab {
        Tab {
            id: TabId::new(id),
            position,
            category: CategoryId::new(1),
            name: format!("Tab {id}"),
            url: None,
            url_local: None,
            is_default: false,
            enabled: true,
            access: LevelRange::new(level(least_access), level(most_access))
                .expect("the range is valid"),
            add_to_admin: false,
            image: None,
            kind: TabKind::Iframe,
            splash: false,
            ping: false,
            ping_url: None,
            timeout: false,
            timeout_ms: None,
            preload: false,
        }
    }

    #[test]
    fn tab_is_shown_to_the_levels_of_its_range() {
        let for_users = tab(1, 1, 4, 0);

        for allowed in [0, 1, 4] {
            assert!(is_shown(&for_users, level(allowed)), "level {allowed}");
        }
        for refused in [5, 998, 999] {
            assert!(!is_shown(&for_users, level(refused)), "level {refused}");
        }
    }

    #[test]
    fn tab_for_guests_only_is_not_shown_to_others() {
        // "Guest" as the level with the least access and as the level with the most.
        let for_guests = tab(1, 1, 999, 999);

        assert!(is_shown(&for_guests, level(999)));
        assert!(!is_shown(&for_guests, level(4)));
        assert!(!is_shown(&for_guests, level(0)));
    }

    #[test]
    fn tab_that_is_not_enabled_is_shown_to_nobody() {
        let mut disabled = tab(1, 1, 999, 0);
        disabled.enabled = false;
        disabled.add_to_admin = true;

        for any in [0, 1, 4, 999] {
            assert!(!is_shown(&disabled, level(any)), "level {any}");
        }
    }

    #[test]
    fn tab_marked_add_to_admin_is_also_shown_to_admins_and_co_admins() {
        let mut for_guests = tab(1, 1, 999, 999);
        for_guests.add_to_admin = true;

        assert!(is_shown(&for_guests, level(0)));
        assert!(is_shown(&for_guests, level(1)));
        assert!(!is_shown(&for_guests, level(2)), "a Super User is no admin");
        assert!(is_shown(&for_guests, level(999)));
    }

    #[test]
    fn menu_order_depends_on_the_unsorted_tabs_setting() {
        let tabs = || {
            vec![
                tab(1, 2, 999, 0),
                tab(2, 1, 999, 0),
                tab(3, 3, 0, 0),
                tab(4, 2, 999, 0),
            ]
        };
        let ids = |unsorted| -> Vec<i64> {
            visible_tabs(tabs(), level(4), unsorted)
                .iter()
                .map(|tab| tab.id.get())
                .collect()
        };

        // Tab 3 is for Admins only. Tabs 1 and 4 have the same place and follow their
        // id, in both directions.
        assert_eq!(ids(UnsortedTabs::Bottom), [2, 1, 4]);
        assert_eq!(ids(UnsortedTabs::Top), [1, 4, 2]);
    }

    #[test]
    fn tab_that_an_admin_sees_only_by_its_mark_comes_last_among_tabs_of_one_place() {
        let mut for_guests_only = tab(1, 5, 999, 999);
        for_guests_only.add_to_admin = true;
        let mut for_users = tab(2, 5, 4, 2);
        for_users.add_to_admin = true;
        let tabs = || {
            vec![
                for_guests_only.clone(),
                for_users.clone(),
                tab(3, 5, 999, 0),
                tab(4, 6, 999, 0),
                tab(5, 5, 4, 0),
            ]
        };
        let ids = |level_of_caller, unsorted| -> Vec<i64> {
            visible_tabs(tabs(), level(level_of_caller), unsorted)
                .iter()
                .map(|tab| tab.id.get())
                .collect()
        };

        // An Admin is outside the levels of tabs 1 and 2, and sees them by their mark.
        assert_eq!(ids(0, UnsortedTabs::Top), [4, 3, 5, 1, 2]);
        assert_eq!(ids(0, UnsortedTabs::Bottom), [3, 5, 1, 2, 4]);
        // A user of level 4 is inside the levels of tab 2.
        assert_eq!(ids(4, UnsortedTabs::Top), [4, 2, 3, 5]);
    }
}
