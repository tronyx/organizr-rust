//! An Emby server or a Jellyfin server: what plays now, what was added, the details of
//! an item, and pictures.
//!
//! Jellyfin started as a copy of Emby, and the two answer the requests of this module
//! in the same way. [`item`] turns one entry of an answer into the form that the
//! homepage of the PHP application reads.

use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::http::{Outbound, without_address};

/// How long the server has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// What the server tells about an item, on top of the fields that it always sends.
const FIELDS: &str = "Overview,People,Genres,CriticRating,Studios,Taglines";

/// The types of items that the list of new items of Emby holds.
const NEW_TYPES: &str = "Series,Episode,MusicAlbum,Audio,Movie,Video";

/// The width and the height of the picture of an actor.
const ACTOR_SIZE: (u32, u32) = (300, 450);

/// Which of the two servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An Emby server.
    Emby,
    /// A Jellyfin server.
    Jellyfin,
}

impl Kind {
    /// The name of the server, as texts show it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Emby => "Emby",
            Self::Jellyfin => "Jellyfin",
        }
    }

    /// The name of the server in addresses, and at the start of the names of cached
    /// pictures.
    pub fn source(self) -> &'static str {
        match self {
            Self::Emby => "emby",
            Self::Jellyfin => "jellyfin",
        }
    }

    /// The header that carries the API key, and its value.
    pub fn key_header(self, token: &str) -> (&'static str, String) {
        match self {
            Self::Emby => ("X-Emby-Token", token.to_owned()),
            Self::Jellyfin => (
                "Authorization",
                format!("MediaBrowser Token=\"{}\"", token.replace('"', "")),
            ),
        }
    }
}

/// Why the server gave no answer.
#[derive(Debug, thiserror::Error)]
pub enum EmbyError {
    /// The server cannot be reached, or it did not answer in time.
    #[error("the server cannot be reached: {0}")]
    Unreachable(String),
}

/// An Emby or a Jellyfin server, and how it is asked.
#[derive(Debug, Clone, Copy)]
pub struct Server<'a> {
    /// Which of the two servers.
    pub kind: Kind,
    /// The address, complete.
    pub url: &'a str,
    /// The API key.
    pub token: &'a str,
    /// How long the server has to answer.
    pub timeout: Option<Duration>,
    /// Whether the certificate of the server is left unchecked.
    pub never_check_certificate: bool,
}

