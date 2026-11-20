//! Stand-ins for an LDAP server and an FTP server, for the login back ends.
//!
//! Each one listens on a port of this machine and speaks just enough of its protocol
//! for a login: LDAP binds, a search of one entry and the unbind, in the BER encoding
//! of RFC 4511; and FTP `USER`, `PASS` and `QUIT`. Neither offers TLS. All names and
//! passwords are made up.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    reason = "a stand-in for a test reads its own small messages"
)]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

/// The account with which Organizr binds first.
pub const LDAP_SERVICE: (&str, &str) = ("cn=organizr,dc=example,dc=com", "the-bind-password");

/// The prefix of the names of people.
pub const LDAP_PREFIX: &str = "uid=";

/// The suffix of the names of people.
pub const LDAP_SUFFIX: &str = ",ou=people,dc=example,dc=com";

/// The base DN of the stand-in.
pub const LDAP_BASE: &str = "dc=example,dc=com";

/// An entry of the stand-in for an LDAP server.
#[derive(Debug, Clone, Copy)]
pub struct LdapEntry {
    /// The distinguished name.
    pub dn: &'static str,
    /// The password.
    pub password: &'static str,
    /// The value of `mail`, when the entry has one.
    pub mail: Option<&'static str>,
}

/// The people of the stand-in: Robin with an address, and Kim without one.
pub const LDAP_PEOPLE: [LdapEntry; 2] = [
    LdapEntry {
        dn: "uid=robin,ou=people,dc=example,dc=com",
        password: "robin-ldap-password",
        mail: Some("robin@ldap.example.com"),
    },
    LdapEntry {
        dn: "uid=kim,ou=people,dc=example,dc=com",
        password: "kim-ldap-password",
        mail: None,
    },
];

type Log = Arc<Mutex<Vec<String>>>;

/// A running stand-in. It stops when the value is dropped.
#[derive(Debug)]
pub struct Listening {
    address: SocketAddr,
    seen: Log,
    task: JoinHandle<()>,
}

impl Listening {
    /// The port of the stand-in, on 127.0.0.1.
    pub fn port(&self) -> u16 {
        self.address.port()
    }

    /// What the stand-in got, oldest first. LDAP: `bind <name>`, `search <base>` and
    /// `unbind`. FTP: each command line, with the password of `PASS` as it came.
    pub fn seen(&self) -> Vec<String> {
        self.seen.lock().expect("the log can be locked").clone()
    }
}

impl Drop for Listening {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn listen<F, Fut>(serve: F) -> Listening
where
    F: Fn(TcpStream, Log) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port is free");
    let address = listener.local_addr().expect("the port is known");
    let seen = Log::default();
    let log = Arc::clone(&seen);
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve(stream, Arc::clone(&log)));
        }
    });
    Listening {
        address,
        seen,
        task,
    }
}

/// A port of this machine on which nobody listens.
pub async fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port is free");
    listener.local_addr().expect("the port is known").port()
}

// --- LDAP ---------------------------------------------------------------------------

/// Reads the length of a BER element at `at`. Returns the length and the place of the
/// content.
fn ber_length(bytes: &[u8], at: usize) -> Option<(usize, usize)> {
    let first = *bytes.get(at)?;
    if first < 0x80 {
        return Some((usize::from(first), at + 1));
    }
    let count = usize::from(first & 0x7f);
    let mut length = 0usize;
    for index in 0..count {
        length = (length << 8) | usize::from(*bytes.get(at + 1 + index)?);
    }
    Some((length, at + 1 + count))
}

/// The elements in a run of BER: their tags and their contents.
fn ber_elements(bytes: &[u8]) -> Vec<(u8, &[u8])> {
    let mut elements = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let tag = bytes[at];
        let Some((length, start)) = ber_length(bytes, at + 1) else {
            break;
        };
        let Some(content) = bytes.get(start..start + length) else {
            break;
        };
        elements.push((tag, content));
        at = start + length;
    }
    elements
}

/// One BER element.
fn ber(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let length = content.len();
    if length < 0x80 {
        out.push(u8::try_from(length).expect("it is small"));
    } else {
        let bytes = u32::try_from(length).expect("it fits").to_be_bytes();
        out.push(0x84);
        out.extend_from_slice(&bytes);
    }
    out.extend_from_slice(content);
    out
}

/// An LDAP result: the code, an empty matched DN and an empty text.
fn ldap_result(tag: u8, code: u8) -> Vec<u8> {
    let mut content = ber(0x0a, &[code]);
    content.extend(ber(0x04, b""));
    content.extend(ber(0x04, b""));
    ber(tag, &content)
}

fn ldap_message(id: &[u8], operation: &[u8]) -> Vec<u8> {
    let mut content = ber(0x02, id);
    content.extend_from_slice(operation);
    ber(0x30, &content)
}

