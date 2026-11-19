//! A stand-in for the mailer of the application: it keeps every e-mail for the test.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use organizr_plugins::mail::{MailError, Mailer, Message, SmtpServer};

/// The e-mails that the application sent through the stand-in.
#[derive(Debug, Clone, Default)]
pub struct Outbox {
    sent: Arc<Mutex<Vec<(SmtpServer, Message)>>>,
}

impl Outbox {
    /// A mailer for the application, and the outbox that gets its e-mails.
    pub fn new() -> (Mailer, Self) {
        let outbox = Self::default();
        let sent = Arc::clone(&outbox.sent);
        let mailer = Mailer::new(move |server: SmtpServer, message: Message| {
            sent.lock()
                .expect("the list can be locked")
                .push((server, message));
            async { Ok(()) }
        });
        (mailer, outbox)
    }

    /// A mailer whose server takes no e-mail.
    pub fn failing() -> Mailer {
        Mailer::new(|_, _| async { Err(MailError::NoRecipient) })
    }

    /// The e-mails so far, oldest first.
    pub fn mails(&self) -> Vec<Message> {
        self.sent
            .lock()
            .expect("the list can be locked")
            .iter()
            .map(|(_, message)| message.clone())
            .collect()
    }

    /// The server that each e-mail went to.
    pub fn servers(&self) -> Vec<SmtpServer> {
        self.sent
            .lock()
            .expect("the list can be locked")
            .iter()
            .map(|(server, _)| server.clone())
            .collect()
    }

    /// Waits until the outbox holds `count` e-mails, and returns them. The application
    /// sends some e-mails after it has answered. Panics when they do not come.
    pub async fn wait_for(&self, count: usize) -> Vec<Message> {
        for _ in 0..500 {
            let mails = self.mails();
            if mails.len() >= count {
                return mails;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!(
            "the application sent {} e-mails, and the test waits for {count}",
            self.mails().len()
        );
    }

    /// Gives the application a moment for e-mails that it must not send, and returns
    /// what the outbox holds then.
    pub async fn after_a_moment(&self) -> Vec<Message> {
        tokio::time::sleep(Duration::from_millis(150)).await;
        self.mails()
    }
}
