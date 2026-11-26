//! A Plex Media Server: what plays now, what was added, playlists, and pictures.
//!
//! The server answers in XML. [`item`] turns one entry of an answer into the form
//! that the homepage of the PHP application reads.

use std::time::Duration;

use roxmltree::Node;
use serde_json::{Map, Value, json};

use crate::http::{Outbound, without_address};

/// How long the server has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// Why the server gave no usable answer.
#[derive(Debug, thiserror::Error)]
pub enum PlexServerError {
    /// The server cannot be reached, or it did not answer in time.
    #[error("the Plex server cannot be reached: {0}")]
    Unreachable(String),
    /// The answer is no XML.
    #[error("the answer of the Plex server cannot be read: {0}")]
    Unreadable(String),
}

/// A Plex Media Server, and how it is asked.
#[derive(Debug, Clone, Copy)]
pub struct Server<'a> {
    /// The address, complete.
    pub url: &'a str,
    /// The Plex token.
    pub token: &'a str,
    /// How long the server has to answer.
    pub timeout: Option<Duration>,
    /// Whether the certificate of the server is left unchecked.
    pub never_check_certificate: bool,
}

impl Server<'_> {
    fn request(
        &self,
        http: &Outbound,
        path: &str,
        query: &[(&str, &str)],
    ) -> reqwest::RequestBuilder {
        let url = format!("{}{path}", self.url.trim_end_matches('/'));
        http.client(&url, self.never_check_certificate)
            .get(&url)
            .query(query)
            // The token goes in a header, so it is in no address and in no log.
            .header("X-Plex-Token", self.token)
            .timeout(self.timeout.unwrap_or(TIME_LIMIT))
    }

    /// The XML at a path of the server, or nothing when the server refused the request.
    pub async fn xml(
        &self,
        http: &Outbound,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Option<String>, PlexServerError> {
        let response = self
            .request(http, path, query)
            .send()
            .await
            .map_err(|error| PlexServerError::Unreachable(without_address(error)))?;
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), path, "the Plex server refused the request");
            return Ok(None);
        }
        response
            .text()
            .await
            .map(Some)
            .map_err(|error| PlexServerError::Unreachable(without_address(error)))
    }

    /// A picture of the server in the given size, at most `limit` bytes. Nothing when
    /// the server refused the request or sent more.
    pub async fn picture(
        &self,
        http: &Outbound,
        picture: &str,
        (width, height): (u32, u32),
        limit: usize,
    ) -> Result<Option<Vec<u8>>, PlexServerError> {
        let (width, height) = (width.to_string(), height.to_string());
        let query = [
            ("height", height.as_str()),
            ("width", width.as_str()),
            ("upscale", "1"),
            ("url", picture),
        ];
        let mut response = self
            .request(http, "/photo/:/transcode", &query)
            .send()
            .await
            .map_err(|error| PlexServerError::Unreachable(without_address(error)))?;
        if !response.status().is_success() {
            return Ok(None);
        }
        let mut content = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| PlexServerError::Unreachable(without_address(error)))?
        {
            if content.len() + chunk.len() > limit {
                return Ok(None);
            }
            content.extend_from_slice(&chunk);
        }
        Ok(Some(content))
    }
}

/// What the items of one answer share: the settings, and who asks.
#[derive(Clone)]
pub struct ItemContext<'a> {
    /// Whether the caller sees who plays.
    pub show_names: bool,
    /// Whether the caller sees the address of the player.
    pub show_addresses: bool,
    /// Other names for the users, by their Plex name.
    pub custom_names: Option<&'a Map<String, Value>>,
    /// The `plexTabURL` setting.
    pub tab_url: &'a str,
    /// The `plexTabName` setting.
    pub tab_name: &'a str,
    /// The id of the server.
    pub server_id: &'a str,
    /// The width and the height of a poster.
    pub poster: (u32, u32),
    /// The width and the height of a wide picture.
    pub wide: (u32, u32),
    /// The present time, in seconds since 1970.
    pub now: i64,
    /// A text that changes with each answer. It makes the address of a picture new,
    /// so the server fetches the picture again.
    pub fresh: &'a str,
    /// Whether the picture cache holds a fresh picture with the given key.
    pub is_cached: &'a (dyn Fn(&str) -> bool + Sync),
}

