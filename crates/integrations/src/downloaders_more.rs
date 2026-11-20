//! More download clients of the homepage: Deluge, rTorrent, uTorrent and JDownloader.
//!
//! Each client is asked as the PHP application asks it. The functions that read an
//! answer follow the rules of PHP for "empty", for text and for comparing, because the
//! answers that the homepage sends depend on them.

use std::cmp::Ordering;
use std::collections::hash_map::RandomState;
use std::fmt::Write as _;
use std::hash::{BuildHasher, Hasher};
use std::time::Duration;

use md5::{Digest, Md5};
use reqwest::header::{self, HeaderMap};
use reqwest::{Response, StatusCode};
use roxmltree::Node;
use serde_json::{Map, Value, json};

use crate::downloaders::{Client, Login};
use crate::http::Outbound;

/// The most characters of a text of a download client that an error holds.
const SHOWN: usize = 300;

/// Why a download client gave no usable answer.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The client cannot be reached, or it did not answer in time. The text names no
    /// address, because the address of uTorrent holds its token.
    #[error("the download client cannot be reached: {0}")]
    Unreachable(String),
    /// The client answered, and refused the login or the request. The text is the one
    /// of the PHP application.
    #[error("{0}")]
    Refused(String),
}

fn unreachable(error: reqwest::Error) -> ClientError {
    ClientError::Unreachable(error.without_url().to_string())
}

fn refused(text: &str) -> ClientError {
    ClientError::Refused(text.chars().take(SHOWN).collect())
}

/// Whether PHP takes a value for true.
fn php_true(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(on) => *on,
        Value::Number(number) => number.as_f64() != Some(0.0),
        Value::String(text) => !text.is_empty() && text != "0",
        Value::Array(list) => !list.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

/// A value as PHP writes it into a text.
fn php_text(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(false) => String::new(),
        Value::Bool(true) => "1".to_owned(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The number in a text that PHP takes for a number, for example ` 12`, `-1.5` or
/// `1e3`.
fn numeric(text: &str) -> Option<f64> {
    let trimmed = text.trim_matches([' ', '\t', '\n', '\r', '\x0B', '\x0C']);
    let digits = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
    let starts_as_a_number = digits
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_digit() || byte == b'.');
    let holds_a_number_only = digits
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E' | b'+' | b'-'));
    if starts_as_a_number && holds_a_number_only {
        trimmed.parse().ok()
    } else {
        None
    }
}

/// The number that a value is for PHP when it compares the value with a number.
fn php_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => numeric(text),
        _ => None,
    }
}

/// Whether a value is the given whole number for the `==` of PHP.
fn is_number(value: &Value, wanted: f64) -> bool {
    php_number(value).is_some_and(|number| (number - wanted).abs() < f64::EPSILON)
}

/// The order of two values for the `<=>` of PHP: two numbers by their size, and
/// everything else as text.
fn php_compare(first: Option<&Value>, second: Option<&Value>) -> Ordering {
    let first = first.unwrap_or(&Value::Null);
    let second = second.unwrap_or(&Value::Null);
    match (php_number(first), php_number(second)) {
        (Some(first), Some(second)) => first.partial_cmp(&second).unwrap_or(Ordering::Equal),
        _ => php_text(first).cmp(&php_text(second)),
    }
}

/// The cookies that an answer sets, as the text of a `Cookie` header.
fn cookies_of(headers: &HeaderMap) -> Option<String> {
    let cookies: Vec<&str> = headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|cookie| cookie.split(';').next())
        .map(str::trim)
        .filter(|cookie| cookie.contains('='))
        .collect();
    (!cookies.is_empty()).then(|| cookies.join("; "))
}

/// How long Deluge has to answer. The PHP application gives it ten seconds.
pub const DELUGE_TIME_LIMIT: Duration = Duration::from_secs(10);

/// The fields of a torrent that Deluge is asked for. The PHP application sends them as
/// one text, and the `WebAPI` plugin of Deluge reads them so.
const DELUGE_FIELDS: &str = "comment, download_payload_rate, eta, hash, is_finished, is_seed, \
     message, name, paused, progress, queue, state, total_size, upload_payload_rate, \
     tracker_status";

/// One request to the JSON-RPC API of the web interface of Deluge.
struct DelugeCall<'a> {
    web: &'a reqwest::Client,
    url: &'a str,
    limit: Duration,
    cookie: Option<&'a str>,
    id: u32,
    method: &'a str,
}

impl DelugeCall<'_> {
    /// The result of the request, and the headers of the answer.
    async fn send(&self, params: Value) -> Result<(Value, HeaderMap), ClientError> {
        let method = self.method;
        let body = json!({ "id": self.id, "method": method, "params": params });
        let mut request = self
            .web
            .post(self.url)
            .header(header::ACCEPT, "application/json")
            .json(&body)
            .timeout(self.limit);
        if let Some(cookie) = self.cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let response = request.send().await.map_err(unreachable)?;
        let status = response.status();
        if status != StatusCode::OK {
            return Err(refused(&format!(
                "Request for method {method} returned http code {}",
                status.as_u16()
            )));
        }
        let headers = response.headers().clone();
        let answer: Value = response.json().await.unwrap_or(Value::Null);
        if let Some(error) = answer.get("error").filter(|error| !error.is_null()) {
            let part = |name: &str| error.get(name).map_or_default(php_text);
            return Err(refused(&format!(
                "Method request returned an error (no. {}): {}",
                part("code"),
                part("message")
            )));
        }
        if answer.get("id").and_then(Value::as_u64) != Some(u64::from(self.id)) {
            return Err(refused("Response id did not match request id"));
        }
        Ok((
            answer.get("result").cloned().unwrap_or(Value::Null),
            headers,
        ))
    }
}

