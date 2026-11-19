//! Decides what an import stores: the records of the new application for the rows and
//! the configuration of the PHP install.
//!
//! The PHP application checks less than this one. Its database can hold two tabs whose
//! names differ only in letter case, a tab in a category that is gone, or a user in a
//! group that is gone. The plan repairs such records so that nothing is lost, and the
//! report names each repair.

use std::collections::{BTreeMap, HashSet};

use organizr_auth::password::{self, HashError};
use organizr_auth::secrets::{
    SecretBox, SecretError, legacy_setting_name, plex_token_name, two_factor_name,
};
use organizr_db::{ImportData, ImportedUser, SealedTwoFactor, StoredSetting, UserLogin};
use organizr_domain::{
    Category, CategoryId, CategoryNumber, Group, GroupId, GroupLevel, LevelRange, PasswordHash,
    Tab, TabId, TabKind, Timestamp, User, UserId,
};
use organizr_settings::{Settings, SettingsError};
use serde_json::Value;
use thiserror::Error;

use crate::legacy_aes;
use crate::legacy_db::LegacyRow;
use crate::report::Report;

/// The settings of the PHP application that this application does not need: it gets its
/// database from `ORGANIZR_DATABASE_URL`, and it seals secrets with a key of its own.
pub const RETIRED_SETTINGS: [&str; 8] = [
    "configVersion",
    "dbHost",
    "dbLocation",
    "dbName",
    "dbPassword",
    "dbUsername",
    "driver",
    "organizrHash",
];

/// The level of the group "User" of a new install.
const USER_LEVEL: u16 = 4;

/// What was read from a PHP install.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LegacyInstall {
    /// The settings of the configuration file.
    pub config: Vec<(String, Value)>,
    /// The rows of the table `groups`.
    pub groups: Vec<LegacyRow>,
    /// The rows of the table `users`.
    pub users: Vec<LegacyRow>,
    /// The rows of the table `categories`.
    pub categories: Vec<LegacyRow>,
    /// The rows of the table `tabs`.
    pub tabs: Vec<LegacyRow>,
    /// The other tables: name and number of rows.
    pub other_tables: Vec<(String, u64)>,
}

/// Why no plan can be made.
#[derive(Debug, Error)]
pub enum PlanError {
    /// A secret cannot be sealed.
    #[error(transparent)]
    Secret(#[from] SecretError),
    /// The registration password cannot be hashed.
    #[error(transparent)]
    Hash(#[from] HashError),
}

/// What to store, and what to tell the person who runs the import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The records.
    pub data: ImportData,
    /// What was done and what needs attention.
    pub report: Report,
}

/// Makes the plan for a PHP install. Secrets are sealed with `secrets`.
pub fn plan(install: &LegacyInstall, secrets: &SecretBox) -> Result<Plan, PlanError> {
    let mut report = Report::default();

    let groups = plan_groups(&install.groups, &mut report);
    let users = plan_users(&install.users, &groups, secrets, &mut report)?;
    let categories = plan_categories(&install.categories, &mut report);
    let tabs = plan_tabs(&install.tabs, &categories, &mut report);
    let (settings, legacy_settings) = plan_settings(&install.config, secrets, &mut report)?;

    for (table, rows) in &install.other_tables {
        if table == "tokens" {
            report.sessions = *rows;
        } else {
            report.tables_not_imported.push((table.clone(), *rows));
        }
    }
    report.groups = groups.len();
    report.users = users.len();
    report.categories = categories.len();
    report.tabs = tabs.len();
    report.settings = settings.len();

    Ok(Plan {
        data: ImportData {
            groups,
            users,
            categories,
            tabs,
            settings,
            legacy_settings,
        },
        report,
    })
}

/// Gives out names that differ in more than letter case.
#[derive(Debug, Default)]
struct Names(HashSet<String>);

impl Names {
    /// The wanted name, or the name with a number behind it when the name is taken.
    fn unique(&mut self, wanted: &str) -> String {
        if self.0.insert(wanted.to_lowercase()) {
            return wanted.to_owned();
        }
        let mut number = 2_u32;
        loop {
            let candidate = format!("{wanted} ({number})");
            if self.0.insert(candidate.to_lowercase()) {
                return candidate;
            }
            number = number.saturating_add(1);
        }
    }

