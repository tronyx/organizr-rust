//! The request id of D-13: the header `X-Request-Id` of each answer.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call, send};
use organizr_testkit::{installed_app, seed};

fn id_of(answer: &Answer) -> String {
    answer
        .headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .expect("the answer has an X-Request-Id")
        .to_owned()
}

fn is_uuid(id: &str) -> bool {
    id.len() == 36
        && id.char_indices().all(|(place, letter)| {
            if [8, 13, 18, 23].contains(&place) {
                letter == '-'
            } else {
                letter.is_ascii_hexdigit()
            }
        })
}

#[tokio::test]
async fn every_answer_has_a_request_id_of_its_own() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let answers = [
            call(&app, As::Admin, "GET", "/tabs", None).await,
            call(&app, As::Guest, "GET", "/tabs", None).await,
            call(&app, As::Guest, "GET", "/no-such-route", None).await,
            call(&app, As::Guest, "DELETE", "/status", None).await,
        ];
        let statuses: Vec<StatusCode> = answers.iter().map(|answer| answer.status).collect();
        assert_eq!(
            statuses,
            [
                StatusCode::OK,
                StatusCode::UNAUTHORIZED,
                StatusCode::NOT_FOUND,
                StatusCode::NOT_FOUND
            ]
        );
        let ids: Vec<String> = answers.iter().map(id_of).collect();
        for id in &ids {
            assert!(is_uuid(id), "{id}");
        }
        let mut distinct = ids.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), ids.len(), "{ids:?}");
    })
    .await;
}

#[tokio::test]
async fn the_request_id_of_a_client_is_not_used() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let request = Request::builder()
            .uri("/api/v2/status")
            .header("x-request-id", "the client chose this")
            .body(Body::empty())
            .expect("the request is valid");
        let answer = send(&app, request).await;
        assert_eq!(answer.status, StatusCode::OK);
        let id = id_of(&answer);
        assert!(is_uuid(&id), "{id}");
        assert_eq!(answer.headers.get_all("x-request-id").iter().count(), 1);
    })
    .await;
}