/// The torrents of Deluge, as the `WebAPI` plugin of Deluge sent them.
///
/// Deluge is asked in two steps: a login to its web interface with the password, and
/// then `webapi.get_torrents` with the session cookie of the login.
pub async fn deluge(
    http: &Outbound,
    client: Client<'_>,
    password: &str,
) -> Result<Vec<Value>, ClientError> {
    let url = format!("{}/json", client.base());
    let web = http.client(&url, client.never_check_certificate);
    let login = DelugeCall {
        web,
        url: &url,
        limit: client.limit(),
        cookie: None,
        id: 0,
        method: "auth.login",
    };
    let (logged_in, headers) = login.send(json!([password])).await?;
    match logged_in {
        Value::Bool(true) => {}
        Value::String(text) => return Err(refused(&text)),
        _ => return Err(refused("Login failed")),
    }
    let cookie = cookies_of(&headers);
    let list = DelugeCall {
        cookie: cookie.as_deref(),
        id: 1,
        method: "webapi.get_torrents",
        ..login
    };
    let (result, _) = list.send(json!([null, DELUGE_FIELDS])).await?;
    Ok(result
        .get("torrents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

/// The state of a torrent of Deluge for the "hide" settings, by the rules of the PHP
/// application. A torrent counts as finished only when Deluge sends its progress as the
/// text `100`. Deluge sends a number, so a torrent is either seeding or downloading.
fn deluge_status(torrent: &Value) -> &'static str {
    let part = |name: &str| torrent.get(name).unwrap_or(&Value::Null);
    let state = part("state").as_str().unwrap_or_default();
    if is_number(part("queue"), -1.0)
        && is_number(part("progress"), 100.0)
        && matches!(state, "Seeding" | "Queued" | "Paused")
    {
        "Seeding"
    } else if part("progress").as_str() == Some("100") {
        "Finished"
    } else {
        "Downloading"
    }
}

/// Which torrents of Deluge the settings hide.
#[derive(Debug, Clone, Copy, Default)]
pub struct DelugeView {
    /// Leave out the torrents that are complete and not in the queue.
    pub hide_seeding: bool,
    /// Leave out the torrents that count as finished.
    pub hide_completed: bool,
    /// Send no status of the tracker.
    pub hide_status: bool,
}

/// The torrents of Deluge without the ones that the settings hide.
pub fn shown_deluge(torrents: Vec<Value>, view: DelugeView) -> Vec<Value> {
    torrents
        .into_iter()
        .filter(|torrent| match deluge_status(torrent) {
            "Seeding" => !view.hide_seeding,
            "Finished" => !view.hide_completed,
            _ => true,
        })
        .map(|mut torrent| {
            if view.hide_status
                && let Some(fields) = torrent.as_object_mut()
            {
                fields.insert("tracker_status".to_owned(), Value::String(String::new()));
            }
            torrent
        })
        .collect()
}

/// A text as the content of an XML element.
fn xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A request of XML-RPC whose parameters are all texts.
fn xml_rpc_request(method: &str, params: &[&str]) -> String {
    let mut request = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><methodCall>");
    let _ = write!(
        request,
        "<methodName>{}</methodName><params>",
        xml_text(method)
    );
    for param in params {
        let _ = write!(
            request,
            "<param><value><string>{}</string></value></param>",
            xml_text(param)
        );
    }
    request.push_str("</params></methodCall>");
    request
}

/// How deep the lists of an XML-RPC answer are read.
const XML_RPC_DEPTH: u8 = 16;

fn element<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
}

/// A `value` element of XML-RPC as JSON. A 64-bit number becomes a text, as in the PHP
/// application, which is how rTorrent sends every number.
fn xml_rpc_value(value: Node<'_, '_>, depth: u8) -> Value {
    let Some(typed) = value.children().find(Node::is_element) else {
        // A value without a type is a text.
        return Value::String(value.text().unwrap_or_default().to_owned());
    };
    let text = typed.text().unwrap_or_default();
    match typed.tag_name().name() {
        "i4" | "int" => text.trim().parse::<i64>().map_or(Value::Null, Value::from),
        "boolean" => Value::Bool(text.trim() == "1"),
        "double" => text.trim().parse::<f64>().map_or(Value::Null, Value::from),
        "array" if depth > 0 => element(typed, "data").map_or_else(
            || Value::Array(Vec::new()),
            |data| {
                data.children()
                    .filter(|child| child.is_element() && child.tag_name().name() == "value")
                    .map(|child| xml_rpc_value(child, depth - 1))
                    .collect()
            },
        ),
        "struct" if depth > 0 => typed
            .children()
            .filter(|child| child.is_element() && child.tag_name().name() == "member")
            .filter_map(|member| {
                let name = element(member, "name")?.text().unwrap_or_default();
                let value = xml_rpc_value(element(member, "value")?, depth - 1);
                Some((name.to_owned(), value))
            })
            .collect::<Map<String, Value>>()
            .into(),
        "array" | "struct" => Value::Null,
        // `i8`, `string`, and the types that PHP reads as text.
        _ => Value::String(text.to_owned()),
    }
}

/// The value of an answer of XML-RPC, or nothing when the answer is a fault or no
/// XML-RPC at all.
fn xml_rpc_answer(body: &str) -> Option<Value> {
    let document = roxmltree::Document::parse(body).ok()?;
    let root = document.root_element();
    if root.tag_name().name() != "methodResponse" {
        return None;
    }
    let value = element(element(element(root, "params")?, "param")?, "value")?;
    Some(xml_rpc_value(value, XML_RPC_DEPTH))
}

/// The fields of a `WWW-Authenticate` challenge after the name of its scheme, with
/// their names in lower case.
fn challenge_fields(text: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let mut rest = text;
    loop {
        rest = rest.trim_start_matches([' ', '\t', ',']);
        let Some((name, after)) = rest.split_once('=') else {
            break;
        };
        let after = after.trim_start();
        let (value, next) = if let Some(quoted) = after.strip_prefix('"') {
            let mut value = String::new();
            let mut end = quoted.len();
            let mut characters = quoted.char_indices();
            while let Some((at, character)) = characters.next() {
                match character {
                    '\\' => value.extend(characters.next().map(|(_, escaped)| escaped)),
                    '"' => {
                        end = at + 1;
                        break;
                    }
                    other => value.push(other),
                }
            }
            (value, quoted.get(end..).unwrap_or_default())
        } else {
            let end = after.find(',').unwrap_or(after.len());
            (
                after.get(..end).unwrap_or_default().trim().to_owned(),
                after.get(end..).unwrap_or_default(),
            )
        };
        fields.push((name.trim().to_ascii_lowercase(), value));
        rest = next;
    }
    fields
}

fn md5_hex(text: &str) -> String {
    let mut hex = String::new();
    for byte in &Md5::digest(text.as_bytes()) {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// A text inside the quotes of a header.
fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The `Authorization` header that answers a Digest challenge with MD5, or nothing for
/// a challenge that asks for something else.
fn digest_answer(
    login: Login<'_>,
    method: &str,
    uri: &str,
    challenge: &[(String, String)],
    client_nonce: &str,
) -> Option<String> {
    let field = |name: &str| {
        challenge
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value.as_str())
    };
    let realm = field("realm")?;
    let nonce = field("nonce")?;
    if field("algorithm").is_some_and(|algorithm| !algorithm.eq_ignore_ascii_case("md5")) {
        return None;
    }
    let first = md5_hex(&format!("{}:{realm}:{}", login.username, login.password));
    let second = md5_hex(&format!("{method}:{uri}"));
    let mut answer = format!(
        "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\"",
        quoted(login.username),
        quoted(realm),
        quoted(nonce),
        quoted(uri)
    );
    let response = match field("qop") {
        Some(qop) => {
            // Only the plain kind of protection is answered.
            if !qop.split(',').any(|kind| kind.trim() == "auth") {
                return None;
            }
            let count = "00000001";
            let _ = write!(answer, ", qop=auth, nc={count}, cnonce=\"{client_nonce}\"");
            md5_hex(&format!(
                "{first}:{nonce}:{count}:{client_nonce}:auth:{second}"
            ))
        }
        None => md5_hex(&format!("{first}:{nonce}:{second}")),
    };
    let _ = write!(answer, ", response=\"{response}\"");
    if let Some(opaque) = field("opaque") {
        let _ = write!(answer, ", opaque=\"{}\"", quoted(opaque));
    }
    Some(answer)
}

/// A number that nobody can guess easily, for the answer to a Digest challenge.
fn client_nonce() -> String {
    format!("{:016x}", RandomState::new().build_hasher().finish())
}

/// How a server wants the name and the password.
enum Wanted {
    /// As a Digest answer: the text is the `Authorization` header.
    Digest(String),
    /// As they are, in a Basic header.
    Basic,
}

/// What the challenges of a refused request ask for. Digest comes before Basic,
/// because it does not send the password.
fn wanted(headers: &HeaderMap, login: Login<'_>, method: &str, url: &str) -> Option<Wanted> {
    let challenges: Vec<(String, &str)> = headers
        .get_all(header::WWW_AUTHENTICATE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(|challenge| {
            let (scheme, fields) = challenge
                .trim()
                .split_once(char::is_whitespace)
                .unwrap_or((challenge.trim(), ""));
            (scheme.to_ascii_lowercase(), fields)
        })
        .collect();
    let uri = reqwest::Url::parse(url).map_or_else(
        |_| "/".to_owned(),
        |url| match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_owned(),
        },
    );
    challenges
        .iter()
        .filter(|(scheme, _)| scheme == "digest")
        .find_map(|(_, fields)| {
            digest_answer(
                login,
                method,
                &uri,
                &challenge_fields(fields),
                &client_nonce(),
            )
        })
        .map(Wanted::Digest)
        .or_else(|| {
            challenges
                .iter()
                .any(|(scheme, _)| scheme == "basic")
                .then_some(Wanted::Basic)
        })
}

/// Sends an XML-RPC request. The name and the password go to the server only after the
/// server asked for them, in the way that it asked for, as curl does for the PHP
/// application.
async fn post_xml(
    web: &reqwest::Client,
    url: &str,
    body: &str,
    login: Login<'_>,
    limit: Duration,
) -> Result<Response, ClientError> {
    let request = || {
        web.post(url)
            .header(header::CONTENT_TYPE, "text/xml")
            .body(body.to_owned())
            .timeout(limit)
    };
    let response = request().send().await.map_err(unreachable)?;
    if response.status() != StatusCode::UNAUTHORIZED
        || login.username.is_empty()
        || login.password.is_empty()
    {
        return Ok(response);
    }
    let request = match wanted(response.headers(), login, "POST", url) {
        Some(Wanted::Digest(answer)) => request().header(header::AUTHORIZATION, answer),
        Some(Wanted::Basic) => request().basic_auth(login.username, Some(login.password)),
        None => return Ok(response),
    };
    request.send().await.map_err(unreachable)
}

/// Asks rTorrent with a method of XML-RPC. `client.url` is the complete address of the
/// XML-RPC endpoint. Gives nothing when rTorrent refused the request, answered with a
/// fault, or answered with something that is no XML-RPC.
async fn rtorrent_call(
    http: &Outbound,
    client: Client<'_>,
    login: Login<'_>,
    method: &str,
    params: &[&str],
) -> Result<Option<Value>, ClientError> {
    let web = http.client(client.url, client.never_check_certificate);
    let body = xml_rpc_request(method, params);
    let response = post_xml(web, client.url, &body, login, client.limit()).await?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), method, "rTorrent refused the request");
        return Ok(None);
    }
    let body = response.text().await.unwrap_or_default();
    let answer = xml_rpc_answer(&body);
    if answer.is_none() {
        tracing::warn!(method, "rTorrent answered with a fault or with no XML-RPC");
    }
    Ok(answer)
}

