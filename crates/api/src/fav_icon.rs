//! The icons of the page from the setting `favIcon`.
//!
//! PHP writes the setting, HTML that a favicon generator made, into the head of the page
//! as it is. Here only its icon links are kept: `<link>` elements whose `rel` is `icon`,
//! `shortcut icon` or `apple-touch-icon`, and whose address is on this site or uses
//! `https`. The interface makes a link element for each one. No other HTML of the setting
//! reaches the page (D-93).

use serde::Serialize;

/// The most icon links that the page gets.
const MOST_LINKS: usize = 10;

/// The kinds of links that are kept.
const KEPT_RELS: [&str; 3] = ["icon", "shortcut icon", "apple-touch-icon"];

/// One icon link of the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IconLink {
    /// `icon`, `shortcut icon` or `apple-touch-icon`.
    pub rel: String,
    /// The address: a path of this site, or an address that uses `https`.
    pub href: String,
    /// The media type, for example `image/png`, when the link names one.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    /// The sizes, for example `32x32`, when the link names them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sizes: Option<String>,
}

/// The icon links in the HTML of the setting `favIcon`. Everything else is dropped.
pub fn icon_links(html: &str) -> Vec<IconLink> {
    let mut links = Vec::new();
    let mut rest = html;
    while let Some(start) = find_link_tag(rest) {
        let after = &rest[start + "<link".len()..];
        let (attributes, remainder) = read_attributes(after);
        rest = remainder;
        // A tag without its `>` is no element.
        let Some(attributes) = attributes else {
            break;
        };
        if let Some(link) = icon_link(&attributes) {
            links.push(link);
            if links.len() == MOST_LINKS {
                break;
            }
        }
    }
    links
}