impl std::fmt::Debug for ItemContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ItemContext")
            .field("show_names", &self.show_names)
            .field("show_addresses", &self.show_addresses)
            .field("server_id", &self.server_id)
            .finish_non_exhaustive()
    }
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
}

fn attribute<'a>(node: Option<Node<'a, '_>>, name: &str) -> &'a str {
    node.and_then(|node| node.attribute(name))
        .unwrap_or_default()
}

fn whole(text: &str) -> i64 {
    text.trim().parse().unwrap_or(0)
}

/// How a stream is made, in the words of the PHP application.
fn stream_type(decision: &str) -> &'static str {
    match decision {
        "transcode" | "Transcode" => "Transcode",
        "copy" | "DirectStream" => "Direct Stream",
        _ => "Direct Play",
    }
}

/// The name of a picture of the server in the cache: a short hash of the path and the
/// size of the picture, with the ending of the key (`-np` for a wide picture, `-list`
/// for a poster). The route that fetches the picture makes the same name from what it
/// fetches, so a caller can fill only the name of the picture that it asks for (D-153).
/// A key with another ending is the name as it is.
pub fn picture_name(picture: &str, (width, height): (u32, u32), key: &str) -> String {
    use sha1::Digest as _;
    use std::fmt::Write as _;
    let ending = if key.ends_with("-np") {
        "np"
    } else if key.ends_with("-list") {
        "list"
    } else {
        return key.to_owned();
    };
    let digest = sha1::Sha1::digest(
        format!(
            "{picture}
{width}x{height}"
        )
        .as_bytes(),
    );
    let mut name = digest
        .iter()
        .take(10)
        .fold(String::new(), |mut name, byte| {
            // Writing into a text cannot fail.
            let _ = write!(name, "{byte:02x}");
            name
        });
    name.push('-');
    name.push_str(ending);
    name
}

/// Whether a path is a picture of the library of a Plex server: of an item
/// (`/library/metadata/<id>/thumb|art|…[/<version>]`), of a collection, or the picture
/// that Plex makes of a playlist. Nothing else goes to the transcoder of the server,
/// which would fetch any address that it is given (D-153).
pub fn is_library_picture(path: &str) -> bool {
    const KINDS: [&str; 6] = ["thumb", "art", "banner", "clearLogo", "theme", "composite"];
    let is_number = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let parts: Vec<&str> = path.split('/').collect();
    match parts.as_slice() {
        ["", "library", "metadata" | "collections", id, kind] | ["", "playlists", id, kind] => {
            is_number(id) && KINDS.contains(kind)
        }
        ["", "library", "metadata" | "collections", id, kind, version]
        | ["", "playlists", id, kind, version] => {
            is_number(id) && KINDS.contains(kind) && is_number(version)
        }
        _ => false,
    }
}

/// The address of the route that gives a picture of the server.
fn picture_route(picture: &str, size: (u32, u32), key: &str) -> String {
    let (width, height) = size;
    let name = picture_name(picture, size, key);
    format!(
        "api/v2/homepage/image?source=plex&img={picture}&height={height}&width={width}&key={name}"
    )
}