/// The parameters of the request for the torrents of rTorrent: the view, and then the
/// values of each torrent, in the order that `rtorrent_items` reads them.
const RTORRENT_FIELDS: [&str; 27] = [
    "",
    "main",
    "d.name=",
    "d.base_path=",
    "d.up.total=",
    "d.size_bytes=",
    "d.down.total=",
    "d.completed_bytes=",
    "d.connection_current=",
    "d.down.rate=",
    "d.up.rate=",
    "d.timestamp.started=",
    "d.state=",
    "d.group.name=",
    "d.hash=",
    "d.complete=",
    "d.ratio=",
    "d.chunk_size=",
    "f.size_bytes=",
    "f.size_chunks=",
    "f.completed_chunks=",
    "d.custom=",
    "d.custom1=",
    "d.custom2=",
    "d.custom3=",
    "d.custom4=",
    "d.custom5=",
];

/// The torrents of rTorrent: for each torrent the list of its values, as rTorrent sent
/// them. Nothing when rTorrent gave no list.
pub async fn rtorrent(
    http: &Outbound,
    client: Client<'_>,
    login: Login<'_>,
) -> Result<Option<Vec<Value>>, ClientError> {
    let answer = rtorrent_call(http, client, login, "d.multicall2", &RTORRENT_FIELDS).await?;
    Ok(match answer {
        Some(Value::Array(rows)) => Some(rows),
        _ => None,
    })
}

