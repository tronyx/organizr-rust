//! Forward auth: the routes that a reverse proxy asks before it lets a request through
//! to another application.
//!
//! The proxy sends each request for the other application here first, with the cookies
//! of the browser. The answer `200` lets the request through, and every other answer
//! stops it. The path names the group that the application needs, as a level or as the
//! name of a tab:
//!
//! - `/auth-{group}`: the short form. Lists of addresses come in the query: `whitelist`
//!   and `blacklist`, or `type` and `ips`.
//! - `/auth/{group}/{type}/{ips}` and `/organizr-auth/...`: the long form. Without a
//!   `type`, the kind of list is `deny` and its only address is `192.0.0.0`.
//!
//! Every method asks the same question. A query that holds `group=` always asks in the
//! short form, with everything after `group=` as the group, as in the PHP application.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::Extension;
use axum::extract::{RawPathParams, RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::caller::Caller;
use crate::client::Client;
use crate::cookies::cookie_value;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::mail::request_site;
use crate::router::API_BASE;
use crate::state::AppState;

/// The cookie that names the device of the caller.
const DEVICE_COOKIE: &str = "organizr_user_uuid";

/// What the long form asks when the path names no list.
const DEFAULT_KIND: &str = "deny";
const DEFAULT_ADDRESSES: &str = "192.0.0.0";

/// How long the name of a host in a list may take to look up.
const LOOKUP_TIME: Duration = Duration::from_secs(2);

/// How many entries of a list count (D-141).
const MOST_ENTRIES: usize = 100;

/// How long the addresses of a host name are remembered, and for how many names.
const REMEMBERED_FOR: Duration = Duration::from_mins(1);
const MOST_REMEMBERED: usize = 256;

/// The host names of the lists, each with when it was looked up and what was found.
type LookedUp = HashMap<String, (Instant, Vec<IpAddr>)>;

/// The host names that lookups found a moment ago.
static NAMES: LazyLock<Mutex<LookedUp>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// What every answer tells about the caller.
#[derive(Debug, Clone, Serialize)]
pub struct CallerInfo {
    user: String,
    group: u16,
    email: Option<String>,
    user_ip: String,
    requested_group: i64,
    uuid: String,
}

/// The question of one request.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Question {
    /// The group as the request names it: a number, or the name of a tab.
    group: String,
    /// The addresses that pass whatever the group.
    allowed: Option<String>,
    /// The addresses that never pass.
    denied: Option<String>,
}

/// A kind of list that the request names and that does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
struct UnknownKind(String);

/// The value of a key of the query, as PHP reads it.
fn query_value(query: &str, key: &str) -> Option<String> {
    form_urlencoded::parse(query.as_bytes())
        .filter(|(name, _)| name == key)
        .last()
        .map(|(_, value)| value.into_owned())
}

