//! The changes of this version that need the attention of the Admin of an imported
//! install: what worked in PHP Organizr and now works differently for this install.
//!
//! Each check looks at the imported records and settings, so the list names only what
//! applies to this install. `docs/moving-from-php.md` lists every such change, also the
//! ones that the data of an install doesn't show, such as the forward auth of a proxy.

use organizr_db::ImportedUser;
use organizr_domain::Group;
use organizr_settings::Settings;
use serde_json::Value;

use crate::report::escape_controls;

/// The settings that end like a service address but hold an address for the browser,
/// which may stay a path (D-137).
const BROWSER_ADDRESSES: [&str; 5] = [
    "authProxyLogoutURL",
    "embyTabURL",
    "jellyfinSSOURL",
    "oidcAutoRedirectLogoutUrl",
    "plexTabURL",
];

/// Whether a setting holds the address of a service that Organizr sends requests to.
fn is_service_address(key: &str) -> bool {
    let looks_like_one = key == "authBackendHost"
        || ["URL", "Url", "-uri", "-url"]
            .iter()
            .any(|end| key.ends_with(end));
    looks_like_one && !BROWSER_ADDRESSES.contains(&key)
}

/// The settings of service addresses that are only a path, such as `/sonarr`.
fn path_only_addresses(settings: &Settings) -> Vec<String> {
    Settings::KEYS
        .iter()
        .filter(|key| is_service_address(key))
        .filter_map(|key| match settings.get(key) {
            Some(Value::String(address)) if address.trim_start().starts_with('/') => {
                Some(format!("`{key}` ({})", escape_controls(address.trim())))
            }
            _ => None,
        })
        .collect()
}

/// The names of the users in the groups of a level, in the order of the import.
fn users_of_level(groups: &[Group], users: &[ImportedUser], level: u16) -> Vec<String> {
    users
        .iter()
        .filter(|imported| {
            groups
                .iter()
                .any(|group| group.id == imported.user.group && group.level.get() == level)
        })
        .map(|imported| escape_controls(&imported.user.username))
        .collect()
}

/// The changes of this version that apply to an install with these records.
pub fn breaking_changes(
    groups: &[Group],
    users: &[ImportedUser],
    settings: &Settings,
) -> Vec<String> {
    let mut changes = Vec::new();

    let paths = path_only_addresses(settings);
    if !paths.is_empty() {
        changes.push(format!(
            "Service addresses that are only a path work only on the hosts in Allowed Hosts \
             (Settings › System Settings › Login & Security, or ORGANIZR_ALLOWED_HOSTS). \
             While the list is empty, the first Admin login stores the host it used; add any \
             other host that you reach Organizr at (D-154, D-155). These settings hold such \
             an address: {}.",
            paths.join(", ")
        ));
    }

    let privileged: Vec<String> = users
        .iter()
        .filter(|imported| imported.login.auth_service == "oidc")
        .filter(|imported| {
            groups
                .iter()
                .any(|group| group.id == imported.user.group && group.level.get() <= 1)
        })
        .map(|imported| escape_controls(&imported.user.username))
        .collect();
    if !privileged.is_empty() {
        changes.push(format!(
            "These Admin and Co-Admin accounts are linked to OpenID Connect: {}. An OpenID \
             Connect login opens such an account only when the provider sends the account's \
             own e-mail address, marked as checked. Otherwise, log in with the password once \
             and link the account again in your settings (D-148, D-156).",
            privileged.join(", ")
        ));
    }

    let co_admins = users_of_level(groups, users, 1);
    if !co_admins.is_empty() {
        changes.push(format!(
            "These accounts are Co-Admins: {}. Only an Admin now changes trusted custom HTML \
             items, the sign-in settings, OpenID Connect, the services of the health checks, \
             single sign-on, the allowed hosts and trusted proxies, the mail server and the              password-reset e-mail, and the accounts of other Co-Admins (D-131 to D-135,              D-155, D-188, D-197).",
            co_admins.join(", ")
        ));
    }

    if settings.sso_komga && !settings.komga_sso_master_password.is_empty() {
        changes.push(
            "Single sign-on to Komga uses the master password. After a Plex login, Organizr \
             asks Komga with the e-mail address of the Plex account, not of the Organizr \
             account, so a user whose Komga account has another address gets no Komga session \
             (D-150)."
                .to_owned(),
        );
    }

    if settings.homepage_donate_show_user_history {
        changes.push(
            "The Donate item shows each user's history. It holds only the payments that this \
             version marked with the account, so payments made before the move don't show \
             (D-152)."
                .to_owned(),
        );
    }

    if settings.mailer_enabled && settings.mailer_domain.trim().is_empty() {
        changes.push(
            "E-mails name the site from the setting Domain of the e-mail settings, which is \
             empty. The first Admin login fills it in with the address it used; check it \
             afterward (D-138)."
                .to_owned(),
        );
    }

    changes
}

