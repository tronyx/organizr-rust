//! A stand-in for an SMTP server: it takes every e-mail and keeps it for the test.
//!
//! It speaks plain SMTP on a port of this machine, without encryption. It offers a
//! login and takes every name and password.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// One e-mail as the stand-in got it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Received {
    /// The address after `MAIL FROM`.
    pub from: String,
    /// The addresses after `RCPT TO`: everybody who gets the e-mail.
    pub to: Vec<String>,
    /// The message: its headers and its body.
    pub data: String,
    /// Whether the client logged in.
    pub logged_in: bool,
}

/// A running stand-in. It stops when the value is dropped.
#[derive(Debug)]
pub struct SmtpSink {
    address: SocketAddr,
    received: Arc<Mutex<Vec<Received>>>,
    task: tokio::task::JoinHandle<()>,
}

impl SmtpSink {
    /// Starts a stand-in that takes every e-mail.
    pub async fn start() -> Self {
        Self::start_with(true).await
    }

    /// Starts a stand-in that refuses every recipient.
    pub async fn refusing() -> Self {
        Self::start_with(false).await
    }

    async fn start_with(accepts: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a local port can be opened");
        let address = listener.local_addr().expect("the port has an address");
        let received = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&received);
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(serve(stream, Arc::clone(&kept), accepts));
            }
        });
        Self {
            address,
            received,
            task,
        }
    }

    /// The port that the stand-in listens on, at `127.0.0.1`.
    pub const fn port(&self) -> u16 {
        self.address.port()
    }

    /// The e-mails so far, oldest first.
    pub fn received(&self) -> Vec<Received> {
        self.received
            .lock()
            .expect("the list can be locked")
            .clone()
    }
}

impl Drop for SmtpSink {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// The address between `<` and `>` of a command.
fn address_in(line: &str) -> String {
    line.split_once('<')
        .and_then(|(_, rest)| rest.split_once('>'))
        .map_or_else(String::new, |(address, _)| address.to_owned())
}

/// Talks to one client until it leaves.
async fn serve(stream: TcpStream, received: Arc<Mutex<Vec<Received>>>, accepts: bool) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let mut current = Received::default();
    let mut logged_in = false;
    // How many answers of the client a login still needs.
    let mut login_steps = 0_u8;
    if writer.write_all(b"220 stand-in ESMTP\r\n").await.is_err() {
        return;
    }

    while let Ok(Some(line)) = lines.next_line().await {
        let command = line.to_ascii_uppercase();
        let answer: &[u8] = if login_steps > 0 {
            login_steps -= 1;
            if login_steps == 0 {
                logged_in = true;
                b"235 2.7.0 logged in\r\n"
            } else {
                b"334 UGFzc3dvcmQ6\r\n"
            }
        } else if command.starts_with("EHLO") {
            b"250-stand-in\r\n250-AUTH PLAIN LOGIN\r\n250 8BITMIME\r\n"
        } else if command.starts_with("AUTH PLAIN") {
            logged_in = true;
            b"235 2.7.0 logged in\r\n"
        } else if command.starts_with("AUTH LOGIN") {
            login_steps = 2;
            b"334 VXNlcm5hbWU6\r\n"
        } else if command.starts_with("MAIL FROM") {
            current = Received {
                from: address_in(&line),
                logged_in,
                ..Received::default()
            };
            b"250 2.1.0 ok\r\n"
        } else if command.starts_with("RCPT TO") {
            if accepts {
                current.to.push(address_in(&line));
                b"250 2.1.5 ok\r\n"
            } else {
                b"550 5.1.1 no such mailbox\r\n"
            }
        } else if command == "DATA" {
            if writer.write_all(b"354 go on\r\n").await.is_err() {
                return;
            }
            let mut data = String::new();
            while let Ok(Some(text)) = lines.next_line().await {
                if text == "." {
                    break;
                }
                // A line of the message that starts with a dot has one more dot.
                data.push_str(text.strip_prefix('.').unwrap_or(&text));
                data.push_str("\r\n");
            }
            current.data = data;
            received
                .lock()
                .expect("the list can be locked")
                .push(std::mem::take(&mut current));
            b"250 2.0.0 taken\r\n"
        } else if command == "QUIT" {
            let _ = writer.write_all(b"221 2.0.0 bye\r\n").await;
            return;
        } else {
            b"250 2.0.0 ok\r\n"
        };
        if writer.write_all(answer).await.is_err() {
            return;
        }
    }
}
