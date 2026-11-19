//! Sending an e-mail to an SMTP server.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use lettre::message::header::ContentType;
use lettre::message::{Mailbox, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{Address, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use thiserror::Error;

/// How long the server has for each step of a delivery.
const TIMEOUT: Duration = Duration::from_secs(20);

/// How the connection to the server is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Security {
    /// The whole connection is encrypted: the kind `ssl` of the PHP application.
    Tls,
    /// The connection starts plain and must change to an encrypted one: the kind `tls`.
    StartTls,
    /// The connection changes to an encrypted one when the server offers that: the
    /// kind `n/a`.
    Opportunistic,
}

/// The SMTP server that sends the e-mails of the install, and who they come from.
#[derive(Clone, PartialEq, Eq)]
pub struct SmtpServer {
    /// The name or the address of the server.
    pub host: String,
    /// The port.
    pub port: u16,
    /// How the connection is protected.
    pub security: Security,
    /// The name and the password of the account, when the server asks for a login.
    pub credentials: Option<(String, String)>,
    /// Whether the certificate of the server must be valid.
    pub verify_certificate: bool,
    /// The name that a mail program shows as the sender.
    pub sender_name: String,
    /// The address of the sender.
    pub sender_address: String,
}

impl std::fmt::Debug for SmtpServer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The password of the account never goes into a log line.
        formatter
            .debug_struct("SmtpServer")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("security", &self.security)
            .field("has_credentials", &self.credentials.is_some())
            .field("verify_certificate", &self.verify_certificate)
            .field("sender_address", &self.sender_address)
            .finish_non_exhaustive()
    }
}

/// Somebody who gets an e-mail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipient {
    /// The address.
    pub address: String,
    /// The name that a mail program shows.
    pub name: Option<String>,
}

/// One e-mail.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Message {
    /// Who the e-mail is for.
    pub to: Option<Recipient>,
    /// Addresses that get the e-mail too, and that no recipient sees.
    pub bcc: Vec<String>,
    /// The subject.
    pub subject: String,
    /// The whole e-mail as HTML.
    pub html: String,
}

/// Why an e-mail was not sent.
#[derive(Debug, Error)]
pub enum MailError {
    /// The settings name no SMTP server.
    #[error("no SMTP server is set")]
    NoServer,
    /// An address is not an e-mail address.
    #[error("`{0}` is not an e-mail address")]
    Address(String),
    /// The e-mail has nobody to go to.
    #[error("the e-mail has no recipient")]
    NoRecipient,
    /// The message cannot be put together.
    #[error("the e-mail cannot be put together: {0}")]
    Message(#[from] lettre::error::Error),
    /// The server refused the e-mail, or it cannot be reached.
    #[error("the SMTP server did not take the e-mail: {0}")]
    Smtp(#[from] lettre::transport::smtp::Error),
}

type Sending = Pin<Box<dyn Future<Output = Result<(), MailError>> + Send>>;

/// Sends e-mails. The application has one that talks to the SMTP server. A test puts
/// one in its place that keeps the messages.
#[derive(Clone)]
pub struct Mailer(Arc<dyn Fn(SmtpServer, Message) -> Sending + Send + Sync>);

impl Mailer {
    /// A mailer that runs the given function for each message.
    pub fn new<F, Fut>(send: F) -> Self
    where
        F: Fn(SmtpServer, Message) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), MailError>> + Send + 'static,
    {
        Self(Arc::new(move |server, message| {
            Box::pin(send(server, message))
        }))
    }

    /// The mailer of the application: it hands each message to the SMTP server.
    pub fn smtp() -> Self {
        Self::new(deliver)
    }

    /// Sends one message through a server.
    pub async fn send(&self, server: SmtpServer, message: Message) -> Result<(), MailError> {
        (self.0)(server, message).await
    }
}

impl std::fmt::Debug for Mailer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Mailer")
    }
}

fn address(text: &str) -> Result<Address, MailError> {
    text.trim()
        .parse()
        .map_err(|_| MailError::Address(text.to_owned()))
}

