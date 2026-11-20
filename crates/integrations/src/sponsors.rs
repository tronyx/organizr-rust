//! The people who give money to the PHP project Organizr: the members of its page at
//! Open Collective, and the sponsors on the GitHub page of its author.
//!
//! The PHP application shows them on its settings page. The addresses are those of the
//! PHP application.

use std::time::Duration;

use serde_json::{Value, json};

use crate::http::{Outbound, without_address};

/// The list of the members at Open Collective, as the PHP application asks for it.
pub const OPEN_COLLECTIVE: &str =
    "https://opencollective.com/organizr/members/users.json?limit=100&offset=0";

/// The GitHub page of the sponsors of the author of PHP Organizr.
pub const GITHUB_SPONSORS: &str = "https://github.com/sponsors/causefx";

/// How long a service has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(15);

/// Why a list could not be read.
#[derive(Debug, thiserror::Error)]
pub enum SponsorError {
    /// The service cannot be reached. The text names no address.
    #[error("{0}")]
    Unreachable(String),
    /// The service answered with a status that is no success.
    #[error("the service refused the request")]
    Refused,
}

async fn text_at(http: &Outbound, url: &str) -> Result<String, SponsorError> {
    let response = http
        .client_for(url)
        .get(url)
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(|error| SponsorError::Unreachable(without_address(error)))?;
    if !response.status().is_success() {
        return Err(SponsorError::Refused);
    }
    response
        .text()
        .await
        .map_err(|error| SponsorError::Unreachable(without_address(error)))
}

/// The members at Open Collective, as the service sends them, each with the field
/// `sortName`: the name in small letters. An answer that is no JSON list reads as
/// `null`, as in the PHP application.
pub async fn open_collective(http: &Outbound, url: &str) -> Result<Value, SponsorError> {
    let text = text_at(http, url).await?;
    Ok(with_sort_names(
        serde_json::from_str(&text).unwrap_or(Value::Null),
    ))
}

fn with_sort_names(mut list: Value) -> Value {
    if let Some(members) = list.as_array_mut() {
        for member in members.iter_mut().filter_map(Value::as_object_mut) {
            let name = member
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            member.insert("sortName".to_owned(), Value::from(name));
        }
    } else {
        return Value::Null;
    }
    list
}

/// The sponsors on the GitHub page, in the form of the members at Open Collective.
pub async fn github(http: &Outbound, url: &str) -> Result<Vec<Value>, SponsorError> {
    let page = text_at(http, url).await?;
    Ok(github_sponsors_in(&page))
}

/// Reads the sponsors of the page: each link inside the element with the id
/// `sponsors`, with the first `@name` of the link and the first address in it. The
/// PHP application reads the same parts with an HTML parser and two patterns.
pub fn github_sponsors_in(page: &str) -> Vec<Value> {
    let Some(region) = sponsors_region(page) else {
        return Vec::new();
    };
    links(region)
        .filter_map(|link| {
            let username = user_name_in(link)?;
            let image = address_in(link)?;
            Some(json!({
                "name": username,
                "sortName": username.to_ascii_lowercase(),
                "image": image.replace("s=60", "s=200"),
                "isActive": true,
                "type": "USER",
                "role": "BACKER",
            }))
        })
        .collect()
}

/// The inside of the `div` with the id `sponsors`.
fn sponsors_region(page: &str) -> Option<&str> {
    let lower = page.to_ascii_lowercase();
    let mut from = 0;
    let start = loop {
        let at = from + lower.get(from..)?.find("<div")?;
        let end = at + lower.get(at..)?.find('>')?;
        let tag = lower.get(at..end)?;
        if tag.contains("id=\"sponsors\"") || tag.contains("id='sponsors'") {
            break end + 1;
        }
        from = end;
    };
    let mut depth = 1_usize;
    let mut position = start;
    while depth > 0 {
        let rest = lower.get(position..)?;
        let open = rest.find("<div");
        let close = rest.find("</div")?;
        match open {
            Some(open) if open < close => {
                depth += 1;
                position += open + 4;
            }
            _ => {
                depth -= 1;
                position += close + 5;
            }
        }
    }
    page.get(start..position.saturating_sub(5))
}