/// Whether rTorrent answers the question for its methods with a list that holds
/// something.
pub async fn rtorrent_answers(
    http: &Outbound,
    client: Client<'_>,
    login: Login<'_>,
) -> Result<bool, ClientError> {
    let answer = rtorrent_call(http, client, login, "system.listMethods", &[]).await?;
    Ok(matches!(answer, Some(Value::Array(methods)) if !methods.is_empty()))
}

/// How the torrents of rTorrent show.
#[derive(Debug, Clone, Copy)]
pub struct RtorrentView<'a> {
    /// Leave out the torrents that upload.
    pub hide_seeding: bool,
    /// Leave out the torrents that are complete and closed.
    pub hide_completed: bool,
    /// The labels whose torrents are left out, as one text.
    pub ignored_labels: &'a str,
    /// The field that the torrents are sorted by, and then `a` for upwards or `d` for
    /// downwards, for example `datea`.
    pub sort_order: &'a str,
    /// The most torrents. Zero means one thousand.
    pub limit: u32,
}

/// The state of a torrent of rTorrent in a word, by the rules of the PHP application.
/// For a mix that the rules do not name, it is the state or the connection as rTorrent
/// sent it.
fn rtorrent_status(complete: &Value, state: &Value, connection: &Value) -> Value {
    let word = match (php_true(complete), php_true(state), connection.as_str()) {
        (true, true, Some("seed")) => "Seeding",
        (false, false, Some("leech")) => "Stopped",
        (false, true, Some("leech")) => "Downloading",
        (true, false, Some("seed" | "leech")) => "Finished",
        _ if php_true(state) => return state.clone(),
        _ => return connection.clone(),
    };
    Value::String(word.to_owned())
}

/// One torrent of rTorrent as the homepage gets it, or nothing for a torrent that the
/// settings hide.
fn rtorrent_item(row: &Value, view: &RtorrentView<'_>) -> Option<Value> {
    let at = |index: usize| row.get(index).unwrap_or(&Value::Null);
    let joined = |first: usize| {
        Value::String(format!(
            "{} - {} - {}",
            php_text(at(first)),
            php_text(at(first + 1)),
            php_text(at(first + 2))
        ))
    };
    let status = rtorrent_status(at(13), at(10), at(6));
    let label = php_text(at(20));
    let hidden = match status.as_str() {
        Some("Seeding") if view.hide_seeding => true,
        Some("Finished") if view.hide_completed => true,
        // A torrent without a label is never left out for its label.
        _ => {
            !label.is_empty()
                && view
                    .ignored_labels
                    .to_lowercase()
                    .contains(&label.to_lowercase())
        }
    };
    if hidden {
        return None;
    }
    let on = |index: usize, yes: &str, no: &str| {
        Value::String(if php_true(at(index)) { yes } else { no }.to_owned())
    };
    let mut item = Map::new();
    let fields = [
        ("name", at(0).clone()),
        ("base", at(1).clone()),
        ("upTotal", at(2).clone()),
        ("size", at(3).clone()),
        ("downTotal", at(4).clone()),
        ("downloaded", at(5).clone()),
        ("connectionState", at(6).clone()),
        ("leech", at(7).clone()),
        ("seed", at(8).clone()),
        ("date", at(9).clone()),
        ("state", on(10, "on", "off")),
        ("group", at(11).clone()),
        ("hash", at(12).clone()),
        ("complete", on(13, "yes", "no")),
        ("ratio", at(14).clone()),
        ("label", at(20).clone()),
        ("status", status),
        ("temp", joined(16)),
        ("custom", joined(19)),
        ("custom2", joined(22)),
    ];
    for (name, value) in fields {
        item.insert(name.to_owned(), value);
    }
    Some(Value::Object(item))
}

