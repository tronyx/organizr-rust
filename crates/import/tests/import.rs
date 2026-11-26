//! The import of a PHP install, from its files to a working application.
//!
//! The install is made up, and the real PHP application wrote it:
//! `parity/legacy-install/generate.py` stores its configuration file and its database
//! under `tests/fixtures/legacy`. Some tests add rows that only an old or damaged
//! install holds.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::path::{Path, PathBuf};

use axum::http::StatusCode;
use organizr_auth::password::{self, PasswordCheck};
use organizr_auth::secrets::{legacy_setting_name, plex_token_name, two_factor_name};
use organizr_db::Database;
use organizr_domain::{GroupLevel, PasswordHash, TabKind, UserId};
use organizr_import::{ImportError, ImportRequest, dry_run, run};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call};
use organizr_testkit::{seed, state_with};
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use tempfile::TempDir;

const CONFIG: &str = include_str!("fixtures/legacy/config.php");
const DATABASE: &str = include_str!("fixtures/legacy/organizr.sql");

/// Where the configuration of the fixture says its database is.
const DATABASE_LINE: &str = "'dbLocation' => '/var/www/html/data/parity-db/',";

/// A PHP install on the disk.
struct Legacy {
    folder: TempDir,
}

impl Legacy {
    /// The install of the fixture, with more SQL statements run on its database.
    async fn with(more_sql: &str) -> Self {
        Self::build(CONFIG, "data/parity-db/parity.db", more_sql).await
    }

    async fn build(config: &str, database_file: &str, more_sql: &str) -> Self {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let legacy = Self { folder };
        let config_file = legacy.root().join("data/config/config.php");
        std::fs::create_dir_all(config_file.parent().expect("the file is in a folder"))
            .expect("the folder can be made");
        std::fs::write(&config_file, config).expect("the configuration file can be written");
        make_database(&legacy.root().join(database_file), more_sql).await;
        legacy
    }

    fn root(&self) -> &Path {
        self.folder.path()
    }

    /// The data folder of the new install.
    fn data_dir(&self) -> PathBuf {
        self.root().join("new-data")
    }

    fn request(&self) -> ImportRequest<'_> {
        ImportRequest {
            from: self.root(),
            database_file: None,
        }
    }
}

/// Makes the SQLite file of the fixture.
async fn make_database(file: &Path, more_sql: &str) {
    std::fs::create_dir_all(file.parent().expect("the file is in a folder"))
        .expect("the folder can be made");
    let url = format!(
        "sqlite:{}?mode=rwc",
        file.display().to_string().replace('\\', "/")
    );
    let connection = sea_orm::Database::connect(url)
        .await
        .expect("the file can be made");
    for sql in [DATABASE, more_sql] {
        connection
            .execute_unprepared(sql)
            .await
            .unwrap_or_else(|error| panic!("the statements run: {error}"));
    }
    connection.close().await.expect("the file can be closed");
}

/// The settings of the new install, read as the server reads them.
async fn settings_of(database: &Database) -> Box<organizr_settings::Settings> {
    let (settings, problems) = organizr_api::stored_settings(database, &seed::secrets())
        .await
        .expect("the settings can be read");
    assert_eq!(problems, [], "every stored setting can be used");
    // The settings are large, so they do not stand in the future of a test itself.
    Box::new(settings)
}

async fn user(database: &Database, name: &str) -> organizr_domain::User {
    database
        .users()
        .find_by_username(name)
        .await
        .expect("the users can be read")
        .unwrap_or_else(|| panic!("the user {name} is there"))
}

fn text(setting: Option<Value>) -> String {
    setting
        .and_then(|value| value.as_str().map(str::to_owned))
        .expect("the setting is a text")
}

#[tokio::test]
async fn install_of_php_is_imported_whole() {
    on_each_engine(async |database| {
        let legacy = Legacy::with("").await;
        let secrets = seed::secrets();

        let report = run(legacy.request(), database, &secrets, &legacy.data_dir())
            .await
            .expect("the import runs");

        assert!(!report.dry_run);
        assert_eq!(
            (report.groups, report.users, report.categories, report.tabs),
            (7, 4, 2, 4)
        );
        assert_eq!(report.repairs, [] as [String; 0]);
        assert_eq!(report.skipped, [] as [String; 0]);
        assert_eq!(report.sessions, 1);
        assert_eq!(report.tables_not_imported, [("options".to_owned(), 0)]);
        assert_eq!(report.legacy_settings, ["MB-URL"]);
        // The settings of Shuck Stop, whose service shut down (D-116), are not needed either.
        let (shuck_stop, retired): (Vec<&String>, Vec<&String>) = report
            .retired_settings
            .iter()
            .partition(|key| key.starts_with("SHUCKSTOP-"));
        assert_eq!(shuck_stop.len(), 14, "{shuck_stop:?}");
        assert_eq!(
            retired,
            [
                "configVersion",
                "dbHost",
                "dbLocation",
                "dbName",
                "dbPassword",
                "dbUsername",
                "driver",
                "organizrHash"
            ]
        );

        // The chat messages keep their ids, and a new one gets the next id.
        assert_eq!(report.chat_messages, 1);
        let messages = database.chat().newest(10).await.expect("the messages load");
        assert_eq!(messages.len(), 1);
        assert_eq!(
            (
                messages[0].id,
                messages[0].username.as_str(),
                messages[0].message.as_str()
            ),
            (1, "admin", "Hello")
        );
        let mut next = messages[0].clone();
        next.sent_at += 1;
        let next = database
            .chat()
            .insert(next)
            .await
            .expect("a message can be added");
        assert_eq!(next.id, 2);

        // The groups keep their ids, levels and names.
        let groups = database.groups().list().await.expect("the groups load");
        let editors = groups
            .iter()
            .find(|group| group.name == "Editors")
            .expect("the added group is there");
        assert_eq!((editors.id.get(), editors.level.get()), (7, 5));
        let default: Vec<&str> = groups
            .iter()
            .filter(|group| group.is_default)
            .map(|group| group.name.as_str())
            .collect();
        assert_eq!(default, ["User"]);

        // The users keep their ids, groups and passwords.
        let admin = user(database, "admin").await;
        assert_eq!(admin.id.get(), 1);
        assert_eq!(admin.email.as_deref(), Some("admin@example.com"));
        assert!(matches!(
            password::verify(seed::ADMIN_PASSWORD, &admin.password_hash),
            PasswordCheck::Correct { .. }
        ));
        let sam = user(database, "Sam").await;
        let super_user = groups
            .iter()
            .find(|group| group.level.get() == 2)
            .expect("the group is there");
        assert_eq!(sam.group, super_user.id);
        assert!(!sam.locked);

        // 2026-11-19 17:23:25, the new form, read as UTC.
        assert_eq!(admin.created_at.unix_seconds(), 1_795_109_005);
    })
    .await;
}