/// The message in the form that goes over the wire.
fn wire_message(server: &SmtpServer, message: Message) -> Result<lettre::Message, MailError> {
    let sender = Mailbox::new(
        Some(server.sender_name.clone()).filter(|name| !name.is_empty()),
        address(&server.sender_address)?,
    );
    let mut builder = lettre::Message::builder()
        .from(sender.clone())
        .reply_to(sender)
        .subject(message.subject);
    let mut has_recipient = false;
    if let Some(to) = message.to {
        builder = builder.to(Mailbox::new(
            to.name.filter(|name| !name.is_empty()),
            address(&to.address)?,
        ));
        has_recipient = true;
    }
    for hidden in message.bcc.iter().filter(|text| !text.trim().is_empty()) {
        builder = builder.bcc(Mailbox::new(None, address(hidden)?));
        has_recipient = true;
    }
    if !has_recipient {
        return Err(MailError::NoRecipient);
    }
    Ok(builder.singlepart(
        SinglePart::builder()
            .header(ContentType::TEXT_HTML)
            .body(message.html),
    )?)
}

/// Hands one message to the SMTP server.
async fn deliver(server: SmtpServer, message: Message) -> Result<(), MailError> {
    let wire = wire_message(&server, message)?;

    let parameters = || {
        TlsParameters::builder(server.host.clone())
            .dangerous_accept_invalid_certs(!server.verify_certificate)
            .build()
    };
    let tls = match server.security {
        Security::Tls => Tls::Wrapper(parameters()?),
        Security::StartTls => Tls::Required(parameters()?),
        Security::Opportunistic => Tls::Opportunistic(parameters()?),
    };
    let mut transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&server.host)
        .port(server.port)
        .tls(tls)
        .timeout(Some(TIMEOUT));
    if let Some((name, password)) = server.credentials {
        transport = transport.credentials(Credentials::new(name, password));
    }
    transport.build().send(wire).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MailError, Message, Recipient, Security, SmtpServer, wire_message};

    fn server() -> SmtpServer {
        SmtpServer {
            host: "smtp.example.com".to_owned(),
            port: 587,
            security: Security::StartTls,
            credentials: Some(("mailer".to_owned(), "the password of the mailer".to_owned())),
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
            bcc: vec!["hidden@example.com".to_owned(), "  ".to_owned()],
            subject: "Password Reset".to_owned(),
            html: "<p>Hello</p>".to_owned(),
        }
    }

    #[test]
    fn message_names_the_sender_and_hides_the_hidden_recipients() {
        let wire = wire_message(&server(), message()).expect("the message is valid");

        let text = String::from_utf8(wire.formatted()).expect("the message is text");
        assert!(
            text.contains("From: \"My Organizr\" <organizr@example.com>\r\n"),
            "{text}"
        );
        assert!(
            text.contains("Reply-To: \"My Organizr\" <organizr@example.com>\r\n"),
            "{text}"
        );
        assert!(text.contains("To: robin <robin@example.com>\r\n"), "{text}");
        assert!(text.contains("Subject: Password Reset\r\n"), "{text}");
        assert!(
            text.contains("Content-Type: text/html; charset=utf-8\r\n"),
            "{text}"
        );
        assert!(text.contains("<p>Hello</p>"), "{text}");
        assert!(!text.contains("hidden@example.com"), "{text}");
        // The server is told about every recipient.
        let recipients: Vec<String> = wire
            .envelope()
            .to()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(recipients, ["robin@example.com", "hidden@example.com"]);
    }

    #[test]
    fn message_needs_a_recipient_and_real_addresses() {
        let nobody = Message {
            to: None,
            bcc: vec![String::new()],
            ..message()
        };
        assert!(matches!(
            wire_message(&server(), nobody),
            Err(MailError::NoRecipient)
        ));

        let wrong = Message {
            bcc: vec!["not an address".to_owned()],
            ..message()
        };
        assert!(matches!(
            wire_message(&server(), wrong),
            Err(MailError::Address(text)) if text == "not an address"
        ));

        let wrong_sender = SmtpServer {
            sender_address: "nobody".to_owned(),
            ..server()
        };
        assert!(matches!(
            wire_message(&wrong_sender, message()),
            Err(MailError::Address(_))
        ));
    }

    #[test]
    fn debug_output_shows_no_password() {
        let shown = format!("{:?}", server());

        assert!(shown.contains("smtp.example.com"), "{shown}");
        assert!(!shown.contains("the password of the mailer"), "{shown}");
    }
}
