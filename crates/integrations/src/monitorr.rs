//! Monitorr: the services that a Monitorr install watches, read from the HTML that
//! its page loads.
//!
//! Monitorr has no API. Its page asks `assets/php/loop.php` for a piece of HTML with
//! one tile for each service, and the PHP application reads that HTML with regular
//! expressions. This module reads it with a scan for the same marks: the picture, the
//! title and the state of a tile, in this order.

use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::http::{Outbound, without_address};

/// How long Monitorr has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// The largest picture of a service that is passed on, in bytes.
const LARGEST_PICTURE: usize = 512 * 1024;

/// Why the services could not be read.
#[derive(Debug, thiserror::Error)]
pub enum MonitorrError {
    /// Monitorr cannot be reached, or it did not answer in time. The text names no
    /// address.
    #[error("Monitorr cannot be reached: {0}")]
    Unreachable(String),
    /// Monitorr answered with a status that is no success.
    #[error("Monitorr answered with the status {0}")]
    Status(u16),
}

/// The state of a service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The service answers.
    Online,
    /// The service does not answer.
    Offline,
    /// The service answers slowly or not every time.
    Unresponsive,
}

/// One service of the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    /// The name on the tile.
    pub name: String,
    /// The state.
    pub state: State,
    /// The path of the picture under `assets` of Monitorr, when the tile has one.
    pub picture: Option<String>,
    /// The address that the tile opens, when it has one.
    pub link: Option<String>,
}

/// An install of Monitorr, and how it is asked.
#[derive(Debug, Clone, Copy)]
pub struct Install<'a> {
    /// The address, complete.
    pub url: &'a str,
    /// How long the install has to answer.
    pub timeout: Option<Duration>,
    /// Whether the certificate of the install is left unchecked.
    pub never_check_certificate: bool,
}

impl Install<'_> {
    fn get(&self, http: &Outbound, url: &str) -> reqwest::RequestBuilder {
        http.client(url, self.never_check_certificate)
            .get(url)
            .timeout(self.timeout.unwrap_or(TIME_LIMIT))
    }

    /// The HTML with the tiles of the services.
    pub async fn page(&self, http: &Outbound) -> Result<String, MonitorrError> {
        let url = format!("{}/assets/php/loop.php", self.url.trim_end_matches('/'));
        let unreachable = |error| MonitorrError::Unreachable(without_address(error));
        let response = self.get(http, &url).send().await.map_err(unreachable)?;
        if !response.status().is_success() {
            return Err(MonitorrError::Status(response.status().as_u16()));
        }
        response.text().await.map_err(unreachable)
    }

    /// The picture of a service as a `data:` address, or nothing when it cannot be
    /// loaded, when it is too large, or when its name does not end like a picture.
    pub async fn picture(&self, http: &Outbound, path: &str) -> Option<String> {
        let kind = picture_kind(path)?;
        let url = format!("{}/assets{path}", self.url.trim_end_matches('/'));
        let mut response = self.get(http, &url).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }
        let mut bytes = Vec::new();
        while let Some(piece) = response.chunk().await.ok()? {
            if bytes.len() + piece.len() > LARGEST_PICTURE {
                return None;
            }
            bytes.extend_from_slice(&piece);
        }
        Some(format!(
            "data:image/{kind};base64,{}",
            STANDARD.encode(bytes)
        ))
    }
}

/// The kind of a picture for a `data:` address, from the end of its name.
fn picture_kind(path: &str) -> Option<String> {
    let (_, ending) = path.rsplit_once('.')?;
    let ending = ending.to_ascii_lowercase();
    if ending.is_empty() || ending.len() > 5 || !ending.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(if ending == "svg" {
        "svg+xml".to_owned()
    } else {
        ending
    })
}

/// A text of the HTML with its most common character references read.
fn decoded(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((before, after)) = rest.split_once('&') {
        result.push_str(before);
        let reference = after
            .split_once(';')
            .filter(|(name, _)| name.len() <= 8)
            .and_then(|(name, tail)| {
                let character = match name {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    "nbsp" => '\u{a0}',
                    _ => {
                        let number = name.strip_prefix('#')?;
                        let code = match number.strip_prefix(['x', 'X']) {
                            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                            None => number.parse().ok()?,
                        };
                        char::from_u32(code)?
                    }
                };
                Some((character, tail))
            });
        if let Some((character, tail)) = reference {
            result.push(character);
            rest = tail;
        } else {
            result.push('&');
            rest = after;
        }
    }
    result.push_str(rest);
    result
}

/// The start of every title of a tile.
const TITLE: &str = "<div id=\"servicetitle";

/// The name and the state of the tile whose title starts before `after`, and the HTML
/// after the tile.
fn title_and_state(after: &str) -> Option<(&str, State, &str)> {
    // The three kinds of title: with a link, without one, and of a service that is
    // offline.
    let after = ["offline\"", "nolink\"", "\""]
        .iter()
        .find_map(|ending| after.strip_prefix(ending))?;
    let no_tag = |text: &str| !text.contains(['<', '>']);
    let (attributes, after) = after.split_once("><div>")?;
    let (name, after) = after.split_once("</div></div>")?;
    if !no_tag(attributes) || !no_tag(name) || name.is_empty() {
        return None;
    }
    let after = after.strip_prefix("<div class=\"btn")?;
    let (button, after) = after.split_once('"')?;
    if !["online", "offline", "unknown"].contains(&button) {
        return None;
    }
    let (attributes, after) = after.split_once('>')?;
    let (state, after) = after.split_once("</div>")?;
    if !no_tag(attributes) {
        return None;
    }
    let state = match state {
        "Online" => State::Online,
        "Offline" => State::Offline,
        "Unresponsive" => State::Unresponsive,
        _ => return None,
    };
    Some((name, state, after))
}