#[tokio::test]
async fn logins_of_users_are_imported_with_sealed_secrets() {
    on_each_engine(async |database| {
        let legacy = Legacy::with("").await;
        let secrets = seed::secrets();
        run(legacy.request(), database, &secrets, &legacy.data_dir())
            .await
            .expect("the import runs");

        // A second factor and a Plex token are sealed, each under its own name.
        let robin = user(database, "robin").await;
        let login = database
            .users()
            .login_of(robin.id)
            .await
            .expect("the login loads")
            .expect("the user is there");
        assert_eq!(login.auth_service, "internal");
        let factor = login.two_factor.expect("the user has a second factor");
        assert_eq!(factor.kind, "google");
        assert!(!factor.sealed_secret.contains("JBSWY3DPEHPK3PXP"));
        assert_eq!(
            secrets
                .open(&two_factor_name(robin.id), &factor.sealed_secret)
                .expect("the secret opens"),
            "JBSWY3DPEHPK3PXP"
        );

        let pat = user(database, "pat").await;
        // 2018-05-08T01:28:03Z, the old form of a date.
        assert_eq!(pat.created_at.unix_seconds(), 1_525_742_883);
        let login = database
            .users()
            .login_of(pat.id)
            .await
            .expect("the login loads")
            .expect("the user is there");
        assert_eq!(login.auth_service, "plex");
        assert_eq!(login.two_factor, None);
        let token = login.sealed_plex_token.expect("the user has a token");
        assert_eq!(
            secrets
                .open(&plex_token_name(pat.id), &token)
                .expect("the token opens"),
            "made-up-token-of-a-plex-user"
        );
    })
    .await;
}

#[tokio::test]
async fn tabs_and_categories_keep_their_place() {
    on_each_engine(async |database| {
        let legacy = Legacy::with("").await;
        run(
            legacy.request(),
            database,
            &seed::secrets(),
            &legacy.data_dir(),
        )
        .await
        .expect("the import runs");

        let categories = database.categories().list().await.expect("they load");
        let media = categories
            .iter()
            .find(|category| category.name == "Media")
            .expect("the category is there");
        assert_eq!((media.id.get(), media.number.get()), (2, 7));
        assert_eq!(media.image.as_deref(), Some("fontawesome::film"));

        let tabs = database.tabs().list().await.expect("they load");
        let plex = tabs
            .iter()
            .find(|tab| tab.name == "Plex")
            .expect("is there");
        assert_eq!(plex.id.get(), 3);
        assert_eq!(plex.category, media.id);
        assert_eq!(plex.url.as_deref(), Some("https://{domain}/plex"));
        assert_eq!(plex.url_local.as_deref(), Some("http://10.0.0.6:32400/web"));
        assert_eq!(plex.kind, TabKind::Iframe);
        assert!(plex.enabled && plex.is_default && plex.ping && plex.preload);
        assert!(!plex.splash && !plex.timeout && !plex.add_to_admin);
        assert_eq!(plex.ping_url.as_deref(), Some("10.0.0.6:32400"));
        assert_eq!(plex.access.least_access().get(), 4);
        assert_eq!(plex.access.most_access(), GroupLevel::ADMIN);

        let docs = tabs
            .iter()
            .find(|tab| tab.name == "Docs")
            .expect("is there");
        assert_eq!(docs.kind, TabKind::NewWindow);
        assert!(!docs.enabled && docs.add_to_admin);
        assert_eq!(docs.url_local, None);
        assert_eq!(
            (
                docs.access.least_access().get(),
                docs.access.most_access().get()
            ),
            (999, 2)
        );
        let settings_tab = tabs.iter().find(|tab| tab.id.get() == 1).expect("is there");
        assert_eq!(settings_tab.kind, TabKind::Internal);
    })
    .await;
}

#[tokio::test]
async fn settings_are_imported_and_secrets_are_sealed_again() {
    on_each_engine(async |database| {
        let legacy = Legacy::with("").await;
        let secrets = seed::secrets();
        run(legacy.request(), database, &secrets, &legacy.data_dir())
            .await
            .expect("the import runs");

        let settings = settings_of(database).await;
        assert_eq!(settings.get("title"), Some(json!("Legacy Install")));
        assert_eq!(settings.get("unsortedTabs"), Some(json!("bottom")));
        assert_eq!(settings.get("localIPList"), Some(json!("10.0.0.0/8")));
        assert_eq!(settings.get("homepagePlexStreams"), Some(json!(true)));
        assert_eq!(
            settings.get("uuid").map(|uuid| text(Some(uuid)).len()),
            Some(36)
        );
        assert_eq!(settings.get("organizrAPI"), Some(json!(seed::API_KEY)));
        assert_eq!(settings.get("plexToken"), Some(json!("made-up-plex-token")));
        // PHP had encrypted these with the hash of the install.
        assert_eq!(
            settings.get("qBittorrentPassword"),
            Some(json!("made-up torrent password"))
        );
        assert_eq!(
            settings.get("PHPMAILER-smtpHostPassword"),
            Some(json!("made-up mail password"))
        );
        // The registration password is a hash now.
        let hash = PasswordHash::new(text(settings.get("registrationPassword")));
        assert!(matches!(
            password::verify("let-new-users-in", &hash),
            PasswordCheck::Correct { .. }
        ));

        // In the database, no secret is in plain text, and nothing is stored for a
        // setting that has its default.
        let stored = database.settings().all().await.expect("they load");
        let mut all_text = String::new();
        for setting in &stored {
            all_text.push_str(&setting.key);
            all_text.push('=');
            all_text.push_str(&setting.value);
            all_text.push('\n');
        }
        for secret in [
            "made-up torrent password",
            "made-up mail password",
            "made-up-plex-token",
            seed::API_KEY,
            "let-new-users-in",
            "parity-hash-key",
        ] {
            assert!(
                !all_text.contains(secret),
                "{secret} is stored in plain text"
            );
        }
        assert!(stored.iter().all(|setting| setting.key != "debugAreaAuth"));
        assert!(stored.len() < 30, "{} settings are stored", stored.len());

        // A setting that the application does not know is kept, sealed.
        let legacy_settings = database
            .import()
            .legacy_settings()
            .await
            .expect("they load");
        assert_eq!(legacy_settings.len(), 1);
        assert_eq!(legacy_settings[0].key, "MB-URL");
        assert!(!legacy_settings[0].value.contains("mediabutler"));
        assert_eq!(
            secrets
                .open(&legacy_setting_name("MB-URL"), &legacy_settings[0].value)
                .expect("the value opens"),
            "\"https://mediabutler.example.com\""
        );
    })
    .await;
}