/// Whether the text can be the id of an item or of a user: letters, digits and hyphens
/// only. An id goes into the path of a request, so it must not hold a slash or a dot.
pub fn is_id(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
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
            .timeout(self.timeout.unwrap_or(TIME_LIMIT))
    }

    /// The JSON at a path of the server. Nothing when the server refused the request.
    /// An answer that is no JSON reads as `null`, as in the PHP application.
    pub(crate) async fn json(
        &self,
        http: &Outbound,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Option<Value>, EmbyError> {
        // The key goes in a header, so it is in no address and in no log.
        let (header, value) = self.kind.key_header(self.token);
        let response = self
            .request(http, path, query)
            .header(header, value)
            .send()
            .await
            .map_err(|error| EmbyError::Unreachable(without_address(error)))?;
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), path, "the media server refused the request");
            return Ok(None);
        }
        let text = response
            .text()
            .await
            .map_err(|error| EmbyError::Unreachable(without_address(error)))?;
        Ok(Some(serde_json::from_str(&text).unwrap_or(Value::Null)))
    }

    /// The sessions of the server, or nothing when the server refused the request.
    pub async fn sessions(&self, http: &Outbound) -> Result<Option<Value>, EmbyError> {
        self.json(http, "/Sessions", &[("Fields", FIELDS)]).await
    }

    /// The users of the server, or nothing when the server refused the request.
    pub async fn users(&self, http: &Outbound) -> Result<Option<Value>, EmbyError> {
        self.json(http, "/Users", &[]).await
    }

    /// The items that were added last and that the user did not play, or nothing when
    /// the server refused the request.
    pub async fn latest(
        &self,
        http: &Outbound,
        user: &str,
        limit: u32,
    ) -> Result<Option<Value>, EmbyError> {
        if !is_id(user) {
            return Ok(None);
        }
        let limit = limit.to_string();
        let mut query = vec![
            ("EnableImages", "true"),
            ("Limit", limit.as_str()),
            ("IsPlayed", "false"),
            ("Fields", FIELDS),
        ];
        if self.kind == Kind::Emby {
            query.push(("IncludeItemTypes", NEW_TYPES));
        }
        self.json(http, &format!("/Users/{user}/Items/Latest"), &query)
            .await
    }

    /// One item as the user sees it, or nothing when the server refused the request.
    pub async fn details(
        &self,
        http: &Outbound,
        user: &str,
        item: &str,
        limit: u32,
    ) -> Result<Option<Value>, EmbyError> {
        if !is_id(user) || !is_id(item) {
            return Ok(None);
        }
        let limit = limit.to_string();
        let query = [
            ("EnableImages", "true"),
            ("Limit", limit.as_str()),
            ("IsPlayed", "false"),
            ("Fields", FIELDS),
        ];
        self.json(http, &format!("/Users/{user}/Items/{item}"), &query)
            .await
    }

    /// A picture of an item, at most as large as the given size and at most `limit`
    /// bytes. Nothing when the server refused the request or sent more.
    ///
    /// The server gives pictures without a key, so the request carries none.
    pub async fn picture(
        &self,
        http: &Outbound,
        (item, picture): (&str, &str),
        (width, height): (u32, u32),
        limit: usize,
    ) -> Result<Option<Vec<u8>>, EmbyError> {
        let is_type = !picture.is_empty() && picture.chars().all(|c| c.is_ascii_alphabetic());
        if !is_id(item) || !is_type {
            return Ok(None);
        }
        let (width, height) = (width.to_string(), height.to_string());
        let query = [("maxHeight", height.as_str()), ("maxWidth", width.as_str())];
        let mut response = self
            .request(http, &format!("/Items/{item}/Images/{picture}"), &query)
            .send()
            .await
            .map_err(|error| EmbyError::Unreachable(without_address(error)))?;
        if !response.status().is_success() {
            return Ok(None);
        }
        let mut content = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| EmbyError::Unreachable(without_address(error)))?
        {
            if content.len() + chunk.len() > limit {
                return Ok(None);
            }
            content.extend_from_slice(&chunk);
        }
        Ok(Some(content))
    }
}

/// The id of the user of the server whose eyes the lists use: the user with the given
/// name, whatever the letter case, or else the last administrator. Nothing when the
/// server has neither.
pub fn user_of(users: &Value, name: Option<&str>) -> Option<String> {
    let name = name.map(str::to_lowercase);
    let mut found = None;
    for user in users.as_array().into_iter().flatten() {
        let id = || user.get("Id").map(|id| text(Some(id)));
        if user.pointer("/Policy/IsAdministrator").is_some_and(truthy) {
            found = id();
        }
        let user_name = user
            .get("Name")
            .and_then(Value::as_str)
            .map(str::to_lowercase);
        if name.is_some() && user_name == name {
            found = id();
            break;
        }
    }
    found.filter(|id| is_id(id))
}

/// What the items of one answer share: the settings, and who asks.
#[derive(Clone)]
pub struct ItemContext<'a> {
    /// Which of the two servers.
    pub kind: Kind,
    /// Whether the caller sees who plays, and from which address.
    pub show_names: bool,
    /// The address of an item in the web interface of the server, with `{id}` and
    /// `{serverId}` where the ids go.
    pub link: &'a str,
    /// Whether an item opens in a tab of Organizr.
    pub open_tab: bool,
    /// The name of that tab.
    pub tab_name: &'a str,
    /// The width and the height of a poster.
    pub poster: (u32, u32),
    /// The width and the height of a wide picture.
    pub wide: (u32, u32),
    /// A text that changes with each answer. It makes the address of a picture new,
    /// so the server fetches the picture again.
    pub fresh: &'a str,
    /// Whether the picture cache holds a fresh picture with the given key.
    pub is_cached: &'a (dyn Fn(&str) -> bool + Sync),
}

impl std::fmt::Debug for ItemContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ItemContext")
            .field("kind", &self.kind)
            .field("show_names", &self.show_names)
            .finish_non_exhaustive()
    }
}

/// A value as PHP makes a text of it.
fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(true)) => "1".to_owned(),
        _ => String::new(),
    }
}