/// The insides of the links of a part of a page.
fn links(region: &str) -> impl Iterator<Item = &str> {
    let lower = region.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut position = 0;
    while let Some(at) = lower.get(position..).and_then(|rest| rest.find("<a")) {
        let tag_start = position + at;
        let next = lower.as_bytes().get(tag_start + 2).copied();
        if !matches!(next, Some(b' ' | b'>' | b'\t' | b'\n' | b'\r')) {
            position = tag_start + 2;
            continue;
        }
        let Some(inside) = lower
            .get(tag_start..)
            .and_then(|rest| rest.find('>'))
            .map(|end| tag_start + end + 1)
        else {
            break;
        };
        let Some(end) = lower
            .get(inside..)
            .and_then(|rest| rest.find("</a"))
            .map(|end| inside + end)
        else {
            break;
        };
        if let Some(text) = region.get(inside..end) {
            found.push(text);
        }
        position = end;
    }
    found.into_iter()
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The first `@`, a letter and more word characters, without the `@`.
fn user_name_in(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    bytes.iter().enumerate().find_map(|(at, byte)| {
        if *byte != b'@' || !bytes.get(at + 1)?.is_ascii_alphabetic() {
            return None;
        }
        let length = bytes
            .get(at + 2..)?
            .iter()
            .take_while(|b| is_word(**b))
            .count();
        (length > 0).then(|| text.get(at + 1..at + 2 + length))?
    })
}

/// The first address that starts with `http://` or `https://`, up to a space, a quote
/// or an angle bracket, without punctuation at its end.
fn address_in(text: &str) -> Option<&str> {
    let lower = text.to_ascii_lowercase();
    let start = lower
        .find("http://")
        .into_iter()
        .chain(lower.find("https://"))
        .min()?;
    let rest = text.get(start..)?;
    let length = rest
        .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '(' | ')'))
        .unwrap_or(rest.len());
    let address = rest
        .get(..length)?
        .trim_end_matches(|c: char| "`!()[]{};:'\".,<>?«»“”‘’".contains(c));
    (address.len() > "https://".len()).then_some(address)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{github_sponsors_in, with_sort_names};

    #[test]
    fn members_get_their_name_in_small_letters() {
        assert_eq!(
            with_sort_names(json!([{"name": "Robin Hood"}, {"name": null}, 3])),
            json!([{"name": "Robin Hood", "sortName": "robin hood"}, {"name": null, "sortName": ""}, 3])
        );
        assert_eq!(with_sort_names(json!({"error": "no"})), json!(null));
    }

    #[test]
    fn sponsors_are_read_from_the_links_of_their_part_of_the_page() {
        let page = r#"<html><body>
<a href="/elsewhere"><img src="https://example.com/a.png" alt="@outside"></a>
<div class="list" id="sponsors">
  <div class="row">
    <a href="/robin" class="x"><img src="https://avatars.example.com/u/1?s=60&amp;v=4" alt="@Robin_Hood"></a>
    <a href="/nobody"><img src="https://avatars.example.com/u/2?s=60" alt="no name"></a>
  </div>
  <A HREF="/pat"><img src='https://avatars.example.com/u/3?s=60' alt="@pat"></A>
</div>
<a href="/after"><img src="https://example.com/b.png" alt="@after"></a>
</body></html>"#;
        assert_eq!(
            github_sponsors_in(page),
            vec![
                json!({"name": "Robin_Hood", "sortName": "robin_hood", "image": "https://avatars.example.com/u/1?s=200&amp;v=4", "isActive": true, "type": "USER", "role": "BACKER"}),
                json!({"name": "pat", "sortName": "pat", "image": "https://avatars.example.com/u/3?s=200", "isActive": true, "type": "USER", "role": "BACKER"}),
            ]
        );
        let nothing: Vec<serde_json::Value> = Vec::new();
        assert_eq!(
            github_sponsors_in("<div id=\"other\"><a>@x1 https://e.com/</a></div>"),
            nothing
        );
        assert_eq!(github_sponsors_in("<div id=\"sponsors\"><a>@x1"), nothing);
    }
}