/// The torrents of rTorrent as the homepage gets them: without the ones that the
/// settings hide, in the order of the settings, and not more than the limit.
pub fn rtorrent_items(rows: &[Value], view: &RtorrentView<'_>) -> Vec<Value> {
    let mut items: Vec<Value> = rows
        .iter()
        .filter_map(|row| rtorrent_item(row, view))
        .collect();
    let (field, direction) = view
        .sort_order
        .char_indices()
        .next_back()
        .map_or(("", ' '), |(at, direction)| {
            (view.sort_order.get(..at).unwrap_or_default(), direction)
        });
    items.sort_by(|first, second| match direction {
        'a' => php_compare(first.get(field), second.get(field)),
        'd' => php_compare(second.get(field), first.get(field)),
        _ => php_compare(second.get("date"), first.get("date")),
    });
    let limit = if view.limit == 0 { 1000 } else { view.limit };
    items.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    items
}

/// The token in the token page of uTorrent: the text of the element with the id
/// `token`. A token with a character that cannot stand in an address is no token.
fn utorrent_token(page: &str) -> Option<&str> {
    let after_id = ["id='token'", "id=\"token\"", "id=token"]
        .iter()
        .find_map(|id| page.find(id).map(|at| at + id.len()))?;
    let rest = page.get(after_id..)?;
    let rest = rest.get(rest.find('>')? + 1..)?;
    let token = rest.get(..rest.find('<')?)?.trim();
    let fits = token
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"-_=.~+/".contains(&byte));
    (!token.is_empty() && fits).then_some(token)
}

/// The `GUID` cookie that uTorrent sets with its token.
fn utorrent_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|cookie| cookie.split(';').next()?.split_once('='))
        .find(|(name, value)| name.trim() == "GUID" && !value.is_empty())
        .map(|(_, value)| format!("GUID={value}"))
}

/// The text that the homepage answers with when uTorrent gave no token.
const NO_TOKEN: &str = "uTorrent Connect Function - Error: Could not get token";

/// The torrents of uTorrent: for each torrent the list of its values, as uTorrent sent
/// them. Nothing when uTorrent refused the request for the list.
///
/// uTorrent is asked in two steps: its token page gives a token and a cookie, and the
/// request for the list carries both.
pub async fn utorrent(
    http: &Outbound,
    client: Client<'_>,
    login: Login<'_>,
) -> Result<Option<Vec<Value>>, ClientError> {
    let url = format!("{}/gui/token.html", client.base());
    let web = http.client(&url, client.never_check_certificate);
    let request = web
        .post(&url)
        .form(&[("username", login.username), ("password", login.password)])
        .timeout(client.limit());
    let response = login.on(request).send().await.map_err(unreachable)?;
    let status = response.status();
    let cookie = utorrent_cookie(response.headers());
    let page = response.text().await.unwrap_or_default();
    let Some(token) = utorrent_token(&page) else {
        tracing::warn!(%status, "uTorrent gave no token");
        return Err(refused(NO_TOKEN));
    };

    // The token stands in the address as it came, as in the PHP application.
    let url = format!("{}/gui/?token={token}&list=1", client.base());
    let mut request = web.get(&url).timeout(client.limit());
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let response = login.on(request).send().await.map_err(unreachable)?;
    if response.status() != StatusCode::OK {
        tracing::warn!(status = %response.status(), "uTorrent refused the request");
        return Ok(None);
    }
    let answer = response.json::<Value>().await.unwrap_or(Value::Null);
    Ok(answer.get("torrents").and_then(Value::as_array).cloned())
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

/// How many bytes from `at` on are of a kind.
fn run(bytes: &[u8], at: usize, kind: fn(u8) -> bool) -> usize {
    bytes
        .get(at..)
        .unwrap_or_default()
        .iter()
        .take_while(|byte| kind(**byte))
        .count()
}

/// Whether a percentage starts at `at`: digits, one character, and a digit.
fn is_percentage(bytes: &[u8], at: usize) -> bool {
    let digits = run(bytes, at, |byte| byte.is_ascii_digit());
    (1..=digits).any(|taken| {
        bytes.get(at + taken).is_some_and(|byte| *byte != b'\n')
            && bytes.get(at + taken + 1).is_some_and(u8::is_ascii_digit)
    })
}

/// The status and the percentage in the message of a torrent of uTorrent, each without
/// its spaces: `Downloading 45.3 %` gives `Downloading` and `45.3%`.
///
/// This is the pattern `(?<Status>(\w+\s+)+)(?<Percentage>\d+.\d+.*)` of the PHP
/// application: words with white space after each, and then a percentage up to the end
/// of the line.
fn utorrent_message(message: &str) -> Option<(String, String)> {
    let bytes = message.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        if !bytes.get(start).copied().is_some_and(is_word) {
            start += 1;
            continue;
        }
        // The places after each word and its white space. The percentage starts at the
        // last of them that a percentage can start at.
        let mut at = start;
        let mut places = Vec::new();
        let rest = loop {
            let words = run(bytes, at, is_word);
            let spaces = run(bytes, at + words, is_space);
            if words == 0 || spaces == 0 {
                break at + words;
            }
            at += words + spaces;
            places.push(at);
        };
        if let Some(percentage) = places
            .into_iter()
            .rev()
            .find(|place| is_percentage(bytes, *place))
        {
            let end = bytes
                .get(percentage..)
                .unwrap_or_default()
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |line| percentage + line);
            let part =
                |from: usize, to: usize| message.get(from..to).unwrap_or_default().replace(' ', "");
            return Some((part(start, percentage), part(percentage, end)));
        }
        // A later start inside these words finds the same places, or fewer.
        start = rest.max(start + 1);
    }
    None
}