    /// A unique name for a record. The report gets a line when the name had to change.
    fn give(
        &mut self,
        kind: &str,
        stored: Option<&str>,
        fallback: &str,
        report: &mut Report,
    ) -> String {
        let Some(wanted) = stored else {
            let name = self.unique(fallback);
            report
                .repairs
                .push(format!("A {kind} without a name is now called \"{name}\"."));
            return name;
        };
        let name = self.unique(wanted);
        if name != wanted {
            report.repairs.push(format!(
                "The {kind} \"{wanted}\" is now called \"{name}\", because another {kind} has the same name."
            ));
        }
        name
    }
}

fn text(value: Option<&str>) -> Option<String> {
    value.map(str::to_owned)
}

fn plan_groups(rows: &[LegacyRow], report: &mut Report) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    let mut names = Names::default();
    for row in rows {
        let label = row.text("group").unwrap_or("without a name");
        let Some(id) = row.number("id") else {
            report
                .skipped
                .push(format!("The group \"{label}\" has no id and is left out."));
            continue;
        };
        let Some(level) = row
            .number("group_id")
            .and_then(|level| GroupLevel::try_from(level).ok())
        else {
            report.skipped.push(format!(
                "The group \"{label}\" has no level from 0 to 999 and is left out."
            ));
            continue;
        };
        if groups.iter().any(|group| group.level == level) {
            report.skipped.push(format!(
                "The group \"{label}\" has the level {} of another group and is left out. \
                 Its users are in that other group.",
                level.get()
            ));
            continue;
        }
        let name = names.give(
            "group",
            row.filled("group"),
            &format!("Group {}", level.get()),
            report,
        );
        groups.push(Group {
            id: GroupId::new(id),
            level,
            name,
            image: text(row.filled("image")),
            is_default: row.is_on("default"),
        });
    }

    for (level, name, image) in [
        (
            GroupLevel::ADMIN,
            "Admin",
            "plugins/images/groups/admin.png",
        ),
        (
            GroupLevel::GUEST,
            "Guest",
            "plugins/images/groups/guest.png",
        ),
    ] {
        if !groups.iter().any(|group| group.level == level) {
            let name = add_group(&mut groups, &mut names, level, name, image, false);
            report.repairs.push(format!(
                "The install had no group of level {}. The group \"{name}\" was added.",
                level.get()
            ));
        }
    }
    one_default_group(&mut groups, &mut names, report);
    groups
}

/// Adds a group with the next free id and returns its name.
fn add_group(
    groups: &mut Vec<Group>,
    names: &mut Names,
    level: GroupLevel,
    name: &str,
    image: &str,
    is_default: bool,
) -> String {
    let id = groups.iter().map(|group| group.id.get()).max().unwrap_or(0) + 1;
    let name = names.unique(name);
    groups.push(Group {
        id: GroupId::new(id),
        level,
        name: name.clone(),
        image: Some(image.to_owned()),
        is_default,
    });
    name
}

/// Makes exactly one group the group of new users.
fn one_default_group(groups: &mut Vec<Group>, names: &mut Names, report: &mut Report) {
    let mut defaults = groups.iter_mut().filter(|group| group.is_default);
    if let Some(kept) = defaults.next() {
        let kept = kept.name.clone();
        for other in defaults {
            other.is_default = false;
            report.repairs.push(format!(
                "The groups \"{kept}\" and \"{}\" were both the group of new users. Now only \"{kept}\" is.",
                other.name
            ));
        }
        return;
    }

    // The group with the least access that is no Admin, Co-Admin or Guest.
    let candidate = groups
        .iter_mut()
        .filter(|group| group.level > GroupLevel::CO_ADMIN && group.level < GroupLevel::GUEST)
        .max_by_key(|group| group.level);
    let name = if let Some(group) = candidate {
        group.is_default = true;
        group.name.clone()
    } else {
        let level = GroupLevel::new(USER_LEVEL).unwrap_or(GroupLevel::GUEST);
        add_group(
            groups,
            names,
            level,
            "User",
            "plugins/images/groups/user.png",
            true,
        )
    };
    report.repairs.push(format!(
        "No group was the group of new users. Now \"{name}\" is."
    ));
}

