//! Sonarr, Radarr and Lidarr: their calendars and their download queues.
//!
//! The three applications share one API. Sonarr and Radarr have version 3 of it, and
//! Lidarr has version 1. The functions that format a calendar give the events in the
//! form of the PHP application, which the calendar of the homepage reads.

use std::time::Duration;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::{Map, Value, json};

use crate::http::Outbound;

/// The picture of an event that has none.
pub const NO_PICTURE: &str = "/plugins/images/homepage/no-np.png";

/// One of the three applications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Series.
    Sonarr,
    /// Films.
    Radarr,
    /// Music.
    Lidarr,
}

impl Kind {
    const fn api(self) -> &'static str {
        match self {
            Self::Sonarr | Self::Radarr => "v3",
            Self::Lidarr => "v1",
        }
    }
}

/// Why an application gave no usable answer.
#[derive(Debug, thiserror::Error)]
pub enum ArrError {
    /// The application cannot be reached, or it did not answer in time.
    #[error("the application cannot be reached: {0}")]
    Unreachable(#[source] reqwest::Error),
    /// The application answered with a status that is no success.
    #[error("the application answered with the status {0}")]
    Status(u16),
    /// The answer is not JSON.
    #[error("the answer cannot be read: {0}")]
    Unreadable(#[source] reqwest::Error),
}

/// One install of an application, and how it is asked.
#[derive(Debug, Clone, Copy)]
pub struct Install<'a> {
    /// Which application.
    pub kind: Kind,
    /// The address, complete.
    pub url: &'a str,
    /// The API key.
    pub key: &'a str,
    /// How long the install has to answer. Without a limit, 30 seconds.
    pub timeout: Option<Duration>,
    /// Whether the certificate of the install is left unchecked.
    pub never_check_certificate: bool,
}

impl Install<'_> {
    async fn get(
        &self,
        http: &Outbound,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Value, ArrError> {
        let url = format!(
            "{}/api/{}/{path}",
            self.url.trim_end_matches('/'),
            self.kind.api()
        );
        let response = http
            .keyed_client(&url, self.never_check_certificate)
            .get(&url)
            .query(query)
            .header("X-Api-Key", self.key)
            .timeout(self.timeout.unwrap_or(Duration::from_secs(30)))
            .send()
            .await
            .map_err(ArrError::Unreachable)?;
        if !response.status().is_success() {
            return Err(ArrError::Status(response.status().as_u16()));
        }
        response.json().await.map_err(ArrError::Unreadable)
    }

    /// The entries of the calendar from `start` to `end`, which are dates such as
    /// `2026-11-01`.
    pub async fn calendar(
        &self,
        http: &Outbound,
        start: &str,
        end: &str,
        unmonitored: bool,
    ) -> Result<Vec<Value>, ArrError> {
        let mut query = vec![("start", start), ("end", end)];
        if unmonitored {
            query.push(("unmonitored", "true"));
        }
        match self.kind {
            Kind::Sonarr => query.push(("includeSeries", "true")),
            Kind::Lidarr => query.push(("includeArtist", "true")),
            Kind::Radarr => {}
        }
        match self.get(http, "calendar", &query).await? {
            Value::Array(entries) => Ok(entries),
            _ => Ok(Vec::new()),
        }
    }

    /// The downloads in the queue.
    pub async fn queue(&self, http: &Outbound) -> Result<Vec<Value>, ArrError> {
        let mut query = vec![("includeUnknownSeriesItems", "false"), ("pageSize", "1000")];
        if self.kind == Kind::Sonarr {
            query.push(("includeSeries", "true"));
            query.push(("includeEpisode", "true"));
        }
        match self.get(http, "queue", &query).await? {
            Value::Object(mut page) => match page.remove("records") {
                Some(Value::Array(records)) => Ok(records),
                _ => Ok(Vec::new()),
            },
            Value::Array(records) => Ok(records),
            _ => Ok(Vec::new()),
        }
    }
}

/// What the events of one application share: the settings of its links.
#[derive(Debug, Clone, Copy)]
pub struct Links<'a> {
    /// The address that a link of an event starts with: the "calendar link" setting,
    /// or else the first address of the application.
    pub base: &'a str,
    /// The `...FrameTarget` setting.
    pub frame: &'a str,
    /// The `...Icon` setting.
    pub show_link: bool,
}

/// A picture that the server must fetch and keep, because the browser cannot load it
/// from the application without the API key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// The name of the file in the picture cache, without its ending.
    pub name: String,
    /// Where the picture comes from: an address, or a path of the application.
    pub source: String,
}

