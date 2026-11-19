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

use std::net::IpAddr;
use std::time::Duration;

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

/// Reads the question from the path and the query. `long_form` says whether the path is
/// one of the long forms, and `parts` are the pieces of the path after the name of the
/// route.
fn question(long_form: bool, parts: &[String], query: &str) -> Result<Question, UnknownKind> {
    let mut group = parts.first().cloned();
    let mut kind = query_value(query, "type");
    let mut addresses = query_value(query, "ips");

    // The PHP application turns every request whose query holds `group=` into the short
    // form, with the rest of the query as the group.
    if query.to_ascii_lowercase().contains("group=") {
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

    let mut allowed = query_value(query, "whitelist");
    let mut denied = query_value(query, "blacklist");
    if let Some(kind) = kind {
        match kind.to_lowercase().as_str() {
            "whitelist" | "white" | "w" | "wl" | "allow" => allowed = addresses,
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
    for entry in list.split(',').map(str::trim) {
        if let Ok(listed) = entry.parse::<IpAddr>() {
            if listed.to_canonical() == address {
                return true;
            }
        } else if !entry.is_empty() && !entry.starts_with(|first: char| first.is_ascii_digit()) {
            // A host name stands for the addresses that it has at this moment.
            let lookup = tokio::time::timeout(LOOKUP_TIME, tokio::net::lookup_host((entry, 0)));
            if let Ok(Ok(found)) = lookup.await
                && found
                    .into_iter()
                    .any(|found| found.ip().to_canonical() == address)
            {
                return true;
            }
        }
    }
    false
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
    let question = match question(long_form, parts, query.unwrap_or_default()) {
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
    use super::{Question, UnknownKind, level_of_number, percent_decoded, question};

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
        assert_eq!(both.allowed.as_deref(), Some("10.0.0.9"));
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
        let listed = asked(false, &["4"], "whitelist=10.0.0.1&blacklist=10.0.0.2");
        assert_eq!(listed.allowed.as_deref(), Some("10.0.0.1"));
        assert_eq!(listed.denied.as_deref(), Some("10.0.0.2"));
        // `type` and `ips` take the place of the list of their kind.
        let by_kind = asked(false, &["4"], "type=wl&ips=10.0.0.3&whitelist=10.0.0.1");
        assert_eq!(by_kind.allowed.as_deref(), Some("10.0.0.3"));
        // A kind without addresses, and the text `0`, are no list.
        assert_eq!(
            asked(false, &["4"], "type=allow&whitelist=10.0.0.1").allowed,
            None
        );
        assert_eq!(asked(false, &["4"], "whitelist=0&blacklist=").allowed, None);
        assert_eq!(asked(false, &["4"], "whitelist=0&blacklist=").denied, None);
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
        // The group of the path does not count then, and neither does its list.
        let rewritten = asked(true, &["0", "allow", "10.0.0.1"], "group=4");
        assert_eq!(
            rewritten,
            Question {
                group: "4".to_owned(),
                ..Question::default()
            }
        );
        assert_eq!(asked(false, &["4"], "group=My%20Tab").group, "My Tab");
        // Everything after `group=` is the group, so it is the name of no tab.
        assert_eq!(
            asked(false, &["4"], "group=4&whitelist=10.0.0.1").group,
            "4&whitelist=10.0.0.1"
        );
        assert_eq!(asked(false, &["4"], "x=1&group=7/").group, "7");
        // PHP looks for `group=` in any letter case, and cuts at small letters only.
        assert_eq!(asked(false, &["4"], "GROUP=7").group, "");
    }

    #[test]
    fn percent_parts_are_decoded_and_a_plus_stays() {
        assert_eq!(percent_decoded("My%20Tab+1%2f%ZZ%4"), "My Tab+1/%ZZ%4");
        assert_eq!(percent_decoded(""), "");
    }
}