/// The parts of the `auth_service` text of the PHP application.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Login {
    service: String,
    provider: Option<String>,
    /// The kind and the secret of the second factor.
    two_factor: Option<(String, String)>,
}

/// Reads `auth_service`: `internal`, the name of a service, `oidc::<provider>`, or
/// `<kind>::<secret>` for a second factor.
fn login_of(auth_service: Option<&str>) -> Login {
    let internal = || "internal".to_owned();
    match auth_service.map(str::trim).filter(|text| !text.is_empty()) {
        None => Login {
            service: internal(),
            provider: None,
            two_factor: None,
        },
        Some(text) => match text.split_once("::") {
            None => Login {
                service: text.to_owned(),
                provider: None,
                two_factor: None,
            },
            Some(("oidc", provider)) => Login {
                service: "oidc".to_owned(),
                provider: Some(provider.to_owned()),
                two_factor: None,
            },
            Some((kind, secret)) => Login {
                service: internal(),
                provider: None,
                two_factor: Some((kind.to_owned(), secret.to_owned())),
            },
        },
    }
}

/// The moment a text of the PHP application names. Old rows hold
/// `2018-05-08T01:28:03Z`, newer ones `2027-02-13 00:08:49`, which is read as UTC.
pub(crate) fn moment(text: &str) -> Option<Timestamp> {
    let text = text.trim();
    if let Ok(seconds) = text.parse::<i64>() {
        return Some(Timestamp::from_unix_seconds(seconds));
    }
    let moment = text.parse::<jiff::Timestamp>().ok().or_else(|| {
        let civil = text.parse::<jiff::civil::DateTime>().ok()?;
        jiff::tz::TimeZone::UTC.to_timestamp(civil).ok()
    })?;
    Some(Timestamp::from_unix_seconds(moment.as_second()))
}

fn plan_users(
    rows: &[LegacyRow],
    groups: &[Group],
    secrets: &SecretBox,
    report: &mut Report,
) -> Result<Vec<ImportedUser>, PlanError> {
    let default_group = groups
        .iter()
        .find(|group| group.is_default)
        .or_else(|| groups.iter().max_by_key(|group| group.level));
    let mut users = Vec::new();
    let mut names = Names::default();
    for row in rows {
        let label = row.text("username").unwrap_or("without a name");
        let (Some(id), Some(fallback)) = (row.number("id"), default_group) else {
            report
                .skipped
                .push(format!("The user \"{label}\" has no id and is left out."));
            continue;
        };
        let username = names.give(
            "user",
            row.filled("username"),
            &format!("user-{id}"),
            report,
        );
        let group = row
            .number("group_id")
            .and_then(|level| groups.iter().find(|group| i64::from(group.level) == level));
        let group = group.unwrap_or_else(|| {
            report.repairs.push(format!(
                "The user \"{username}\" was in a group that is gone and is now in \"{}\".",
                fallback.name
            ));
            fallback
        });
        let created_at = match row.filled("register_date") {
            None => Timestamp::from_unix_seconds(0),
            Some(date) => moment(date).unwrap_or_else(|| {
                report.repairs.push(format!(
                    "The date \"{date}\" of the user \"{username}\" is no date. The user has no date now."
                ));
                Timestamp::from_unix_seconds(0)
            }),
        };

        let user_id = UserId::new(id);
        let login = login_of(row.text("auth_service"));
        let two_factor = match login.two_factor {
            None => None,
            Some((kind, secret)) => Some(SealedTwoFactor {
                kind,
                sealed_secret: secrets.seal(&two_factor_name(user_id), &secret)?,
            }),
        };
        let sealed_plex_token = match row.filled("plex_token") {
            None => None,
            Some(token) => Some(secrets.seal(&plex_token_name(user_id), token)?),
        };
        users.push(ImportedUser {
            user: User {
                id: user_id,
                username,
                email: text(row.filled("email")),
                password_hash: PasswordHash::new(row.text("password").unwrap_or("").to_owned()),
                group: group.id,
                locked: row.is_on("locked"),
                image: text(row.filled("image")),
                created_at,
            },
            login: UserLogin {
                auth_service: login.service,
                auth_provider: login.provider,
                two_factor,
                sealed_plex_token,
            },
        });
    }
    Ok(users)
}