/// The titles, the keys and the pictures of an item, which depend on its type.
#[expect(
    clippy::too_many_lines,
    reason = "one line for each field of the item, in the order of the PHP application"
)]
fn by_type(item: Node<'_, '_>, out: &mut Map<String, Value>) {
    let get = |name: &str| item.attribute(name).unwrap_or_default();
    let or = |first: &str, second: &str| {
        item.attribute(first)
            .or_else(|| item.attribute(second))
            .unwrap_or_default()
    };
    let is_live = item.attribute("live").is_some();
    let mut set = |key: &str, value: String| {
        out.insert(key.to_owned(), Value::String(value));
    };
    match get("type") {
        "show" => {
            set("type", "tv".to_owned());
            set("title", get("title").to_owned());
            set("secondaryTitle", get("year").to_owned());
            set("summary", get("summary").to_owned());
            set("ratingKey", get("ratingKey").to_owned());
            set("thumb", get("thumb").to_owned());
            set("key", format!("{}-list", get("ratingKey")));
            set("nowPlayingThumb", get("art").to_owned());
            set("nowPlayingKey", format!("{}-np", get("ratingKey")));
            set("nowPlayingTitle", get("title").to_owned());
            set("nowPlayingBottom", get("year").to_owned());
            set("metadataKey", get("ratingKey").to_owned());
        }
        "season" => {
            set("type", "tv".to_owned());
            set("title", get("parentTitle").to_owned());
            set("secondaryTitle", get("title").to_owned());
            set("summary", get("parentSummary").to_owned());
            set("ratingKey", get("parentRatingKey").to_owned());
            set("thumb", get("thumb").to_owned());
            set("key", format!("{}-list", get("ratingKey")));
            set("nowPlayingThumb", get("art").to_owned());
            set("nowPlayingKey", format!("{}-np", get("ratingKey")));
            set("metadataKey", get("parentRatingKey").to_owned());
        }
        "episode" => {
            set("type", "tv".to_owned());
            set("title", get("grandparentTitle").to_owned());
            set(
                "secondaryTitle",
                format!("{} - Episode {}", get("parentTitle"), get("index")),
            );
            set("summary", get("title").to_owned());
            set("ratingKey", or("parentRatingKey", "ratingKey").to_owned());
            let thumb = if get("parentThumb").is_empty() {
                get("grandparentThumb")
            } else {
                get("parentThumb")
            };
            set("thumb", thumb.to_owned());
            set("key", format!("{}-list", get("ratingKey")));
            set("nowPlayingThumb", get("grandparentArt").to_owned());
            set(
                "nowPlayingKey",
                format!("{}-np", get("grandparentRatingKey")),
            );
            set(
                "nowPlayingTitle",
                format!("{} - {}", get("grandparentTitle"), get("title")),
            );
            set(
                "nowPlayingBottom",
                format!("S{} · E{}", get("parentIndex"), get("index")),
            );
            let key = item
                .attribute("grandparentRatingKey")
                .unwrap_or_else(|| or("parentRatingKey", "ratingKey"));
            set("metadataKey", key.to_owned());
        }
        "clip" => {
            let title = if is_live { "Live TV" } else { get("title") };
            set("type", "clip".to_owned());
            set("title", title.to_owned());
            set("secondaryTitle", String::new());
            set("summary", get("summary").to_owned());
            set("ratingKey", get("parentRatingKey").to_owned());
            set("thumb", get("thumb").to_owned());
            set("key", format!("{}-list", get("ratingKey")));
            set("nowPlayingThumb", get("art").to_owned());
            let key = match item.attribute("ratingKey") {
                Some(key) => format!("{key}-np"),
                None if is_live => "livetv.png".to_owned(),
                None => ":)".to_owned(),
            };
            set("nowPlayingKey", key);
            set("nowPlayingTitle", title.to_owned());
            let bottom = if item.attribute("extraType").is_some() {
                "Trailer"
            } else if is_live {
                "Live TV"
            } else {
                ":)"
            };
            set("nowPlayingBottom", bottom.to_owned());
        }
        "album" | "track" => {
            set("type", "music".to_owned());
            set("title", get("parentTitle").to_owned());
            set("secondaryTitle", get("title").to_owned());
            set("summary", get("title").to_owned());
            set("ratingKey", get("parentRatingKey").to_owned());
            set("thumb", get("thumb").to_owned());
            set("key", format!("{}-list", get("ratingKey")));
            let thumb = if get("parentThumb").is_empty() {
                get("art")
            } else {
                get("parentThumb")
            };
            set("nowPlayingThumb", thumb.to_owned());
            set("nowPlayingKey", format!("{}-np", get("parentRatingKey")));
            set(
                "nowPlayingTitle",
                format!("{} - {}", get("grandparentTitle"), get("title")),
            );
            set("nowPlayingBottom", get("parentTitle").to_owned());
            set(
                "metadataKey",
                or("grandparentRatingKey", "parentRatingKey").to_owned(),
            );
        }
        _ => {
            set("type", "movie".to_owned());
            set("title", get("title").to_owned());
            set("secondaryTitle", get("year").to_owned());
            set("summary", get("summary").to_owned());
            set("ratingKey", get("ratingKey").to_owned());
            set("thumb", get("thumb").to_owned());
            set("key", format!("{}-list", get("ratingKey")));
            set("nowPlayingThumb", get("art").to_owned());
            set("nowPlayingKey", format!("{}-np", get("ratingKey")));
            set("nowPlayingTitle", get("title").to_owned());
            set("nowPlayingBottom", get("year").to_owned());
            set("metadataKey", get("ratingKey").to_owned());
        }
    }
}

