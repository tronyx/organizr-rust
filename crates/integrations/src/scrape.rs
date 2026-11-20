//! One page of the wide web, fetched for a caller who names its address.
//!
//! The address comes from a request, so the fetch has rules: the host is reached at
//! the one address that the caller of this module checked, no redirect is followed,
//! the certificate is checked, and the page has a largest size.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::http::{Outbound, without_address};

/// How long the server of a page has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(15);

/// The largest page, in bytes.
pub const LARGEST_PAGE: usize = 2 * 1024 * 1024;

/// Why a page was not fetched.
#[derive(Debug, thiserror::Error)]
pub enum ScrapeError {
    /// The server cannot be reached. The text names no address.
    #[error("{0}")]
    Unreachable(String),
    /// The page is larger than [`LARGEST_PAGE`].
    #[error("The page is larger than {LARGEST_PAGE} bytes")]
    TooLarge,
}

/// What the server of a page answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The HTTP status.
    pub status: u16,
    /// The body as text.
    pub body: String,
}

/// The scheme, the host and the port of an address, when it is an address of a page:
/// `http` or `https`, with a host.
pub fn target(url: &str) -> Option<(String, u16)> {
    let parsed = reqwest::Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let host = parsed.host_str()?.trim_matches(['[', ']']).to_owned();
    Some((host, parsed.port_or_known_default()?))
}

fn is_public_v4(address: Ipv4Addr) -> bool {
    let [first, second, ..] = address.octets();
    !(address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_unspecified()
        || address.is_broadcast()
        || address.is_multicast()
        // The addresses that an internet provider shares between its customers.
        || (first == 100 && (64..128).contains(&second))
        || first == 0)
}

fn is_public_v6(address: Ipv6Addr) -> bool {
    if let Some(inner) = address.to_ipv4_mapped() {
        return is_public_v4(inner);
    }
    let first = address.segments()[0];
    !(address.is_loopback()
        || address.is_unspecified()
        || address.is_multicast()
        // Unique local addresses, and link-local addresses.
        || first & 0xfe00 == 0xfc00
        || first & 0xffc0 == 0xfe80)
}

/// Whether an address is one of the wide web: not of this computer, and not of a
/// private or a link-local network.
pub fn is_public(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_v4(address),
        IpAddr::V6(address) => is_public_v6(address),
    }
}

/// The parts of an address, as `qualifyURL` of the PHP application lists them.
pub fn parts(url: &str) -> Value {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return Value::Null;
    };
    let after_host = url.split_once("://").map_or("", |(_, rest)| rest);
    // An address without a path has none, also when the parser gives it a slash.
    let path = if after_host.contains('/') {
        parsed.path()
    } else {
        ""
    };
    let host = parsed.host_str().unwrap_or_default();
    let mut digest = Map::new();
    digest.insert("scheme".to_owned(), json!(parsed.scheme()));
    digest.insert("host".to_owned(), json!(host));
    if let Some(port) = parsed.port() {
        digest.insert("port".to_owned(), json!(port));
    }
    if !path.is_empty() {
        digest.insert("path".to_owned(), json!(path));
    }
    if let Some(query) = parsed.query() {
        digest.insert("query".to_owned(), json!(query));
    }
    if let Some(fragment) = parsed.fragment() {
        digest.insert("fragment".to_owned(), json!(fragment));
    }
    json!({
        "scheme": parsed.scheme(),
        "host": host,
        "port": parsed.port().map_or_else(String::new, |port| format!(":{port}")),
        "path": path,
        "query": parsed.query().map_or_else(String::new, |query| format!("?{query}")),
        "fragment": parsed.fragment().map_or_else(String::new, |fragment| format!("#{fragment}")),
        "digest": digest,
    })
}

/// Fetches a page. `host` is the host of the address, and `address` is where that
/// host is reached: the caller resolved the name and checked the address, and the
/// request goes to that address and to no other.
pub async fn fetch(
    http: &Outbound,
    url: &str,
    host: &str,
    address: SocketAddr,
) -> Result<Page, ScrapeError> {
    let unreachable = |error: reqwest::Error| ScrapeError::Unreachable(without_address(error));
    let client = http.pinned(host, address).map_err(unreachable)?;
    let mut response = client
        .get(url)
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(unreachable)?;
    let status = response.status().as_u16();
    let mut bytes = Vec::new();
    while let Some(piece) = response.chunk().await.map_err(unreachable)? {
        if bytes.len() + piece.len() > LARGEST_PAGE {
            return Err(ScrapeError::TooLarge);
        }
        bytes.extend_from_slice(&piece);
    }
    Ok(Page {
        status,
        body: String::from_utf8_lossy(&bytes).into_owned(),
    })
}

