//! Settings must be stored the same way on `SQLite` and on `PostgreSQL`.
//!
//! Every test here runs once per engine. See `organizr_testkit::engines` for how the
//! engines are chosen.

use organizr_db::StoredSetting;
use organizr_settings::{AuthType, Settings};
use organizr_testkit::engines::on_each_engine;

fn stored(key: &str, value: &str) -> StoredSetting {
    StoredSetting {
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

#[tokio::test]
async fn new_database_stores_no_settings() {
    on_each_engine(async |database| {
        let all = database
            .settings()
            .all()
            .await
            .expect("the settings can be read");

        assert_eq!(all, []);
    })
    .await;
}

#[tokio::test]
async fn stored_settings_come_back_ordered_by_key() {
    on_each_engine(async |database| {
        let settings = database.settings();
        settings
            .put("rememberMeDays", "\"14\"")
            .await
            .expect("the value can be stored");
        settings
            .put("authType", "\"both\"")
            .await
            .expect("the value can be stored");
        settings
            .put("lockScreen", "true")
            .await
            .expect("the value can be stored");

        let all = settings.all().await.expect("the settings can be read");

        assert_eq!(
            all,
            [
                stored("authType", "\"both\""),
                stored("lockScreen", "true"),
                stored("rememberMeDays", "\"14\""),
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn key_order_is_the_same_on_every_engine() {
    on_each_engine(async |database| {
        let settings = database.settings();
        for key in [
            "alpha",
            "Zeta",
            "Beta",
            "HEALTHCHECKS-enabled",
            "homepageOrderPlex",
        ] {
            settings
                .put(key, "true")
                .await
                .expect("the value can be stored");
        }

        let all = settings.all().await.expect("the settings can be read");

        // Byte order: capital letters come before small letters.
        let keys: Vec<&str> = all.iter().map(|setting| setting.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "Beta",
                "HEALTHCHECKS-enabled",
                "Zeta",
                "alpha",
                "homepageOrderPlex"
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn storing_a_key_again_replaces_its_value() {
    on_each_engine(async |database| {
        let settings = database.settings();
        settings
            .put("title", "\"First\"")
            .await
            .expect("the value can be stored");

        settings
            .put("title", "\"Second\"")
            .await
            .expect("the value can be replaced");

        let all = settings.all().await.expect("the settings can be read");
        assert_eq!(all, [stored("title", "\"Second\"")]);
    })
    .await;
}

#[tokio::test]
async fn value_with_any_characters_comes_back_unchanged() {
    on_each_engine(async |database| {
        let value = "\"Zoë's \\\"home\\\" – 日本語 😀 <b>&amp;</b>\\n\"";
        let settings = database.settings();

        settings
            .put("title", value)
            .await
            .expect("the value can be stored");

        let all = settings.all().await.expect("the settings can be read");
        assert_eq!(all, [stored("title", value)]);
    })
    .await;
}

#[tokio::test]
async fn stored_value_can_be_removed_once() {
    on_each_engine(async |database| {
        let settings = database.settings();
        settings
            .put("title", "\"Home\"")
            .await
            .expect("the value can be stored");

        let first = settings.remove("title").await.expect("the removal works");
        let second = settings.remove("title").await.expect("the removal works");

        assert!(first, "a value was stored");
        assert!(!second, "the value is gone");
        assert_eq!(settings.all().await.expect("the settings can be read"), []);
    })
    .await;
}

#[tokio::test]
async fn typed_settings_are_built_from_what_is_stored() {
    on_each_engine(async |database| {
        let store = database.settings();
        store
            .put("rememberMeDays", "\"14\"")
            .await
            .expect("the value can be stored");
        store
            .put("authType", "\"both\"")
            .await
            .expect("the value can be stored");

        let all = store.all().await.expect("the settings can be read");
        let (settings, problems) = Settings::from_stored(
            all.iter()
                .map(|setting| (setting.key.as_str(), setting.value.as_str())),
        );

        assert_eq!(problems, []);
        assert_eq!(settings.remember_me_days, 14);
        assert_eq!(settings.auth_type, AuthType::Both);
        assert_eq!(
            settings.login_attempts, 5,
            "a setting without a row keeps its default"
        );
    })
    .await;
}

#[tokio::test]
async fn several_settings_are_stored_and_removed_in_one_step() {
    on_each_engine(async |database| {
        let settings = database.settings();
        settings
            .put("title", "\"Old\"")
            .await
            .expect("the value can be stored");
        settings
            .put("lockScreen", "true")
            .await
            .expect("the value can be stored");
        settings
            .put("style", "\"light\"")
            .await
            .expect("the value can be stored");

        settings
            .apply(
                &[
                    stored("title", "\"Home\""),
                    stored("rememberMeDays", "\"14\""),
                ],
                &["lockScreen".to_owned(), "notStored".to_owned()],
            )
            .await
            .expect("the change can be stored");

        let all = settings.all().await.expect("the settings can be read");
        assert_eq!(
            all,
            [
                stored("rememberMeDays", "\"14\""),
                stored("style", "\"light\""),
                stored("title", "\"Home\""),
            ]
        );

        // A change without anything in it is fine, and changes nothing.
        settings
            .apply(&[], &[])
            .await
            .expect("an empty change can be stored");
        assert_eq!(settings.all().await.expect("the settings can be read"), all);
    })
    .await;
}