/// Whether PHP takes the value for true.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(on) => *on,
        Value::Number(number) => number.as_f64() != Some(0.0),
        Value::String(text) => !text.is_empty() && text != "0",
        Value::Array(list) => !list.is_empty(),
        Value::Object(_) => true,
    }
}

/// A whole number, as PHP makes one of a value.
fn whole(value: &Value) -> i64 {
    match value {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a fraction is cut off, as in PHP; a number out of range becomes the nearest end of the range"
        )]
        Value::Number(number) => number
            .as_i64()
            .unwrap_or_else(|| number.as_f64().map_or(0, |float| float as i64)),
        Value::String(text) => text.trim().parse().unwrap_or(0),
        Value::Bool(true) => 1,
        _ => 0,
    }
}

/// How a stream is made, in the words of the PHP application.
fn stream_type(method: &str) -> &'static str {
    match method {
        "transcode" | "Transcode" => "Transcode",
        "copy" | "DirectStream" => "Direct Stream",
        _ => "Direct Play",
    }
}

/// The address of the route that gives a picture of the server.
pub(crate) fn picture_route(
    kind: Kind,
    picture: &str,
    id: &str,
    (width, height): (u32, u32),
    key: &str,
) -> String {
    format!(
        "api/v2/homepage/image?source={}&type={picture}&img={id}&height={height}&width={width}&key={key}",
        kind.source()
    )
}

/// The field of an object, when it is there and not `null`.
fn field<'a>(object: &'a Value, name: &str) -> Option<&'a Value> {
    object.get(name).filter(|value| !value.is_null())
}

/// The value of the first field of the two that is there, or `false`.
fn first_of(item: &Value, first: &str, second: &str) -> Value {
    field(item, first)
        .or_else(|| field(item, second))
        .cloned()
        .unwrap_or(Value::Bool(false))
}

/// The titles, the keys and the pictures of an item, which depend on its type. `false`
/// for a type that the homepage does not show.
#[expect(
    clippy::too_many_lines,
    reason = "one line for each field of the item, in the order of the PHP application"
)]
fn by_type(item: &Value, out: &mut Map<String, Value>) -> bool {
    let value = |name: &str| item.get(name).cloned().unwrap_or(Value::Null);
    let id = text(item.get("Id"));
    let wide = |tags: &str| {
        if field(item, "ImageTags")
            .and_then(|tags| field(tags, "Thumb"))
            .is_some()
        {
            "Thumb"
        } else if item.pointer(tags).is_some_and(|tag| !tag.is_null()) {
            "Backdrop"
        } else {
            ""
        }
    };
    let kind = item.get("Type").and_then(Value::as_str).unwrap_or_default();
    let mut set = |key: &str, value: Value| {
        out.insert(key.to_owned(), value);
    };
    let (shown, title) = match kind {
        "Series" | "Episode" => (
            "tv",
            if kind == "Series" {
                "Name"
            } else {
                "SeriesName"
            },
        ),
        "MusicAlbum" | "Audio" => ("music", "Name"),
        "Movie" => ("movie", "Name"),
        "Video" => ("video", "Name"),
        _ => return false,
    };
    set("type", json!(shown));
    set("title", value(title));
    set("secondaryTitle", json!(""));
    set("summary", json!(""));
    set("ratingKey", value("Id"));
    match kind {
        "Episode" => {
            let series = text(field(item, "SeriesId").or_else(|| item.get("Id")));
            let parent = first_of(item, "ParentThumbItemId", "ParentBackdropItemId");
            set("thumb", json!(series));
            set("key", json!(format!("{series}-list")));
            let key = match &parent {
                Value::Bool(_) => Value::Bool(false),
                parent => json!(format!("{}-np", text(Some(parent)))),
            };
            set("nowPlayingThumb", parent);
            set("nowPlayingKey", key);
            set("metadataKey", value("Id"));
            set(
                "nowPlayingImageType",
                json!(wide("/ParentBackdropImageTags/0")),
            );
            set(
                "nowPlayingTitle",
                json!(format!(
                    "{} - {}",
                    text(item.get("SeriesName")),
                    text(item.get("Name"))
                )),
            );
            set(
                "nowPlayingBottom",
                json!(format!(
                    "S{} · E{}",
                    text(item.get("ParentIndexNumber")),
                    text(item.get("IndexNumber"))
                )),
            );
        }
        "MusicAlbum" | "Audio" => {
            set("thumb", value("Id"));
            set("key", json!(format!("{id}-list")));
            let album = field(item, "AlbumId");
            set(
                "nowPlayingThumb",
                album
                    .cloned()
                    .unwrap_or_else(|| value("ParentBackdropItemId")),
            );
            set("nowPlayingKey", json!(format!("{id}-np")));
            set("metadataKey", album.cloned().unwrap_or_else(|| value("Id")));
            let picture = if field(item, "ParentBackdropItemId").is_some() {
                "Primary"
            } else {
                "Backdrop"
            };
            set("nowPlayingImageType", json!(picture));
            set(
                "nowPlayingTitle",
                json!(format!(
                    "{} - {}",
                    text(item.get("AlbumArtist")),
                    text(item.get("Name"))
                )),
            );
            set("nowPlayingBottom", value("Album"));
        }
        _ => {
            set("thumb", value("Id"));
            set("key", json!(format!("{id}-list")));
            set("nowPlayingThumb", value("Id"));
            set("nowPlayingKey", json!(format!("{id}-np")));
            set("metadataKey", value("Id"));
            if kind == "Series" {
                set("nowPlayingImageType", json!(wide("/BackdropImageTags/0")));
            } else {
                // A film without a wide picture gets `false`, as in the PHP application.
                let picture = match wide("/BackdropImageTags") {
                    "" => Value::Bool(false),
                    picture => json!(picture),
                };
                set("nowPlayingImageType", picture);
                set("nowPlayingTitle", value("Name"));
                set("nowPlayingBottom", value("ProductionYear"));
            }
        }
    }
    true
}

