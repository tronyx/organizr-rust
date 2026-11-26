//! The donate item of the homepage, against a stand-in for Stripe.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_api::router;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, STRIPE_PAGE, STRIPE_SECRET};
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_settings, seed, state_with};
use serde_json::{Value, json};

const DONATE: &str = "/homepage/donate";

type Changed = Vec<(&'static str, Value)>;

fn settings() -> Changed {
    vec![
        ("homepageDonateEnabled", json!(true)),
        ("homepageDonateAuth", json!("4")),
        (
            "homepageDonatePublicToken",
            json!("pk_test_made-up-public-token"),
        ),
        ("homepageDonateSecretToken", json!(STRIPE_SECRET)),
        ("homepageDonateProductID", json!("prod_made-up")),
        ("homepageDonateMinimum", json!("500")),
        ("homepageDonateShowUserHistory", json!(true)),
    ]
}

/// The settings with another value for one of them.
fn with(mut settings: Changed, key: &'static str, value: Value) -> Changed {
    settings.retain(|(known, _)| *known != key);
    settings.push((key, value));
    settings
}

fn app(database: &Database, stripe: &str, changed: &[(&'static str, Value)]) -> Router {
    let state = state_with(database, installed_settings(changed)).with_stripe(stripe);
    router(state).expect("the route table is valid")
}

/// The session cookie of an account that logs in with its password.
async fn cookie_of(app: &Router, name: &str, password: &str) -> String {
    let body = json!({"username": name, "password": password});
    let answer = call(app, As::Guest, "POST", "/login", Some(body)).await;
    answer
        .cookies
        .iter()
        .find(|cookie| cookie.starts_with("organizr_token_") && !cookie.contains("=;"))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap_or_else(|| panic!("the login sets the session cookie: {}", answer.body))
        .to_owned()
}

fn location(answer: &Answer) -> &str {
    answer.headers["location"].to_str().expect("text")
}

#[tokio::test]
async fn history_has_the_donations_of_the_caller_and_all_of_them_for_an_admin() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        seed::registered_users(database).await;
        let stripe = homepage_services::stripe().await;
        let app = app(database, &stripe.url(), &settings());

        let admin = cookie_of(&app, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        let answer = call(&app, As::User(&admin), "GET", DONATE, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data(),
            json!([
                {"date": "2026-11-20T10:00:00Z", "email": "Admin@Example.com", "amount": 12.5},
                {"date": "2026-11-20T10:00:00Z", "email": "Zoe@Example.com", "amount": 5},
                {"date": "2026-11-20T10:00:00Z", "email": "zoe@example.com", "amount": 7},
            ])
        );
        let seen = stripe.seen();
        assert_eq!(
            seen[0].path,
            "/v1/charges?limit=100&expand%5B%5D=data.payment_intent"
        );
        assert_eq!(
            seen[0].header("authorization"),
            Some(format!("Bearer {STRIPE_SECRET}").as_str())
        );

        // A user gets the donations that carry the mark of the account, not every one
        // with its e-mail address, which the user can change (D-152).
        let zoe = cookie_of(&app, "zoe", seed::ZOE_PASSWORD).await;
        let answer = call(&app, As::User(&zoe), "GET", DONATE, None).await;
        assert_eq!(
            answer.data(),
            json!([{"date": "2026-11-20T10:00:00Z", "email": "Zoe@Example.com", "amount": 5}])
        );
        // A script with the API key counts as the Admin.
        let answer = call(&app, As::Admin, "GET", DONATE, None).await;
        assert_eq!(answer.data().as_array().map(Vec::len), Some(3));

        // An account without a donation gets an empty list.
        let nameless = seed::logged_in(database, &app, "uma", 4).await;
        let answer = call(&app, As::User(&nameless), "GET", DONATE, None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.data(), json!([]));

        // A guest and an item without the list get an empty list, and Stripe is not
        // asked.
        let before = stripe.seen().len();
        assert_eq!(
            call(&app, As::Guest, "GET", DONATE, None).await.data(),
            json!([])
        );
        let no_list = with(settings(), "homepageDonateShowUserHistory", json!(false));
        let no_list = self::app(database, &stripe.url(), &no_list);
        assert_eq!(
            call(&no_list, As::Admin, "GET", DONATE, None).await.data(),
            json!([])
        );
        assert_eq!(stripe.seen().len(), before);
    })
    .await;
}

#[tokio::test]
async fn payment_starts_at_stripe_with_the_amount_and_sends_the_browser_there() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        seed::registered_users(database).await;
        let stripe = homepage_services::stripe().await;
        let app = app(database, &stripe.url(), &settings());
        let zoe = cookie_of(&app, "zoe", seed::ZOE_PASSWORD).await;

        let answer = call(&app, As::User(&zoe), "GET", "/homepage/donate/25", None).await;
        assert_eq!(answer.status, StatusCode::SEE_OTHER, "{}", answer.body);
        assert_eq!(location(&answer), STRIPE_PAGE);
        let sent = stripe.seen().last().expect("a request").clone();
        assert_eq!(sent.method, "POST");
        assert_eq!(sent.path, "/v1/checkout/sessions");
        assert_eq!(
            sent.header("authorization"),
            Some(format!("Bearer {STRIPE_SECRET}").as_str())
        );
        let form = sent.form();
        let field = |name: &str| form.get(name).map(String::as_str);
        assert_eq!(field("payment_method_types[0]"), Some("card"));
        assert_eq!(
            field("line_items[0][price_data][product]"),
            Some("prod_made-up")
        );
        assert_eq!(
            field("line_items[0][price_data][unit_amount]"),
            Some("2500")
        );
        assert_eq!(field("line_items[0][price_data][currency]"), Some("usd"));
        assert_eq!(field("line_items[0][quantity]"), Some("1"));
        assert_eq!(field("mode"), Some("payment"));
        assert_eq!(
            field("success_url"),
            Some("http://organizr.example.com/api/v2/homepage/donate/success")
        );
        assert_eq!(
            field("cancel_url"),
            Some("http://organizr.example.com/api/v2/homepage/donate/cancel")
        );
        assert_eq!(field("customer_email"), Some("zoe@example.com"));
        // The payment carries the mark of the account, for the history (D-152).
        assert_eq!(
            field("payment_intent_data[metadata][organizr_account]"),
            Some(homepage_services::stripe_mark_of_zoe().as_str())
        );

        // The amounts of the other forms of the route, in cents.
        let amount = async |method: &str, path: &str, who: As<'_>| {
            let answer = call(&app, who, method, path, None).await;
            assert_eq!(
                answer.status,
                StatusCode::SEE_OTHER,
                "{path}: {}",
                answer.body
            );
            let sent = stripe.seen().last().expect("a request").form();
            (
                sent["line_items[0][price_data][unit_amount]"].clone(),
                sent.contains_key("customer_email"),
            )
        };
        let user = As::User(&zoe);
        // Without an amount, and with one that is no number: the minimum.
        assert_eq!(amount("GET", "/homepage/donate/", user).await.0, "500");
        assert_eq!(amount("GET", "/homepage/donate/much", user).await.0, "500");
        // An amount below the minimum counts as the minimum.
        assert_eq!(amount("GET", "/homepage/donate/2", user).await.0, "500");
        assert_eq!(
            amount("POST", "/homepage/donate?amount=7.5", user).await.0,
            "750"
        );
        assert_eq!(amount("POST", "/homepage/donate", user).await.0, "500");
        assert_eq!(
            amount("POST", "/homepage/donate?amount=1", user).await.0,
            "500"
        );
        // A caller without an e-mail address pays without one.
        let (cents, has_email) = amount("GET", "/homepage/donate/10", As::Admin).await;
        assert_eq!((cents.as_str(), has_email), ("1000", false));
    })
    .await;
}