#[tokio::test]
async fn imported_install_works_and_new_records_get_new_ids() {
    on_each_engine(async |database| {
        let legacy = Legacy::with("").await;
        run(legacy.request(), database, &seed::secrets(), &legacy.data_dir())
            .await
            .expect("the import runs");
        let state = state_with(database, *settings_of(database).await);
        let app = organizr_api::router(state).expect("the route table is valid");

        // A user logs in with the password of the PHP install.
        let body = json!({"username": "Sam", "password": "the password of sam"});
        let answer = call(&app, As::Guest, "POST", "/login", Some(body)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        // A user with a second factor also needs the code, as before the import.
        let mut body = json!({"username": "robin", "password": "the password of robin"});
        let answer = call(&app, As::Guest, "POST", "/login", Some(body.clone())).await;
        assert_eq!(answer.message(), json!("2FA Code Needed"), "{}", answer.body);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock is after 1970")
            .as_secs();
        let code = organizr_auth::two_factor::code_at("JBSWY3DPEHPK3PXP", now);
        body["tfaCode"] = json!(code.expect("the secret is valid"));
        let answer = call(&app, As::Guest, "POST", "/login", Some(body)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

        // The API key of the PHP install still counts as the Admin.
        let tab = json!({"name": "New", "url": "https://new.example.com/", "image": "", "enabled": 1});
        let added = call(&app, As::Admin, "POST", "/tabs", Some(tab)).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
        let group = json!({"group": "Newcomers", "image": "plugins/images/groups/user.png"});
        let added = call(&app, As::Admin, "POST", "/groups", Some(group)).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
        let category = json!({"category": "More", "image": "fontawesome::plus"});
        let added = call(&app, As::Admin, "POST", "/categories", Some(category)).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
        let person = json!({"username": "newcomer", "email": "new@example.com", "password": "a new password"});
        let added = call(&app, As::Admin, "POST", "/users", Some(person)).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);

        // Each new record has the id after the highest imported one.
        let tabs = database.tabs().list().await.expect("they load");
        let new = tabs.iter().find(|tab| tab.name == "New").expect("is there");
        assert_eq!(new.id.get(), 5);
        let groups = database.groups().list().await.expect("they load");
        let new = groups.iter().find(|group| group.name == "Newcomers");
        assert_eq!(new.map(|group| group.id.get()), Some(8));
        let categories = database.categories().list().await.expect("they load");
        let new = categories.iter().find(|category| category.name == "More");
        assert_eq!(new.map(|category| category.id.get()), Some(3));
        assert_eq!(user(database, "newcomer").await.id, UserId::new(5));
    })
    .await;
}

/// Rows that the PHP application lets in and this application does not.
const DAMAGE: &str = r#"
INSERT INTO tabs (id, "order", category_id, name, url, "default", enabled, group_id, type)
    VALUES (10, 10, 0, 'plex', 'https://other.example.com/', 1, 1, 1, 1);
INSERT INTO tabs (id, "order", category_id, name, url, enabled, group_id, type)
    VALUES (11, 11, 42, 'Orphan', 'https://orphan.example.com/', 1, 5000, 9);
INSERT INTO tabs (id, "order", category_id, name, url, enabled, group_id, group_id_max, type)
    VALUES (12, 12, 0, NULL, '', 1, 1, 4, 1);
INSERT INTO categories (id, "order", category, category_id, image, "default")
    VALUES (9, 9, 'media', 7, '', 1);
INSERT INTO users (id, username, password, email, "group", group_id, register_date)
    VALUES (20, 'SAM', 'x', '', 'Gone', 77, 'last week');
INSERT INTO users (id, username, password, group_id) VALUES (21, '', NULL, 4);
INSERT INTO groups (id, "group", group_id, image, "default") VALUES (30, 'editors', 5, '', 1);
INSERT INTO groups (id, "group", group_id, image, "default") VALUES (31, 'Far', 5000, '', 0);
INSERT INTO groups (id, "group", group_id, image, "default") VALUES (32, 'user', 6, '', 1);
"#;

#[tokio::test]
async fn records_that_the_new_application_cannot_hold_are_repaired() {
    on_each_engine(async |database| {
        let legacy = Legacy::with(DAMAGE).await;

        let report = run(
            legacy.request(),
            database,
            &seed::secrets(),
            &legacy.data_dir(),
        )
        .await
        .expect("the import runs");

        let tabs = database.tabs().list().await.expect("they load");
        let named = |name: &str| {
            tabs.iter()
                .find(|tab| tab.name == name)
                .unwrap_or_else(|| panic!("the tab {name} is there"))
        };
        // Two names that differ only in letter case.
        let second = named("plex (2)");
        assert_eq!(second.id.get(), 10);
        // Only one tab opens first: the one that was stored first.
        assert!(named("Plex").is_default && !second.is_default);
        // A tab in a category that is gone, with a level and a type that cannot be.
        let orphan = named("Orphan");
        let unsorted = database
            .categories()
            .find_default()
            .await
            .expect("it loads")
            .expect("there is one");
        assert_eq!(unsorted.name, "Unsorted");
        assert_eq!(orphan.category, unsorted.id);
        assert_eq!(orphan.kind, TabKind::Iframe);
        assert_eq!(orphan.access.least_access(), GroupLevel::ADMIN);
        // A tab without a name, with a range of levels that holds nobody.
        let nameless = named("Tab 12");
        assert_eq!(nameless.url, None);
        assert_eq!(
            (
                nameless.access.least_access(),
                nameless.access.most_access()
            ),
            (GroupLevel::ADMIN, GroupLevel::ADMIN)
        );

        // A second category with the number and the name of another one.
        let categories = database.categories().list().await.expect("they load");
        let second = categories
            .iter()
            .find(|category| category.name == "media (2)")
            .expect("the category is there");
        assert_eq!(second.number.get(), 8);
        assert!(!second.is_default);
        assert_eq!(named("Plex").category.get(), 2);

        // Groups: one shares a level, one has no level, one is a second default.
        let groups = database.groups().list().await.expect("they load");
        let kept: Vec<&str> = groups.iter().map(|group| group.name.as_str()).collect();
        assert!(
            !kept.contains(&"editors") && !kept.contains(&"Far"),
            "{kept:?}"
        );
        let second = groups
            .iter()
            .find(|group| group.name == "user (2)")
            .expect("the group is there");
        assert!(!second.is_default);

        // Users: a name that differs only in letter case, in a group that is gone, and
        // a user without a name.
        let sam = user(database, "SAM (2)").await;
        let default = database
            .groups()
            .find_default()
            .await
            .expect("it loads")
            .expect("there is one");
        assert_eq!(default.name, "User");
        assert_eq!(sam.group, default.id);
        assert_eq!(sam.email, None);
        assert_eq!(sam.created_at.unix_seconds(), 0);
        let nameless = user(database, "user-21").await;
        assert_eq!(nameless.password_hash.expose(), "");

        let said = |part: &str| report.repairs.iter().any(|line| line.contains(part));
        for part in [
            "The tab \"plex\" is now called \"plex (2)\"",
            "The tabs \"Plex\" and \"plex (2)\" were both the tab that opens first",
            "The tab \"Orphan\" was in a category that is gone and is now in \"Unsorted\"",
            "The tab \"Orphan\" had no type from 0 to 2",
            "The tab \"Orphan\" had no level from 0 to 999",
            "A tab without a name is now called \"Tab 12\"",
            "The tab \"Tab 12\" had a range of levels that no user is in",
            "The category \"media\" is now called \"media (2)\"",
            "The category \"media (2)\" had no number of its own and now has the number 8",
            "The categories \"Unsorted\" and \"media (2)\" were both the category of new tabs",
            "The group \"user\" is now called \"user (2)\"",
            "The groups \"User\" and \"user (2)\" were both the group of new users",
            "The user \"SAM\" is now called \"SAM (2)\"",
            "The user \"SAM (2)\" was in a group that is gone and is now in \"User\"",
            "The date \"last week\" of the user \"SAM (2)\" is no date",
            "A user without a name is now called \"user-21\"",
        ] {
            assert!(said(part), "no repair says: {part}\n{:#?}", report.repairs);
        }
        assert_eq!(report.repairs.len(), 16, "{:#?}", report.repairs);
        assert_eq!(report.skipped.len(), 2, "{:#?}", report.skipped);
        assert!(report.skipped[0].contains("\"editors\" has the level 5 of another group"));
        assert!(report.skipped[1].contains("\"Far\" has no level from 0 to 999"));
    })
    .await;
}

/// The tables of an old install: columns in another order, and without the columns that
/// later versions of the PHP application added. Numbers are stored as text.
const OLD_LAYOUT: &str = r#"
DROP TABLE tabs;
CREATE TABLE tabs (name TEXT, url TEXT, id INTEGER PRIMARY KEY, enabled TEXT, "group_id" TEXT,
                   category_id TEXT, type TEXT, "order" TEXT, image TEXT, "default" TEXT);
INSERT INTO tabs VALUES ('Old', 'https://old.example.com/', 3, '1', '4', '7', '1', '2', '', '0');
DROP TABLE groups;
DROP TABLE categories;
CREATE TABLE categories (category TEXT, id INTEGER PRIMARY KEY, category_id INTEGER);
INSERT INTO categories VALUES ('Media', 5, 7);
"#;

#[tokio::test]
async fn old_table_layout_is_read_by_the_names_of_its_columns() {
    on_each_engine(async |database| {
        let legacy = Legacy::with(OLD_LAYOUT).await;

        let report = run(
            legacy.request(),
            database,
            &seed::secrets(),
            &legacy.data_dir(),
        )
        .await
        .expect("the import runs");

        let tabs = database.tabs().list().await.expect("they load");
        assert_eq!(tabs.len(), 1);
        let old = &tabs[0];
        assert_eq!(
            (old.id.get(), old.position, old.name.as_str()),
            (3, 2, "Old")
        );
        assert_eq!(old.category.get(), 5);
        assert!(old.enabled && !old.is_default && !old.add_to_admin && !old.preload);
        assert_eq!(old.access.least_access().get(), 4);
        assert_eq!(old.access.most_access(), GroupLevel::ADMIN);
        assert_eq!(old.image, None);

        // An install without groups gets the groups that every install needs, and its
        // users go to the group of new users.
        let groups = database.groups().list().await.expect("they load");
        let mut found: Vec<(u16, &str, bool)> = groups
            .iter()
            .map(|group| (group.level.get(), group.name.as_str(), group.is_default))
            .collect();
        found.sort_unstable();
        assert_eq!(
            found,
            [
                (0, "Admin", false),
                (4, "User", true),
                (999, "Guest", false)
            ]
        );
        let user_group = groups.iter().find(|group| group.is_default).expect("one");
        assert_eq!(user(database, "robin").await.group, user_group.id);
        // The Admin of the old install is in the new Admin group.
        let admin_group = groups
            .iter()
            .find(|group| group.level == GroupLevel::ADMIN)
            .expect("one");
        assert_eq!(user(database, "admin").await.group, admin_group.id);

        let category = database
            .categories()
            .find_default()
            .await
            .expect("it loads")
            .expect("there is one");
        assert_eq!((category.name.as_str(), category.position), ("Media", 5));
        assert!(
            report
                .repairs
                .iter()
                .any(|line| line.contains("No category was the category of new tabs")),
            "{:#?}",
            report.repairs
        );
    })
    .await;
}

#[tokio::test]
async fn dry_run_reports_and_writes_nothing() {
    let legacy = Legacy::with(DAMAGE).await;
    let before = std::fs::read(legacy.root().join("data/parity-db/parity.db")).expect("readable");

    let report = dry_run(legacy.request(), &seed::secrets())
        .await
        .expect("the dry run works");

    assert!(report.dry_run);
    assert_eq!((report.users, report.tabs), (6, 7));
    assert_eq!(report.repairs.len(), 16);
    let shown = report.to_string();
    assert!(
        shown.starts_with("Dry run: nothing was written."),
        "{shown}"
    );
    assert!(shown.contains("     6  users"), "{shown}");
    assert!(shown.contains("     1  chat messages"), "{shown}");
    assert!(shown.contains("  - options: 0 rows"), "{shown}");
    assert!(shown.contains("     1  invites"), "{shown}");
    assert!(shown.contains("  - MB-URL"), "{shown}");
    assert!(
        shown.contains("Sessions are not imported (1 found)"),
        "{shown}"
    );
    // The breaking changes come with a pointer to the docs, also when the data shows none.
    assert!(
        shown.contains("Breaking changes for this install"),
        "{shown}"
    );
    assert!(shown.contains("\"Moving from PHP Organizr\""), "{shown}");
    // The report names records and settings, and holds no secret.
    for secret in [
        "made-up",
        "JBSWY3DPEHPK3PXP",
        "parity-hash-key",
        seed::API_KEY,
        "$2y$",
    ] {
        assert!(!shown.contains(secret), "the report shows {secret}");
    }

    let after = std::fs::read(legacy.root().join("data/parity-db/parity.db")).expect("readable");
    assert_eq!(before, after, "the old database is not changed");
    let files: Vec<PathBuf> = std::fs::read_dir(legacy.root().join("data/parity-db"))
        .expect("the folder can be listed")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    assert_eq!(
        files.len(),
        1,
        "no file was added next to the old database: {files:?}"
    );
}

#[tokio::test]
async fn import_needs_an_empty_database() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let legacy = legacy_with_files().await;

        let refused = run(
            legacy.request(),
            database,
            &seed::secrets(),
            &legacy.data_dir(),
        )
        .await;

        assert!(matches!(refused, Err(ImportError::NotEmpty)), "{refused:?}");
        assert!(!legacy.data_dir().exists(), "no file was copied");
        // The database itself refuses too, whoever asks.
        let refused = database
            .import()
            .run(organizr_db::ImportData::default())
            .await;
        assert!(
            matches!(refused, Err(organizr_db::DbError::NotEmpty)),
            "{refused:?}"
        );
        let users = database.users().list().await.expect("they load");
        assert_eq!(users.len(), 1, "the install is as it was");
    })
    .await;
}