/// The events of a calendar, and the pictures that they need from the cache.
#[derive(Debug, Default, PartialEq)]
pub struct Events {
    /// The events, in the form of the PHP application.
    pub events: Vec<Value>,
    /// The pictures to fetch.
    pub pictures: Vec<Picture>,
}

/// A time of an answer as a moment, as PHP's `strtotime` reads it. A text that is no
/// time is the start of 1970, as in PHP.
fn moment(value: &Value) -> Timestamp {
    let text = value.as_str().unwrap_or_default();
    text.parse::<Timestamp>()
        .or_else(|_| {
            // A date without a time is the start of that day.
            format!("{text}T00:00:00Z").parse::<Timestamp>()
        })
        .unwrap_or(Timestamp::UNIX_EPOCH)
}

/// Whether PHP takes the value for true in a loose comparison with `"1"`.
fn is_true(value: &Value) -> bool {
    match value {
        Value::Bool(on) => *on,
        Value::Number(number) => number.as_f64() == Some(1.0),
        Value::String(text) => text == "1",
        _ => false,
    }
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

/// A number or a text as PHP writes it into a text.
fn written(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        _ => String::new(),
    }
}

fn two_digits(value: &Value) -> String {
    format!("{:02}", value.as_i64().unwrap_or(0))
}

/// The link of an event: the base, a path and the id, in lower case.
fn link(base: &str, part: &str, id: &str) -> String {
    if base.is_empty() {
        return String::new();
    }
    format!("{base}/{part}/{id}")
        .replace(&format!("//{part}/"), &format!("/{part}/"))
        .to_lowercase()
}

/// The name of a series as Sonarr writes it into the address of its page.
fn series_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut in_space = false;
    for character in name.chars() {
        if character.is_whitespace() {
            if !in_space {
                slug.push('-');
            }
            in_space = true;
        } else {
            in_space = false;
            slug.push(character);
        }
    }
    slug.replace('&', "and")
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || *character == ' ' || *character == '-'
        })
        .collect()
}

/// What the file of an episode or a film says about itself, or `"unknown"` without a
/// file.
fn of_file(has_file: bool, value: Option<&Value>) -> Value {
    if has_file {
        value.cloned().unwrap_or(Value::Null)
    } else {
        json!("unknown")
    }
}

/// The events of a Sonarr calendar. `number` is the place of the install in the
/// settings, from 0.
pub fn sonarr_events(entries: &[Value], number: usize, links: Links<'_>, now: Timestamp) -> Events {
    let mut out = Events::default();
    for (index, child) in entries.iter().enumerate() {
        let series = &child["series"];
        let name = text(&series["title"]);
        let id = written(&series["tvdbId"]);
        let aired = moment(&child["airDateUtc"]);
        let not_aired = now < aired;
        let has_file = is_true(&child["hasFile"]);
        let is_first = is_true(&child["episodeNumber"]);

        // An episode that is not the first of its season starts one second later, so
        // the first one stands above it in the calendar.
        let start = if is_first {
            child["airDateUtc"].clone()
        } else {
            let later = aired.checked_add(Duration::from_secs(1)).unwrap_or(aired);
            json!(later.strftime("%Y-%m-%dT%H:%M:%SZ").to_string())
        };
        let state = if has_file {
            "text-success"
        } else if not_aired && is_first {
            "text-primary animated flash"
        } else if not_aired && !is_true(&child["monitored"]) {
            "text-dark"
        } else if not_aired {
            "text-info"
        } else {
            "text-danger"
        };

        // The last fan art counts, and its address on the internet before its path
        // in Sonarr.
        let mut source = None;
        for image in series["images"].as_array().into_iter().flatten() {
            if text(&image["coverType"]) != "fanart" {
                continue;
            }
            for field in ["url", "remoteUrl"] {
                if !text(&image[field]).is_empty() {
                    source = Some(text(&image[field]).to_owned());
                }
            }
        }
        let picture = match source {
            Some(source) if !id.is_empty() => {
                let name = format!("tv-{id}");
                let path = format!("data/cache/{name}.jpg");
                out.pictures.push(Picture { name, source });
                path
            }
            _ => NO_PICTURE.to_owned(),
        };

        let file = &child["episodeFile"];
        let (ratings, quality) = (&series["ratings"], &file["quality"]);
        let media = file.get("mediaInfo");
        let has_media = has_file && media.is_some();
        let season_count = series["seasons"].as_array().map_or(0, Vec::len);
        let details = json!({
            "seasonCount": season_count,
            "status": series["status"],
            "topTitle": name,
            "bottomTitle": format!(
                "S{}E{} - {}",
                two_digits(&child["seasonNumber"]),
                two_digits(&child["episodeNumber"]),
                text(&child["title"])
            ),
            "overview": child.get("overview").filter(|value| !value.is_null()).cloned().unwrap_or_else(|| json!("")),
            "runtime": series["runtime"],
            "image": picture,
            "ratings": ratings["value"],
            "videoQuality": quality["quality"].get("name").filter(|_| has_file).cloned().unwrap_or_else(|| json!("unknown")),
            "audioChannels": of_file(has_media, media.and_then(|media| media.get("audioChannels"))),
            "audioCodec": of_file(has_media, media.and_then(|media| media.get("audioCodec"))),
            "videoCodec": of_file(has_media, media.and_then(|media| media.get("videoCodec"))),
            "size": file.get("size").filter(|_| has_file).cloned().unwrap_or_else(|| json!("unknown")),
            "genres": series["genres"],
            "href": link(links.base, "series", &series_slug(name)),
            "icon": "/plugins/images/tabs/sonarr.png",
            "frame": links.frame,
            "showLink": links.show_link,
        });
        out.events.push(json!({
            "id": format!("Sonarr-{number}-{}", index + 1),
            "title": name,
            "start": start,
            "className": format!("inline-popups bg-calendar calendar-item tvID--{id}"),
            "imagetype": format!("tv {state}"),
            "imagetypeFilter": "tv",
            "downloadFilter": state,
            "bgColor": state.replace("text", "bg"),
            "details": details,
        }));
    }
    out
}