/// How the item is played, when it comes from the list of the sessions.
fn stream_of(details: &Value) -> Value {
    let method = details.pointer("/PlayState/PlayMethod");
    let decision = stream_type(method.and_then(Value::as_str).unwrap_or_default());
    let at = |path: &str| details.pointer(path).cloned().unwrap_or(Value::Null);
    let first = details.pointer("/NowPlayingItem/MediaStreams/0");
    let codec = |stream: Option<&Value>| {
        stream.map(|stream| stream.get("Codec").cloned().unwrap_or(Value::Null))
    };
    let second = details.pointer("/NowPlayingItem/MediaStreams/1");
    json!({
        "platform": text(details.get("Client")),
        "product": text(details.get("Client")),
        "device": text(details.get("DeviceName")),
        "stream": at("/PlayState/PlayMethod"),
        "videoResolution": first
            .and_then(|stream| field(stream, "Width"))
            .cloned()
            .unwrap_or_else(|| json!("")),
        "throttled": false,
        "sourceVideoCodec": codec(first).unwrap_or_else(|| json!("")),
        "videoCodec": at("/TranscodingInfo/VideoCodec"),
        "audioCodec": at("/TranscodingInfo/AudioCodec"),
        "sourceAudioCodec": codec(second).or_else(|| codec(first)).unwrap_or_else(|| json!("")),
        "videoDecision": decision,
        "audioDecision": decision,
        "container": field(details, "NowPlayingItem")
            .and_then(|item| field(item, "Container"))
            .cloned()
            .unwrap_or_else(|| json!("")),
        "audioChannels": at("/TranscodingInfo/AudioChannels"),
    })
}

/// What the server knows about the item: the plot, the genres and the actors.
fn metadata_of(item: &Value, context: &ItemContext<'_>) -> Value {
    let source = context.kind.source();
    let genres = field(item, "Genres").map_or_else(
        || json!(""),
        |genres| json!(genres.as_array().cloned().unwrap_or_default()),
    );
    // An actor is shown with a picture and a role only.
    let actors = field(item, "People").map_or_else(
        || json!(""),
        |people| {
            let actors: Vec<Value> = people
                .as_array()
                .into_iter()
                .flatten()
                .filter(|person| {
                    person.get("PrimaryImageTag").is_some_and(truthy)
                        && person.get("Role").is_some_and(truthy)
                })
                .map(|person| {
                    let id = text(person.get("Id"));
                    let key = format!("{id}-cast");
                    let thumb = if !is_id(&id) {
                        String::new()
                    } else if (context.is_cached)(&key) {
                        format!("data/cache/{source}-{key}.jpg")
                    } else {
                        picture_route(context.kind, "Primary", &id, ACTOR_SIZE, &key)
                    };
                    json!({
                        "name": text(person.get("Name")),
                        "role": text(person.get("Role")),
                        "thumb": thumb,
                    })
                })
                .collect();
            json!(actors)
        },
    );
    json!({
        "guid": item.get("Id").cloned().unwrap_or(Value::Null),
        "summary": text(item.get("Overview")),
        "rating": text(item.get("CommunityRating")),
        "duration": text(item.get("RunTimeTicks")),
        "originallyAvailableAt": text(item.get("PremiereDate")),
        "year": field(item, "ProductionYear").cloned().unwrap_or_else(|| json!("")),
        "tagline": text(item.pointer("/Taglines/0")),
        "genres": genres,
        "actors": actors,
    })
}