#[tokio::test]
async fn import_that_fails_stores_nothing() {
    on_each_engine(async |database| {
        // The second user has the id of the first one, which no database of PHP can
        // hold. The table has no key, so SQLite lets the test store it.
        let legacy = Legacy::with(
            "CREATE TABLE copy AS SELECT * FROM users; DROP TABLE users; \
             ALTER TABLE copy RENAME TO users; \
             INSERT INTO users (id, username, group_id) VALUES (1, 'twin', 4);",
        )
        .await;

        let refused = run(
            legacy.request(),
            database,
            &seed::secrets(),
            &legacy.data_dir(),
        )
        .await;

        assert!(
            matches!(refused, Err(ImportError::Database(_))),
            "{refused:?}"
        );
        assert!(
            database
                .import()
                .is_possible()
                .await
                .expect("it can be read"),
            "the new database is still empty"
        );
    })
    .await;
}

#[tokio::test]
async fn database_is_found_in_the_volume_of_a_container() {
    on_each_engine(async |database| {
        // The layout of a container image: the application in `www/organizr`, and the
        // database where the configuration says, by its path inside the container.
        let config = CONFIG.replace(DATABASE_LINE, "'dbLocation' => '/config/www/db/',");
        assert_ne!(config, CONFIG);
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let web_root = folder.path().join("www/organizr");
        std::fs::create_dir_all(web_root.join("data/config")).expect("the folder can be made");
        std::fs::write(web_root.join("data/config/config.php"), config).expect("writable");
        make_database(&folder.path().join("www/db/parity.db"), "").await;

        let request = ImportRequest {
            from: folder.path(),
            database_file: None,
        };
        let report = run(
            request,
            database,
            &seed::secrets(),
            &folder.path().join("new"),
        )
        .await
        .expect("the import runs");

        assert_eq!(report.users, 4);
    })
    .await;
}