/// Which release dates of a film the calendar shows.
#[derive(Debug, Clone, Copy)]
pub struct Releases {
    /// The release on a disc.
    pub physical: bool,
    /// The release for download.
    pub digital: bool,
    /// The release in cinemas.
    pub cinema: bool,
}

/// The other titles of a film, with commas between them. Radarr names the list in
/// two ways.
fn joined_titles(child: &Value) -> String {
    ["alternativeTitles", "alternateTitles"]
        .iter()
        .find_map(|field| child[*field].as_array().filter(|titles| !titles.is_empty()))
        .map_or_else(String::new, |titles| {
            titles
                .iter()
                .map(|title| text(&title["title"]))
                .collect::<Vec<_>>()
                .join(", ")
        })
}

/// The picture of a film: the address of its fan art on the internet, or a file of
/// the picture cache when only Radarr has the picture.
fn radarr_picture(child: &Value, id: &str, pictures: &mut Vec<Picture>) -> String {
    let mut picture = NO_PICTURE.to_owned();
    let mut source = None;
    for image in child["images"].as_array().into_iter().flatten() {
        let kind = text(&image["coverType"]);
        if kind != "banner" && kind != "fanart" {
            continue;
        }
        let (remote, local) = (text(&image["remoteUrl"]), text(&image["url"]));
        if kind == "fanart" && !remote.is_empty() {
            remote.clone_into(&mut picture);
            source = None;
        } else if local.contains("://") {
            local.clone_into(&mut picture);
            source = None;
        } else {
            // A path in Radarr, which only the server can load.
            source = Some(local.to_owned());
        }
    }
    if let Some(source) = source.filter(|source| !source.is_empty() && !id.is_empty()) {
        let name = format!("movie-{id}");
        picture = format!("data/cache/{name}.jpg");
        pictures.push(Picture { name, source });
    }
    picture
}

/// A field of a film, or the given value when the film has none.
fn or_else(child: &Value, field: &str, default: Value) -> Value {
    child
        .get(field)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(default)
}

/// The `details` of the event of a film.
fn radarr_details(child: &Value, picture: &str, href: &str, links: Links<'_>) -> Value {
    let has_file = is_true(&child["hasFile"]);
    let file = &child["movieFile"];
    let media = &file["mediaInfo"];
    let quality = &file["quality"];
    json!({
        "topTitle": child["title"],
        "bottomTitle": joined_titles(child),
        "status": child["status"],
        "overview": child["overview"],
        "runtime": child["runtime"],
        "image": picture,
        "ratings": or_else(&child["ratings"], "value", json!(0)),
        "videoQuality": of_file(has_file, quality["quality"].get("name")),
        "audioChannels": of_file(has_file, media.get("audioChannels")),
        "audioCodec": of_file(has_file, media.get("audioFormat")),
        "videoCodec": of_file(has_file, media.get("videoCodec")),
        "size": of_file(has_file, file.get("size")),
        "genres": child["genres"],
        "year": or_else(child, "year", json!("")),
        "studio": or_else(child, "studio", json!("")),
        "href": href,
        "icon": "/plugins/images/tabs/radarr.png",
        "frame": links.frame,
        "showLink": links.show_link,
    })
}