fn plan_categories(rows: &[LegacyRow], report: &mut Report) -> Vec<Category> {
    let mut categories: Vec<Category> = Vec::new();
    let mut names = Names::default();
    let mut without_number = Vec::new();
    for row in rows {
        let label = row.text("category").unwrap_or("without a name");
        let Some(id) = row.number("id") else {
            report.skipped.push(format!(
                "The category \"{label}\" has no id and is left out."
            ));
            continue;
        };
        let name = names.give(
            "category",
            row.filled("category"),
            &format!("Category {id}"),
            report,
        );
        let number = row
            .number("category_id")
            .filter(|number| categories.iter().all(|other| other.number.get() != *number));
        if number.is_none() {
            without_number.push(categories.len());
        }
        categories.push(Category {
            id: CategoryId::new(id),
            number: CategoryNumber::new(number.unwrap_or(0)),
            position: row.number("order").unwrap_or(id),
            name,
            image: text(row.filled("image")),
            is_default: row.is_on("default"),
        });
    }

    // A category without a number of its own gets the next free one.
    for index in without_number {
        let taken: Vec<i64> = categories
            .iter()
            .zip(0_usize..)
            .filter(|(_, other)| *other != index)
            .map(|(category, _)| category.number.get())
            .collect();
        let next = taken
            .iter()
            .copied()
            .max()
            .map_or(0, |highest| highest.max(0) + 1);
        if let Some(category) = categories.get_mut(index) {
            category.number = CategoryNumber::new(next);
            report.repairs.push(format!(
                "The category \"{}\" had no number of its own and now has the number {next}. \
                 Tabs with its old number are in the category that had the number first.",
                category.name
            ));
        }
    }

    if categories.is_empty() {
        categories.push(Category {
            id: CategoryId::new(1),
            number: CategoryNumber::new(0),
            position: 1,
            name: "Unsorted".to_owned(),
            image: Some("fontawesome::question".to_owned()),
            is_default: true,
        });
        report
            .repairs
            .push("The install had no category. The category \"Unsorted\" was added.".to_owned());
    }
    one_default_category(&mut categories, report);
    categories
}

/// Makes exactly one category the category of new tabs.
fn one_default_category(categories: &mut [Category], report: &mut Report) {
    let mut defaults = categories.iter_mut().filter(|category| category.is_default);
    if let Some(kept) = defaults.next() {
        let kept = kept.name.clone();
        for other in defaults {
            other.is_default = false;
            report.repairs.push(format!(
                "The categories \"{kept}\" and \"{}\" were both the category of new tabs. Now only \"{kept}\" is.",
                other.name
            ));
        }
    } else if let Some(first) = categories
        .iter_mut()
        .min_by_key(|category| (category.position, category.id.get()))
    {
        first.is_default = true;
        report.repairs.push(format!(
            "No category was the category of new tabs. Now \"{}\" is.",
            first.name
        ));
    }
}

/// The levels that see a tab. Levels that cannot be are replaced so that fewer people
/// see the tab, never more.
fn access_of(row: &LegacyRow, name: &str, report: &mut Report) -> LevelRange {
    let level = |column: &str| {
        row.number(column)
            .and_then(|level| GroupLevel::try_from(level).ok())
    };
    let Some(least_access) = level("group_id") else {
        report.repairs.push(format!(
            "The tab \"{name}\" had no level from 0 to 999. Now only the Admin sees it."
        ));
        return LevelRange::up_to(GroupLevel::ADMIN);
    };
    let most_access = if row.text("group_id_max").is_none() {
        Some(GroupLevel::ADMIN)
    } else {
        level("group_id_max")
    };
    most_access
        .and_then(|most_access| LevelRange::new(least_access, most_access).ok())
        .unwrap_or_else(|| {
            report.repairs.push(format!(
                "The tab \"{name}\" had a range of levels that no user is in. Now only the Admin sees it."
            ));
            LevelRange::up_to(GroupLevel::ADMIN)
        })
}

