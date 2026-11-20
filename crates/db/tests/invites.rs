//! Invites must be stored the same way on `SQLite` and on `PostgreSQL`.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_db::DbError;
use organizr_domain::NewInvite;
use organizr_testkit::engines::on_each_engine;

fn invite(code: &str, by: &str) -> NewInvite {
    NewInvite {
        code: code.to_owned(),
        created_at: 1_795_000_000,
        email: "pat@example.com".to_owned(),
        username: "pat".to_owned(),
        kind: "plex".to_owned(),
        invited_by: by.to_owned(),
    }
}

#[tokio::test]
async fn invite_is_found_by_its_code_and_used_once() {
    on_each_engine(async |database| {
        let invites = database.invites();
        let stored = invites
            .insert(invite("AbC123", "robin"))
            .await
            .expect("it is stored");
        assert!(stored.is_valid);
        assert_eq!(stored.used_at, None);

        // A code is found whatever its letter case, and cannot be stored twice.
        assert_eq!(
            invites.find("abc123").await.expect("it loads"),
            Some(stored.clone())
        );
        assert_eq!(invites.find("other").await.expect("it loads"), None);
        assert!(matches!(
            invites.insert(invite("ABC123", "zoe")).await,
            Err(DbError::Duplicate)
        ));

        let used = invites
            .mark_used(
                "ABC123",
                Some("pat@plex.example.com"),
                "192.0.2.7",
                1_795_000_100,
            )
            .await
            .expect("it works");
        assert!(used);
        let now = invites
            .find("AbC123")
            .await
            .expect("it loads")
            .expect("it is there");
        assert!(!now.is_valid);
        assert_eq!(now.used_at, Some(1_795_000_100));
        assert_eq!(now.used_by.as_deref(), Some("pat@plex.example.com"));
        assert_eq!(now.used_from.as_deref(), Some("192.0.2.7"));
        // A second use changes nothing.
        let again = invites
            .mark_used("abc123", Some("somebody else"), "192.0.2.8", 1_795_000_200)
            .await
            .expect("it works");
        assert!(!again);
        let still = invites
            .find("AbC123")
            .await
            .expect("it loads")
            .expect("it is there");
        assert_eq!(still.used_by.as_deref(), Some("pat@plex.example.com"));

        // A restored invite is as it was before its use.
        invites.restore("ABC123").await.expect("it works");
        assert_eq!(
            invites.find("abc123").await.expect("it loads"),
            Some(stored)
        );
        assert!(
            !invites
                .mark_used("none", None, "192.0.2.8", 1)
                .await
                .expect("it works")
        );
    })
    .await;
}

#[tokio::test]
async fn invites_are_listed_counted_and_deleted() {
    on_each_engine(async |database| {
        let invites = database.invites();
        for (code, by) in [("one", "robin"), ("two", "zoe"), ("three", "robin")] {
            invites
                .insert(invite(code, by))
                .await
                .expect("it is stored");
        }

        let codes = |list: Vec<organizr_domain::Invite>| {
            list.into_iter().map(|one| one.code).collect::<Vec<_>>()
        };
        assert_eq!(
            codes(invites.list(None).await.expect("it loads")),
            ["one", "two", "three"]
        );
        assert_eq!(
            codes(invites.list(Some("robin")).await.expect("it loads")),
            ["one", "three"]
        );
        assert_eq!(invites.count_of("robin").await.expect("it works"), 2);
        assert_eq!(invites.count_of("nobody").await.expect("it works"), 0);

        assert!(invites.delete("ONE").await.expect("it works"));
        assert!(!invites.delete("one").await.expect("it works"));
        assert_eq!(invites.list(None).await.expect("it loads").len(), 2);
    })
    .await;
}