/// Decodes the `%xx` parts of a piece of an address. A `+` stays a `+`.
fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(byte) = bytes.get(index) {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        let (decoded_byte, length) = match hex {
            Some(value) if *byte == b'%' => (value, 3),
            _ => (*byte, 1),
        };
        decoded.push(decoded_byte);
        index += length;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// The entries of a list that are addresses, in their order, with commas between them.
fn addresses_only(list: &str) -> String {
    list.split(',')
        .map(str::trim)
        .filter(|entry| entry.parse::<IpAddr>().is_ok())
        .collect::<Vec<_>>()
        .join(",")
}

/// Reads the question from the path and the query. `long_form` says whether the path is
/// one of the long forms, and `parts` are the pieces of the path after the name of the
/// route.
///
/// A proxy such as Caddy sends the query of the visitor with the address of the check. So
/// when the path names a group, the query cannot change the group or add an allow list;
/// it can only add a deny list, which makes the question stricter (D-136).
#[cfg(test)]
fn question(long_form: bool, parts: &[String], query: &str) -> Result<Question, UnknownKind> {
    ask(long_form, parts, query, false)
}

/// As [`question`]. `query_is_the_proxys` says that a trusted proxy wrote the query of
/// the check itself, so that its allow list counts also when the path names a group
/// (D-157).
fn ask(
    long_form: bool,
    parts: &[String],
    query: &str,
    query_is_the_proxys: bool,
) -> Result<Question, UnknownKind> {
    let mut group = parts.first().cloned();
    let path_names_group = group.as_deref().is_some_and(|group| !group.is_empty());
    // A list from the query is the visitor's to write, so only its addresses count: a
    // host name there would make Organizr look it up (D-141). A list in the path is the
    // operator's, and its host names count.
    let from_query = |key: &str| query_value(query, key).map(|list| addresses_only(&list));
    let mut kind = query_value(query, "type");
    let mut addresses = from_query("ips");

    // The PHP application turns every request whose query holds `group=` into the short
    // form, with the rest of the query as the group.
    if !path_names_group && query.to_ascii_lowercase().contains("group=") {
        let rest = query.split_once("group=").map_or("", |(_, rest)| rest);
        group = Some(percent_decoded(rest.trim_end_matches('/')));
    } else if long_form {
        kind = Some(
            parts
                .get(1)
                .cloned()
                .unwrap_or_else(|| DEFAULT_KIND.to_owned()),
        );
        addresses = Some(
            parts
                .get(2)
                .cloned()
                .unwrap_or_else(|| DEFAULT_ADDRESSES.to_owned()),
        );
    }

    // Whether the allow list of the query counts: the kind and addresses of the long
    // form come from the path, those of the short form from the query.
    let query_allows = !path_names_group || query_is_the_proxys;
    let kind_from_query = !long_form || !path_names_group;
    let mut allowed = query_allows.then(|| from_query("whitelist")).flatten();
    let mut denied = from_query("blacklist");
    if let Some(kind) = kind {
        match kind.to_lowercase().as_str() {
            "whitelist" | "white" | "w" | "wl" | "allow" => {
                if query_allows || !kind_from_query {
                    allowed = addresses;
                }
            }
            "blacklist" | "black" | "b" | "bl" | "deny" => denied = addresses,
            _ => return Err(UnknownKind(kind)),
        }
    }
    // What PHP takes for "no list": nothing, an empty text, and the text `0`.
    let filled = |list: Option<String>| list.filter(|list| !list.is_empty() && list != "0");
    Ok(Question {
        group: group.unwrap_or_default(),
        allowed: filled(allowed),
        denied: filled(denied),
    })
}

/// The level that a number in the path stands for, as PHP reads numbers: `998.9` is 998
/// and `1e3` is 1000. `None` for a text that is no number.
fn level_of_number(text: &str) -> Option<i64> {
    let text = text.trim_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}']);
    let is_number = !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || b"+-.eE".contains(&byte))
        && text.bytes().any(|byte| byte.is_ascii_digit());
    let number = text.parse::<f64>().ok().filter(|_| is_number)?;
    // A number too large for a level asks for the Admin, which lets the fewest in.
    if !number.is_finite() || number.abs() >= 9.0e18 {
        return Some(0);
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the number is cut to a whole number on purpose, and it is in range"
    )]
    Some(number.trunc() as i64)
}

/// Whether the address of the caller is in a list of addresses and host names, which
/// commas separate.
async fn lists(list: &str, address: IpAddr) -> bool {
    // Only the first entries count, so one list costs a few lookups at most (D-141).
    for entry in list.split(',').map(str::trim).take(MOST_ENTRIES) {
        if let Ok(listed) = entry.parse::<IpAddr>() {
            if listed.to_canonical() == address {
                return true;
            }
        } else if !entry.is_empty() && !entry.starts_with(|first: char| first.is_ascii_digit()) {
            // A host name stands for the addresses that it had a moment ago.
            if addresses_of(entry).await.contains(&address) {
                return true;
            }
        }
    }
    false
}

