//! The notice of an import: the breaking changes that the import found, which an Admin
//! sees after the login until an Admin hides them (D-158).

use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call};
use organizr_testkit::{app_with, installed_settings, seed};
use serde_json::json;

const NOTICE: &str = r#"["Service addresses that are only a path (D-154).","Co-Admins (D-131)."]"#;

#[tokio::test]
async fn admin_sees_the_notice_until_an_admin_hides_it() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with(
            database,
            installed_settings(&[
                ("importNotice", json!(NOTICE)),
                ("loginAttempts", json!("1000000")),
            ]),
        );
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;

        let launch = call(&app, As::Admin, "GET", "/launch", None).await;
        assert_eq!(launch.status, StatusCode::OK, "{}", launch.body);
        assert_eq!(
            launch.data()["importNotice"],
            json!([
                "Service addresses that are only a path (D-154).",
                "Co-Admins (D-131)."
            ])
        );
        // Nobody else gets it.
        for caller in [As::User(&co_admin), As::Guest] {
            let launch = call(&app, caller, "GET", "/launch", None).await;
            assert!(
                launch.data().get("importNotice").is_none(),
                "{}",
                launch.body
            );
        }

        // A Co-Admin neither hides nor changes it.
        let refused = call(&app, As::User(&co_admin), "DELETE", "/import-notice", None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{}", refused.body);
        let body = json!({"importNotice": "[\"Something else.\"]"});
        let refused = call(&app, As::User(&co_admin), "PUT", "/config", Some(body)).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{}", refused.body);

        let hidden = call(&app, As::Admin, "DELETE", "/import-notice", None).await;
        assert_eq!(hidden.status, StatusCode::OK, "{}", hidden.body);
        let launch = call(&app, As::Admin, "GET", "/launch", None).await;
        assert!(
            launch.data().get("importNotice").is_none(),
            "{}",
            launch.body
        );
    })
    .await;
}
