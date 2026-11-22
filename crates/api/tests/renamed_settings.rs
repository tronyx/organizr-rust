//! The values of settings that PHP's defaults name differently from the key that PHP
//! reads move to that key when the server starts (D-112).

#![allow(
    clippy::expect_used,
    reason = "a test reads the stored rows it made, and its helpers may panic"
)]

use organizr_api::adopt_renamed_settings;
use organizr_auth::secrets::{SecretBox, legacy_setting_name};
use organizr_db::{Database, ImportData, StoredSetting};
use organizr_testkit::engines::on_each_engine;
use serde_json::{Value, json};

/// The stored row of a setting, if it has one.
async fn row(database: &Database, key: &str) -> Option<String> {
    database
        .settings()
        .all()
        .await
        .expect("the settings can be read")
        .into_iter()
        .find(|setting| setting.key == key)
        .map(|setting| setting.value)
}

/// A secret as it is stored under `key`.
fn sealed(secrets: &SecretBox, key: &str, secret: &str) -> String {
    Value::String(secrets.seal(key, secret).expect("a secret can be sealed")).to_string()
}

/// The secret in a stored row of `key`.
fn opened(secrets: &SecretBox, key: &str, stored: &str) -> String {
    let sealed: String = serde_json::from_str(stored).expect("a secret is stored as a sealed text");
    secrets
        .open(key, &sealed)
        .expect("the secret opens under its name")
}

#[tokio::test]
async fn values_under_the_names_of_php_defaults_move_to_the_names_that_php_reads() {
    on_each_engine(async |database| {
        let secrets = SecretBox::with_random_key().expect("a key can be made");
        let settings = database.settings();
        settings
            .put("ProwlarrURL", &json!("http://prowlarr:9696").to_string())
            .await
            .expect("stored");
        settings
            .put(
                "ProwlarrToken",
                &sealed(&secrets, "ProwlarrToken", "the-key"),
            )
            .await
            .expect("stored");
        // A value under the new name wins over the one under the old name.
        settings
            .put("ProwlarrDisableCertCheck", "true")
            .await
            .expect("stored");
        settings
            .put("prowlarrDisableCertCheck", "false")
            .await
            .expect("stored");

        let notes = adopt_renamed_settings(database, &secrets)
            .await
            .expect("the values move");

        assert_eq!(notes.len(), 3, "{notes:#?}");
        assert_eq!(
            row(database, "prowlarrURL").await.as_deref(),
            Some("\"http://prowlarr:9696\"")
        );
        let token = row(database, "prowlarrToken").await.expect("a token");
        assert_eq!(opened(&secrets, "prowlarrToken", &token), "the-key");
        assert_eq!(
            row(database, "prowlarrDisableCertCheck").await.as_deref(),
            Some("false")
        );
        for old in ["ProwlarrURL", "ProwlarrToken", "ProwlarrDisableCertCheck"] {
            assert_eq!(row(database, old).await, None, "`{old}` is gone");
        }

        // The second start finds nothing to do.
        let again = adopt_renamed_settings(database, &secrets)
            .await
            .expect("nothing moves");
        assert_eq!(again, [] as [String; 0]);
    })
    .await;
}

#[tokio::test]
async fn values_that_an_earlier_import_kept_aside_are_used() {
    on_each_engine(async |database| {
        let secrets = SecretBox::with_random_key().expect("a key can be made");
        let legacy = |key: &str, value: Value| StoredSetting {
            key: key.to_owned(),
            value: secrets
                .seal(&legacy_setting_name(key), &value.to_string())
                .expect("a value can be sealed"),
        };
        database
            .import()
            .run(ImportData {
                legacy_settings: vec![
                    legacy("prowlarrURL", json!("http://prowlarr:9696")),
                    legacy("prowlarrToken", json!("the-key")),
                    legacy("prowlarrUseCustomCertificate", json!(false)),
                    legacy("somethingElse", json!("stays")),
                ],
                ..ImportData::default()
            })
            .await
            .expect("the import runs");

        let notes = adopt_renamed_settings(database, &secrets)
            .await
            .expect("the values move");

        assert_eq!(notes.len(), 2, "{notes:#?}");
        assert_eq!(
            row(database, "prowlarrURL").await.as_deref(),
            Some("\"http://prowlarr:9696\"")
        );
        let token = row(database, "prowlarrToken").await.expect("a token");
        assert_eq!(opened(&secrets, "prowlarrToken", &token), "the-key");
        // A default needs no row.
        assert_eq!(row(database, "prowlarrUseCustomCertificate").await, None);
        let left: Vec<String> = database
            .import()
            .legacy_settings()
            .await
            .expect("the legacy settings can be read")
            .into_iter()
            .map(|setting| setting.key)
            .collect();
        assert_eq!(left, ["somethingElse"]);
    })
    .await;
}