/// How far the item is played and how it is sent, from the session.
fn progress_of(details: &Value, item: &Value, out: &mut Map<String, Value>) {
    let elapsed = details
        .pointer("/PlayState/PositionTicks")
        .filter(|ticks| !ticks.is_null())
        .map(whole);
    let duration = details
        .pointer("/NowPlayingItem/RunTimeTicks")
        .filter(|ticks| !ticks.is_null())
        .or_else(|| field(item, "RunTimeTicks"))
        .map_or(0, whole);
    let watched = match elapsed {
        Some(elapsed) if elapsed != 0 && duration != 0 => elapsed.saturating_mul(100) / duration,
        _ => 0,
    };
    let transcoded = details
        .pointer("/TranscodingInfo/CompletionPercentage")
        .filter(|percent| !percent.is_null())
        .map_or(100, whole);
    // The bit rate is in bits per second. The homepage shows kilobits.
    let bandwidth = details
        .pointer("/TranscodingInfo/Bitrate")
        .and_then(Value::as_f64)
        .map_or_else(
            || json!(""),
            |rate| {
                let rate = rate / 1000.0;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the number has no fraction, and a bit rate is far inside the range"
                )]
                if rate.fract() == 0.0 {
                    json!(rate as i64)
                } else {
                    json!(rate)
                }
            },
        );
    let method = details.pointer("/PlayState/PlayMethod");
    let is_transcoded = method.and_then(Value::as_str) == Some("Transcode");
    let is_paused = details
        .pointer("/PlayState/IsPaused")
        .is_some_and(|paused| text(Some(paused)) == "1");

    out.insert("elapsed".to_owned(), json!(elapsed));
    out.insert("duration".to_owned(), json!(duration));
    out.insert("watched".to_owned(), json!(watched));
    out.insert("transcoded".to_owned(), json!(transcoded));
    out.insert("stream".to_owned(), method.cloned().unwrap_or(Value::Null));
    out.insert(
        "id".to_owned(),
        item.get("ServerId").cloned().unwrap_or(Value::Null),
    );
    out.insert(
        "session".to_owned(),
        details.get("DeviceId").cloned().unwrap_or(Value::Null),
    );
    out.insert("bandwidth".to_owned(), bandwidth);
    out.insert("bandwidthType".to_owned(), json!("wan"));
    out.insert(
        "sessionType".to_owned(),
        json!(if is_transcoded {
            "Transcoding"
        } else {
            "Direct Playing"
        }),
    );
    out.insert(
        "state".to_owned(),
        json!(if is_paused { "pause" } else { "play" }),
    );
}