/// The events of a Radarr calendar: one for each release date of a film that the
/// settings show. Releases on the same day are one event.
pub fn radarr_events(
    entries: &[Value],
    number: usize,
    links: Links<'_>,
    releases: Releases,
    now: Timestamp,
    zone: &TimeZone,
) -> Events {
    let mut out = Events::default();
    let mut count = 0;
    let today = now.to_zoned_datetime(zone.clone()).date();
    for child in entries {
        let date = |field: &str| child.get(field).filter(|value| !value.is_null());
        let (physical, digital, cinema) = (
            date("physicalRelease"),
            date("digitalRelease"),
            date("inCinemas"),
        );
        let mut step = 0;
        while step < 3 {
            let mut kinds = Vec::new();
            let release = match (step, physical, digital, cinema) {
                (0, Some(release), _, _) if releases.physical => {
                    kinds.push("physical");
                    if digital == Some(release) {
                        kinds.push("digital");
                        step += 1;
                    }
                    if cinema == Some(release) {
                        kinds.push("cinema");
                        step += 2;
                    }
                    release
                }
                (1, _, Some(release), _) if releases.digital => {
                    kinds.push("digital");
                    if cinema == Some(release) {
                        kinds.push("cinema");
                        step += 1;
                    }
                    release
                }
                (2, _, _, Some(release)) if releases.cinema => {
                    kinds.push("cinema");
                    release
                }
                _ => {
                    step += 1;
                    continue;
                }
            };
            step += 1;
            count += 1;

            let id = written(&child["tmdbId"]);
            let day = moment(release).to_zoned_datetime(zone.clone()).date();
            let has_file = is_true(&child["hasFile"]);
            let state = if has_file {
                "text-success"
            } else if today < day {
                "text-info"
            } else {
                "text-danger"
            };

            let picture = radarr_picture(child, &id, &mut out.pictures);
            let details = radarr_details(child, &picture, &link(links.base, "movie", &id), links);
            out.events.push(json!({
                "id": format!("Radarr-{number}-{count}"),
                "title": child["title"],
                "start": day.strftime("%Y-%m-%d").to_string(),
                "className": format!("inline-popups bg-calendar movieID--{id}"),
                "imagetype": format!("film {state}"),
                "imagetypeFilter": "film",
                "downloadFilter": state,
                "releaseType": kinds,
                "bgColor": state.replace("text", "bg"),
                "details": details,
            }));
        }
    }
    out
}

/// The events of a Lidarr calendar.
pub fn lidarr_events(entries: &[Value], number: usize, links: Links<'_>, now: Timestamp) -> Events {
    let mut out = Events::default();
    for (index, child) in entries.iter().enumerate() {
        let artist = &child["artist"];
        let not_released = now < moment(&child["releaseDate"]);
        let (ratings, statistics) = (&artist["ratings"], &child["statistics"]);
        let complete = match &statistics["percentOfTracks"] {
            Value::Number(percent) => percent.as_f64() == Some(100.0),
            Value::String(percent) => percent.parse::<f64>().ok() == Some(100.0),
            _ => false,
        };
        let state = if complete {
            "text-success"
        } else if not_released {
            "text-info"
        } else {
            "text-danger"
        };
        let picture = artist["images"]
            .as_array()
            .into_iter()
            .flatten()
            .rfind(|image| text(&image["coverType"]) == "fanart")
            .map_or_else(
                || NO_PICTURE.to_owned(),
                |image| text(&image["url"]).replace("http://", "https://"),
            );
        let details = json!({
            "seasonCount": "",
            "status": "",
            "topTitle": child["title"],
            "bottomTitle": artist["artistName"],
            "overview": artist.get("overview").filter(|value| !value.is_null()).cloned().unwrap_or_else(|| json!("")),
            "runtime": "",
            "image": picture,
            "ratings": ratings["value"],
            "videoQuality": "unknown",
            "audioChannels": "unknown",
            "audioCodec": "unknown",
            "videoCodec": "unknown",
            "size": "unknown",
            "genres": child["genres"],
            "href": link(links.base, "artist", text(&artist["foreignArtistId"])),
            "icon": "/plugins/images/tabs/lidarr.png",
            "frame": links.frame,
            "showLink": links.show_link,
        });
        let mut event = Map::new();
        for (key, value) in [
            ("id", json!(format!("Lidarr-{number}-{}", index + 1))),
            ("title", artist["artistName"].clone()),
            ("start", child["releaseDate"].clone()),
            (
                "className",
                json!("inline-popups bg-calendar calendar-item musicID--"),
            ),
            ("imagetype", json!(format!("music {state}"))),
            ("imagetypeFilter", json!("music")),
            ("downloadFilter", json!(state)),
            ("bgColor", json!(state.replace("text", "bg"))),
            ("details", details),
            ("data", child.clone()),
        ] {
            event.insert(key.to_owned(), value);
        }
        out.events.push(Value::Object(event));
    }
    out
}