#[tokio::test]
async fn payment_is_refused_without_the_item_the_level_or_the_keys() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let stripe = homepage_services::stripe().await;
        let url = stripe.url();
        let start = async |changed: &Changed, who: As<'_>| {
            call(
                &app(database, &url, changed),
                who,
                "GET",
                "/homepage/donate/5",
                None,
            )
            .await
        };

        let off = start(
            &with(settings(), "homepageDonateEnabled", json!(false)),
            As::Admin,
        )
        .await;
        assert_eq!(off.status, StatusCode::CONFLICT);
        assert_eq!(
            off.message(),
            json!("homepageDonateEnabled module is not enabled")
        );
        let guest = start(&settings(), As::Guest).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            guest.message(),
            json!("User not approved to view this homepage item")
        );
        for key in [
            "homepageDonatePublicToken",
            "homepageDonateSecretToken",
            "homepageDonateProductID",
        ] {
            let none = start(&with(settings(), key, json!("")), As::Admin).await;
            assert_eq!(none.status, StatusCode::CONFLICT, "{key}");
            assert_eq!(none.message(), json!("Donation Tokens are not setup"));
        }
        assert_eq!(stripe.seen().len(), 0);

        // Stripe refuses the key, and the answer names the text of Stripe only.
        let wrong_key = "sk_test_a-wrong-secret";
        let wrong = start(
            &with(settings(), "homepageDonateSecretToken", json!(wrong_key)),
            As::Admin,
        )
        .await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            wrong.message(),
            json!("Invalid API Key provided: sk_test_****")
        );
        assert!(!wrong.body.contains(wrong_key), "{}", wrong.body);

        // Nothing listens at this address.
        let away = app(database, "http://127.0.0.1:1", &settings());
        let answer = call(&away, As::Admin, "POST", "/homepage/donate?amount=5", None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!answer.body.contains(STRIPE_SECRET), "{}", answer.body);
        assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);
        let answer = call(&away, As::Admin, "GET", DONATE, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!answer.body.contains(STRIPE_SECRET), "{}", answer.body);
    })
    .await;
}