/// One entry of an answer of the server in the form of the homepage: a session, or an
/// item. Nothing for an item of a type that the homepage does not show.
#[expect(
    clippy::too_many_lines,
    reason = "one line for each field of the item, in the order of the PHP application"
)]
pub fn item(details: &Value, context: &ItemContext<'_>) -> Option<Value> {
    let item = details
        .get("NowPlayingItem")
        .filter(|playing| field(playing, "Id").is_some())
        .unwrap_or(details);
    let mut out = Map::new();
    if !by_type(item, &mut out) {
        return None;
    }
    let kind = context.kind;
    let source = kind.source();
    // The id of a picture goes into an address. A text that is no id is no picture.
    let picture_id = |out: &Map<String, Value>, key: &str| {
        Some(text(out.get(key))).filter(|id| is_id(id) && id != "0")
    };
    let key_of = |out: &Map<String, Value>, key: &str| text(out.get(key));

    out.insert(
        "uid".to_owned(),
        item.get("Id").cloned().unwrap_or(Value::Null),
    );
    let has_poster = field(item, "ImageTags")
        .and_then(|tags| field(tags, "Primary"))
        .is_some();
    let poster_type = if has_poster {
        json!("Primary")
    } else {
        json!(false)
    };
    out.insert("imageType".to_owned(), poster_type);
    progress_of(details, item, &mut out);
    let (user, player_address) = if context.show_names {
        (
            text(details.get("UserName")),
            field(details, "RemoteEndPoint")
                .map_or_else(|| "x.x.x.x".to_owned(), |a| text(Some(a))),
        )
    } else {
        (String::new(), "x.x.x.x".to_owned())
    };
    out.insert("user".to_owned(), json!(user));
    out.insert("userThumb".to_owned(), json!(""));
    out.insert("userAddress".to_owned(), json!(player_address));
    let address = if context.link.is_empty() {
        Value::Null
    } else {
        json!(
            context
                .link
                .replace("{id}", &text(item.get("Id")))
                .replace("{serverId}", &text(item.get("ServerId")))
        )
    };
    out.insert("address".to_owned(), address);

    let wide_type = text(out.get("nowPlayingImageType"));
    let poster_type = if has_poster { "Primary" } else { "" };
    let (wide_id, wide_key) = (
        picture_id(&out, "nowPlayingThumb"),
        key_of(&out, "nowPlayingKey"),
    );
    let (poster_id, poster_key) = (picture_id(&out, "thumb"), key_of(&out, "key"));
    let wide_route = picture_route(
        kind,
        &wide_type,
        wide_id.as_deref().unwrap_or_default(),
        context.wide,
        &wide_key,
    );
    let poster_route = picture_route(
        kind,
        poster_type,
        poster_id.as_deref().unwrap_or_default(),
        context.poster,
        &poster_key,
    );
    out.insert(
        "nowPlayingOriginalImage".to_owned(),
        json!(format!("{wide_route}${}", context.fresh)),
    );
    out.insert(
        "originalImage".to_owned(),
        json!(format!("{poster_route}${}", context.fresh)),
    );
    out.insert("openTab".to_owned(), json!(context.open_tab));
    out.insert("tabName".to_owned(), json!(context.tab_name));
    out.insert("userStream".to_owned(), stream_of(details));
    out.insert("metadata".to_owned(), metadata_of(item, context));

    // A picture that the cache holds comes from there, and else from the route that
    // fetches it.
    let cached_or_route = |route: String, key: &str| {
        if (context.is_cached)(key) {
            format!("data/cache/{source}-{key}.jpg")
        } else {
            route
        }
    };
    out.insert(
        "nowPlayingImageURL".to_owned(),
        json!(cached_or_route(wide_route, &wide_key)),
    );
    out.insert(
        "imageURL".to_owned(),
        json!(cached_or_route(poster_route, &poster_key)),
    );
    if wide_id.is_none() {
        let none = json!("plugins/images/homepage/no-np.png");
        out.insert("nowPlayingOriginalImage".to_owned(), none.clone());
        out.insert("nowPlayingImageURL".to_owned(), none);
        out.insert("nowPlayingKey".to_owned(), json!("no-np"));
    }
    if poster_id.is_none() {
        let none = json!("plugins/images/homepage/no-list.png");
        out.insert("originalImage".to_owned(), none.clone());
        out.insert("imageURL".to_owned(), none);
        out.insert("key".to_owned(), json!("no-list"));
    }
    Some(Value::Object(out))
}