#[cfg(test)]
mod tests {
    use organizr_db::{ImportedUser, UserLogin};
    use organizr_domain::{Group, GroupId, GroupLevel, PasswordHash, Timestamp, User, UserId};
    use organizr_settings::Settings;
    use serde_json::json;

    use super::{breaking_changes, is_service_address};

    fn group(id: i64, level: u16) -> Group {
        Group {
            id: GroupId::new(id),
            level: GroupLevel::new(level).expect("the level is valid"),
            name: format!("Level {level}"),
            image: None,
            is_default: false,
        }
    }

    fn user(id: i64, name: &str, group: i64, service: &str) -> ImportedUser {
        ImportedUser {
            user: User {
                id: UserId::new(id),
                username: name.to_owned(),
                email: None,
                password_hash: PasswordHash::new("hash".to_owned()),
                group: GroupId::new(group),
                locked: false,
                image: None,
                created_at: Timestamp::from_unix_seconds(0),
            },
            login: UserLogin {
                auth_service: service.to_owned(),
                auth_provider: None,
                auth_subject: None,
                two_factor: None,
                sealed_plex_token: None,
            },
        }
    }

    fn set(settings: &mut Settings, key: &str, value: &serde_json::Value) {
        settings.set(key, value).expect("the value fits");
    }

    #[test]
    fn install_like_a_new_one_has_no_breaking_change() {
        let groups = [group(1, 0), group(2, 4)];
        let users = [user(1, "admin", 1, "internal"), user(2, "user", 2, "oidc")];
        assert_eq!(
            breaking_changes(&groups, &users, &Settings::defaults()),
            [] as [String; 0]
        );
    }

    #[test]
    fn each_change_that_applies_is_named_with_its_records() {
        let groups = [group(1, 0), group(2, 1), group(3, 4)];
        let users = [
            user(1, "admin", 1, "oidc"),
            user(2, "cora", 2, "internal"),
            user(3, "olga", 2, "oidc"),
            user(4, "user", 3, "oidc"),
        ];
        let mut settings = Settings::defaults();
        set(&mut settings, "sonarrURL", &json!(" /sonarr"));
        set(&mut settings, "plexTabURL", &json!("/plex"));
        set(&mut settings, "radarrURL", &json!("http://radarr:7878"));
        set(&mut settings, "ssoKomga", &json!(true));
        set(&mut settings, "komgaSSOMasterPassword", &json!("master"));
        set(&mut settings, "homepageDonateShowUserHistory", &json!(true));
        set(&mut settings, "PHPMAILER-enabled", &json!(true));

        let changes = breaking_changes(&groups, &users, &settings);
        assert_eq!(changes.len(), 6, "{changes:#?}");
        assert!(
            changes[0].contains("`sonarrURL` (/sonarr)."),
            "{}",
            changes[0]
        );
        assert!(!changes[0].contains("plexTabURL"), "{}", changes[0]);
        assert!(!changes[0].contains("radarrURL"), "{}", changes[0]);
        assert!(changes[1].contains(": admin, olga."), "{}", changes[1]);
        assert!(changes[2].contains(": cora, olga."), "{}", changes[2]);
        assert!(changes[3].contains("(D-150)"), "{}", changes[3]);
        assert!(changes[4].contains("(D-152)"), "{}", changes[4]);
        assert!(changes[5].contains("(D-138)"), "{}", changes[5]);
    }

    #[test]
    fn control_characters_of_names_and_values_are_escaped() {
        let groups = [group(1, 0), group(2, 1)];
        let users = [
            user(1, "ad\u{1b}[2Jmin", 2, "internal"),
            user(2, "zoë", 2, "internal"),
        ];
        let mut settings = Settings::defaults();
        set(&mut settings, "sonarrURL", &json!("/son\u{9b}arr\u{7}"));

        let changes = breaking_changes(&groups, &users, &settings);
        for change in &changes {
            assert!(!change.chars().any(char::is_control), "{change:?}");
        }
        assert!(
            changes[0].contains(r"`sonarrURL` (/son\u{9b}arr\u{7})."),
            "{}",
            changes[0]
        );
        assert!(
            changes[1].contains(r": ad\u{1b}[2Jmin, zoë."),
            "{}",
            changes[1]
        );
    }

    #[test]
    fn service_addresses_leave_out_the_addresses_for_the_browser() {
        for key in [
            "sonarrURL",
            "authBackendHost",
            "INVITES-komga-uri",
            "jellyStatInternalURL",
        ] {
            assert!(is_service_address(key), "{key}");
        }
        for key in ["plexTabURL", "jellyfinSSOURL", "sonarrToken", "title"] {
            assert!(!is_service_address(key), "{key}");
        }
    }
}