#[tokio::test]
async fn pages_after_a_payment_and_the_item_name_no_key() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let stripe = homepage_services::stripe().await;
        let app = app(database, &stripe.url(), &settings());

        for (path, status, title, notice) in [
            (
                "success",
                StatusCode::OK,
                "Donation Success",
                "Thank you for donating!",
            ),
            (
                "cancel",
                StatusCode::OK,
                "Donation Canceled",
                "Taking you back...",
            ),
            (
                "error",
                StatusCode::INTERNAL_SERVER_ERROR,
                "Donation Error",
                "An error has occurred!",
            ),
        ] {
            let page = call(
                &app,
                As::Guest,
                "GET",
                &format!("/homepage/donate/{path}"),
                None,
            )
            .await;
            assert_eq!(page.status, status, "{path}");
            assert_eq!(
                page.headers["content-type"].to_str().expect("text"),
                "text/html;charset=UTF-8"
            );
            assert!(
                page.body.contains(&format!("<title>{title}</title>")),
                "{}",
                page.body
            );
            assert!(
                page.body.contains(&format!("<p>{notice}</p>")),
                "{}",
                page.body
            );
        }
        assert_eq!(stripe.seen().len(), 0);

        let items = call(&app, As::Admin, "GET", "/homepage/items", None).await;
        assert_eq!(
            items.data(),
            json!([{
                "key": "homepageOrderDonate",
                "kind": "donate",
                "id": "",
                "options": {
                    "heading": "Donate Please",
                    "description": "Hi!  Help me out with a donation...",
                    "minimum": 5,
                    "history": true,
                },
            }])
        );
        assert!(!items.body.contains(STRIPE_SECRET));
        // A guest does not have the level of the item.
        let items = call(&app, As::Guest, "GET", "/homepage/items", None).await;
        assert_eq!(items.data(), json!([]));
    })
    .await;
}