/// The entries of a list of the server that the homepage shows.
pub fn items(list: &Value, context: &ItemContext<'_>) -> Vec<Value> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter(|entry| field(entry, "NowPlayingItem").is_some() || field(entry, "Name").is_some())
        .filter_map(|entry| item(entry, context))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{ItemContext, Kind, is_id, item, items, user_of};

    fn context(kind: Kind, is_cached: &(dyn Fn(&str) -> bool + Sync)) -> ItemContext<'_> {
        ItemContext {
            kind,
            show_names: true,
            link: "https://media.example.com/item?id={id}&serverId={serverId}",
            open_tab: false,
            tab_name: "",
            poster: (400, 600),
            wide: (2400, 1350),
            fresh: "abc",
            is_cached,
        }
    }

    fn episode() -> Value {
        json!({
            "UserName": "robin",
            "Client": "Emby Web",
            "DeviceName": "Chrome",
            "DeviceId": "device-one",
            "RemoteEndPoint": "192.0.2.50",
            "PlayState": {"PositionTicks": 6_000_000_000_i64, "IsPaused": true, "PlayMethod": "Transcode"},
            "TranscodingInfo": {"CompletionPercentage": 41.7, "Bitrate": 4_000_500, "VideoCodec": "h264", "AudioCodec": "aac", "AudioChannels": 2},
            "NowPlayingItem": {
                "Id": "301", "ServerId": "server-one", "Type": "Episode", "Name": "The One",
                "SeriesName": "The Show", "SeriesId": "30", "ParentIndexNumber": 2, "IndexNumber": 3,
                "ParentBackdropItemId": "30", "ParentBackdropImageTags": ["tag"],
                "ImageTags": {"Primary": "tag"}, "RunTimeTicks": 24_000_000_000_i64, "Container": "mkv",
                "MediaStreams": [{"Codec": "hevc", "Width": 1920}, {"Codec": "eac3"}],
            },
        })
    }

    #[test]
    fn session_of_an_episode_reads_as_the_php_application_builds_it() {
        let none = |_: &str| false;
        let built = item(&episode(), &context(Kind::Emby, &none)).expect("an item");

        assert_eq!(built["type"], "tv");
        assert_eq!(built["title"], "The Show");
        assert_eq!(built["thumb"], "30");
        assert_eq!(built["key"], "30-list");
        assert_eq!(built["nowPlayingKey"], "30-np");
        assert_eq!(built["nowPlayingImageType"], "Backdrop");
        assert_eq!(built["nowPlayingTitle"], "The Show - The One");
        assert_eq!(built["nowPlayingBottom"], "S2 · E3");
        assert_eq!(built["imageType"], "Primary");
        assert_eq!(built["elapsed"], 6_000_000_000_i64);
        assert_eq!(built["watched"], 25);
        assert_eq!(built["transcoded"], 41);
        assert_eq!(built["bandwidth"], 4000.5);
        assert_eq!(built["sessionType"], "Transcoding");
        assert_eq!(built["state"], "pause");
        assert_eq!(built["user"], "robin");
        assert_eq!(built["userAddress"], "192.0.2.50");
        assert_eq!(
            built["address"],
            "https://media.example.com/item?id=301&serverId=server-one"
        );
        assert_eq!(
            built["nowPlayingImageURL"],
            "api/v2/homepage/image?source=emby&type=Backdrop&img=30&height=1350&width=2400&key=30-np"
        );
        assert_eq!(
            built["originalImage"],
            "api/v2/homepage/image?source=emby&type=Primary&img=30&height=600&width=400&key=30-list$abc"
        );
        assert_eq!(built["userStream"]["videoResolution"], 1920);
        assert_eq!(built["userStream"]["sourceAudioCodec"], "eac3");
        assert_eq!(built["userStream"]["videoDecision"], "Transcode");
        assert_eq!(built["metadata"]["genres"], "");
        assert_eq!(built["metadata"]["actors"], "");
    }

    #[test]
    fn names_and_addresses_stay_back_and_cached_pictures_come_from_the_cache() {
        let cached = |key: &str| key == "30-list" || key == "9-cast";
        let mut context = context(Kind::Jellyfin, &cached);
        context.show_names = false;
        context.link = "";
        let mut session = episode();
        session["NowPlayingItem"]["People"] = json!([
            {"Id": "9", "Name": "An Actor", "Role": "The Hero", "PrimaryImageTag": "tag"},
            {"Id": "10", "Name": "Another", "Role": "", "PrimaryImageTag": "tag"},
            {"Id": "11", "Name": "A Third", "Role": "The Friend", "PrimaryImageTag": "tag"},
        ]);
        let built = item(&session, &context).expect("an item");

        assert_eq!(built["user"], "");
        assert_eq!(built["userAddress"], "x.x.x.x");
        assert_eq!(built["address"], Value::Null);
        assert_eq!(built["imageURL"], "data/cache/jellyfin-30-list.jpg");
        assert_eq!(
            built["metadata"]["actors"],
            json!([
                {"name": "An Actor", "role": "The Hero", "thumb": "data/cache/jellyfin-9-cast.jpg"},
                {"name": "A Third", "role": "The Friend", "thumb": "api/v2/homepage/image?source=jellyfin&type=Primary&img=11&height=450&width=300&key=11-cast"},
            ])
        );
    }

    #[test]
    fn item_without_pictures_names_the_pictures_for_no_picture() {
        let none = |_: &str| false;
        let context = context(Kind::Emby, &none);
        let built = item(
            &json!({"Id": "301", "Type": "Episode", "Name": "The One", "SeriesName": "The Show"}),
            &context,
        )
        .expect("an item");
        assert_eq!(built["nowPlayingThumb"], false);
        assert_eq!(built["nowPlayingKey"], "no-np");
        assert_eq!(
            built["nowPlayingImageURL"],
            "plugins/images/homepage/no-np.png"
        );
        assert_eq!(built["elapsed"], Value::Null);
        assert_eq!(built["transcoded"], 100);
        assert_eq!(built["bandwidth"], "");

        // An id that could change the address of the picture is no id.
        let built = item(
            &json!({"Id": "5&source=plex", "Type": "Movie", "Name": "The Film"}),
            &context,
        )
        .expect("an item");
        assert_eq!(built["imageURL"], "plugins/images/homepage/no-list.png");
        assert_eq!(built["key"], "no-list");
    }

    #[test]
    fn list_keeps_the_types_that_the_homepage_shows() {
        let none = |_: &str| false;
        let list = json!([
            {"DeviceName": "An idle player"},
            {"Id": "1", "Type": "Movie", "Name": "The Film", "ProductionYear": 2019},
            {"Id": "2", "Type": "TvChannel", "Name": "A channel"},
            {"Id": "3", "Type": "Audio", "Name": "A Song", "AlbumId": "4", "Album": "An Album", "AlbumArtist": "A Band"},
            {"Id": "5", "Type": "Series", "Name": "The Show"},
        ]);
        let built = items(&list, &context(Kind::Emby, &none));

        let titles: Vec<&str> = built
            .iter()
            .filter_map(|item| item["title"].as_str())
            .collect();
        assert_eq!(titles, ["The Film", "A Song", "The Show"]);
        assert_eq!(built[0]["nowPlayingBottom"], 2019);
        assert_eq!(built[0]["nowPlayingImageType"], false);
        assert_eq!(built[1]["type"], "music");
        assert_eq!(built[1]["metadataKey"], "4");
        assert_eq!(built[1]["nowPlayingTitle"], "A Band - A Song");
        assert_eq!(built[1]["nowPlayingImageType"], "Backdrop");
        assert!(built[2].get("nowPlayingTitle").is_none());
        assert_eq!(
            items(&Value::Null, &context(Kind::Emby, &none)),
            Vec::<Value>::new()
        );
    }

    #[test]
    fn user_is_the_one_with_the_name_or_else_an_administrator() {
        let users = json!([
            {"Id": "1", "Name": "Owner", "Policy": {"IsAdministrator": true}},
            {"Id": "2", "Name": "Robin", "Policy": {"IsAdministrator": false}},
            {"Id": "../3", "Name": "Odd"},
        ]);

        assert_eq!(user_of(&users, Some("robin")).as_deref(), Some("2"));
        assert_eq!(user_of(&users, Some("zed")).as_deref(), Some("1"));
        assert_eq!(user_of(&users, None).as_deref(), Some("1"));
        assert_eq!(user_of(&users, Some("odd")), None);
        assert_eq!(user_of(&json!([{"Id": "2", "Name": "Robin"}]), None), None);
        assert_eq!(user_of(&Value::Null, Some("robin")), None);
    }

    #[test]
    fn id_is_letters_digits_and_hyphens() {
        assert!(is_id("815"));
        assert!(is_id("0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0"));
        for wrong in ["", "1/2", "..", "1?x=2", "a b", &"1".repeat(65)] {
            assert!(!is_id(wrong), "{wrong}");
        }
    }

    #[test]
    fn key_goes_in_the_header_of_each_server() {
        assert_eq!(
            Kind::Emby.key_header("abc"),
            ("X-Emby-Token", "abc".to_owned())
        );
        assert_eq!(
            Kind::Jellyfin.key_header("a\"bc"),
            ("Authorization", "MediaBrowser Token=\"abc\"".to_owned())
        );
    }
}