fn plan_tabs(rows: &[LegacyRow], categories: &[Category], report: &mut Report) -> Vec<Tab> {
    let default_category = categories
        .iter()
        .find(|category| category.is_default)
        .or_else(|| categories.first());
    let mut tabs: Vec<Tab> = Vec::new();
    let mut names = Names::default();
    for row in rows {
        let label = row.text("name").unwrap_or("without a name");
        let (Some(id), Some(fallback)) = (row.number("id"), default_category) else {
            report
                .skipped
                .push(format!("The tab \"{label}\" has no id and is left out."));
            continue;
        };
        let name = names.give("tab", row.filled("name"), &format!("Tab {id}"), report);
        let category = row.number("category_id").and_then(|number| {
            categories
                .iter()
                .find(|category| category.number.get() == number)
        });
        let category = category.unwrap_or_else(|| {
            report.repairs.push(format!(
                "The tab \"{name}\" was in a category that is gone and is now in \"{}\".",
                fallback.name
            ));
            fallback
        });
        let kind = row
            .number("type")
            .and_then(|kind| TabKind::try_from(kind).ok())
            .unwrap_or_else(|| {
                report.repairs.push(format!(
                    "The tab \"{name}\" had no type from 0 to 2 and now opens in a frame."
                ));
                TabKind::Iframe
            });
        let is_default = row.is_on("default");
        if is_default && let Some(other) = tabs.iter().find(|tab| tab.is_default) {
            report.repairs.push(format!(
                "The tabs \"{}\" and \"{name}\" were both the tab that opens first. Now only \"{}\" is.",
                other.name, other.name
            ));
        }
        let access = access_of(row, &name, report);
        tabs.push(Tab {
            id: TabId::new(id),
            position: row.number("order").unwrap_or(id),
            category: category.id,
            is_default: is_default && tabs.iter().all(|tab| !tab.is_default),
            name,
            url: text(row.filled("url")),
            url_local: text(row.filled("url_local")),
            enabled: row.is_on("enabled"),
            access,
            add_to_admin: row.is_on("add_to_admin"),
            image: text(row.filled("image")),
            kind,
            splash: row.is_on("splash"),
            ping: row.is_on("ping"),
            ping_url: text(row.filled("ping_url")),
            timeout: row.is_on("timeout"),
            timeout_ms: row.number("timeout_ms"),
            preload: row.is_on("preload"),
        });
    }
    tabs
}

