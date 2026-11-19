//! Setting up a new install, on both engines.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::time::Duration;

use organizr_auth::install::{self, InstallError, SetUp};
use organizr_auth::login::{self, Device};
use organizr_auth::password::{self, PasswordCheck};
use organizr_auth::secrets::PREFIX;
use organizr_db::Database;
use organizr_domain::{GroupLevel, PasswordHash, Timestamp};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::seed;

const NOW: Timestamp = Timestamp::from_unix_seconds(1_800_000_000);

fn request() -> SetUp {
    SetUp {
        username: "Robin".to_owned(),
        password: "the password of robin".to_owned(),
        email: "Robin@Example.com".to_owned(),
        api_key: "abcdefghij0123456789".to_owned(),
        license: "personal".to_owned(),
        registration_password: "let me in".to_owned(),
    }
}

/// The stored settings: key to value, with the JSON quotes taken off.
async fn settings(database: &Database) -> BTreeMap<String, String> {
    database
        .settings()
        .all()
        .await
        .expect("the settings can be read")
        .into_iter()
        .map(|setting| {
            let value: String =
                serde_json::from_str(&setting.value).expect("the stored value is JSON text");
            (setting.key, value)
        })
        .collect()
}

#[tokio::test]
async fn set_up_creates_an_admin_who_can_log_in() {
    on_each_engine(async |database| {
        let admin = install::set_up(database, request(), &seed::secrets(), NOW)
            .await
            .expect("the set-up works");

        assert_eq!(admin.username, "Robin");
        assert_eq!(admin.email.as_deref(), Some("Robin@Example.com"));
        assert_eq!(admin.created_at, NOW);
        // The picture is found by the address in lower case.
        assert_eq!(
            admin.image.as_deref(),
            Some("https://www.gravatar.com/avatar/d3c17ef52c397921a87c1de66f2311c2?s=100&d=mm")
        );
        let group = database
            .groups()
            .find(admin.group)
            .await
            .expect("the lookup works")
            .expect("the group exists");
        assert_eq!(group.level, GroupLevel::ADMIN);

        // The password is stored as a current hash, and it works.
        assert!(admin.password_hash.expose().starts_with("$argon2id$"));
        let device = Device {
            ip: IpAddr::from([192, 0, 2, 7]),
            user_agent: None,
        };
        let logged_in = login::with_password(
            database,
            "robin",
            "the password of robin",
            device,
            NOW,
            Duration::from_hours(24),
        )
        .await
        .expect("the admin can log in");
        assert_eq!(logged_in.user, admin);
    })
    .await;
}

#[tokio::test]
async fn set_up_writes_the_settings_of_the_install() {
    on_each_engine(async |database| {
        install::set_up(database, request(), &seed::secrets(), NOW)
            .await
            .expect("the set-up works");

        let settings = settings(database).await;
        let keys: Vec<&str> = settings.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            ["license", "organizrAPI", "registrationPassword", "uuid"]
        );
        assert_eq!(settings["license"], "personal");

        // The API key is a secret: it is stored sealed, and the key of the install opens it.
        let secrets = seed::secrets();
        let api_key = &settings["organizrAPI"];
        assert!(api_key.starts_with(PREFIX), "{api_key}");
        assert!(!api_key.contains("abcdefghij0123456789"));
        assert_eq!(
            secrets.open("organizrAPI", api_key).expect("the key opens"),
            "abcdefghij0123456789"
        );

        // A new random id, in the form of a UUID.
        let id = &settings["uuid"];
        assert_eq!(id.len(), 36, "{id}");
        assert_eq!(id.matches('-').count(), 4, "{id}");

        // The registration password is stored as a hash, never as it was entered.
        let sealed = &settings["registrationPassword"];
        assert!(sealed.starts_with(PREFIX), "{sealed}");
        let hash = PasswordHash::new(
            secrets
                .open("registrationPassword", sealed)
                .expect("the hash opens"),
        );
        assert!(hash.expose().starts_with("$argon2id$"));
        assert_eq!(
            password::verify("let me in", &hash),
            PasswordCheck::Correct { outdated: false }
        );
        assert_eq!(password::verify("let me out", &hash), PasswordCheck::Wrong);
    })
    .await;
}

#[tokio::test]
async fn every_install_gets_an_id_of_its_own() {
    on_each_engine(async |first| {
        install::set_up(first, request(), &seed::secrets(), NOW)
            .await
            .expect("the set-up works");
        let first_id = settings(first).await["uuid"].clone();

        on_each_engine(async |second| {
            install::set_up(second, request(), &seed::secrets(), NOW)
                .await
                .expect("the set-up works");

            assert_ne!(settings(second).await["uuid"], first_id);
        })
        .await;
    })
    .await;
}

#[tokio::test]
async fn install_cannot_be_set_up_twice() {
    on_each_engine(async |database| {
        install::set_up(database, request(), &seed::secrets(), NOW)
            .await
            .expect("the first set-up works");
        let before = settings(database).await;

        let second = install::set_up(
            database,
            SetUp {
                username: "intruder".to_owned(),
                ..request()
            },
            &seed::secrets(),
            NOW,
        )
        .await;

        assert!(
            matches!(second, Err(InstallError::AlreadySetUp)),
            "{second:?}"
        );
        assert_eq!(database.users().count().await.expect("the count works"), 1);
        assert_eq!(settings(database).await, before);
    })
    .await;
}
