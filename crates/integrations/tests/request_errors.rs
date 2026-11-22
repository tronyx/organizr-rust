//! The text of a failed request names the cause in plain words, with the technical detail
//! after it, and never the address, which can hold a key.

#![allow(
    clippy::expect_used,
    reason = "a test reads the errors it causes, and its helpers may panic"
)]

use std::time::Duration;

use axum::Router;
use axum::routing::get;
use organizr_integrations::http::{Outbound, without_address};
use organizr_testkit::services::StandIn;

/// The text of the error of a GET of `url` through the client that checks certificates.
async fn failure_of(url: &str, timeout: Duration) -> String {
    let http = Outbound::new().expect("the client can be built");
    let error = http
        .client(url, false)
        .get(url)
        .timeout(timeout)
        .send()
        .await
        .expect_err("the request fails");
    without_address(error)
}

#[tokio::test]
async fn a_name_that_does_not_resolve_says_so() {
    // `.invalid` never resolves.
    let text = failure_of(
        "http://prowlarr.invalid:9696/api?apikey=secret-key",
        Duration::from_secs(20),
    )
    .await;

    assert!(
        text.starts_with("the name of the server cannot be found"),
        "{text}"
    );
    assert!(
        !text.contains("secret-key") && !text.contains("prowlarr.invalid"),
        "{text}"
    );
}

#[tokio::test]
async fn a_closed_port_says_that_nothing_answers_there() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("an address").port();
    drop(listener);

    let text = failure_of(
        &format!("http://127.0.0.1:{port}/"),
        Duration::from_secs(10),
    )
    .await;

    assert!(
        text.starts_with("the server refused the connection"),
        "{text}"
    );
}

#[tokio::test]
async fn a_server_that_does_not_answer_in_time_says_so() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port");
    let port = listener.local_addr().expect("an address").port();
    // The connection is accepted and never answered.
    let _quiet = tokio::spawn(async move {
        let _held = listener.accept().await;
        tokio::time::sleep(Duration::from_secs(30)).await;
    });

    let text = failure_of(
        &format!("http://127.0.0.1:{port}/"),
        Duration::from_millis(300),
    )
    .await;

    assert!(
        text.starts_with("the server did not answer in time"),
        "{text}"
    );
}

#[tokio::test]
async fn a_certificate_that_is_not_trusted_says_so() {
    // The stand-in has a certificate of an authority that the client does not know.
    let stand_in = StandIn::start_tls(Router::new().route("/", get(|| async { "ok" }))).await;

    let text = failure_of(
        &format!("{}/", stand_in.https_url()),
        Duration::from_secs(10),
    )
    .await;

    assert!(
        text.starts_with("the certificate of the server is not trusted"),
        "{text}"
    );
}