/// Closes a folder for the test, runs a dry run, and opens the folder again so that
/// the temporary folder can be deleted. `file` is a file in the folder. Returns
/// `None` when the file can still be looked at: the user `root` enters every folder,
/// so the test cannot run as that user.
#[cfg(unix)]
async fn dry_run_with_closed_folder(
    legacy: &Legacy,
    file: &Path,
) -> Option<Result<organizr_import::report::Report, ImportError>> {
    use std::os::unix::fs::PermissionsExt;

    let folder = file.parent().expect("the file is in a folder");
    let set = |mode: u32| {
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(mode))
            .expect("the permissions can be set");
    };
    // Without the execute permission, nobody but `root` can look at a file in it.
    set(0o600);
    let result = if std::fs::metadata(file).is_ok() {
        eprintln!("skipped: this user enters a folder without the permission");
        None
    } else {
        Some(dry_run(legacy.request(), &seed::secrets()).await)
    };
    set(0o755);
    result
}

#[cfg(unix)]
#[tokio::test]
async fn database_in_a_folder_that_cannot_be_entered_is_named_with_the_cause() {
    let legacy = Legacy::with("").await;
    let file = legacy.root().join("data/parity-db/parity.db");

    let Some(refused) = dry_run_with_closed_folder(&legacy, &file).await else {
        return;
    };

    let Err(error) = refused else {
        panic!("the dry run worked");
    };
    let shown = error.to_string();
    let ImportError::Inaccessible { path, source } = error else {
        panic!("{error:?}");
    };
    assert_eq!(path, file.display().to_string());
    assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(shown.contains(&path), "{shown}");
    assert!(
        shown.to_lowercase().contains("permission denied"),
        "{shown}"
    );
    assert!(shown.contains("UID 65532"), "{shown}");
    // With the folder open again, the same request works.
    let report = dry_run(legacy.request(), &seed::secrets())
        .await
        .expect("the dry run works");
    assert_eq!(report.users, 4);
}