/// Where the next `<link` tag starts, in any letter case.
fn find_link_tag(text: &str) -> Option<usize> {
    let lower = text.to_ascii_lowercase();
    let mut from = 0;
    while let Some(found) = lower.get(from..)?.find("<link") {
        let at = from + found;
        let next = lower.as_bytes().get(at + "<link".len()).copied();
        // `<linkx` is another tag.
        if next.is_none_or(|byte| byte.is_ascii_whitespace() || byte == b'/' || byte == b'>') {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

/// The attributes of a tag up to its `>`, with names in lower case, and the text after
/// the tag. `None` when the text ends before the `>`.
fn read_attributes(text: &str) -> (Option<Vec<(String, String)>>, &str) {
    let mut attributes = Vec::new();
    let mut chars = text.char_indices().peekable();
    loop {
        // Space and slashes between the attributes.
        while chars
            .next_if(|(_, letter)| letter.is_whitespace() || *letter == '/')
            .is_some()
        {}
        let Some(&(at, letter)) = chars.peek() else {
            return (None, "");
        };
        if letter == '>' {
            return (Some(attributes), &text[at + 1..]);
        }
        let mut name = String::new();
        while let Some((_, letter)) = chars.next_if(|(_, letter)| {
            !letter.is_whitespace() && !matches!(letter, '=' | '>' | '/' | '"' | '\'')
        }) {
            name.push(letter.to_ascii_lowercase());
        }
        if name.is_empty() {
            // A quote without a name: skip it.
            chars.next();
            continue;
        }
        while chars
            .next_if(|(_, letter)| letter.is_whitespace())
            .is_some()
        {}
        let mut value = String::new();
        if chars.next_if(|(_, letter)| *letter == '=').is_some() {
            while chars
                .next_if(|(_, letter)| letter.is_whitespace())
                .is_some()
            {}
            match chars.peek().map(|(_, letter)| *letter) {
                Some(quote @ ('"' | '\'')) => {
                    chars.next();
                    for (_, letter) in chars.by_ref() {
                        if letter == quote {
                            break;
                        }
                        value.push(letter);
                    }
                }
                _ => {
                    while let Some((_, letter)) =
                        chars.next_if(|(_, letter)| !letter.is_whitespace() && *letter != '>')
                    {
                        value.push(letter);
                    }
                }
            }
        }
        attributes.push((name, decode_entities(&value)));
    }
}

/// The few character references that an address or a name of a generator holds.
fn decode_entities(text: &str) -> String {
    text.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// The icon link that the attributes of a `<link>` tag make, when it is one to keep.
fn icon_link(attributes: &[(String, String)]) -> Option<IconLink> {
    let value = |name: &str| {
        attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    let rel = value("rel")?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if !KEPT_RELS.contains(&rel.as_str()) {
        return None;
    }
    let href = value("href")?.trim();
    if !is_allowed_address(href) {
        return None;
    }
    let media_type = value("type")
        .map(str::trim)
        .filter(|kind| is_media_type(kind))
        .map(str::to_owned);
    let sizes = value("sizes")
        .map(str::trim)
        .filter(|sizes| is_sizes(sizes))
        .map(str::to_owned);
    Some(IconLink {
        rel,
        href: href.to_owned(),
        media_type,
        sizes,
    })
}

/// Whether an address may be the address of an icon: a path of this site, or an address
/// that uses `https`.
fn is_allowed_address(href: &str) -> bool {
    if href.is_empty()
        || href.len() > 2048
        || href.contains('\\')
        || href
            .chars()
            .any(|letter| letter.is_control() || letter.is_whitespace())
    {
        return false;
    }
    if let Some(rest) = href.strip_prefix("https://") {
        // A host must follow.
        return rest
            .chars()
            .next()
            .is_some_and(|letter| letter.is_ascii_alphanumeric() || letter == '[');
    }
    // A path of this site: no scheme, and no address of another host (`//host`).
    let before_path = href.split(['/', '?', '#']).next().unwrap_or_default();
    !href.starts_with("//") && !before_path.contains(':')
}

/// Whether a text is a media type of a picture, such as `image/png`.
fn is_media_type(kind: &str) -> bool {
    kind.strip_prefix("image/").is_some_and(|rest| {
        !rest.is_empty()
            && rest.len() <= 40
            && rest
                .chars()
                .all(|letter| letter.is_ascii_alphanumeric() || matches!(letter, '.' | '+' | '-'))
    })
}

/// Whether a text is a value of `sizes`, such as `32x32` or `any`.
fn is_sizes(sizes: &str) -> bool {
    sizes.eq_ignore_ascii_case("any")
        || (!sizes.is_empty()
            && sizes.len() <= 40
            && sizes
                .chars()
                .all(|letter| letter.is_ascii_digit() || matches!(letter, 'x' | 'X' | ' ')))
}

#[cfg(test)]
mod tests {
    use super::{IconLink, icon_links};

    fn link(rel: &str, href: &str, media_type: Option<&str>, sizes: Option<&str>) -> IconLink {
        IconLink {
            rel: rel.to_owned(),
            href: href.to_owned(),
            media_type: media_type.map(str::to_owned),
            sizes: sizes.map(str::to_owned),
        }
    }

    #[test]
    fn output_of_a_favicon_generator_keeps_its_icon_links() {
        let html = r##"
            <link rel="apple-touch-icon" sizes="180x180" href="data/favicon/apple-touch-icon.png">
            <LINK REL="icon" type="image/png" sizes="32x32" href='data/favicon/favicon-32x32.png'/>
            <link rel="manifest" href="data/favicon/site.webmanifest">
            <link rel="mask-icon" href="data/favicon/safari-pinned-tab.svg" color="#5bbad5">
            <link rel=" Shortcut   Icon " href=/data/favicon/favicon.ico>
            <meta name="msapplication-TileColor" content="#da532c">
            <meta name="theme-color" content="#ffffff">
        "##;
        assert_eq!(
            icon_links(html),
            [
                link(
                    "apple-touch-icon",
                    "data/favicon/apple-touch-icon.png",
                    None,
                    Some("180x180")
                ),
                link(
                    "icon",
                    "data/favicon/favicon-32x32.png",
                    Some("image/png"),
                    Some("32x32")
                ),
                link("shortcut icon", "/data/favicon/favicon.ico", None, None),
            ]
        );
    }

    #[test]
    fn address_must_be_of_this_site_or_use_https() {
        let kept =
            icon_links(r#"<link rel="icon" href="https://cdn.example.com/icon.png?a=1&amp;b=2">"#);
        assert_eq!(kept[0].href, "https://cdn.example.com/icon.png?a=1&b=2");
        for href in [
            "http://example.com/icon.png",
            "//example.com/icon.png",
            "javascript:alert(1)",
            "data:image/png;base64,AAAA",
            "JaVaScRiPt:alert(1)",
            "https://",
            "icon .png",
            "\\\\server\\icon.png",
            "",
        ] {
            let html = format!(r#"<link rel="icon" href="{href}">"#);
            assert_eq!(icon_links(&html), [], "{href}");
        }
    }

    #[test]
    fn nothing_but_the_icon_links_comes_through() {
        let html = r#"<script>alert(1)</script><link rel="icon" href="x.png" onload="alert(1)" type="text/html" sizes="<b>"><style>body{}</style><linkx rel="icon" href="y.png"><img src=x onerror=alert(1)>"#;
        assert_eq!(icon_links(html), [link("icon", "x.png", None, None)]);
    }

    #[test]
    fn empty_setting_has_no_links() {
        assert_eq!(icon_links(""), []);
        assert_eq!(icon_links("<link"), []);
        assert_eq!(icon_links("<link rel=\"icon\" href=\"a.png"), []);
    }
}