/// The entities with a name that are written as their character.
const NAMED: [(&str, char); 28] = [
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("nbsp", '\u{a0}'),
    ("copy", '\u{a9}'),
    ("reg", '\u{ae}'),
    ("trade", '\u{2122}'),
    ("hellip", '\u{2026}'),
    ("mdash", '\u{2014}'),
    ("ndash", '\u{2013}'),
    ("lsquo", '\u{2018}'),
    ("rsquo", '\u{2019}'),
    ("ldquo", '\u{201c}'),
    ("rdquo", '\u{201d}'),
    ("laquo", '\u{ab}'),
    ("raquo", '\u{bb}'),
    ("euro", '\u{20ac}'),
    ("pound", '\u{a3}'),
    ("yen", '\u{a5}'),
    ("cent", '\u{a2}'),
    ("sect", '\u{a7}'),
    ("deg", '\u{b0}'),
    ("middot", '\u{b7}'),
    ("bull", '\u{2022}'),
    ("times", '\u{d7}'),
    ("divide", '\u{f7}'),
    ("para", '\u{b6}'),
];

/// The character of the inside of an entity, for example of `amp` or `#39`.
fn entity(inside: &str) -> Option<char> {
    if let Some(number) = inside.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse().ok()?,
        };
        return char::from_u32(code).filter(|c| *c != '\0');
    }
    NAMED
        .iter()
        .find(|(name, _)| *name == inside)
        .map(|(_, character)| *character)
}

/// A text of HTML with its entities written as characters: the ones with a number,
/// and the common ones with a name. Every other entity stays as it is.
pub fn without_entities(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find('&') {
        let (before, from) = rest.split_at(start);
        out.push_str(before);
        let found = from
            .get(1..)
            .and_then(|after| {
                after
                    .find(';')
                    .filter(|end| *end <= 10)
                    .map(|end| (after, end))
            })
            .and_then(|(after, end)| Some((entity(after.get(..end)?)?, end)));
        if let Some((character, end)) = found {
            out.push(character);
            rest = from.get(end + 2..).unwrap_or_default();
        } else {
            out.push('&');
            rest = from.get(1..).unwrap_or_default();
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use serde_json::json;

    use super::{is_public, parts, target, without_entities};

    #[test]
    fn only_addresses_of_the_wide_web_are_public() {
        for address in [
            "127.0.0.1",
            "127.8.9.1",
            "10.0.0.5",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.1.10",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "255.255.255.255",
            "224.0.0.1",
            "::1",
            "::",
            "fe80::1",
            "fc00::1",
            "fd12:3456::1",
            "::ffff:127.0.0.1",
            "::ffff:192.168.1.10",
            "ff02::1",
        ] {
            let parsed: IpAddr = address.parse().expect("an address");
            assert!(!is_public(parsed), "{address}");
        }
        for address in [
            "93.184.216.34",
            "172.32.0.1",
            "100.128.0.1",
            "2606:2800:220:1::1",
            "::ffff:93.184.216.34",
        ] {
            let parsed: IpAddr = address.parse().expect("an address");
            assert!(is_public(parsed), "{address}");
        }
    }

    #[test]
    fn target_is_the_host_and_the_port_of_a_web_address() {
        assert_eq!(
            target("https://example.com/a?b=c"),
            Some(("example.com".to_owned(), 443))
        );
        assert_eq!(
            target("http://example.com:8080/"),
            Some(("example.com".to_owned(), 8080))
        );
        assert_eq!(target("http://[::1]:81/"), Some(("::1".to_owned(), 81)));
        assert_eq!(
            target("http://user:secret@10.0.0.5/"),
            Some(("10.0.0.5".to_owned(), 80))
        );
        for none in [
            "file:///etc/passwd",
            "ftp://example.com/",
            "example.com",
            "",
            "http://",
        ] {
            assert_eq!(target(none), None, "{none}");
        }
    }

    #[test]
    fn parts_of_an_address_read_as_the_php_application_lists_them() {
        assert_eq!(
            parts("https://example.com:8443/a/b?c=d#e"),
            json!({
                "scheme": "https", "host": "example.com", "port": ":8443", "path": "/a/b",
                "query": "?c=d", "fragment": "#e",
                "digest": {
                    "scheme": "https", "host": "example.com", "port": 8443, "path": "/a/b",
                    "query": "c=d", "fragment": "e",
                },
            })
        );
        assert_eq!(
            parts("http://example.com"),
            json!({
                "scheme": "http", "host": "example.com", "port": "", "path": "",
                "query": "", "fragment": "",
                "digest": {"scheme": "http", "host": "example.com"},
            })
        );
        assert_eq!(parts("no address"), json!(null));
    }

    #[test]
    fn entities_are_written_as_characters() {
        assert_eq!(
            without_entities("&lt;b&gt;Fish &amp; Chips&lt;/b&gt; &quot;hot&quot;"),
            "<b>Fish & Chips</b> \"hot\""
        );
        assert_eq!(
            without_entities("it&#39;s &#x41;&#X42; &copy; 2026"),
            "it's AB \u{a9} 2026"
        );
        // What is no entity stays as it is.
        assert_eq!(
            without_entities("a & b &unknown; &#zz; &amp &; 100&"),
            "a & b &unknown; &#zz; &amp &; 100&"
        );
        assert_eq!(without_entities("&amp;amp;"), "&amp;");
        assert_eq!(without_entities(""), "");
    }
}