#[cfg(unix)]
#[tokio::test]
async fn configuration_file_in_a_folder_that_cannot_be_entered_is_named_with_the_cause() {
    let legacy = Legacy::with("").await;
    let file = legacy.root().join("data/config/config.php");

    let Some(refused) = dry_run_with_closed_folder(&legacy, &file).await else {
        return;
    };

    let Err(ImportError::Inaccessible { path, source }) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(path, file.display().to_string());
    assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
}

#[cfg(unix)]
#[tokio::test]
async fn file_that_can_be_read_is_found_behind_a_folder_that_cannot_be_entered() {
    use std::os::unix::fs::PermissionsExt;

    // The configuration names a folder that this user cannot enter. The database is
    // also under the folder of the install, by the end of that path, and is found there.
    let closed = tempfile::tempdir().expect("a temporary folder can be made");
    let location = closed.path().join("parity-db");
    std::fs::create_dir_all(&location).expect("the folder can be made");
    let line = format!("'dbLocation' => '{}/',", location.display());
    let config = CONFIG.replace(DATABASE_LINE, &line);
    assert_ne!(config, CONFIG);
    let legacy = Legacy::build(&config, "parity-db/parity.db", "").await;

    let set = |mode: u32| {
        std::fs::set_permissions(closed.path(), std::fs::Permissions::from_mode(mode))
            .expect("the permissions can be set");
    };
    set(0o600);
    let report = dry_run(legacy.request(), &seed::secrets()).await;
    set(0o755);

    assert_eq!(report.expect("the dry run works").users, 4);
}

#[tokio::test]
async fn folder_without_an_install_or_a_database_is_refused_with_a_reason() {
    let secrets = seed::secrets();

    let empty = tempfile::tempdir().expect("a temporary folder can be made");
    let request = ImportRequest {
        from: empty.path(),
        database_file: None,
    };
    let refused = dry_run(request, &secrets).await;
    assert!(
        matches!(refused, Err(ImportError::NoInstall(_))),
        "{refused:?}"
    );

    // The database is not where the configuration says.
    let legacy = Legacy::build(CONFIG, "elsewhere/other.db", "").await;
    let refused = dry_run(legacy.request(), &secrets).await;
    let Err(ImportError::NoDatabase(tried)) = refused else {
        panic!("{refused:?}");
    };
    assert!(
        tried.iter().any(|path| path.ends_with("parity.db")),
        "{tried:?}"
    );
    // The option names the file.
    let file = legacy.root().join("elsewhere/other.db");
    let request = ImportRequest {
        from: legacy.root(),
        database_file: Some(&file),
    };
    let report = dry_run(request, &secrets).await.expect("the dry run works");
    assert_eq!(report.users, 4);
    let missing = legacy.root().join("elsewhere/missing.db");
    let request = ImportRequest {
        from: legacy.root(),
        database_file: Some(&missing),
    };
    let refused = dry_run(request, &secrets).await;
    assert!(
        matches!(refused, Err(ImportError::LegacyDatabase(_))),
        "{refused:?}"
    );

    // An install on a database server.
    let config = CONFIG.replace("'driver' => 'sqlite3',", "'driver' => 'mysqli',");
    assert_ne!(config, CONFIG);
    let legacy = Legacy::build(&config, "data/parity-db/parity.db", "").await;
    let refused = dry_run(legacy.request(), &secrets).await;
    let Err(ImportError::UnsupportedDriver(driver)) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(driver, "mysqli");

    // A configuration file that is PHP code of another kind is never run.
    let legacy = Legacy::build(
        "<?php\nreturn load_settings();",
        "data/parity-db/parity.db",
        "",
    )
    .await;
    let refused = dry_run(legacy.request(), &secrets).await;
    assert!(
        matches!(refused, Err(ImportError::Config { .. })),
        "{refused:?}"
    );
}

#[tokio::test]
async fn value_that_the_hash_does_not_open_is_kept_and_reported() {
    on_each_engine(async |database| {
        // The hash of the install was changed after the passwords were stored.
        let config = CONFIG.replace("'parity-hash-key-0123456789'", "'another-hash-key'");
        assert_ne!(config, CONFIG);
        let legacy = Legacy::build(&config, "data/parity-db/parity.db", "").await;
        let secrets = seed::secrets();

        let report = run(legacy.request(), database, &secrets, &legacy.data_dir())
            .await
            .expect("the import runs");

        let settings = settings_of(database).await;
        assert_eq!(settings.get("qBittorrentPassword"), Some(json!("")));
        assert_eq!(settings.get("title"), Some(json!("Legacy Install")));
        let unreadable: Vec<&String> = report
            .repairs
            .iter()
            .filter(|line| line.contains("the hash of the install does not open"))
            .collect();
        assert_eq!(unreadable.len(), 2, "{:#?}", report.repairs);
        assert!(unreadable[0].contains("`PHPMAILER-smtpHostPassword`"));
        let kept = database
            .import()
            .legacy_settings()
            .await
            .expect("they load");
        let keys: Vec<&str> = kept.iter().map(|setting| setting.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "MB-URL",
                "PHPMAILER-smtpHostPassword",
                "qBittorrentPassword"
            ]
        );
        // The registration password that PHP could not open is used as it is stored,
        // which is what the set-up wizard of PHP leaves behind.
        let hash = PasswordHash::new(text(settings.get("registrationPassword")));
        assert!(matches!(
            password::verify("E6VMJPh5cNlfuBAgpil+lcbjVe8X40sfms6dqrEXDvM=", &hash),
            PasswordCheck::Correct { .. }
        ));
    })
    .await;
}