/// The settings to store, and the settings that this application does not know.
fn plan_settings(
    config: &[(String, Value)],
    secrets: &SecretBox,
    report: &mut Report,
) -> Result<(Vec<StoredSetting>, Vec<StoredSetting>), PlanError> {
    let hash = config
        .iter()
        .find(|(key, _)| key == "organizrHash")
        .and_then(|(_, value)| value.as_str());
    let defaults = Settings::defaults();
    let mut settings = Settings::defaults();
    // Sorted by key, so that the same install always gives the same plan.
    let mut stored: BTreeMap<String, String> = BTreeMap::new();
    let mut legacy: BTreeMap<String, String> = BTreeMap::new();

    for (key, value) in config {
        if RETIRED_SETTINGS.contains(&key.as_str()) {
            report.retired_settings.push(key.clone());
            continue;
        }
        let mut keep_as_legacy = |value: &Value| -> Result<(), SecretError> {
            let sealed = secrets.seal(&legacy_setting_name(key), &value.to_string())?;
            legacy.insert(key.clone(), sealed);
            Ok(())
        };

        let Some(value) = opened(key, value, hash) else {
            keep_as_legacy(value)?;
            report.repairs.push(format!(
                "The setting `{key}` holds an encrypted value that the hash of the install does not open. \
                 The setting is empty now. Enter the value again on the settings page."
            ));
            continue;
        };
        let value = match (key.as_str(), &value) {
            ("registrationPassword", Value::String(password)) if !password.is_empty() => {
                Value::String(password::hash(password)?.expose().to_owned())
            }
            _ => value,
        };

        match settings.set(key, &value) {
            Ok(()) => {
                let now = settings.get(key).unwrap_or(value);
                if defaults.get(key).as_ref() != Some(&now) {
                    stored.insert(key.clone(), stored_form(key, &now, secrets)?);
                }
            }
            Err(SettingsError::UnknownKey(_)) => {
                keep_as_legacy(&value)?;
                report.legacy_settings.push(key.clone());
            }
            Err(_) => {
                keep_as_legacy(&value)?;
                report.repairs.push(format!(
                    "The value of the setting `{key}` is not valid here. The setting keeps its default."
                ));
            }
        }
    }

    let list = |map: BTreeMap<String, String>| {
        map.into_iter()
            .map(|(key, value)| StoredSetting { key, value })
            .collect()
    };
    Ok((list(stored), list(legacy)))
}

/// The value of a setting in plain text. PHP encrypts each text whose key holds the word
/// "password"; every other value comes back as it is. `None` for an encrypted value that
/// the hash of the install does not open.
fn opened(key: &str, value: &Value, hash: Option<&str>) -> Option<Value> {
    let encrypted = match value {
        Value::String(text) if !text.is_empty() && key.to_lowercase().contains("password") => text,
        _ => return Some(value.clone()),
    };
    match hash.and_then(|hash| legacy_aes::decrypt(encrypted, hash)) {
        Some(plain) => Some(Value::String(plain)),
        // The set-up wizard of PHP stores this one password as it was entered.
        None if key == "registrationPassword" => Some(value.clone()),
        None => None,
    }
}