/// The text after the last `mark` in `html`, up to `end`.
fn last_between<'a>(html: &'a str, mark: &str, end: &str) -> Option<&'a str> {
    let (_, after) = html.rsplit_once(mark)?;
    let (found, _) = after.split_once(end)?;
    (!found.contains(['<', '>', '"']) && !found.contains(char::is_whitespace)).then_some(found)
}

/// The services of the HTML, in the order of the page. Two tiles with one name are two
/// services here.
pub fn services(html: &str) -> Vec<Service> {
    let mut found = Vec::new();
    let mut rest = html;
    while let Some((before, after)) = rest.split_once(TITLE) {
        let Some((name, state, tail)) = title_and_state(after) else {
            rest = after;
            continue;
        };
        // The picture and the link of a tile stand before its title.
        found.push(Service {
            name: decoded(name),
            state,
            picture: last_between(before, "assets/img/..", "\" class=\"serviceimg")
                .filter(|path| !path.is_empty())
                .map(decoded),
            link: last_between(before, "<a class=\"servicetile\" href=\"", "\"")
                .filter(|link| !link.is_empty())
                .map(decoded),
        });
        rest = tail;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{Service, State, decoded, picture_kind, services};

    /// Two tiles in one line and one in a line of its own, as Monitorr writes them.
    const PAGE: &str = concat!(
        "<div class=\"col-lg-4\"><a class=\"servicetile\" href=\"https://plex.example.com/web?a=1&amp;b=2\" target=\"_blank\" style=\"display: block\"><div id=\"serviceimg\"><div><img id=\"plex-service-img\" src=\"assets/img/../data/usrimg/plex.png\" class=\"serviceimg\" alt=plex></div></div><div id=\"servicetitle\"><div>Plex</div></div><div class=\"btnonline\">Online</div></a></div></div>",
        "<div class=\"col-lg-4\"><div class=\"servicetile\"><div id=\"serviceimg\"><div><img id=\"backup &amp; sync-service-img\" src=\"assets/img/../data/usrimg/backup.svg\" class=\"serviceimg imgoffline\" alt=backup></div></div><div id=\"servicetitleoffline\" style=\"cursor: default\"><div>Backup &amp; Sync</div></div><div class=\"btnoffline\" style=\"cursor: default\">Offline</div></div></div>\n",
        "<div class=\"col-lg-4\"><div class=\"servicetile\"><div id=\"serviceimg\"><div><img id=\"router-service-img\" src=\"assets/img/../img/router.jpg\" class=\"serviceimg\" alt=router></div></div><div id=\"servicetitlenolink\" style=\"cursor: default\"><div>Router</div></div><div class=\"btnunknown\">Unresponsive</div></div></div>\n",
    );

    #[test]
    fn tiles_are_read_with_name_state_picture_and_link() {
        assert_eq!(
            services(PAGE),
            [
                Service {
                    name: "Plex".to_owned(),
                    state: State::Online,
                    picture: Some("/data/usrimg/plex.png".to_owned()),
                    link: Some("https://plex.example.com/web?a=1&b=2".to_owned()),
                },
                Service {
                    name: "Backup & Sync".to_owned(),
                    state: State::Offline,
                    picture: Some("/data/usrimg/backup.svg".to_owned()),
                    link: None,
                },
                Service {
                    name: "Router".to_owned(),
                    state: State::Unresponsive,
                    picture: Some("/img/router.jpg".to_owned()),
                    link: None,
                },
            ]
        );
    }

    #[test]
    fn html_without_tiles_has_no_services() {
        assert_eq!(services(""), []);
        assert_eq!(services("<html><body>Login</body></html>"), []);
        // A title without a state is no tile, and the next tile is still read.
        let page = format!("<div id=\"servicetitle\"><div>Half</div></div><p>no state</p>{PAGE}");
        assert_eq!(services(&page).len(), 3);
        // A state that Monitorr does not have is no tile.
        let page =
            "<div id=\"servicetitle\"><div>Plex</div></div><div class=\"btnonline\">Maybe</div>";
        assert_eq!(services(page), []);
    }

    #[test]
    fn tile_without_a_picture_has_none() {
        let page = "<div id=\"servicetitle\"><div>Plex</div></div><div class=\"btnonline\">Online</div></div></div>";
        assert_eq!(
            services(page),
            [Service {
                name: "Plex".to_owned(),
                state: State::Online,
                picture: None,
                link: None,
            }]
        );
    }

    #[test]
    fn kind_of_a_picture_comes_from_the_end_of_its_name() {
        assert_eq!(
            picture_kind("/data/usrimg/plex.PNG").as_deref(),
            Some("png")
        );
        assert_eq!(picture_kind("/img/a.b.svg").as_deref(), Some("svg+xml"));
        assert_eq!(picture_kind("/img/none"), None);
        assert_eq!(picture_kind("/img/a.php?x=1"), None);
        assert_eq!(picture_kind("/img/a."), None);
    }

    #[test]
    fn character_references_are_read() {
        assert_eq!(
            decoded("A &amp; B &lt;3 &#39;x&#x27; &quot;"),
            "A & B <3 'x' \""
        );
        // Something that is no reference stays.
        assert_eq!(decoded("Q&A; R&D &unknown; &"), "Q&A; R&D &unknown; &");
    }
}