/// The addresses of a host name, as a lookup found them less than
/// [`REMEMBERED_FOR`] ago, or as a new lookup finds them. A lookup that fails or takes
/// too long finds none, and that is remembered too.
async fn addresses_of(name: &str) -> Vec<IpAddr> {
    if let Some(addresses) = remembered(name) {
        return addresses;
    }
    let lookup = tokio::time::timeout(LOOKUP_TIME, tokio::net::lookup_host((name, 0)));
    let addresses: Vec<IpAddr> = match lookup.await {
        Ok(Ok(found)) => found.map(|found| found.ip().to_canonical()).collect(),
        _ => Vec::new(),
    };
    let mut names = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    if names.len() >= MOST_REMEMBERED {
        names.retain(|_, (since, _)| now.duration_since(*since) < REMEMBERED_FOR);
        if names.len() >= MOST_REMEMBERED {
            names.clear();
        }
    }
    names.insert(name.to_ascii_lowercase(), (now, addresses.clone()));
    addresses
}

/// The addresses that a lookup of a host name found less than [`REMEMBERED_FOR`] ago.
fn remembered(name: &str) -> Option<Vec<IpAddr>> {
    let names = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
    names
        .get(&name.to_ascii_lowercase())
        .filter(|(since, _)| since.elapsed() < REMEMBERED_FOR)
        .map(|(_, addresses)| addresses.clone())
}

/// The answer that sends the browser to the login page of Organizr, for a proxy that
/// hands such an answer on to the browser. Traefik does.
fn to_the_login(state: &AppState, headers: &HeaderMap, client: &Client) -> Option<Response> {
    let settings = state.settings();
    let text = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    // Only a proxy that the administrator listed is believed about what it forwards.
    let from_traefik = client.via_trusted_proxy && text("x-forwarded-server") == Some("traefik");
    if !(from_traefik || settings.traefik_auth_enable) {
        return None;
    }

    let site = if settings.traefik_domain_override.is_empty() {
        request_site(headers, client)
    } else {
        settings.traefik_domain_override.clone()
    };
    let wanted = match (
        text("x-forwarded-proto"),
        text("x-forwarded-host"),
        text("x-forwarded-uri"),
    ) {
        (Some(scheme), Some(host), Some(path)) if client.via_trusted_proxy => {
            format!("?return={scheme}://{host}{path}")
        }
        _ => String::new(),
    };
    let location = HeaderValue::from_str(&format!("{site}{wanted}")).ok()?;
    // The PHP application sends this status, this type and this text.
    let mut response = (StatusCode::FOUND, "200").into_response();
    response.headers_mut().insert(header::LOCATION, location);
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=UTF-8"),
    );
    Some(response)
}

/// A header of the answer that tells the other application who the caller is. A value
/// that cannot be a header is left out.
fn tell(response: &mut Response, name: &'static str, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        response.headers_mut().insert(name, value);
    } else {
        tracing::warn!(name, "a value cannot be sent as a header of the answer");
    }
}

/// Every forward-auth route, for every method.
pub async fn check(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    uri: Uri,
    parts: RawPathParams,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiError> {
    let route = uri.path().strip_prefix(API_BASE).unwrap_or_default();
    let long_form = !route.starts_with("/auth-");
    let parts: Vec<String> = parts.iter().map(|(_, value)| value.to_owned()).collect();
    answer(
        &state,
        &caller,
        &client,
        &headers,
        long_form,
        &parts,
        query.as_deref(),
    )
    .await
}

/// Whether a trusted proxy wrote the query of the check itself: it sends the address
/// that the visitor asked for (`X-Forwarded-Uri`, as Caddy and Traefik do), and that
/// address has another query, so the query of the check is not the visitor's (D-157).
fn is_the_proxys(client: &Client, headers: &HeaderMap, query: &str) -> bool {
    if !client.via_trusted_proxy || query.is_empty() {
        return false;
    }
    headers
        .get("x-forwarded-uri")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|asked| asked.split_once('?').map_or("", |(_, rest)| rest) != query)
}

