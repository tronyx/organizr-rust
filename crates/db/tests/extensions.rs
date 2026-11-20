//! Approved extensions must be stored the same way on `SQLite` and on `PostgreSQL`.

use organizr_testkit::engines::on_each_engine;

#[tokio::test]
async fn extension_is_approved_switched_and_forgotten() {
    on_each_engine(async |database| {
        let extensions = database.extensions();
        assert_eq!(extensions.find("hello").await.expect("it loads"), None);
        assert!(
            !extensions
                .set_enabled("hello", true)
                .await
                .expect("it works")
        );

        extensions
            .approve("hello", "abc123", "storage", 4)
            .await
            .expect("it is stored");
        extensions
            .approve("another", "ffff", "", 999)
            .await
            .expect("it is stored");
        let hello = extensions
            .find("hello")
            .await
            .expect("it loads")
            .expect("it is there");
        assert!(hello.enabled);
        assert_eq!(
            (
                hello.sha256.as_str(),
                hello.permissions.as_str(),
                hello.least_access
            ),
            ("abc123", "storage", 4)
        );
        assert_eq!(hello.values, "{}");

        extensions
            .set_values("hello", r#"{"count":"2"}"#)
            .await
            .expect("it works");
        assert!(
            extensions
                .set_enabled("hello", false)
                .await
                .expect("it works")
        );
        let off = extensions
            .find("hello")
            .await
            .expect("it loads")
            .expect("it is there");
        assert!(!off.enabled);
        assert_eq!(off.values, r#"{"count":"2"}"#);

        // A new approval switches the extension on and keeps its values.
        extensions
            .approve("hello", "def456", "", 1)
            .await
            .expect("it is stored");
        let again = extensions
            .find("hello")
            .await
            .expect("it loads")
            .expect("it is there");
        assert!(again.enabled);
        assert_eq!(
            (
                again.sha256.as_str(),
                again.permissions.as_str(),
                again.least_access
            ),
            ("def456", "", 1)
        );
        assert_eq!(again.values, r#"{"count":"2"}"#);

        let ids: Vec<String> = extensions
            .list()
            .await
            .expect("it loads")
            .into_iter()
            .map(|extension| extension.id)
            .collect();
        assert_eq!(ids, ["another", "hello"]);
        assert!(extensions.delete("hello").await.expect("it works"));
        assert!(!extensions.delete("hello").await.expect("it works"));
        assert_eq!(extensions.list().await.expect("it loads").len(), 1);
    })
    .await;
}
