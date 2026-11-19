//! The address of a tab, completed as the PHP application completes it.
//!
//! Before the PHP application sends the address of a tab to the browser, it takes the
//! address apart with PHP's `parse_url()` and puts it together again. An address without
//! a scheme gets `http://`, a user name and a password are left out, and a path such as
//! `/plex` gets the host of Organizr in front. `parse_url()` has rules of its own for
//! text that is no clean address, and stored addresses were written by hand, so this
//! module follows those rules step by step. `tests/php_tab_urls.rs` compares it with
//! what real PHP gives.

/// The parts of an address that the PHP application puts together again. `parse_url()`
/// also finds a user name and a password, which are not used.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Parts {
    scheme: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    path: Option<String>,
    query: Option<String>,
    fragment: Option<String>,
}

/// Where `parse_url()` goes on after it has looked for a scheme.
enum Next {
    /// A port may follow the colon at this place.
    Port(usize),
    /// A host follows.
    Host,
    /// Only a path, a query and a fragment follow.
    Path,
}

/// The bytes of an address, with the reads that `parse_url()` does on them.
struct Text<'a> {
    text: &'a str,
}

impl Text<'_> {
    fn len(&self) -> usize {
        self.text.len()
    }

    fn at(&self, index: usize) -> Option<u8> {
        self.text.as_bytes().get(index).copied()
    }

    fn is_digit(&self, index: usize) -> bool {
        self.at(index).is_some_and(|byte| byte.is_ascii_digit())
    }

    /// Whether two slashes stand at `index`.
    fn has_slashes_at(&self, index: usize) -> bool {
        self.at(index) == Some(b'/') && self.at(index + 1) == Some(b'/')
    }

    /// The place of the first of `wanted` at or after `from` and before `end`.
    fn find(&self, from: usize, end: usize, wanted: &[u8]) -> Option<usize> {
        (from..end.min(self.len()))
            .find(|index| self.at(*index).is_some_and(|b| wanted.contains(&b)))
    }

    /// The place of the last `wanted` at or after `from` and before `end`.
    fn find_last(&self, from: usize, end: usize, wanted: u8) -> Option<usize> {
        (from..end.min(self.len()))
            .rev()
            .find(|index| self.at(*index) == Some(wanted))
    }

    /// The text from `from` to `end`, with each control character replaced by `_`, as
    /// PHP stores a part. Every place that `parse_url()` cuts at holds an ASCII
    /// character, so a cut never falls inside a character.
    fn part(&self, from: usize, end: usize) -> String {
        self.text
            .get(from..end)
            .unwrap_or_default()
            .chars()
            .map(|character| {
                if character.is_ascii_control() {
                    '_'
                } else {
                    character
                }
            })
            .collect()
    }
}

/// What C's `strtol` reads from the start of a text: space, a sign, and digits. `None`
/// when there is no digit. A number that is too large for the type counts as too large
/// for a port.
fn leading_number(text: &str) -> Option<i64> {
    let text = text.trim_start_matches([' ', '\t', '\n', '\x0B', '\x0C', '\r']);
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let end = digits
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(digits.len());
    let digits = digits.get(..end).filter(|digits| !digits.is_empty())?;
    let number = digits.parse::<i64>().unwrap_or(i64::MAX);
    Some(if negative { -number } else { number })
}

/// The port in a text, when the number that the text starts with is one.
fn port_of(text: &str) -> Option<u16> {
    leading_number(text).and_then(|number| u16::try_from(number).ok())
}

/// What a stage of `parse_url()` leads to.
enum Step {
    /// The next stage.
    Go(Next),
    /// The address is taken apart: nothing follows.
    Done,
}

