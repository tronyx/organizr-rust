//! The certificate of an authority that an Admin uploads: which files are taken, and
//! the requests that then trust the authority.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::routing::get;
use organizr_integrations::custom_certificate::{
    CertificateError, CustomCertificate, LARGEST, MOST_CERTIFICATES,
};
use organizr_integrations::http::Outbound;
use organizr_testkit::services::StandIn;
use organizr_testkit::tls::{AUTHORITY, CERTIFICATE_REQUEST, LOCALHOST_CERTIFICATE, LOCALHOST_KEY};

fn parsed(text: &str) -> Result<CustomCertificate, CertificateError> {
    CustomCertificate::parse(text.as_bytes())
}

#[test]
fn certificate_of_an_authority_is_taken_and_written_again() {
    let certificate = parsed(AUTHORITY).expect("the certificate of an authority is taken");
    assert_eq!(certificate.count(), 1);
    assert!(
        certificate
            .pem()
            .starts_with("-----BEGIN CERTIFICATE-----\n")
    );
    // Text around the parts is not kept.
    let with_notes = format!("A note\n{AUTHORITY}\nanother note\n{AUTHORITY}");
    let two = parsed(&with_notes).expect("two certificates are taken");
    assert_eq!(two.count(), 2);
    assert!(!two.pem().contains("note"));
    assert_eq!(
        CustomCertificate::parse(AUTHORITY.replace('\n', "\r\n").as_bytes())
            .expect("lines may end with CR LF")
            .pem(),
        certificate.pem()
    );
}

#[test]
fn files_that_are_no_certificate_of_an_authority_are_refused() {
    assert_eq!(
        parsed(LOCALHOST_CERTIFICATE),
        Err(CertificateError::NotAnAuthority)
    );
    // One certificate that is not that of an authority is enough.
    assert_eq!(
        parsed(&format!("{AUTHORITY}{LOCALHOST_CERTIFICATE}")),
        Err(CertificateError::NotAnAuthority)
    );
    assert_eq!(parsed(LOCALHOST_KEY), Err(CertificateError::PrivateKey));
    assert_eq!(
        parsed(&format!("{AUTHORITY}{LOCALHOST_KEY}")),
        Err(CertificateError::PrivateKey)
    );
    let encrypted =
        "-----BEGIN ENCRYPTED PRIVATE KEY-----\nAAAA\n-----END ENCRYPTED PRIVATE KEY-----\n";
    assert_eq!(
        parsed(&format!("{AUTHORITY}{encrypted}")),
        Err(CertificateError::PrivateKey)
    );
    assert_eq!(
        parsed(CERTIFICATE_REQUEST),
        Err(CertificateError::NotACertificate)
    );
    for garbage in ["", "not a certificate", "\u{0}\u{1}\u{2}"] {
        assert_eq!(
            parsed(garbage),
            Err(CertificateError::NotPem),
            "{garbage:?}"
        );
    }
    let broken =
        "-----BEGIN CERTIFICATE-----\nbm90IGEgY2VydGlmaWNhdGU=\n-----END CERTIFICATE-----\n";
    assert_eq!(parsed(broken), Err(CertificateError::Unreadable));
    let unfinished = "-----BEGIN CERTIFICATE-----\nbm90\n";
    assert_eq!(parsed(unfinished), Err(CertificateError::NotPem));
    assert_eq!(
        parsed(&AUTHORITY.repeat(MOST_CERTIFICATES + 1)),
        Err(CertificateError::TooMany)
    );
    assert_eq!(
        CustomCertificate::parse(&vec![b'a'; LARGEST + 1]),
        Err(CertificateError::TooLarge)
    );
}

#[tokio::test]
async fn requests_trust_the_authority_only_when_the_service_asks_for_it() {
    let stand_in = StandIn::start_tls(Router::new().route("/", get(|| async { "hello" }))).await;
    let url = format!("{}/", stand_in.https_url());
    let outbound = Outbound::new().expect("the client can be built");
    let fetch = async |http: &Outbound| match http.client_for(&url).get(&url).send().await {
        Ok(response) => response.text().await.ok(),
        Err(_) => None,
    };

    // No system trusts the authority of the tests.
    assert_eq!(fetch(&outbound).await, None);
    assert_eq!(fetch(&outbound.with_custom_certificate(true)).await, None);
    assert!(!outbound.has_custom_certificate());

    let certificate = parsed(AUTHORITY).expect("the certificate is taken");
    outbound
        .set_custom_certificate(Some(&certificate))
        .expect("the authority can be trusted");
    assert!(outbound.has_custom_certificate());
    assert_eq!(
        fetch(&outbound.with_custom_certificate(true))
            .await
            .as_deref(),
        Some("hello")
    );
    // A copy made before sees the change too, and a service that does not ask for the
    // authority does not trust it.
    assert_eq!(fetch(&outbound.with_custom_certificate(false)).await, None);
    assert_eq!(fetch(&outbound).await, None);

    outbound
        .set_custom_certificate(None)
        .expect("the authority can be dropped");
    assert_eq!(fetch(&outbound.with_custom_certificate(true)).await, None);
}