/// The names of the values of a torrent of uTorrent, in the order of its list.
const UTORRENT_FIELDS: [&str; 29] = [
    "Hash",
    "TorrentStatus",
    "Name",
    "Size",
    "Progress",
    "Downloaded",
    "Uploaded",
    "Ratio",
    "upSpeed",
    "downSpeed",
    "eta",
    "Labels",
    "PeersConnected",
    "PeersInSwarm",
    "SeedsConnected",
    "SeedsInSwarm",
    "Availability",
    "TorrentQueueOrder",
    "Remaining",
    "DownloadUrl",
    "RssFeedUrl",
    "Message",
    "StreamId",
    "DateAdded",
    "DateCompleted",
    "AppUpdateUrl",
    "RootDownloadPath",
    "Unknown27",
    "Unknown28",
];

/// The torrents of uTorrent as the homepage gets them, each with the names of its
/// values, and without the ones that the settings hide.
pub fn utorrent_items(rows: &[Value], hide_seeding: bool, hide_completed: bool) -> Vec<Value> {
    rows.iter()
        .filter_map(|row| {
            let message = row.get(21).and_then(Value::as_str).unwrap_or_default();
            let (status, percent) = utorrent_message(message).unwrap_or_default();
            if hide_seeding && status == "Seeding" || hide_completed && status == "Finished" {
                return None;
            }
            let mut item = Map::new();
            for (index, name) in UTORRENT_FIELDS.iter().enumerate() {
                let value = row.get(index).cloned().unwrap_or(Value::Null);
                item.insert((*name).to_owned(), value);
            }
            item.insert("Status".to_owned(), Value::String(status));
            item.insert("Percent".to_owned(), Value::String(percent));
            Some(Value::Object(item))
        })
        .collect()
}

/// The answer of the service that reads JDownloader, as it sent it. Nothing when the
/// service refused the request. `client.url` is the complete address that is asked.
///
/// The name and the password go with the request when one of them is set.
pub async fn jdownloader(
    http: &Outbound,
    client: Client<'_>,
    login: Login<'_>,
) -> Result<Option<Value>, ClientError> {
    let mut request = http
        .client(client.url, client.never_check_certificate)
        .get(client.url)
        .timeout(client.limit());
    if !login.username.is_empty() || !login.password.is_empty() {
        request = request.basic_auth(login.username, Some(login.password));
    }
    let response = request.send().await.map_err(unreachable)?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "the service of JDownloader refused the request");
        return Ok(None);
    }
    Ok(Some(response.json().await.unwrap_or(Value::Null)))
}