/// Reads one LDAP message from the connection.
async fn read_message(stream: &mut TcpStream) -> Option<Vec<u8>> {
    let mut head = [0u8; 2];
    stream.read_exact(&mut head).await.ok()?;
    let mut whole = head.to_vec();
    let length = if head[1] < 0x80 {
        usize::from(head[1])
    } else {
        let mut bytes = vec![0u8; usize::from(head[1] & 0x7f)];
        stream.read_exact(&mut bytes).await.ok()?;
        whole.extend_from_slice(&bytes);
        bytes
            .iter()
            .fold(0usize, |length, byte| (length << 8) | usize::from(*byte))
    };
    let mut content = vec![0u8; length];
    stream.read_exact(&mut content).await.ok()?;
    whole.extend_from_slice(&content);
    Some(whole)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The answer of the stand-in to a bind.
fn bind_code(entries: &[LdapEntry], anonymous: bool, name: &str, password: &str) -> u8 {
    if name.is_empty() && password.is_empty() {
        return if anonymous { 0 } else { 48 };
    }
    if name == LDAP_SERVICE.0 && password == LDAP_SERVICE.1 {
        return 0;
    }
    let fits = entries.iter().any(|entry| {
        entry.dn.eq_ignore_ascii_case(name) && !password.is_empty() && entry.password == password
    });
    if fits { 0 } else { 49 }
}

async fn serve_ldap(mut stream: TcpStream, log: Log, entries: Vec<LdapEntry>, anonymous: bool) {
    while let Some(message) = read_message(&mut stream).await {
        let Some((0x30, content)) = ber_elements(&message).first().copied() else {
            return;
        };
        let parts = ber_elements(content);
        let (Some((0x02, id)), Some((operation, body))) =
            (parts.first().copied(), parts.get(1).copied())
        else {
            return;
        };
        let answer = match operation {
            // A bind: the version, the name, and the simple password.
            0x60 => {
                let fields = ber_elements(body);
                let name = fields.get(1).map_or_default(|field| text(field.1));
                let password = fields.get(2).map_or_default(|field| text(field.1));
                log.lock().expect("it locks").push(format!("bind {name}"));
                let code = bind_code(&entries, anonymous, &name, &password);
                ldap_message(id, &ldap_result(0x61, code))
            }
            // A search: only the base is read; the scope is "base".
            0x63 => {
                let base = ber_elements(body)
                    .first()
                    .map_or_default(|field| text(field.1));
                log.lock().expect("it locks").push(format!("search {base}"));
                let mut out = Vec::new();
                match entries
                    .iter()
                    .find(|entry| entry.dn.eq_ignore_ascii_case(&base))
                {
                    Some(entry) => {
                        let mut attributes = Vec::new();
                        if let Some(mail) = entry.mail {
                            let mut attribute = ber(0x04, b"mail");
                            attribute.extend(ber(0x31, &ber(0x04, mail.as_bytes())));
                            attributes.extend(ber(0x30, &attribute));
                        }
                        let mut found = ber(0x04, entry.dn.as_bytes());
                        found.extend(ber(0x30, &attributes));
                        out.extend(ldap_message(id, &ber(0x64, &found)));
                        out.extend(ldap_message(id, &ldap_result(0x65, 0)));
                    }
                    None => out.extend(ldap_message(id, &ldap_result(0x65, 32))),
                }
                out
            }
            // An unbind ends the connection.
            0x42 => {
                log.lock().expect("it locks").push("unbind".to_owned());
                return;
            }
            // Anything else, for example StartTLS, is refused with "protocol error".
            other => {
                log.lock()
                    .expect("it locks")
                    .push(format!("operation {other:#x}"));
                ldap_message(id, &ldap_result(0x78, 2))
            }
        };
        if stream.write_all(&answer).await.is_err() {
            return;
        }
    }
}

/// A stand-in for an LDAP server with the account of [`LDAP_SERVICE`] and the
/// [`LDAP_PEOPLE`]. With `anonymous`, a bind without a name succeeds too.
pub async fn ldap_server(anonymous: bool) -> Listening {
    listen(move |stream, log| serve_ldap(stream, log, LDAP_PEOPLE.to_vec(), anonymous)).await
}

// --- FTP ----------------------------------------------------------------------------

/// The accounts of the stand-in for an FTP server: name and password.
pub const FTP_USERS: [(&str, &str); 1] = [("robin", "robin-ftp-password")];

async fn serve_ftp(stream: TcpStream, log: Log) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    if writer
        .write_all(b"220 Stand-in FTP ready\r\n")
        .await
        .is_err()
    {
        return;
    }
    let mut user = String::new();
    while let Ok(Some(line)) = lines.next_line().await {
        log.lock().expect("it locks").push(line.clone());
        let (command, argument) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        let answer = match command.to_uppercase().as_str() {
            "USER" => {
                argument.clone_into(&mut user);
                "331 Password required\r\n"
            }
            "PASS" => {
                if FTP_USERS.contains(&(user.as_str(), argument)) {
                    "230 Logged in\r\n"
                } else {
                    "530 Login incorrect\r\n"
                }
            }
            "QUIT" => {
                let _ = writer.write_all(b"221 Goodbye\r\n").await;
                return;
            }
            "AUTH" => "504 AUTH TLS is not offered\r\n",
            _ => "502 Command not implemented\r\n",
        };
        if writer.write_all(answer.as_bytes()).await.is_err() {
            return;
        }
    }
}

/// A stand-in for an FTP server with the accounts of [`FTP_USERS`], without TLS.
pub async fn ftp_server() -> Listening {
    listen(serve_ftp).await
}
