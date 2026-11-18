//! E-mail addresses, checked and cleaned as the PHP application does it.
//!
//! The PHP application checks an address with `FILTER_VALIDATE_EMAIL` and stores what
//! `FILTER_SANITIZE_EMAIL` leaves of it. A user who could register there must be able to
//! register here, and the other way round, so both rules are kept. The fixture
//! `tests/fixtures/php_emails.json` holds what real PHP says for a list of texts.
//!
//! One part is simpler than in PHP: an address in brackets in place of a host name
//! (`user@[192.0.2.1]`) is read with the address parser of the standard library. For the
//! forms that people use, the two agree.

use std::net::{Ipv4Addr, Ipv6Addr};

/// The longest text PHP looks at.
const LONGEST_TEXT: usize = 320;

/// The longest address, the longest name before the `@`, and the longest part of a host
/// name between two dots.
const LONGEST_ADDRESS: usize = 254;
const LONGEST_NAME: usize = 64;
const LONGEST_LABEL: usize = 63;

/// Cuts off what PHP's `trim()` cuts off: spaces, tabs, line ends, and zero bytes.
pub fn php_trim(text: &str) -> &str {
    text.trim_matches([' ', '\t', '\n', '\r', '\x0B', '\0'])
}

/// Whether PHP takes the text, after `trim()`, for an e-mail address.
pub fn is_valid(text: &str) -> bool {
    let address = php_trim(text);
    if address.len() > LONGEST_TEXT || address.len() > LONGEST_ADDRESS || !address.is_ascii() {
        return false;
    }
    let Some((name, host)) = split_at_the_name(address) else {
        return false;
    };
    name.len() <= LONGEST_NAME && (is_host_name(host) || is_bracketed_address(host))
}

/// What PHP stores for an address: the text after `trim()`, without any character that
/// is not a letter, a digit or one of ``!#$%&'*+-=?^_`{|}~@.[]``.
pub fn sanitize(text: &str) -> String {
    php_trim(text)
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || "!#$%&'*+-=?^_`{|}~@.[]".contains(*character)
        })
        .collect()
}

/// Splits an address into the name and the host, at the `@` that ends the name. The
/// name is a row of parts with dots between them. Each part is plain or in quotes.
fn split_at_the_name(address: &str) -> Option<(&str, &str)> {
    let bytes = address.as_bytes();
    let mut position = 0;
    loop {
        position = end_of_part(bytes, position)?;
        match bytes.get(position) {
            Some(b'.') => position += 1,
            Some(b'@') => return Some((address.get(..position)?, address.get(position + 1..)?)),
            _ => return None,
        }
    }
}

/// The position after the part of a name that starts at `start`, when there is one.
fn end_of_part(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) == Some(&b'"') {
        let mut position = start + 1;
        loop {
            match *bytes.get(position)? {
                b'"' => return Some(position + 1),
                // A backslash takes the next character with it, whatever it is.
                b'\\' => {
                    bytes.get(position + 1)?;
                    position += 2;
                }
                0x01..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x21 | 0x23..=0x5B | 0x5D..=0x7F => {
                    position += 1;
                }
                _ => return None,
            }
        }
    }

    let length = bytes
        .get(start..)?
        .iter()
        .take_while(|byte| is_plain_name_character(**byte))
        .count();
    (length > 0).then_some(start + length)
}

/// The characters of a part of a name that is not in quotes.
const fn is_plain_name_character(byte: u8) -> bool {
    matches!(byte, b'!' | b'#'..=b'\'' | b'*' | b'+' | b'-' | b'/'..=b'9' | b'=' | b'?' | b'A'..=b'Z' | b'^'..=b'~')
}

/// Whether the text is a host name with at least one dot, as PHP wants it.
fn is_host_name(host: &str) -> bool {
    let labels: Vec<&str> = host.split('.').collect();
    let Some((last, others)) = labels.split_last() else {
        return false;
    };
    let is_label = |label: &&str| {
        !label.is_empty()
            && label.len() <= LONGEST_LABEL
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    };
    // The last label starts with a letter. `xn--` starts with one, too.
    let starts_with_a_letter = last
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic());

    !others.is_empty() && others.iter().all(is_label) && is_label(last) && starts_with_a_letter
}

/// Whether the text is an address in brackets: `[192.0.2.1]` or `[IPv6:2001:db8::1]`.
fn is_bracketed_address(host: &str) -> bool {
    let Some(inner) = host
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return false;
    };
    let prefix = inner.get(..5).unwrap_or_default();
    if prefix.eq_ignore_ascii_case("IPv6:") {
        inner
            .get(5..)
            .is_some_and(|address| address.parse::<Ipv6Addr>().is_ok())
    } else {
        inner.parse::<Ipv4Addr>().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{is_valid, php_trim, sanitize};

    #[test]
    fn trim_cuts_off_what_php_cuts_off() {
        assert_eq!(php_trim(" \t\n\r\x0B\0a b\0\x0B\r\n\t "), "a b");
        assert_eq!(php_trim(""), "");
        assert_eq!(php_trim("  "), "");
        // A no-break space is not cut off.
        assert_eq!(php_trim("\u{a0}a"), "\u{a0}a");
    }

    #[test]
    fn longest_address_has_254_characters() {
        let name = "a".repeat(64);
        let host = |last: usize| {
            format!(
                "{}.{}.{}.com",
                "b".repeat(63),
                "c".repeat(63),
                "d".repeat(last)
            )
        };

        let longest = format!("{name}@{}", host(57));
        let too_long = format!("{name}@{}", host(58));

        assert_eq!((longest.len(), too_long.len()), (254, 255));
        assert!(is_valid(&longest));
        assert!(!is_valid(&too_long));
    }

    #[test]
    fn clean_up_keeps_a_normal_address_as_it_is() {
        for address in ["admin@example.com", "First.Last+tag@sub.example.co.uk"] {
            assert_eq!(sanitize(address), address);
        }
        assert_eq!(sanitize("  Yan@Example.com \n"), "Yan@Example.com");
    }
}