/// How the item is played, when it comes from the list of the sessions.
fn stream_of(item: Node<'_, '_>) -> Value {
    let media = child(item, "Media");
    let part = media.and_then(|media| child(media, "Part"));
    let player = child(item, "Player");
    let transcode = child(item, "TranscodeSession");
    let throttled = attribute(transcode, "throttled") == "1";
    let stream = if media.is_some() {
        let throttled = if throttled { " (Throttled)" } else { "" };
        format!("{}{throttled}", attribute(part, "decision"))
    } else {
        String::new()
    };
    json!({
        "platform": attribute(player, "platform"),
        "product": attribute(player, "product"),
        "device": attribute(player, "device"),
        "stream": stream,
        "videoResolution": attribute(media, "videoResolution"),
        "throttled": throttled,
        "sourceVideoCodec": attribute(transcode, "sourceVideoCodec"),
        "videoCodec": attribute(transcode, "videoCodec"),
        "audioCodec": attribute(transcode, "audioCodec"),
        "sourceAudioCodec": attribute(transcode, "sourceAudioCodec"),
        "videoDecision": stream_type(attribute(transcode, "videoDecision")),
        "audioDecision": stream_type(attribute(transcode, "audioDecision")),
        "container": attribute(transcode, "container"),
        "audioChannels": attribute(transcode, "audioChannels"),
    })
}

/// What Plex knows about the item: the plot, the genres and the actors.
fn metadata_of(item: Node<'_, '_>) -> Value {
    let get = |name: &str| item.attribute(name).unwrap_or_default();
    let named = |name: &'static str| {
        item.children()
            .filter(move |child| child.is_element() && child.tag_name().name() == name)
    };
    // Without genres or actors, the PHP application sends an empty text.
    let genres: Vec<Value> = named("Genre")
        .map(|genre| json!(genre.attribute("tag").unwrap_or_default()))
        .collect();
    let has_roles = named("Role").next().is_some();
    let actors: Vec<Value> = named("Role")
        .filter(|role| !role.attribute("thumb").unwrap_or_default().is_empty())
        .map(|role| {
            json!({
                "name": role.attribute("tag").unwrap_or_default(),
                "role": role.attribute("role").unwrap_or_default(),
                "thumb": role.attribute("thumb").unwrap_or_default(),
            })
        })
        .collect();
    json!({
        "guid": get("guid"),
        "summary": get("summary"),
        "rating": get("rating"),
        "duration": get("duration"),
        "originallyAvailableAt": get("originallyAvailableAt"),
        "year": get("year"),
        "studio": get("studio"),
        "tagline": get("tagline"),
        "genres": if genres.is_empty() { json!("") } else { json!(genres) },
        "actors": if has_roles { json!(actors) } else { json!("") },
    })
}