/// The content of the JDownloader item from the answer of its service: the four lists
/// of packages, and the three states.
pub fn jdownloader_content(answer: &Value) -> Value {
    let packages = |name: &str| {
        answer
            .get("packages")
            .and_then(|packages| packages.get(name))
            .filter(|list| php_true(list))
            .cloned()
            .unwrap_or_else(|| json!([]))
    };
    let state = |name: &str| answer.get(name).cloned().unwrap_or(Value::Null);
    json!({
        "queueItems": packages("downloader"),
        "grabberItems": packages("linkgrabber_decrypted"),
        "encryptedItems": packages("linkgrabber_failed"),
        "offlineItems": packages("linkgrabber_offline"),
        "$status": [
            state("downloader_state"),
            state("grabber_collecting"),
            state("update_ready"),
        ],
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{
        DelugeView, Login, RtorrentView, challenge_fields, digest_answer, jdownloader_content,
        rtorrent_items, shown_deluge, utorrent_items, utorrent_message, utorrent_token,
        xml_rpc_answer, xml_rpc_request,
    };

    fn names(items: &[Value], field: &str) -> Vec<String> {
        items
            .iter()
            .map(|item| item[field].as_str().unwrap_or("?").to_owned())
            .collect()
    }

    #[test]
    fn deluge_hides_what_is_complete_and_out_of_the_queue() {
        let torrent = |name: &str, queue: i64, progress: f64, state: &str| {
            json!({"name": name, "queue": queue, "progress": progress, "state": state,
                   "tracker_status": "Announce OK"})
        };
        let all = vec![
            torrent("loads", 0, 40.5, "Downloading"),
            torrent("uploads", -1, 100.0, "Seeding"),
            torrent("paused and complete", -1, 100.0, "Paused"),
            torrent("checks", -1, 100.0, "Checking"),
        ];
        let view = |hide_seeding, hide_completed, hide_status| DelugeView {
            hide_seeding,
            hide_completed,
            hide_status,
        };

        let shown = shown_deluge(all.clone(), view(false, false, false));
        assert_eq!(shown, all);
        let shown = shown_deluge(all.clone(), view(true, false, true));
        assert_eq!(names(&shown, "name"), ["loads", "checks"]);
        assert_eq!(shown[0]["tracker_status"], json!(""));
        // Deluge sends the progress as a number, so nothing counts as finished.
        let shown = shown_deluge(all, view(false, true, false));
        assert_eq!(shown.len(), 4);
        let text = vec![json!({"name": "odd", "queue": 2, "progress": "100", "state": "Error"})];
        assert_eq!(shown_deluge(text, view(false, true, false)).len(), 0);
    }

    #[test]
    fn xml_rpc_request_holds_each_parameter_as_a_text() {
        assert_eq!(
            xml_rpc_request("d.multicall2", &["", "a<b&c"]),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><methodCall>\
             <methodName>d.multicall2</methodName><params>\
             <param><value><string></string></value></param>\
             <param><value><string>a&lt;b&amp;c</string></value></param>\
             </params></methodCall>"
        );
    }

    #[test]
    fn xml_rpc_answer_is_read_with_long_numbers_as_text() {
        let body = "<?xml version=\"1.0\"?><methodResponse><params><param><value><array><data>\
            <value><array><data>\
            <value><string>The &amp; Show</string></value>\
            <value><i8>1234567890123</i8></value>\
            <value><i4>7</i4></value>\
            <value>plain</value>\
            <value><string></string></value>\
            <value><boolean>1</boolean></value>\
            <value><struct><member><name>a</name><value><int>1</int></value></member></struct></value>\
            </data></array></value>\
            </data></array></value></param></params></methodResponse>";
        assert_eq!(
            xml_rpc_answer(body),
            Some(json!([[
                "The & Show",
                "1234567890123",
                7,
                "plain",
                "",
                true,
                {"a": 1}
            ]]))
        );
        let fault = "<methodResponse><fault><value><struct><member><name>faultCode</name>\
            <value><i4>-501</i4></value></member></struct></value></fault></methodResponse>";
        assert_eq!(xml_rpc_answer(fault), None);
        assert_eq!(xml_rpc_answer("<html>no</html>"), None);
        assert_eq!(xml_rpc_answer("no XML"), None);
    }

    /// A row of rTorrent: name, size, bytes that are complete, connection, time, state,
    /// hash, whether it is complete, and label.
    fn row(
        name: &str,
        connection: &str,
        date: &str,
        state: &str,
        complete: &str,
        label: &str,
    ) -> Value {
        let mut row = vec![json!(""); 25];
        row[0] = json!(name);
        row[3] = json!("1000");
        row[5] = json!("400");
        row[6] = json!(connection);
        row[9] = json!(date);
        row[10] = json!(state);
        row[12] = json!(format!("hash-of-{name}"));
        row[13] = json!(complete);
        row[16] = json!("16");
        row[17] = json!("17");
        row[18] = json!("18");
        row[20] = json!(label);
        Value::Array(row)
    }

    fn rows() -> Vec<Value> {
        vec![
            row("b loads", "leech", "300", "1", "0", "tv"),
            row("a uploads", "seed", "100", "1", "1", ""),
            row("d is closed", "seed", "1000", "0", "1", "films"),
            row("c is stopped", "leech", "20", "0", "0", "Private"),
        ]
    }

    const VIEW: RtorrentView<'static> = RtorrentView {
        hide_seeding: false,
        hide_completed: false,
        ignored_labels: "",
        sort_order: "datea",
        limit: 200,
    };

    #[test]
    fn rtorrent_items_have_the_fields_and_the_states_of_php() {
        let items = rtorrent_items(&rows(), &VIEW);
        // Dates are numbers in text, and are sorted as numbers.
        assert_eq!(
            names(&items, "name"),
            ["c is stopped", "a uploads", "b loads", "d is closed"]
        );
        assert_eq!(
            names(&items, "status"),
            ["Stopped", "Seeding", "Downloading", "Finished"]
        );
        assert_eq!(
            items[2],
            json!({
                "name": "b loads", "base": "", "upTotal": "", "size": "1000", "downTotal": "",
                "downloaded": "400", "connectionState": "leech", "leech": "", "seed": "",
                "date": "300", "state": "on", "group": "", "hash": "hash-of-b loads",
                "complete": "no", "ratio": "", "label": "tv", "status": "Downloading",
                "temp": "16 - 17 - 18", "custom": " - tv - ", "custom2": " -  - ",
            })
        );
        let keys: Vec<&String> = items[2].as_object().unwrap().keys().collect();
        assert_eq!(keys[0], "name");
        assert_eq!(keys[19], "custom2");
    }

    #[test]
    fn rtorrent_items_follow_the_settings() {
        let view = |change: fn(&mut RtorrentView<'static>)| {
            let mut view = VIEW;
            change(&mut view);
            names(&rtorrent_items(&rows(), &view), "name")
        };
        assert_eq!(
            view(|view| view.hide_seeding = true),
            ["c is stopped", "b loads", "d is closed"]
        );
        assert_eq!(
            view(|view| view.hide_completed = true),
            ["c is stopped", "a uploads", "b loads"]
        );
        // A label is left out when the setting holds it, in any case of its letters. A
        // torrent without a label stays.
        assert_eq!(
            view(|view| view.ignored_labels = "private,TV"),
            ["a uploads", "d is closed"]
        );
        assert_eq!(
            view(|view| view.sort_order = "named"),
            ["d is closed", "c is stopped", "b loads", "a uploads"]
        );
        // An order that is no order is the newest first.
        assert_eq!(
            view(|view| view.sort_order = ""),
            ["d is closed", "b loads", "a uploads", "c is stopped"]
        );
        assert_eq!(view(|view| view.limit = 1), ["c is stopped"]);
        assert_eq!(view(|view| view.limit = 0).len(), 4);
    }

    #[test]
    fn utorrent_token_is_the_text_of_the_token_element() {
        let page = "<html><div id='token' style='display:none;'>made-up_Token123</div></html>";
        assert_eq!(utorrent_token(page), Some("made-up_Token123"));
        assert_eq!(
            utorrent_token("<div id=\"token\"> abc= </div>"),
            Some("abc=")
        );
        assert_eq!(utorrent_token("<html>401 Unauthorized</html>"), None);
        assert_eq!(utorrent_token("<div id='token'></div>"), None);
        assert_eq!(utorrent_token("<div id='token'>a b&c</div>"), None);
    }

    #[test]
    fn utorrent_message_is_split_as_the_pattern_of_php_splits_it() {
        let parts = |message: &str| {
            utorrent_message(message).map(|(status, percent)| format!("{status}|{percent}"))
        };
        assert_eq!(
            parts("Downloading 45.3 %").as_deref(),
            Some("Downloading|45.3%")
        );
        assert_eq!(parts("Seeding 100.0 %").as_deref(), Some("Seeding|100.0%"));
        assert_eq!(
            parts("[F] Seeding 100.0 %").as_deref(),
            Some("Seeding|100.0%")
        );
        assert_eq!(
            parts("Queued Seed 100.0 %").as_deref(),
            Some("QueuedSeed|100.0%")
        );
        // A word of digits belongs to the status when a percentage still follows.
        assert_eq!(parts("Part 2 10.5 %").as_deref(), Some("Part2|10.5%"));
        assert_eq!(
            parts("Stopped 0.0 %\nmore").as_deref(),
            Some("Stopped|0.0%")
        );
        assert_eq!(parts("Finished"), None);
        assert_eq!(parts("45.3 %"), None);
        assert_eq!(parts(""), None);
    }

    #[test]
    fn utorrent_items_have_names_and_leave_out_what_the_settings_hide() {
        let row = |name: &str, message: &str| {
            let mut row = vec![json!(0); 29];
            row[2] = json!(name);
            row[21] = json!(message);
            Value::Array(row)
        };
        let rows = vec![
            row("loads", "Downloading 45.3 %"),
            row("uploads", "Seeding 100.0 %"),
            row("done", "Finished 100.0 %"),
            json!(["short"]),
        ];

        let items = utorrent_items(&rows, false, false);
        assert_eq!(items.len(), 4);
        assert_eq!(items[0]["Name"], json!("loads"));
        assert_eq!(items[0]["Status"], json!("Downloading"));
        assert_eq!(items[0]["Percent"], json!("45.3%"));
        assert_eq!(items[0].as_object().map(serde_json::Map::len), Some(31));
        // A value that uTorrent did not send is `null`.
        assert_eq!(items[3]["Hash"], json!("short"));
        assert_eq!(items[3]["Name"], Value::Null);
        assert_eq!(items[3]["Status"], json!(""));
        assert_eq!(
            names(&utorrent_items(&rows, true, false), "Status"),
            ["Downloading", "Finished", ""]
        );
        assert_eq!(
            names(&utorrent_items(&rows, true, true), "Status"),
            ["Downloading", ""]
        );
    }

    #[test]
    fn jdownloader_content_has_a_list_for_each_kind_of_package() {
        let answer = json!({
            "downloader_state": "RUNNING",
            "grabber_collecting": false,
            "update_ready": true,
            "packages": {
                "downloader": [{"name": "A.Package"}],
                "linkgrabber_decrypted": [],
                "linkgrabber_failed": null,
            },
        });
        assert_eq!(
            jdownloader_content(&answer),
            json!({
                "queueItems": [{"name": "A.Package"}],
                "grabberItems": [],
                "encryptedItems": [],
                "offlineItems": [],
                "$status": ["RUNNING", false, true],
            })
        );
        assert_eq!(
            jdownloader_content(&Value::Null)["$status"],
            json!([null, null, null])
        );
    }

    #[test]
    fn digest_challenge_is_answered_as_the_standard_says() {
        // The example of RFC 2617, section 3.5.
        let fields = challenge_fields(
            "realm=\"testrealm@host.com\", qop=\"auth,auth-int\", \
             nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\", \
             opaque=\"5ccc069c403ebaf9f0171e9517f40e41\"",
        );
        let login = Login {
            username: "Mufasa",
            password: "Circle Of Life",
        };
        let answer = digest_answer(login, "GET", "/dir/index.html", &fields, "0a4f113b");
        assert_eq!(
            answer.as_deref(),
            Some(
                "Digest username=\"Mufasa\", realm=\"testrealm@host.com\", \
                 nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\", uri=\"/dir/index.html\", \
                 qop=auth, nc=00000001, cnonce=\"0a4f113b\", \
                 response=\"6629fae49393a05397450978507c4ef1\", \
                 opaque=\"5ccc069c403ebaf9f0171e9517f40e41\""
            )
        );
        // Another algorithm than MD5 is not answered.
        let other = challenge_fields("realm=\"r\", nonce=\"n\", algorithm=SHA-256");
        assert_eq!(digest_answer(login, "POST", "/RPC2", &other, "c"), None);
        let plain = challenge_fields("realm=\"r\", nonce=\"n\"");
        assert!(digest_answer(login, "POST", "/RPC2", &plain, "c").is_some());
    }
}