/// The first stage: is there a scheme? `start` is moved past what was read.
fn scheme_stage(text: &Text<'_>, parts: &mut Parts, start: &mut usize) -> Step {
    let end = text.len();
    let Some(colon) = text.find(0, end, b":") else {
        return Step::Go(if text.has_slashes_at(0) {
            *start = 2;
            Next::Host
        } else {
            Next::Path
        });
    };
    if colon == 0 {
        // The text starts with a colon.
        return Step::Go(Next::Port(colon));
    }

    let is_scheme = (0..colon).all(|index| {
        text.at(index)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'))
    });
    if !is_scheme {
        let end_of_path = text.find(0, end, b"?#").unwrap_or(end);
        return Step::Go(if colon + 1 < end && colon < end_of_path {
            Next::Port(colon)
        } else if text.has_slashes_at(0) {
            *start = 2;
            Next::Host
        } else {
            Next::Path
        });
    }
    if colon + 1 == end {
        // Only a scheme.
        parts.scheme = Some(text.part(0, colon));
        return Step::Done;
    }
    if text.at(colon + 1) != Some(b'/') {
        // A scheme such as `mailto:` has no slash. But `a.com:80` is a host and a port:
        // up to five digits, and then the end or a slash.
        let mut after_digits = colon + 1;
        while text.is_digit(after_digits) {
            after_digits += 1;
        }
        let ends = after_digits == end || text.at(after_digits) == Some(b'/');
        if ends && after_digits - colon < 7 {
            return Step::Go(Next::Port(colon));
        }
        parts.scheme = Some(text.part(0, colon));
        *start = colon + 1;
        return Step::Go(Next::Path);
    }

    let scheme = text.part(0, colon);
    let is_file = scheme.eq_ignore_ascii_case("file");
    parts.scheme = Some(scheme);
    if text.at(colon + 2) != Some(b'/') {
        *start = colon + 1;
        return Step::Go(Next::Path);
    }
    *start = colon + 3;
    if is_file && text.at(colon + 3) == Some(b'/') {
        // A drive letter of Windows, as in `file:///c:/dir/file.txt`.
        if text.at(colon + 5) == Some(b':') {
            *start = colon + 4;
        }
        return Step::Go(Next::Path);
    }
    Step::Go(Next::Host)
}

/// The stage for a port that may follow the colon at `colon`, before the host is read.
/// `None` stands for the `false` of PHP.
fn port_stage(text: &Text<'_>, parts: &mut Parts, start: &mut usize, colon: usize) -> Option<Next> {
    let end = text.len();
    let first = colon + 1;
    let mut after_digits = first;
    while after_digits - first < 6 && text.is_digit(after_digits) {
        after_digits += 1;
    }
    let digits = after_digits - first;
    let ends = after_digits == end || text.at(after_digits) == Some(b'/');
    if digits > 0 && digits < 6 && ends {
        parts.port = Some(port_of(&text.part(first, after_digits))?);
    } else if digits == 0 && after_digits == end {
        return None;
    } else if !text.has_slashes_at(*start) {
        return Some(Next::Path);
    }
    if text.has_slashes_at(*start) {
        *start += 2;
    }
    Some(Next::Host)
}

/// The stage for the host, with a port after it. `None` stands for the `false` of PHP.
/// Returns whether a path follows.
fn host_stage(text: &Text<'_>, parts: &mut Parts, start: &mut usize) -> Option<bool> {
    let end = text.len();
    let host_end = text.find(*start, end, b"/?#").unwrap_or(end);
    // A user name and a password stand before the last `@`.
    if let Some(at_sign) = text.find_last(*start, host_end, b'@') {
        *start = at_sign + 1;
    }
    // An address of IPv6 in brackets holds colons that start no port.
    let in_brackets =
        text.at(*start) == Some(b'[') && host_end > 0 && text.at(host_end - 1) == Some(b']');
    let colon = if in_brackets {
        None
    } else {
        text.find_last(*start, host_end, b':')
    };
    let host_end_without_port = match colon {
        // PHP looks for a port here only when it has none yet.
        Some(colon) if parts.port.is_none() => {
            let first = colon + 1;
            if host_end - first > 5 {
                return None;
            }
            if host_end > first {
                parts.port = Some(port_of(&text.part(first, host_end))?);
            }
            colon
        }
        Some(colon) => colon,
        None => host_end,
    };
    if host_end_without_port <= *start {
        return None;
    }
    parts.host = Some(text.part(*start, host_end_without_port));
    *start = host_end;
    Some(host_end != end)
}

/// The last stage: the path, the query and the fragment.
fn path_stage(text: &Text<'_>, parts: &mut Parts, start: usize) {
    let end = text.len();
    let mut path_end = end;
    if let Some(hash) = text.find(start, path_end, b"#") {
        parts.fragment = Some(text.part(hash + 1, path_end));
        path_end = hash;
    }
    if let Some(question_mark) = text.find(start, path_end, b"?") {
        parts.query = Some(text.part(question_mark + 1, path_end));
        path_end = question_mark;
    }
    if start < path_end || start == end {
        parts.path = Some(text.part(start, path_end));
    }
}

/// Takes an address apart as PHP's `parse_url()` does. `None` stands for the `false`
/// that PHP returns for text it does not take for an address.
fn parse(address: &str) -> Option<Parts> {
    let text = Text { text: address };
    let mut parts = Parts::default();
    let mut start = 0;

    let next = match scheme_stage(&text, &mut parts, &mut start) {
        Step::Done => return Some(parts),
        Step::Go(Next::Port(colon)) => port_stage(&text, &mut parts, &mut start, colon)?,
        Step::Go(next) => next,
    };
    let has_path = match next {
        Next::Host => host_stage(&text, &mut parts, &mut start)?,
        Next::Path | Next::Port(_) => true,
    };
    if has_path {
        path_stage(&text, &mut parts, start);
    }
    Some(parts)
}