/// One entry of an answer of the server in the form of the homepage.
#[expect(
    clippy::too_many_lines,
    reason = "one line for each field of the item, in the order of the PHP application"
)]
pub fn item(item: Node<'_, '_>, context: &ItemContext<'_>) -> Value {
    let mut out = Map::new();
    by_type(item, &mut out);
    let text = |out: &Map<String, Value>, key: &str| {
        out.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let get = |name: &str| item.attribute(name).unwrap_or_default();
    let (media, player, session, transcode, user) = (
        child(item, "Media"),
        child(item, "Player"),
        child(item, "Session"),
        child(item, "TranscodeSession"),
        child(item, "User"),
    );
    let stream = media
        .and_then(|media| child(media, "Part"))
        .and_then(|part| child(part, "Stream"));

    let elapsed = item.attribute("viewOffset").map(whole);
    let duration = item
        .attribute("duration")
        .map_or_else(|| whole(attribute(media, "duration")), whole);
    let added = item.attribute("addedAt").map(whole);
    let watched = match elapsed {
        Some(elapsed) if elapsed != 0 && duration != 0 => elapsed.saturating_mul(100) / duration,
        _ => 0,
    };
    let progress = transcode.and_then(|transcode| transcode.attribute("progress"));
    // Plex sends the progress with a fraction. The PHP application reads its whole part.
    let transcoded = progress.map_or_else(
        || json!(""),
        |progress| json!(whole(progress.split('.').next().unwrap_or_default()) - watched),
    );
    let name = if context.show_names {
        let name = attribute(user, "title");
        context
            .custom_names
            .and_then(|names| names.get(name))
            .and_then(Value::as_str)
            .unwrap_or(name)
    } else {
        ""
    };
    let details = format!("/details?key=/library/metadata/{}", get("ratingKey"));
    let address = if context.tab_url.is_empty() {
        format!(
            "https://app.plex.tv/web/app#!/server/{}{details}",
            context.server_id
        )
    } else {
        format!(
            "{}/web/index.html#!/server/{}{details}",
            context.tab_url, context.server_id
        )
    };

    out.insert("originalType".to_owned(), json!(get("type")));
    out.insert("uid".to_owned(), json!(get("ratingKey")));
    out.insert("elapsed".to_owned(), json!(elapsed));
    out.insert("duration".to_owned(), json!(duration));
    out.insert("addedAt".to_owned(), json!(added));
    out.insert("watched".to_owned(), json!(watched));
    out.insert("transcoded".to_owned(), transcoded);
    out.insert("stream".to_owned(), json!(attribute(stream, "decision")));
    out.insert(
        "id".to_owned(),
        json!(attribute(player, "machineIdentifier").replace('"', "")),
    );
    out.insert("session".to_owned(), json!(attribute(session, "id")));
    out.insert(
        "bandwidth".to_owned(),
        json!(attribute(session, "bandwidth")),
    );
    out.insert(
        "bandwidthType".to_owned(),
        json!(attribute(session, "location")),
    );
    let kind = if progress.is_some() {
        "Transcoding"
    } else {
        "Direct Playing"
    };
    out.insert("sessionType".to_owned(), json!(kind));
    let state = if attribute(player, "state") == "paused" {
        "pause"
    } else {
        "play"
    };
    out.insert("state".to_owned(), json!(state));
    out.insert("user".to_owned(), json!(name));
    let thumb = if context.show_names {
        attribute(user, "thumb")
    } else {
        ""
    };
    out.insert("userThumb".to_owned(), json!(thumb));
    let player_address = if context.show_addresses {
        attribute(player, "address")
    } else {
        "x.x.x.x"
    };
    out.insert("userAddress".to_owned(), json!(player_address));
    out.insert("address".to_owned(), json!(address));

    let (wide_picture, wide_key) = (text(&out, "nowPlayingThumb"), text(&out, "nowPlayingKey"));
    let (poster, poster_key) = (text(&out, "thumb"), text(&out, "key"));
    out.insert(
        "nowPlayingOriginalImage".to_owned(),
        json!(format!(
            "{}${}",
            picture_route(&wide_picture, context.wide, &wide_key),
            context.fresh
        )),
    );
    out.insert(
        "originalImage".to_owned(),
        json!(format!(
            "{}${}",
            picture_route(&poster, context.poster, &poster_key),
            context.fresh
        )),
    );
    out.insert(
        "openTab".to_owned(),
        json!(!context.tab_url.is_empty() && !context.tab_name.is_empty()),
    );
    out.insert("tabName".to_owned(), json!(context.tab_name));
    out.insert("userStream".to_owned(), stream_of(item));
    out.insert("metadata".to_owned(), metadata_of(item));

    // A picture that the cache holds comes from there, and else from the route that
    // fetches it.
    let cached_or_route = |picture: &str, size: (u32, u32), key: &str| {
        let name = picture_name(picture, size, key);
        if (context.is_cached)(&name) {
            format!("data/cache/plex-{name}.jpg")
        } else {
            picture_route(picture, size, key)
        }
    };
    out.insert(
        "nowPlayingImageURL".to_owned(),
        json!(cached_or_route(&wide_picture, context.wide, &wide_key)),
    );
    out.insert(
        "imageURL".to_owned(),
        json!(cached_or_route(&poster, context.poster, &poster_key)),
    );
    if wide_picture.is_empty() {
        let none = json!("plugins/images/homepage/no-np.png");
        out.insert("nowPlayingOriginalImage".to_owned(), none.clone());
        out.insert("nowPlayingImageURL".to_owned(), none);
        out.insert("nowPlayingKey".to_owned(), json!("no-np"));
    }
    // Plex has no poster for an item of the last five minutes yet.
    if poster.is_empty() || added.is_some_and(|added| added >= context.now - 300) {
        let none = json!("plugins/images/homepage/no-list.png");
        out.insert("originalImage".to_owned(), none.clone());
        out.insert("imageURL".to_owned(), none);
        out.insert("key".to_owned(), json!("no-list"));
    }
    if item.attribute("live").is_some() && matches!(get("type"), "episode" | "clip" | "movie" | "")
    {
        out.insert(
            "useImage".to_owned(),
            json!("plugins/images/homepage/livetv.png"),
        );
    }
    Value::Object(out)
}

/// The entries of an answer that the homepage shows: every entry of a library that is
/// not left out, and whose type is not in `ignored`.
pub fn items(
    xml: &str,
    context: &ItemContext<'_>,
    excluded_libraries: &[&str],
    ignored_types: &[&str],
) -> Result<Vec<Value>, PlexServerError> {
    let document = roxmltree::Document::parse(xml)
        .map_err(|error| PlexServerError::Unreadable(error.to_string()))?;
    Ok(document
        .root_element()
        .children()
        .filter(Node::is_element)
        .filter(|entry| {
            let kind = entry.attribute("type").unwrap_or_default();
            let library = entry.attribute("librarySectionID");
            !ignored_types.contains(&kind)
                && library.is_some_and(|library| !excluded_libraries.contains(&library))
        })
        .map(|entry| item(entry, context))
        .collect())
}

/// A playlist of the server: its title and the address of its entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    /// The title.
    pub title: String,
    /// The path of the entries on the server.
    pub key: String,
}