/// Answers the question of a request.
pub async fn answer(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    headers: &HeaderMap,
    long_form: bool,
    parts: &[String],
    query: Option<&str>,
) -> Result<Response, ApiError> {
    let refused = |message: String| Envelope::error(StatusCode::UNAUTHORIZED, message, NoData);
    if !state.database().install().is_done().await? {
        return Ok(
            refused("Organizr is not setup or an error occurred".to_owned()).into_response(),
        );
    }
    let query = query.unwrap_or_default();
    let question = match ask(
        long_form,
        parts,
        query,
        is_the_proxys(client, headers, query),
    ) {
        Ok(question) => question,
        Err(UnknownKind(kind)) => {
            return Ok(refused(format!("{kind} is not a valid type")).into_response());
        }
    };

    // A number is a level. Every other text is the name of a tab, and stands for the
    // level of that tab. A name that no tab has stands for the Admin.
    let requested = match level_of_number(&question.group) {
        Some(level) => level,
        None if question.group.is_empty() => 0,
        None => state
            .database()
            .tabs()
            .find_by_name(&question.group)
            .await?
            .map_or(0, |tab| i64::from(tab.access.least_access().get())),
    };

    let user = caller.user().map(|caller| &caller.authenticated.user);
    let (name, email) = match user {
        Some(user) => (user.username.clone(), user.email.clone()),
        None if caller.has_api_key => ("Organizr API".to_owned(), None),
        None => ("Guest".to_owned(), None),
    };
    let level = caller.level();
    let info = CallerInfo {
        user: name.clone(),
        group: level.get(),
        email: email.clone(),
        user_ip: client.address.to_string(),
        requested_group: requested,
        uuid: cookie_value(headers, DEVICE_COOKIE)
            .filter(|value| !value.is_empty())
            .unwrap_or("n/a")
            .to_owned(),
    };

    if let Some(list) = &question.allowed
        && lists(list, client.address).await
    {
        tracing::debug!(user = %name, client = %client.address, "forward auth: the address is on the allow list");
        return Ok(Envelope::success(info)
            .with_message("User is whitelisted")
            .into_response());
    }
    if let Some(list) = &question.denied
        && lists(list, client.address).await
    {
        tracing::debug!(user = %name, client = %client.address, "forward auth: the address is on the deny list");
        return Ok(
            Envelope::error(StatusCode::UNAUTHORIZED, "User is blacklisted", info).into_response(),
        );
    }

    let locked = user.is_some_and(|user| user.locked);
    let passes = i64::from(level.get()) <= requested && !locked;
    if !passes {
        tracing::debug!(user = %name, client = %client.address, requested, "forward auth: refused");
        if let Some(redirect) = to_the_login(state, headers, client) {
            return Ok(redirect);
        }
        return Ok(Envelope::error(
            StatusCode::UNAUTHORIZED,
            "User is not Authorized or User is locked",
            info,
        )
        .into_response());
    }

    tracing::debug!(user = %name, client = %client.address, requested, "forward auth: passed");
    let mut response = Envelope::success(info)
        .with_message("User is authorized")
        .into_response();
    tell(&mut response, "x-organizr-user", &name);
    tell(
        &mut response,
        "x-organizr-email",
        email.as_deref().unwrap_or_default(),
    );
    tell(&mut response, "x-organizr-group", &level.get().to_string());
    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use super::{
        Question, UnknownKind, ask, level_of_number, lists, percent_decoded, question, remembered,
    };

    fn parts(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_owned()).collect()
    }

    fn asked(long_form: bool, path: &[&str], query: &str) -> Question {
        question(long_form, &parts(path), query).expect("the kind of list is known")
    }

    #[test]
    fn numbers_are_levels_as_php_reads_them() {
        for (text, level) in [
            ("0", 0),
            ("999", 999),
            (" 4 ", 4),
            ("998.9", 998),
            ("1e3", 1000),
            ("-1", -1),
            ("+7", 7),
            (".5", 0),
            ("1e400", 0),
            ("-1e30", 0),
        ] {
            assert_eq!(level_of_number(text), Some(level), "{text:?}");
        }
        for text in ["", "Plex", "4a", "0x10", "inf", "NaN", "e", "-", "1 2"] {
            assert_eq!(level_of_number(text), None, "{text:?}");
        }
    }

    #[test]
    fn long_form_asks_about_the_deny_list_of_one_address_by_default() {
        assert_eq!(
            asked(true, &[], ""),
            Question {
                group: String::new(),
                allowed: None,
                denied: Some("192.0.0.0".to_owned()),
            }
        );
        assert_eq!(
            asked(true, &["4", "allow"], ""),
            Question {
                group: "4".to_owned(),
                allowed: Some("192.0.0.0".to_owned()),
                denied: None,
            }
        );
        let both = asked(
            true,
            &["4", "DENY", "10.0.0.1,10.0.0.2"],
            "whitelist=10.0.0.9&blacklist=x",
        );
        // With a group in the path, an allow list of the query does not count (D-136).
        assert_eq!(both.allowed, None);
        assert_eq!(both.denied.as_deref(), Some("10.0.0.1,10.0.0.2"));
    }

    #[test]
    fn short_form_takes_its_lists_from_the_query() {
        assert_eq!(
            asked(false, &["4"], ""),
            Question {
                group: "4".to_owned(),
                ..Question::default()
            }
        );
        // A deny list counts; an allow list does not, with a group in the path (D-136).
        let listed = asked(false, &["4"], "whitelist=10.0.0.1&blacklist=10.0.0.2");
        assert_eq!(listed.allowed, None);
        assert_eq!(listed.denied.as_deref(), Some("10.0.0.2"));
        // Without a group in the path, the query names both lists, as in PHP.
        let listed = asked(false, &[], "whitelist=10.0.0.1&blacklist=10.0.0.2");
        assert_eq!(listed.allowed.as_deref(), Some("10.0.0.1"));
        assert_eq!(listed.denied.as_deref(), Some("10.0.0.2"));
        // `type` and `ips` take the place of the list of their kind.
        let by_kind = asked(false, &[], "type=wl&ips=10.0.0.3&whitelist=10.0.0.1");
        assert_eq!(by_kind.allowed.as_deref(), Some("10.0.0.3"));
        // A kind without addresses, and the text `0`, are no list.
        assert_eq!(
            asked(false, &[], "type=allow&whitelist=10.0.0.1").allowed,
            None
        );
        assert_eq!(asked(false, &[], "whitelist=0&blacklist=").allowed, None);
        assert_eq!(asked(false, &["4"], "whitelist=0&blacklist=").denied, None);
    }

    #[test]
    fn query_cannot_loosen_the_question_of_a_path_with_a_group() {
        // A proxy such as Caddy sends the query of the visitor with the address of the
        // check, so with a group in the path the query may only add deny lists (D-136).
        for (long_form, path) in [
            (true, &["1"][..]),
            (false, &["1"][..]),
            (true, &["1", "deny", "10.0.0.5"][..]),
        ] {
            let asked = asked(
                long_form,
                path,
                "group=999&whitelist=10.0.0.1&type=allow&ips=10.0.0.2",
            );
            assert_eq!(asked.group, "1", "{path:?}");
            assert_eq!(asked.allowed, None, "{path:?}");
        }
        assert_eq!(
            asked(false, &["1"], "blacklist=10.0.0.3").denied.as_deref(),
            Some("10.0.0.3")
        );
        assert_eq!(
            asked(false, &["1"], "type=deny&ips=10.0.0.4")
                .denied
                .as_deref(),
            Some("10.0.0.4")
        );
        // The allow list of the path still counts.
        assert_eq!(
            asked(true, &["1", "allow", "10.0.0.6"], "whitelist=10.0.0.1")
                .allowed
                .as_deref(),
            Some("10.0.0.6")
        );
        // Without a group in the path, the query asks, as in PHP.
        let bare = asked(true, &[], "group=1&x");
        assert_eq!(bare.group, "1&x");
        assert_eq!(
            asked(false, &[], "whitelist=10.0.0.1").allowed.as_deref(),
            Some("10.0.0.1")
        );
    }

    #[test]
    fn allow_list_of_the_proxys_own_query_counts() {
        // A proxy that sends the address the visitor asked for, with another query,
        // wrote this query itself, so its allow list counts (D-157).
        for (long_form, path) in [(false, &["1"][..]), (true, &["1"][..])] {
            let own = ask(long_form, &parts(path), "whitelist=10.0.0.1", true).expect("known");
            assert_eq!(own.allowed.as_deref(), Some("10.0.0.1"), "{path:?}");
            // The group of the path still wins over the query.
            let group = ask(long_form, &parts(path), "group=999", true).expect("known");
            assert_eq!(group.group, "1", "{path:?}");
        }
        let by_kind = ask(false, &parts(&["1"]), "type=allow&ips=10.0.0.2", true).expect("known");
        assert_eq!(by_kind.allowed.as_deref(), Some("10.0.0.2"));
    }

    #[test]
    fn list_in_the_query_holds_addresses_only() {
        // A visitor can add a list to the query; its host names would make Organizr look
        // them up, so only addresses count there (D-141).
        let allowed = asked(
            false,
            &[],
            "whitelist=10.0.0.1,example.com,%20localhost,10.0.0.2",
        );
        assert_eq!(allowed.allowed.as_deref(), Some("10.0.0.1,10.0.0.2"));
        let denied = asked(false, &["1"], "type=deny&ips=nas.example.com");
        assert_eq!(denied.denied, None);
        // The path is the operator's, and its host names count.
        let path = asked(true, &["1", "allow", "nas.example.com,10.0.0.3"], "");
        assert_eq!(path.allowed.as_deref(), Some("nas.example.com,10.0.0.3"));
    }

    #[tokio::test]
    async fn list_looks_at_its_first_hundred_entries_and_remembers_names() {
        let address: IpAddr = "127.0.0.1".parse().expect("the address is valid");
        let mut long: Vec<String> = (1..=100)
            .map(|n| format!("10.9.{}.{}", n / 200, n % 200 + 1))
            .collect();
        long.push("127.0.0.1".to_owned());
        assert!(!lists(&long.join(","), address).await);
        assert!(lists("10.9.9.1,127.0.0.1", address).await);

        // A name is looked up once, and its addresses are remembered for a while.
        assert!(lists("localhost", address).await);
        assert!(remembered("LOCALHOST").is_some());
    }

    #[test]
    fn every_name_of_a_kind_of_list_is_known() {
        for kind in ["whitelist", "white", "w", "wl", "allow", "ALLOW"] {
            assert!(
                asked(true, &["4", kind, "10.0.0.1"], "").allowed.is_some(),
                "{kind}"
            );
        }
        for kind in ["blacklist", "black", "b", "bl", "deny", "Bl"] {
            assert!(
                asked(true, &["4", kind, "10.0.0.1"], "").denied.is_some(),
                "{kind}"
            );
        }
        assert_eq!(
            question(true, &parts(&["4", "other"]), ""),
            Err(UnknownKind("other".to_owned()))
        );
        assert_eq!(
            question(false, &parts(&["4"]), "type=Other"),
            Err(UnknownKind("Other".to_owned()))
        );
    }

    #[test]
    fn group_in_the_query_asks_in_the_short_form_with_the_rest_of_the_query() {
        assert_eq!(asked(true, &[], "group=999").group, "999");
        assert_eq!(asked(false, &[], "group=My%20Tab").group, "My Tab");
        // Everything after `group=` is the group, so it is the name of no tab.
        assert_eq!(
            asked(false, &[], "group=4&whitelist=10.0.0.1").group,
            "4&whitelist=10.0.0.1"
        );
        assert_eq!(asked(false, &[], "x=1&group=7/").group, "7");
        // PHP looks for `group=` in any letter case, and cuts at small letters only.
        assert_eq!(asked(false, &[], "GROUP=7").group, "");
        // A group in the path wins, and keeps its own list (D-136).
        let kept = asked(true, &["0", "allow", "10.0.0.1"], "group=4");
        assert_eq!(kept.group, "0");
        assert_eq!(kept.allowed.as_deref(), Some("10.0.0.1"));
        assert_eq!(asked(false, &["4"], "group=999").group, "4");
    }

    #[test]
    fn percent_parts_are_decoded_and_a_plus_stays() {
        assert_eq!(percent_decoded("My%20Tab+1%2f%ZZ%4"), "My Tab+1/%ZZ%4");
        assert_eq!(percent_decoded(""), "");
    }
}