/// The text to store for a setting: JSON, with a secret sealed.
fn stored_form(key: &str, value: &Value, secrets: &SecretBox) -> Result<String, SecretError> {
    match value {
        Value::String(secret) if Settings::is_secret(key) => {
            Ok(Value::String(secrets.seal(key, secret)?).to_string())
        }
        other => Ok(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use organizr_auth::secrets::SecretBox;
    use serde_json::json;

    use organizr_domain::Timestamp;

    use super::{LegacyInstall, Login, Names, login_of, moment, plan};
    use crate::legacy_db::LegacyRow;

    #[test]
    fn name_that_is_taken_gets_the_next_free_number() {
        let mut names = Names::default();

        assert_eq!(names.unique("Plex"), "Plex");
        assert_eq!(names.unique("plex"), "plex (2)");
        assert_eq!(names.unique("PLEX"), "PLEX (3)");
        // The name with a number is taken too.
        assert_eq!(names.unique("Plex (2)"), "Plex (2) (2)");
        assert_eq!(names.unique("Sonarr"), "Sonarr");
    }

    #[test]
    fn login_text_of_php_is_split_into_its_parts() {
        let internal = Login {
            service: "internal".to_owned(),
            provider: None,
            two_factor: None,
        };
        assert_eq!(login_of(None), internal);
        assert_eq!(login_of(Some("")), internal);
        assert_eq!(login_of(Some("  ")), internal);
        assert_eq!(login_of(Some("internal")), internal);

        assert_eq!(login_of(Some("plex")).service, "plex");
        assert_eq!(login_of(Some("plex")).two_factor, None);

        let oidc = login_of(Some("oidc::authentik"));
        assert_eq!(oidc.service, "oidc");
        assert_eq!(oidc.provider.as_deref(), Some("authentik"));
        assert_eq!(oidc.two_factor, None);

        let second_factor = login_of(Some("google::JBSWY3DP::EHPK3PXP"));
        assert_eq!(second_factor.service, "internal");
        assert_eq!(second_factor.provider, None);
        assert_eq!(
            second_factor.two_factor,
            Some(("google".to_owned(), "JBSWY3DP::EHPK3PXP".to_owned()))
        );
    }

    #[test]
    fn dates_of_php_are_read_in_both_forms() {
        let seconds = |text: &str| moment(text).map(Timestamp::unix_seconds);

        assert_eq!(seconds("2018-05-08T01:28:03Z"), Some(1_525_742_883));
        assert_eq!(seconds("2018-05-08 01:28:03"), Some(1_525_742_883));
        assert_eq!(seconds(" 2018-05-08T01:28:03 "), Some(1_525_742_883));
        assert_eq!(seconds("2018-05-08T03:28:03+02:00"), Some(1_525_742_883));
        assert_eq!(seconds("1525742883"), Some(1_525_742_883));
        // A date without a time is the start of that day.
        assert_eq!(seconds("2018-05-08"), Some(1_525_737_600));
        assert_eq!(seconds("last week"), None);
        assert_eq!(seconds(""), None);
    }

    #[test]
    fn install_without_rows_gets_what_every_install_needs() {
        let secrets = SecretBox::with_random_key().expect("a key can be made");
        let install = LegacyInstall {
            config: vec![
                ("title".to_owned(), json!("Empty")),
                ("driver".to_owned(), json!("sqlite3")),
            ],
            tabs: vec![LegacyRow::of([
                ("id", Some("1")),
                ("name", Some("Only")),
                ("group_id", Some("1")),
                ("type", Some("1")),
            ])],
            other_tables: vec![("tokens".to_owned(), 12), ("invites".to_owned(), 3)],
            ..LegacyInstall::default()
        };

        let plan = plan(&install, &secrets).expect("a plan can be made");

        let groups: Vec<(u16, &str, bool)> = plan
            .data
            .groups
            .iter()
            .map(|group| (group.level.get(), group.name.as_str(), group.is_default))
            .collect();
        assert_eq!(
            groups,
            [
                (0, "Admin", false),
                (999, "Guest", false),
                (4, "User", true)
            ]
        );
        let ids: Vec<i64> = plan
            .data
            .groups
            .iter()
            .map(|group| group.id.get())
            .collect();
        assert_eq!(ids, [1, 2, 3]);
        assert_eq!(plan.data.categories.len(), 1);
        let unsorted = &plan.data.categories[0];
        assert_eq!(
            (unsorted.name.as_str(), unsorted.is_default),
            ("Unsorted", true)
        );
        assert_eq!(plan.data.tabs[0].category, unsorted.id);
        assert_eq!(plan.data.settings.len(), 1);
        assert_eq!(plan.data.settings[0].key, "title");
        assert_eq!(plan.data.settings[0].value, "\"Empty\"");
        assert_eq!(plan.report.sessions, 12);
        assert_eq!(plan.report.tables_not_imported, [("invites".to_owned(), 3)]);
        assert_eq!(plan.report.retired_settings, ["driver"]);
        assert_eq!(plan.report.repairs.len(), 5, "{:#?}", plan.report.repairs);
    }

    #[test]
    fn setting_with_a_value_of_the_wrong_kind_keeps_its_default_and_is_kept() {
        let secrets = SecretBox::with_random_key().expect("a key can be made");
        let install = LegacyInstall {
            config: vec![
                ("loginAttempts".to_owned(), json!(["not", "a", "number"])),
                ("title".to_owned(), json!("Organizr V2")),
            ],
            ..LegacyInstall::default()
        };

        let plan = plan(&install, &secrets).expect("a plan can be made");

        // The title is the default, so nothing is stored for it.
        assert_eq!(plan.data.settings, []);
        assert_eq!(plan.data.legacy_settings.len(), 1);
        assert_eq!(plan.data.legacy_settings[0].key, "loginAttempts");
        assert!(
            plan.report
                .repairs
                .iter()
                .any(|line| line.contains("`loginAttempts` is not valid here")),
            "{:#?}",
            plan.report.repairs
        );
        assert_eq!(plan.report.legacy_settings, [] as [String; 0]);
    }
}
