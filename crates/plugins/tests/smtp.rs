//! Sending an e-mail over the wire, to a stand-in for an SMTP server.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_plugins::mail::{MailError, Mailer, Message, Recipient, Security, SmtpServer};
use organizr_testkit::smtp::SmtpSink;

fn server(port: u16) -> SmtpServer {
    SmtpServer {
        host: "127.0.0.1".to_owned(),
        port,
        // The stand-in offers no encryption, so the connection stays plain.
        security: Security::Opportunistic,
        credentials: None,
        verify_certificate: true,
        sender_name: "My Organizr".to_owned(),
        sender_address: "organizr@example.com".to_owned(),
    }
}

fn message() -> Message {
    Message {
        to: Some(Recipient {
            address: "robin@example.com".to_owned(),
            name: Some("robin".to_owned()),
        }),
        bcc: vec!["hidden@example.com".to_owned()],
        subject: "Password Reset".to_owned(),
        html: "<h2>Hey there robin!</h2>\r\n.A line that starts with a dot.\r\n<p>Grüße</p>"
            .to_owned(),
    }
}

#[tokio::test]
async fn message_reaches_the_server_with_all_its_recipients() {
    let sink = SmtpSink::start().await;

    Mailer::smtp()
        .send(server(sink.port()), message())
        .await
        .expect("the server takes the e-mail");

    let received = sink.received();
    assert_eq!(received.len(), 1);
    let mail = &received[0];
    assert_eq!(mail.from, "organizr@example.com");
    assert_eq!(mail.to, ["robin@example.com", "hidden@example.com"]);
    assert!(!mail.logged_in);
    assert!(
        mail.data.contains("Subject: Password Reset\r\n"),
        "{}",
        mail.data
    );
    assert!(
        mail.data.contains("To: robin <robin@example.com>\r\n"),
        "{}",
        mail.data
    );
    assert!(!mail.data.contains("hidden@example.com"), "{}", mail.data);
    assert!(
        mail.data.contains("<h2>Hey there robin!</h2>"),
        "{}",
        mail.data
    );
    assert!(
        mail.data.contains(".A line that starts with a dot."),
        "{}",
        mail.data
    );
}

#[tokio::test]
async fn account_of_the_settings_logs_in() {
    let sink = SmtpSink::start().await;
    let with_account = SmtpServer {
        credentials: Some(("mailer".to_owned(), "the password".to_owned())),
        ..server(sink.port())
    };

    Mailer::smtp()
        .send(with_account, message())
        .await
        .expect("the server takes the e-mail");

    assert!(sink.received()[0].logged_in);
}

#[tokio::test]
async fn failure_of_the_server_is_an_error() {
    let sink = SmtpSink::refusing().await;
    let refused = Mailer::smtp().send(server(sink.port()), message()).await;
    assert!(matches!(refused, Err(MailError::Smtp(_))), "{refused:?}");
    assert_eq!(sink.received(), []);

    // A server that needs encryption, which the stand-in does not offer.
    let sink = SmtpSink::start().await;
    for security in [Security::StartTls, Security::Tls] {
        let encrypted = SmtpServer {
            security,
            ..server(sink.port())
        };
        let refused = Mailer::smtp().send(encrypted, message()).await;
        assert!(matches!(refused, Err(MailError::Smtp(_))), "{refused:?}");
    }
    assert_eq!(sink.received(), [], "nothing was sent without encryption");

    // No server listens on the port.
    let port = sink.port();
    drop(sink);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let refused = Mailer::smtp().send(server(port), message()).await;
    assert!(matches!(refused, Err(MailError::Smtp(_))), "{refused:?}");
}
