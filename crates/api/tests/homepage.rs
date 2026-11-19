//! The list of the items of the homepage, and the custom HTML items.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::json;

const HTML: &str = "<div class=\"note\">Hello <script>window.x = 1</script></div>";

fn header<'a>(answer: &'a Answer, name: &str) -> Option<&'a str> {
    answer
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
}

#[tokio::test]
async fn items_are_the_ones_that_the_caller_sees_in_the_order_of_the_homepage() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                // Item 03 comes first, for users. Item 01 is for Admins only.
                ("homepageCustomHTML01Enabled", json!(true)),
                ("customHTML01", json!(HTML)),
                ("homepageCustomHTML03Enabled", json!(true)),
                ("homepageCustomHTML03Auth", json!("4")),
                ("customHTML03", json!("<p>Three</p>")),
                ("homepageOrdercustomhtml03", json!("0")),
                // Item 02 is on and holds nothing. Item 04 holds HTML and is off.
                ("homepageCustomHTML02Enabled", json!(true)),
                ("customHTML04", json!("<p>Four</p>")),
            ],
        );
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let three = json!({"key": "homepageOrdercustomhtml03", "kind": "customhtml", "id": "03"});
        let one = json!({"key": "homepageOrdercustomhtml01", "kind": "customhtml", "id": "01"});

        let admin = call(&app, As::Admin, "GET", "/homepage/items", None).await;
        assert_eq!(admin.status, StatusCode::OK, "{}", admin.body);
        assert_eq!(admin.data(), json!([three, one]));

        let seen = call(&app, As::User(&user), "GET", "/homepage/items", None).await;
        assert_eq!(seen.data(), json!([three]));

        let guest = call(&app, As::Guest, "GET", "/homepage/items", None).await;
        assert_eq!(guest.status, StatusCode::OK);
        assert_eq!(guest.data(), json!([]));
    })
    .await;
}

#[tokio::test]
async fn custom_html_is_a_document_of_no_site() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("homepageCustomHTML01Enabled", json!(true)),
                ("customHTML01", json!(HTML)),
            ],
        );

        let path = "/homepage/html/01?style=/assets/index-abc.css";
        let shown = call(&app, As::Admin, "GET", path, None).await;

        assert_eq!(shown.status, StatusCode::OK, "{}", shown.body);
        assert_eq!(
            header(&shown, "content-type"),
            Some("text/html; charset=utf-8")
        );
        // The HTML is not changed, and the style sheet of the interface is linked.
        assert!(shown.body.contains(HTML), "{}", shown.body);
        assert!(
            shown
                .body
                .contains("<link rel=\"stylesheet\" href=\"/assets/index-abc.css\">")
        );
        let policy = header(&shown, "content-security-policy").expect("the answer has a policy");
        assert!(policy.starts_with("sandbox allow-scripts "), "{policy}");
        assert!(!policy.contains("allow-same-origin"), "{policy}");
        // The document depends on who asks, so a browser does not keep it.
        assert!(header(&shown, "cache-control").is_some_and(|value| value.contains("no-store")));
        assert_eq!(header(&shown, "x-content-type-options"), Some("nosniff"));

        // A style sheet of another site is not linked.
        let other = "/homepage/html/01?style=https://evil.example.net/x.css";
        let shown = call(&app, As::Admin, "GET", other, None).await;
        assert_eq!(shown.status, StatusCode::OK);
        assert!(!shown.body.contains("stylesheet"), "{}", shown.body);
    })
    .await;
}

#[tokio::test]
async fn custom_html_is_refused_as_the_php_application_refuses_an_item() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("homepageCustomHTML01Enabled", json!(true)),
                ("customHTML01", json!(HTML)),
                ("homepageCustomHTML02Enabled", json!(true)),
                ("customHTML03", json!("<p>Three</p>")),
            ],
        );
        let user = seed::logged_in(database, &app, "zed", 4).await;

        // The level of the item is 1: Admins and Co-Admins.
        for who in [As::Guest, As::User(&user)] {
            let refused = call(&app, who, "GET", "/homepage/html/01", None).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
            assert_eq!(
                refused.message(),
                json!("User not approved to view this homepage item")
            );
            assert!(!refused.body.contains("Hello"));
        }

        let empty = call(&app, As::Admin, "GET", "/homepage/html/02", None).await;
        assert_eq!(empty.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(empty.message(), json!("customHTML02 was not supplied"));

        let off = call(&app, As::Admin, "GET", "/homepage/html/03", None).await;
        assert_eq!(off.status, StatusCode::CONFLICT);
        assert_eq!(
            off.message(),
            json!("homepageCustomHTML03Enabled module is not enabled")
        );
        // The switch is looked at before the level.
        let off = call(&app, As::Guest, "GET", "/homepage/html/03", None).await;
        assert_eq!(off.status, StatusCode::CONFLICT);

        for number in ["09", "1", "one"] {
            let missing = call(
                &app,
                As::Admin,
                "GET",
                &format!("/homepage/html/{number}"),
                None,
            )
            .await;
            assert_eq!(missing.status, StatusCode::NOT_FOUND, "{number}");
        }
    })
    .await;
}