/// Records that use pictures which a person added to the PHP install.
const OWN_PICTURES: &str = r#"
UPDATE tabs SET image = 'plugins/images/tabs/custom.png' WHERE name = 'Docs';
INSERT INTO tabs (id, "order", category_id, name, url, enabled, group_id, type, image) VALUES
    (20, 20, 0, 'Clash', 'https://a.example.com/', 1, 1, 1, 'plugins/images/tabs/OWN.PNG'),
    (21, 21, 0, 'Gone', 'https://b.example.com/', 1, 1, 1, 'plugins/images/tabs/gone.png'),
    (22, 22, 0, 'Drawing', 'https://c.example.com/', 1, 1, 1, 'plugins/images/tabs/drawing.webp'),
    (23, 23, 0, 'Outside', 'https://d.example.com/', 1, 1, 1, 'plugins/images/../../outside.png'),
    (24, 24, 0, 'Same', 'https://e.example.com/', 1, 1, 1, 'plugins/images/tabs/custom.png'),
    (25, 25, 0, 'Uploaded', 'https://f.example.com/', 1, 1, 1, 'data/userTabs/own.png'),
    (26, 26, 0, 'Web', 'https://g.example.com/', 1, 1, 1, 'https://example.com/plugins/images/x.png'),
    (27, 27, 0, 'Vector', 'https://h.example.com/', 1, 1, 1, 'plugins/images/tabs/vector.svg');
UPDATE categories SET image = 'plugins/images/categories/mine.gif' WHERE category = 'Media';
UPDATE groups SET image = 'plugins/images/groups/team.jpeg' WHERE "group" = 'Editors';
"#;

/// Writes a file of the made-up install. The content says which file it is.
fn write(root: &Path, path: &str) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().expect("the file is in a folder"))
        .expect("the folder can be made");
    std::fs::write(&file, path).expect("the file can be written");
}

/// The files below a folder, by their path from it, sorted.
fn files_below(folder: &Path) -> Vec<String> {
    fn walk(folder: &Path, from: &Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, from, found);
            } else if let Ok(inside) = path.strip_prefix(from) {
                found.push(inside.display().to_string().replace('\\', "/"));
            }
        }
    }
    let mut found = Vec::new();
    walk(folder, folder, &mut found);
    found.sort();
    found
}

/// The made-up install with pictures and files that a person added.
async fn legacy_with_files() -> Legacy {
    let config = CONFIG.replace(
        "'logo' => 'plugins/images/organizr/organizr-logo-h.png',",
        "'logo' => 'plugins/images/own-logo.png',",
    );
    assert_ne!(config, CONFIG);
    let legacy = Legacy::build(&config, "data/parity-db/parity.db", OWN_PICTURES).await;
    for path in [
        "data/userTabs/own.png",
        "data/userTabs/second.JPG",
        "data/favicon/favicon.ico",
        "data/favicon/site.webmanifest",
        "data/cert/custom.pem",
        "data/cache/poster.jpg",
        "data/pages/custom.php",
        "data/pages/help.html",
        "data/routes/more/extra.PHP",
        "data/backup-of-mine/old.txt",
        "plugins/images/tabs/custom.png",
        "plugins/images/tabs/OWN.PNG",
        "plugins/images/tabs/drawing.webp",
        "plugins/images/tabs/vector.svg",
        "plugins/images/tabs/unused.jpg",
        "plugins/images/tabs/notes.txt",
        // A picture that the application ships: it stays where it is.
        "plugins/images/tabs/plex.png",
        "plugins/images/categories/mine.gif",
        "plugins/images/categories/unused.png",
        "plugins/images/groups/team.jpeg",
        "plugins/images/own-logo.png",
        "outside.png",
    ] {
        write(legacy.root(), path);
    }
    legacy
}

#[tokio::test]
async fn pictures_and_files_of_the_install_are_copied() {
    on_each_engine(async |database| {
        let legacy = legacy_with_files().await;
        let data_dir = legacy.data_dir();

        let report = run(legacy.request(), database, &seed::secrets(), &data_dir)
            .await
            .expect("the import runs");

        assert_eq!(
            files_below(&data_dir),
            [
                "cert/custom.pem",
                "favicon/favicon.ico",
                "favicon/site.webmanifest",
                "userTabs/custom.png",
                "userTabs/mine.gif",
                "userTabs/own-logo.png",
                "userTabs/own.png",
                "userTabs/second.JPG",
                // The name `own.png` was taken by an uploaded picture.
                "userTabs/tabs-OWN.PNG",
                "userTabs/team.jpeg",
                "userTabs/unused.jpg",
                "userTabs/vector.svg",
            ]
        );
        let content = |path: &str| {
            std::fs::read_to_string(data_dir.join(path)).expect("the file can be read")
        };
        assert_eq!(content("userTabs/own.png"), "data/userTabs/own.png");
        assert_eq!(
            content("userTabs/tabs-OWN.PNG"),
            "plugins/images/tabs/OWN.PNG"
        );
        assert_eq!(
            content("userTabs/mine.gif"),
            "plugins/images/categories/mine.gif"
        );

        // The records point at the new place of each picture that moved.
        let tabs = database.tabs().list().await.expect("they load");
        let picture = |name: &str| {
            tabs.iter()
                .find(|tab| tab.name == name)
                .unwrap_or_else(|| panic!("the tab {name} is there"))
                .image
                .clone()
                .unwrap_or_default()
        };
        assert_eq!(picture("Docs"), "data/userTabs/custom.png");
        assert_eq!(picture("Same"), "data/userTabs/custom.png");
        assert_eq!(picture("Clash"), "data/userTabs/tabs-OWN.PNG");
        assert_eq!(picture("Uploaded"), "data/userTabs/own.png");
        // What does not move stays as it is.
        assert_eq!(picture("Plex"), "plugins/images/tabs/plex.png");
        assert_eq!(picture("Gone"), "plugins/images/tabs/gone.png");
        assert_eq!(picture("Drawing"), "plugins/images/tabs/drawing.webp");
        assert_eq!(picture("Vector"), "data/userTabs/vector.svg");
        assert_eq!(picture("Outside"), "plugins/images/../../outside.png");
        assert_eq!(picture("Web"), "https://example.com/plugins/images/x.png");
        let categories = database.categories().list().await.expect("they load");
        let media = categories.iter().find(|category| category.name == "Media");
        assert_eq!(
            media.and_then(|category| category.image.as_deref()),
            Some("data/userTabs/mine.gif")
        );
        let groups = database.groups().list().await.expect("they load");
        let editors = groups.iter().find(|group| group.name == "Editors");
        assert_eq!(
            editors.and_then(|group| group.image.as_deref()),
            Some("data/userTabs/team.jpeg")
        );
        let settings = settings_of(database).await;
        assert_eq!(
            settings.get("logo"),
            Some(json!("data/userTabs/own-logo.png"))
        );
        assert_eq!(
            settings.get("loginLogo"),
            Some(json!("plugins/images/organizr/organizr-logo-h.png"))
        );

        assert_eq!((report.pictures, report.other_files), (9, 3));
        assert_eq!(
            report.php_files,
            ["data/pages/custom.php", "data/routes/more/extra.PHP"]
        );
        assert_eq!(
            report.notes,
            [
                "7 pictures that a person added to the picture folders of the PHP application \
                 are now with the uploaded pictures. 6 of them are used by a record, which \
                 points to the new place.",
                "A record uses the picture plugins/images/tabs/gone.png, which the PHP install \
                 does not have. Choose another picture for it.",
                "A record uses the picture plugins/images/tabs/drawing.webp. The application shows \
                 only GIF, PNG, JPEG and SVG pictures that were added by a person. Choose \
                 another picture for it.",
                "The folder data/backup-of-mine of the PHP install is not copied. Look at what \
                 it holds.",
            ]
        );
    })
    .await;
}