/// The host and the port of an address to check, as `formatPingHost()` of the PHP
/// application reads them. Text without a scheme and without a port is a host, up to
/// its first slash. Returns `None` for text that names no host.
pub fn host_and_port(address: &str) -> Option<(String, Option<u16>)> {
    let parts = parse(address)?;
    let host = match parts.host {
        Some(host) => host,
        None => parts.path?.split('/').next()?.to_owned(),
    };
    (!host.is_empty()).then_some((host, parts.port))
}

/// Completes the address of a tab as the PHP application does before it sends the
/// address to the browser (`qualifyURL`).
///
/// `server` is the host of Organizr as the browser named it, and `https` says whether
/// the browser reached Organizr over HTTPS. Both are used for an address that starts
/// with a slash, which is a path on the host of Organizr.
pub fn qualify(address: &str, https: bool, server: &str) -> String {
    let on_this_host;
    let address = if address.starts_with('/') {
        let scheme = if https { "https" } else { "http" };
        on_this_host = format!("{scheme}://{server}{address}");
        &on_this_host
    } else {
        address
    };

    // For text that PHP does not take for an address, every part is empty.
    let parts = parse(address).unwrap_or_default();
    let mut qualified = parts.scheme.unwrap_or_else(|| "http".to_owned());
    qualified.push_str("://");
    qualified.push_str(parts.host.as_deref().unwrap_or_default());
    if let Some(port) = parts.port {
        qualified.push(':');
        qualified.push_str(&port.to_string());
    }
    qualified.push_str(parts.path.as_deref().unwrap_or_default());
    if let Some(query) = parts.query {
        qualified.push('?');
        qualified.push_str(&query);
    }
    if let Some(fragment) = parts.fragment {
        qualified.push('#');
        qualified.push_str(&fragment);
    }
    qualified
}

#[cfg(test)]
mod tests {
    use super::{Parts, leading_number, parse, qualify};

    #[test]
    fn address_is_taken_apart() {
        assert_eq!(
            parse("https://user:secret@plex.example.com:32400/web/index.html?x=1#top"),
            Some(Parts {
                scheme: Some("https".to_owned()),
                host: Some("plex.example.com".to_owned()),
                port: Some(32400),
                path: Some("/web/index.html".to_owned()),
                query: Some("x=1".to_owned()),
                fragment: Some("top".to_owned()),
            })
        );
        assert_eq!(
            parse("plex.example.com:8080"),
            Some(Parts {
                host: Some("plex.example.com".to_owned()),
                port: Some(8080),
                ..Parts::default()
            })
        );
        assert_eq!(
            parse("plex/app"),
            Some(Parts {
                path: Some("plex/app".to_owned()),
                ..Parts::default()
            })
        );
        assert_eq!(parse("http:///x"), None);
        assert_eq!(parse("a.com:99999"), None);
    }

    #[test]
    fn number_is_read_from_the_start_of_a_text_as_strtol_reads_it() {
        assert_eq!(leading_number("80"), Some(80));
        assert_eq!(leading_number("080"), Some(80));
        assert_eq!(leading_number("80a"), Some(80));
        assert_eq!(leading_number(" +80"), Some(80));
        assert_eq!(leading_number("-1"), Some(-1));
        assert_eq!(leading_number(""), None);
        assert_eq!(leading_number("a80"), None);
        assert_eq!(leading_number("-"), None);
    }

    #[test]
    fn address_is_completed() {
        let qualify = |address| qualify(address, false, "organizr.example.com");

        assert_eq!(
            qualify("https://plex.example.com/"),
            "https://plex.example.com/"
        );
        assert_eq!(
            qualify("plex.example.com/web"),
            "http://plex.example.com/web"
        );
        assert_eq!(qualify("/plex"), "http://organizr.example.com/plex");
        assert_eq!(
            qualify("https://user:secret@plex.example.com/"),
            "https://plex.example.com/"
        );
        assert_eq!(qualify("http:///x"), "http://");
    }

    #[test]
    fn path_on_this_host_gets_the_scheme_of_the_request() {
        assert_eq!(
            qualify("/plex?x=1", true, "organizr.example.com"),
            "https://organizr.example.com/plex?x=1"
        );
    }
}
