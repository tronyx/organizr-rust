//! The client of Komga for the invites plugin: the libraries, and a new account.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test reads the answers it knows, and its helpers may panic"
)]

use std::time::Duration;

use axum::Router;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use organizr_integrations::http::Outbound;
use organizr_integrations::komga::{Library, NewAccount, create_account, libraries};
use organizr_testkit::services::StandIn;
use serde_json::json;

const KEY: &str = "the-api-key-of-komga";
const TIMEOUT: Duration = Duration::from_secs(10);

fn keyed(headers: &HeaderMap) -> bool {
    headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        == Some(KEY)
}

async fn stand_in() -> StandIn {
    StandIn::start(
        Router::new()
            .route(
                "/api/v1/libraries",
                get(|headers: HeaderMap| async move {
                    if !keyed(&headers) {
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                    Ok(axum::Json(json!([
                        {"id": "0A1B", "name": "Comics", "root": "/comics"},
                        {"id": "2C3D", "name": "Manga", "root": "/manga"},
                    ])))
                }),
            )
            .route(
                "/api/v2/users",
                post(|headers: HeaderMap| async move {
                    if keyed(&headers) {
                        StatusCode::CREATED
                    } else {
                        StatusCode::UNAUTHORIZED
                    }
                }),
            ),
    )
    .await
}

#[tokio::test]
async fn libraries_are_read_with_the_api_key() {
    let stand_in = stand_in().await;
    let http = Outbound::new().expect("a client");

    let found = libraries(&http, &stand_in.url(), KEY, TIMEOUT)
        .await
        .expect("Komga answers");

    assert_eq!(
        found,
        [
            Library {
                id: "0A1B".to_owned(),
                name: "Comics".to_owned()
            },
            Library {
                id: "2C3D".to_owned(),
                name: "Manga".to_owned()
            },
        ]
    );
    assert!(
        libraries(&http, &stand_in.url(), "a wrong key", TIMEOUT)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn an_account_is_made_with_its_roles_and_its_libraries() {
    let stand_in = stand_in().await;
    let http = Outbound::new().expect("a client");
    let account = NewAccount {
        email: "robin@example.com",
        password: "the first password",
        roles: &["FILE_DOWNLOAD".to_owned(), "PAGE_STREAMING".to_owned()],
        library_ids: &["0A1B".to_owned()],
    };

    create_account(&http, &stand_in.url(), KEY, account, TIMEOUT)
        .await
        .expect("the account is made");

    let sent = stand_in.seen().pop().expect("a request").json();
    assert_eq!(
        sent,
        json!({
            "email": "robin@example.com",
            "password": "the first password",
            "roles": ["FILE_DOWNLOAD", "PAGE_STREAMING"],
            "sharedLibraries": {"all": false, "libraryIds": ["0A1B"]},
        })
    );
    assert!(
        create_account(&http, &stand_in.url(), "a wrong key", account, TIMEOUT)
            .await
            .is_err()
    );
}