/// The custom certificate of PHP goes to `cert/custom.pem`, where the server reads it.
/// A file that holds a private key is not copied, so that no key comes into the data
/// folder.
#[tokio::test]
async fn custom_certificate_is_copied_unless_it_holds_a_private_key() {
    on_each_engine(async |database| {
        let legacy = Legacy::build(CONFIG, "data/parity-db/parity.db", OWN_PICTURES).await;
        let file = legacy.root().join("data/cert/custom.pem");
        std::fs::create_dir_all(file.parent().expect("the file is in a folder"))
            .expect("the folder can be made");
        std::fs::write(
            &file,
            concat!(
                "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n",
                "-----BEGIN PRIVATE KEY-----\nMIIE\n-----END PRIVATE KEY-----\n",
            ),
        )
        .expect("the file can be written");
        let data_dir = legacy.data_dir();

        let report = run(legacy.request(), database, &seed::secrets(), &data_dir)
            .await
            .expect("the import runs");

        assert!(!data_dir.join("cert/custom.pem").exists());
        assert_eq!(report.other_files, 0);
        assert!(
            report.notes.contains(
                &"The custom certificate data/cert/custom.pem of the PHP install holds a \
                  private key, so it is not copied. Upload the certificate of the authority \
                  alone in Settings, General."
                    .to_owned()
            ),
            "{:?}",
            report.notes
        );
    })
    .await;
}

#[tokio::test]
async fn dry_run_counts_the_files_and_copies_none() {
    let legacy = legacy_with_files().await;
    let before = files_below(legacy.root());

    let report = dry_run(legacy.request(), &seed::secrets())
        .await
        .expect("the dry run works");

    assert_eq!((report.pictures, report.other_files), (9, 3));
    assert_eq!(report.php_files.len(), 2);
    assert_eq!(report.notes.len(), 4);
    assert_eq!(files_below(legacy.root()), before);
    let shown = report.to_string();
    assert!(shown.contains("     9  uploaded pictures"), "{shown}");
    assert!(shown.contains("  - data/pages/custom.php"), "{shown}");
}

#[tokio::test]
async fn import_runs_again_after_one_that_failed_late() {
    on_each_engine(async |database| {
        let legacy = legacy_with_files().await;
        let data_dir = legacy.data_dir();
        // A file of an earlier try is in the way.
        write(&data_dir, "userTabs/own.png");

        run(legacy.request(), database, &seed::secrets(), &data_dir)
            .await
            .expect("the import runs");

        assert_eq!(
            std::fs::read_to_string(data_dir.join("userTabs/own.png")).expect("readable"),
            "data/userTabs/own.png"
        );
    })
    .await;
}

#[tokio::test]
async fn bookmarks_of_the_plugin_are_imported_with_their_ids() {
    on_each_engine(async |database| {
        let legacy = Legacy::with("").await;
        let secrets = seed::secrets();

        let report = run(legacy.request(), database, &secrets, &legacy.data_dir())
            .await
            .expect("the import runs");

        // The invites plugin brings its invite, which can still be used.
        assert_eq!(report.invites, 1);
        let invites = database.invites();
        let invite = invites
            .find("madeup")
            .await
            .expect("the invite loads")
            .expect("the invite is there");
        assert_eq!((invite.id, invite.code.as_str()), (1, "MADEUP"));
        assert_eq!(invite.created_at, 1_767_323_045);
        assert_eq!(invite.email, "guest@example.com");
        assert_eq!(
            (invite.kind.as_str(), invite.invited_by.as_str()),
            ("plex", "admin")
        );
        assert!(invite.is_valid);
        // A new invite gets the id after the highest one.
        let next = invites
            .insert(organizr_domain::NewInvite {
                code: "next".to_owned(),
                created_at: 1,
                email: "pat@example.com".to_owned(),
                username: "pat".to_owned(),
                kind: "plex".to_owned(),
                invited_by: "admin".to_owned(),
            })
            .await
            .expect("an invite can be added");
        assert_eq!(next.id, 2);

        // The bookmark plugin brings its category and its bookmark, with their ids.
        assert_eq!((report.bookmark_categories, report.bookmarks), (1, 1));
        let categories = database
            .bookmarks()
            .categories()
            .await
            .expect("the categories load");
        assert_eq!(categories.len(), 1);
        assert_eq!(
            (
                categories[0].id,
                categories[0].number,
                categories[0].name.as_str()
            ),
            (1, 1, "Reading")
        );
        assert!(categories[0].is_default);
        let bookmarks = database
            .bookmarks()
            .list()
            .await
            .expect("the bookmarks load");
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].name, "News");
        assert_eq!(bookmarks[0].url, "https://news.example.com/");
        assert_eq!(bookmarks[0].least_access, 999);
        assert_eq!(bookmarks[0].background_color, "#000000");
        // A new bookmark gets the id after the one that was imported.
        let next = database
            .bookmarks()
            .insert(organizr_domain::NewBookmark {
                position: 2,
                category: 1,
                name: "Wiki".to_owned(),
                url: "https://wiki.example.com/".to_owned(),
                enabled: true,
                least_access: 4,
                image: String::new(),
                background_color: "#fff".to_owned(),
                text_color: "#000".to_owned(),
            })
            .await
            .expect("a new bookmark is stored");
        assert_eq!(next.id, 2);
    })
    .await;
}