/// The video playlists of an answer whose title does not hold the word "private".
pub fn playlists(xml: &str) -> Result<Vec<Playlist>, PlexServerError> {
    let document = roxmltree::Document::parse(xml)
        .map_err(|error| PlexServerError::Unreadable(error.to_string()))?;
    Ok(document
        .root_element()
        .children()
        .filter(Node::is_element)
        .filter(|entry| {
            entry.attribute("playlistType") == Some("video")
                && !entry
                    .attribute("title")
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains("private")
        })
        .map(|entry| Playlist {
            title: entry.attribute("title").unwrap_or_default().to_owned(),
            key: entry.attribute("key").unwrap_or_default().to_owned(),
        })
        .collect())
}

/// The videos of a playlist, in the form of the homepage.
pub fn playlist_videos(
    xml: &str,
    context: &ItemContext<'_>,
) -> Result<Vec<Value>, PlexServerError> {
    let document = roxmltree::Document::parse(xml)
        .map_err(|error| PlexServerError::Unreadable(error.to_string()))?;
    Ok(document
        .root_element()
        .children()
        .filter(|entry| entry.is_element() && entry.tag_name().name() == "Video")
        .map(|entry| item(entry, context))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{is_library_picture, picture_name};

    #[test]
    fn picture_name_comes_from_the_picture_and_its_size() {
        let poster = picture_name("/library/metadata/815/thumb/1", (200, 300), "815-list");
        assert!(poster.ends_with("-list"), "{poster}");
        assert_eq!(poster.len(), 25);
        // The same picture under the key of another item has the same name; another
        // picture or another size has another one (D-153).
        assert_eq!(
            picture_name("/library/metadata/815/thumb/1", (200, 300), "999-list"),
            poster
        );
        assert_ne!(
            picture_name("/library/metadata/816/thumb/1", (200, 300), "815-list"),
            poster
        );
        assert_ne!(
            picture_name("/library/metadata/815/thumb/1", (300, 300), "815-list"),
            poster
        );
        assert!(
            picture_name("/library/metadata/815/art/1", (1280, 720), "815-np").ends_with("-np")
        );
        // A key with another ending stays as it is.
        assert_eq!(picture_name("/x", (1, 1), "livetv.png"), "livetv.png");
    }

    #[test]
    fn only_pictures_of_the_library_go_to_the_transcoder() {
        for path in [
            "/library/metadata/815/thumb/1599999999",
            "/library/metadata/815/art",
            "/library/collections/12/composite/1600000000",
            "/playlists/456/composite/1600000000",
        ] {
            assert!(is_library_picture(path), "{path}");
        }
        for path in [
            "/photo/:/transcode?url=http%3A%2F%2Fevil.example%2Fa.png",
            "/library/metadata/815/thumb/1?x=1",
            "/library/metadata/abc/thumb/1",
            "/library/metadata/815/file/1",
            "//evil.example/a.png",
            "https://evil.example/a.png",
            "/library/metadata/815/thumb/1/../../x",
            "",
        ] {
            assert!(!is_library_picture(path), "{path}");
        }
    }
}
