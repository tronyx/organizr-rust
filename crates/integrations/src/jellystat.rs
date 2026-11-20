//! JellyStat: the statistics of an Emby or a Jellyfin server.
//!
//! The functions that format an answer give the statistics in the form of the PHP
//! application.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::emby::{Kind, is_id, picture_route};
use crate::http::{Outbound, escaped, without_address};
use crate::probe::Answer;

/// How long JellyStat has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// The number of rows of the history that one request asks for.
const PAGE_SIZE: usize = 1000;

/// The most pages of the history that are read, so that a history without an end
/// does not hold the request for ever.
const MOST_PAGES: usize = 100;

/// The picture of an item that has none.
const NO_PICTURE: &str = "plugins/images/homepage/no-np.png";

/// The picture of an actor who has none.
const NO_ACTOR: &str = "plugins/images/homepage/no-list.png";

/// The size of the picture of an item, and of an actor.
const ITEM_SIZE: (u32, u32) = (400, 600);
const ACTOR_SIZE: (u32, u32) = (300, 450);

/// One install of JellyStat, and how it is asked.
#[derive(Clone, Copy)]
pub struct Install<'a> {
    /// The address for the requests of the server, complete.
    pub url: &'a str,
    /// The API key.
    pub key: &'a str,
    /// Whether the certificate of the install is left unchecked.
    pub never_check_certificate: bool,
}

impl Install<'_> {
    /// Asks a path of the install. The key goes in a header, so it is in no address
    /// and in no log. Fails when the install cannot be reached. The text of the error
    /// names no address.
    pub async fn get(
        &self,
        http: &Outbound,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Answer, String> {
        let url = format!("{}{path}", self.url.trim_end_matches('/'));
        let mut request = http
            .client(&url, self.never_check_certificate)
            .get(&url)
            .timeout(TIME_LIMIT);
        if !query.is_empty() {
            request = request.query(query);
        }
        if !self.key.is_empty() {
            request = request.header("x-api-token", self.key);
        }
        let response = request.send().await.map_err(without_address)?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(without_address)?;
        Ok(Answer { status, body })
    }

    /// The libraries, or nothing when the install refused the request or sent
    /// something that is no list.
    pub async fn libraries(&self, http: &Outbound) -> Result<Option<Vec<Value>>, String> {
        let answer = self.get(http, "/api/getLibraries", &[]).await?;
        if !answer.is_success() {
            return Ok(None);
        }
        match serde_json::from_str(&answer.body) {
            Ok(Value::Array(libraries)) => Ok(Some(libraries)),
            _ => Ok(None),
        }
    }

    /// Every row of the history from the day `start` on. The rows come in pages, and
    /// the reading stops at the first page that does not come.
    pub async fn history(&self, http: &Outbound, start: &str) -> Result<Vec<Value>, String> {
        let size = PAGE_SIZE.to_string();
        let mut all = Vec::new();
        for page in 1..=MOST_PAGES {
            let page = page.to_string();
            let query = [
                ("page", page.as_str()),
                ("size", size.as_str()),
                ("startDate", start),
            ];
            let answer = self.get(http, "/api/getHistory", &query).await?;
            if !answer.is_success() {
                break;
            }
            let rows = match serde_json::from_str::<Value>(&answer.body) {
                Ok(Value::Object(mut body)) => match body.remove("results") {
                    Some(Value::Array(rows)) => rows,
                    _ => break,
                },
                _ => break,
            };
            let count = rows.len();
            all.extend(rows);
            if count != PAGE_SIZE {
                break;
            }
        }
        Ok(all)
    }

    /// What the install knows about one item, or nothing. `key` is the id of the
    /// item. The install is asked in the ways of the PHP application, one after the
    /// other.
    pub async fn item(&self, http: &Outbound, key: &str) -> Option<Value> {
        if !is_id(key) {
            return None;
        }
        let proxy = format!("/proxy/Items/{key}");
        let fields = [(
            "Fields",
            "Overview,People,Genres,CommunityRating,CriticRating,Studios,Taglines,ProductionYear,PremiereDate",
        )];
        let id = [("id", key)];
        let mut ways: Vec<(&str, &[(&str, &str)])> = Vec::new();
        if !self.key.is_empty() {
            ways.push(("/api/getItem", &id));
            ways.push(("/api/getItemById", &id));
        }
        ways.push((&proxy, &[]));
        ways.push((&proxy, &fields));
        for (path, query) in ways {
            let Ok(answer) = self.get(http, path, query).await else {
                continue;
            };
            if !answer.is_success() {
                continue;
            }
            if let Ok(Value::Object(details)) = serde_json::from_str::<Value>(&answer.body)
                && !details.is_empty()
                && !details.contains_key("error")
            {
                return Some(Value::Object(details));
            }
        }
        None
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

/// A field that is there and not `null`.
fn field<'a>(object: &'a Value, name: &str) -> Option<&'a Value> {
    object.get(name).filter(|value| !value.is_null())
}

/// A field, or the given value when the answer does not have it.
fn or_else(object: &Value, name: &str, otherwise: Value) -> Value {
    field(object, name).cloned().unwrap_or(otherwise)
}

/// A field that PHP does not take for empty.
fn filled<'a>(object: &'a Value, name: &str) -> Option<&'a Value> {
    object.get(name).filter(|value| truthy(value))
}

/// A value as PHP makes a text of it.
fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number
            .as_f64()
            .filter(|float| float.fract() == 0.0 && number.is_f64())
            .map_or_else(|| number.to_string(), |float| format!("{float:.0}")),
        Value::Bool(true) => "1".to_owned(),
        _ => String::new(),
    }
}

/// The number of a field: a number, or a text that holds one. Anything else is 0.
fn number(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or(0.0),
        Some(Value::String(text)) => text.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// A sum as JSON: a whole number when it is one.
fn counted(sum: f64) -> Value {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the number is whole and in the range that a float holds without a gap"
    )]
    if sum.fract() == 0.0 && sum.abs() < 9.0e15 {
        json!(sum as i64)
    } else {
        json!(sum)
    }
}

/// The word for a type of library.
pub fn collection_label(kind: &str) -> String {
    match kind {
        "movies" => "Movies".to_owned(),
        "tvshows" => "TV Shows".to_owned(),
        "music" => "Music".to_owned(),
        "mixed" => "Mixed Content".to_owned(),
        "unknown" => "Other".to_owned(),
        other => {
            let mut letters = other.chars();
            letters.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(letters).collect()
            })
        }
    }
}

/// A time of play in the words of the PHP application: minutes and seconds below one
/// hour, and hours, minutes and seconds from there. The number is in ticks, which are
/// ten-millionths of a second. The hours start again at 24, as in the PHP
/// application.
pub fn play_time(ticks: f64) -> String {
    if ticks == 0.0 {
        return "Unknown".to_owned();
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a fraction of a second is cut off, as in PHP"
    )]
    let seconds = ((ticks / 10_000_000.0) as i64).rem_euclid(86_400);
    let (hours, minutes, seconds) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
    if hours == 0 {
        format!("{minutes:02}:{seconds:02}")
    } else {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }
}

/// The type of a library as text.
fn collection_of(library: &Value) -> String {
    field(library, "CollectionType").map_or_else(|| "unknown".to_owned(), text)
}

/// The libraries of an answer, the sums over them, and the id of the server.
fn libraries_of(libraries: &[Value], now: &str) -> (Value, Value, Value) {
    let listed: Vec<Value> = libraries
        .iter()
        .map(|library| {
            let played = library.get("total_play_time");
            let kind = collection_of(library);
            json!({
                "name": or_else(library, "Name", json!("Unknown Library")),
                "type": collection_label(&kind),
                "item_count": or_else(library, "item_count", json!(0)),
                "season_count": or_else(library, "season_count", json!(0)),
                "episode_count": or_else(library, "episode_count", json!(0)),
                "total_play_time": if played.is_some_and(truthy) {
                    play_time(number(played))
                } else {
                    "0 min".to_owned()
                },
                "play_time_raw": or_else(library, "total_play_time", json!(0)),
                "collection_type": kind,
            })
        })
        .collect();

    let sum = |name: &str| -> f64 {
        libraries
            .iter()
            .map(|library| number(library.get(name)))
            .sum()
    };
    // The types stand in the order in which the libraries name them.
    let mut order: Vec<String> = Vec::new();
    let mut by_type: HashMap<String, (u32, f64, f64)> = HashMap::new();
    for library in libraries {
        let kind = collection_of(library);
        if !by_type.contains_key(&kind) {
            order.push(kind.clone());
        }
        let entry = by_type.entry(kind).or_default();
        entry.0 += 1;
        entry.1 += number(library.get("item_count"));
        entry.2 += number(library.get("total_play_time"));
    }
    let breakdown: Map<String, Value> = order
        .into_iter()
        .map(|kind| {
            let (count, items, played) = by_type.get(&kind).copied().unwrap_or_default();
            let entry = json!({
                "count": count,
                "items": counted(items),
                "play_time": counted(played),
                "label": collection_label(&kind),
            });
            (kind, entry)
        })
        .collect();
    let played = sum("total_play_time");
    let totals = json!({
        "total_libraries": libraries.len(),
        "total_items": counted(sum("item_count")),
        "total_seasons": counted(sum("season_count")),
        "total_episodes": counted(sum("episode_count")),
        "total_play_time": play_time(played),
        "total_play_time_raw": counted(played),
        // PHP writes a list without entries as `[]`.
        "type_breakdown": if breakdown.is_empty() { json!([]) } else { Value::Object(breakdown) },
    });
    let server = json!({
        "server_id": libraries
            .first()
            .and_then(|library| field(library, "ServerId"))
            .cloned()
            .unwrap_or_else(|| json!("Unknown")),
        "last_updated": now,
    });
    (Value::Array(listed), totals, server)
}

/// The first four digits of a date, when it starts with a year.
fn year_of_date(date: &Value) -> Option<String> {
    let year: String = date.as_str()?.chars().take(4).collect();
    (year.len() == 4 && year.chars().all(|c| c.is_ascii_digit())).then_some(year)
}

/// Whether four characters are a year of the two last centuries.
fn is_year(text: &str) -> bool {
    text.len() == 4
        && (text.starts_with("19") || text.starts_with("20"))
        && text.chars().all(|c| c.is_ascii_digit())
}

/// A year in brackets in a title, for example `Show (2019)`: the year, and the title
/// without it.
fn year_in_brackets(title: &str) -> Option<(String, String)> {
    let mut from = 0;
    while let Some(found) = title.get(from..).and_then(|rest| rest.find('(')) {
        let open = from + found;
        let year = title.get(open + 1..open + 5).filter(|year| is_year(year));
        if let Some(year) = year
            && title.get(open + 5..open + 6) == Some(")")
        {
            let without = format!(
                "{}{}",
                title.get(..open).unwrap_or_default(),
                title.get(open + 6..).unwrap_or_default()
            );
            return Some((year.to_owned(), without.trim().to_owned()));
        }
        from = open + 1;
    }
    None
}

/// A year that stands as a word of its own in a text.
fn year_word(text: &str) -> Option<String> {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let characters: Vec<char> = text.chars().collect();
    (0..characters.len().saturating_sub(3)).find_map(|start| {
        let word: String = characters.get(start..start + 4)?.iter().collect();
        let before = start.checked_sub(1).and_then(|index| characters.get(index));
        let after = characters.get(start + 4);
        (is_year(&word)
            && !before.is_some_and(|c| is_word(*c))
            && !after.is_some_and(|c| is_word(*c)))
        .then_some(word)
    })
}

/// What a row of the history is about.
struct Played {
    /// `movie`, `show` or `music`.
    kind: &'static str,
    /// What makes two rows rows of one item: the name of a series, or the id of an item.
    same: String,
    title: String,
    year: Option<String>,
    /// The id of the item for its picture.
    id: Value,
}

/// The first field of the list that PHP does not take for empty.
fn first_filled(row: &Value, names: &[&str]) -> Option<Value> {
    names.iter().find_map(|name| filled(row, name).cloned())
}

/// Reads one row of the history, as the PHP application does. Nothing for a row
/// that names no item.
fn played_of(row: &Value) -> Option<Played> {
    if let Some(series) = filled(row, "SeriesName") {
        let name = text(series);
        let mut title = name.clone();
        let year = filled(row, "SeriesProductionYear")
            .or_else(|| filled(row, "ProductionYear"))
            .map(text)
            .or_else(|| filled(row, "PremiereDate").and_then(year_of_date))
            .or_else(|| {
                let (year, without) = year_in_brackets(&title)?;
                title = without;
                Some(year)
            })
            .or_else(|| filled(row, "EpisodeName").and_then(|name| year_word(&text(name))));
        let id = first_filled(row, &["SeriesId", "ShowId", "NowPlayingItemId", "ParentId"])
            .unwrap_or_else(|| json!(name));
        return Some(Played {
            kind: "show",
            same: name,
            title,
            year,
            id,
        });
    }
    let name = text(filled(row, "NowPlayingItemName")?);
    let has_video = row
        .get("MediaStreams")
        .and_then(Value::as_array)
        .is_some_and(|streams| {
            streams
                .iter()
                .any(|stream| stream.get("Type").and_then(Value::as_str) == Some("Video"))
        });
    let id = field(row, "NowPlayingItemId").cloned();
    let same = id.as_ref().map_or_else(|| name.clone(), text);
    let id = id.unwrap_or_else(|| json!(name));
    // A row without a video, or one of less than ten minutes, counts as music.
    if !has_video || number(row.get("PlaybackDuration")) < 600.0 {
        return Some(Played {
            kind: "music",
            same,
            title: name,
            year: None,
            id,
        });
    }
    let mut title = name;
    let year = filled(row, "ProductionYear")
        .map(text)
        .or_else(|| filled(row, "PremiereDate").and_then(year_of_date))
        .or_else(|| {
            let (year, without) = year_in_brackets(&title)?;
            title = without;
            Some(year)
        });
    Some(Played {
        kind: "movie",
        same,
        title,
        year,
        id,
    })
}

/// One item of the history, with the count of its rows.
struct Counted {
    played: Played,
    count: u64,
    seconds: f64,
    server: Value,
    first: Option<String>,
    last: Option<String>,
}

/// The items of the history, each one with the number of times it was played: the
/// films, the series and the music, each list with the most played item first.
fn most_played(history: &[Value]) -> [Vec<Value>; 3] {
    let mut items: Vec<Counted> = Vec::new();
    let mut places: HashMap<String, usize> = HashMap::new();
    for row in history {
        let Some(played) = played_of(row) else {
            continue;
        };
        let name = format!("{}_{}", played.kind, played.same);
        let when = field(row, "ActivityDateInserted").map(text);
        let place = *places.entry(name).or_insert_with(|| {
            items.push(Counted {
                played,
                count: 0,
                seconds: 0.0,
                server: or_else(row, "ServerId", Value::Null),
                first: when.clone(),
                last: when.clone(),
            });
            items.len() - 1
        });
        let Some(item) = items.get_mut(place) else {
            continue;
        };
        item.count += 1;
        item.seconds += number(row.get("PlaybackDuration"));
        if let Some(when) = when.filter(|when| !when.is_empty()) {
            if item.last.as_ref().is_none_or(|last| &when > last) {
                item.last = Some(when.clone());
            }
            if item.first.as_ref().is_none_or(|first| &when < first) {
                item.first = Some(when);
            }
        }
    }
    // The order of the first rows stays for items with the same count.
    items.sort_by_key(|item| std::cmp::Reverse(item.count));
    let of = |kind: &str| -> Vec<Value> {
        items
            .iter()
            .filter(|item| item.played.kind == kind)
            .map(|item| {
                json!({
                    "id": item.played.id,
                    "title": item.played.title,
                    "type": item.played.kind,
                    "play_count": item.count,
                    "total_duration": counted(item.seconds),
                    "year": item.played.year,
                    "server_id": item.server,
                    "poster_path": null,
                    "first_played": item.first,
                    "last_played": item.last,
                })
            })
            .collect()
    };
    [of("movie"), of("show"), of("music")]
}

/// The users of the history, each one with the number of rows and the last one. The
/// user with the most rows is first.
fn users_of(history: &[Value]) -> Vec<Value> {
    let mut users: Vec<(String, u64, Option<String>)> = Vec::new();
    let mut places: HashMap<String, usize> = HashMap::new();
    for row in history {
        let name = field(row, "UserName")
            .or_else(|| field(row, "User"))
            .map_or_else(|| "Unknown User".to_owned(), text);
        let place = *places.entry(name.clone()).or_insert_with(|| {
            users.push((name, 0, None));
            users.len() - 1
        });
        let Some(user) = users.get_mut(place) else {
            continue;
        };
        user.1 += 1;
        let when = field(row, "ActivityDateInserted")
            .or_else(|| field(row, "Date"))
            .map(text)
            .filter(|when| !when.is_empty());
        if let Some(when) = when
            && user.2.as_ref().is_none_or(|last| &when > last)
        {
            user.2 = Some(when);
        }
    }
    users.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.cmp(&a.2)));
    users
        .into_iter()
        .map(
            |(name, count, last)| json!({"name": name, "play_count": count, "last_activity": last}),
        )
        .collect()
}

/// Which lists of the most played items the statistics hold.
#[derive(Debug, Clone, Copy)]
pub struct Shown {
    /// The days that the statistics look back.
    pub days: u32,
    /// Whether the films are listed.
    pub movies: bool,
    /// Whether the series are listed.
    pub shows: bool,
    /// Whether the music is listed.
    pub music: bool,
    /// The most items of a list.
    pub most: usize,
}

/// The statistics of `GET /homepage/jellystat`, from the libraries and the history
/// of an install. `now` is the moment of the answer, as text.
pub fn statistics(
    libraries: Option<&[Value]>,
    history: &[Value],
    shown: Shown,
    now: &str,
) -> Value {
    let (listed, totals, server) = libraries.map_or_else(
        || (json!([]), json!([]), json!([])),
        |libraries| libraries_of(libraries, now),
    );
    let [films, series, music] = most_played(history);
    let cut = |list: Vec<Value>, is_shown: bool| -> Vec<Value> {
        if is_shown {
            list.into_iter().take(shown.most).collect()
        } else {
            Vec::new()
        }
    };
    json!({
        "period": format!("{} days", shown.days),
        "libraries": listed,
        "library_totals": totals,
        "server_info": server,
        "users": users_of(history),
        "most_watched_movies": cut(films, shown.movies),
        "most_watched_shows": cut(series, shown.shows),
        "most_listened_music": cut(music, shown.music),
    })
}

/// The media server whose items JellyStat counts, when the homepage shows that server.
#[derive(Debug, Clone, Copy)]
pub struct MediaServer<'a> {
    /// Emby or Jellyfin.
    pub kind: Kind,
    /// The address of the server, complete.
    pub url: &'a str,
    /// The link to an item that the Admin set, or an empty text.
    pub link: &'a str,
    /// The name of the tab of the server in Organizr.
    pub tab_name: &'a str,
    /// Whether the link opens that tab.
    pub open_tab: bool,
}

/// The address of a picture that JellyStat passes on from the media server.
fn proxy_picture(jellystat: &str, id: &str, picture: &str, tag: Option<&str>) -> String {
    let tag = tag.map_or_else(String::new, |tag| format!("tag={}&", escaped(tag)));
    format!(
        "{}/proxy/Items/{}/Images/{picture}?{tag}fillWidth=400&quality=90",
        jellystat.trim_end_matches('/'),
        escaped(id)
    )
}

/// The address of the poster of an item at JellyStat, by the id of the item alone.
fn proxy_poster(jellystat: &str, id: &str) -> Option<String> {
    (!jellystat.is_empty() && !id.is_empty()).then(|| {
        format!(
            "{}/proxy/Items/Images/Primary?id={}&fillWidth=200&quality=90",
            jellystat.trim_end_matches('/'),
            escaped(id)
        )
    })
}

/// The type of picture that the details name, and its tag.
fn picture_of(details: &Value) -> (&'static str, Option<String>) {
    let tag = |pointer: &str| {
        details
            .pointer(pointer)
            .filter(|tag| !tag.is_null())
            .map(text)
    };
    if let Some(primary) = tag("/ImageTags/Primary") {
        ("Primary", Some(primary))
    } else if let Some(thumb) = tag("/ImageTags/Thumb") {
        ("Thumb", Some(thumb))
    } else if let Some(backdrop) = tag("/BackdropImageTags/0") {
        ("Backdrop", Some(backdrop))
    } else {
        ("Primary", None)
    }
}

/// The actors of the details, each one with a role.
fn actors_of(details: &Value, server: Option<&MediaServer<'_>>, jellystat: &str) -> Vec<Value> {
    let people = details.get("People").and_then(Value::as_array);
    people
        .into_iter()
        .flatten()
        .filter(|person| field(person, "Name").is_some() && filled(person, "Role").is_some())
        .map(|person| {
            let id = filled(person, "Id").map_or_default(text);
            let thumb = match server {
                _ if id.is_empty() => NO_ACTOR.to_owned(),
                // The picture comes through Organizr, so no key goes to the browser.
                Some(server) if is_id(&id) => picture_route(
                    server.kind,
                    "Primary",
                    &id,
                    ACTOR_SIZE,
                    &format!("{id}-cast"),
                ),
                Some(_) => NO_ACTOR.to_owned(),
                None => format!(
                    "{}/proxy/Items/{}/Images/Primary?fillWidth=300&quality=90",
                    jellystat.trim_end_matches('/'),
                    escaped(&id)
                ),
            };
            json!({
                "name": text(&person["Name"]),
                "role": text(&person["Role"]),
                "thumb": thumb,
            })
        })
        .collect()
}

/// What the item of the details shows.
struct Shows {
    title: String,
    kind: &'static str,
    year: String,
    summary: String,
    tagline: String,
    genres: Value,
    actors: Vec<Value>,
    rating: String,
    duration: String,
    picture: String,
    server_id: String,
}

fn shows_of(
    key: &str,
    details: Option<&Value>,
    server: Option<&MediaServer<'_>>,
    jellystat: &str,
) -> Shows {
    let Some(details) =
        details.filter(|details| details.as_object().is_some_and(|d| !d.is_empty()))
    else {
        return Shows {
            title: "Unknown Item".to_owned(),
            kind: "movie",
            year: String::new(),
            summary: "This item is available in your media library. Click to view more details in JellyStat.".to_owned(),
            tagline: "View in JellyStat".to_owned(),
            genres: json!([]),
            actors: Vec::new(),
            rating: "0".to_owned(),
            duration: "0".to_owned(),
            picture: proxy_poster(jellystat, key).unwrap_or_else(|| NO_PICTURE.to_owned()),
            server_id: String::new(),
        };
    };
    let mut title = field(details, "Name")
        .or_else(|| field(details, "OriginalTitle"))
        .map_or_else(|| "Unknown Item".to_owned(), text);
    let tagline = match field(details, "Taglines") {
        Some(taglines) => taglines.get(0).map_or_default(text),
        None => field(details, "Tagline").map_or_default(text),
    };
    let year = field(details, "ProductionYear")
        .map(text)
        .or_else(|| field(details, "PremiereDate").and_then(year_of_date))
        .unwrap_or_default();
    let rating = field(details, "CommunityRating")
        .or_else(|| field(details, "CriticRating"))
        .filter(|rating| {
            rating.is_number() || rating.as_str().is_some_and(|t| t.parse::<f64>().is_ok())
        })
        .map_or_else(|| "0".to_owned(), text);
    // A tick is one ten-millionth of a second, and the homepage wants milliseconds.
    let duration = field(details, "RunTimeTicks")
        .filter(|ticks| ticks.is_number())
        .map_or_else(
            || "0".to_owned(),
            |ticks| format!("{:.0}", (number(Some(ticks)) / 10_000.0).floor()),
        );
    let kind = match text(&details["Type"]).to_lowercase().as_str() {
        "series" => "tv",
        "episode" => {
            if let Some(series) = field(details, "SeriesName") {
                title = text(series);
            }
            "tv"
        }
        "audio" | "musicalbum" | "musicvideo" => "music",
        _ => "movie",
    };
    let id = field(details, "Id").map_or_else(|| key.to_owned(), text);
    let (picture_type, tag) = picture_of(details);
    let picture = match server {
        Some(server) if is_id(&id) => picture_route(
            server.kind,
            picture_type,
            &id,
            ITEM_SIZE,
            &format!("{id}-jellystat"),
        ),
        Some(_) => NO_PICTURE.to_owned(),
        None if tag.is_some() => proxy_picture(jellystat, &id, picture_type, tag.as_deref()),
        None => proxy_poster(jellystat, &id).unwrap_or_else(|| NO_PICTURE.to_owned()),
    };
    Shows {
        title,
        kind,
        year,
        summary: field(details, "Overview").map_or_default(text),
        tagline,
        genres: field(details, "Genres")
            .filter(|genres| genres.is_array())
            .cloned()
            .unwrap_or_else(|| json!([])),
        actors: actors_of(details, server, jellystat),
        rating,
        duration,
        picture,
        server_id: field(details, "ServerId").map_or_default(text),
    }
}

/// The address of an item in the web interface of the media server: the link of the
/// Admin with the ids in it, or the page of the item at the address of the server.
fn link_of(server: Option<&MediaServer<'_>>, key: &str, server_id: &str) -> String {
    let Some(server) = server.filter(|server| !server.url.is_empty()) else {
        return String::new();
    };
    if server.link.is_empty() {
        format!(
            "{}/web/index.html#!/item?id={key}&serverId={server_id}",
            server.url.trim_end_matches('/')
        )
    } else {
        server
            .link
            .replace("{id}", key)
            .replace("{serverId}", server_id)
    }
}

/// The item of `POST /homepage/jellystat/metadata`, in the form of the items of the
/// media servers. `details` is what the media server or JellyStat knows about the
/// item, when one of them answered. `jellystat` is the address of JellyStat for the
/// browser.
pub fn details_item(
    key: &str,
    details: Option<&Value>,
    server: Option<&MediaServer<'_>>,
    jellystat: &str,
) -> Value {
    let shows = shows_of(key, details, server, jellystat);
    json!({
        "uid": key,
        "title": shows.title,
        "secondaryTitle": "",
        "type": shows.kind,
        "ratingKey": key,
        "thumb": key,
        "key": format!("{key}-list"),
        "nowPlayingThumb": key,
        "nowPlayingKey": format!("{key}-np"),
        "metadataKey": key,
        "nowPlayingImageURL": shows.picture,
        "imageURL": shows.picture,
        "originalImage": shows.picture,
        "nowPlayingOriginalImage": shows.picture,
        "address": link_of(server, key, &shows.server_id),
        "tabName": server.map_or("", |server| server.tab_name),
        "openTab": server.is_some_and(|server| server.open_tab),
        "metadata": {
            "guid": key,
            "summary": shows.summary,
            "rating": shows.rating,
            "duration": shows.duration,
            "originallyAvailableAt": "",
            "year": shows.year,
            "tagline": shows.tagline,
            "genres": shows.genres,
            "actors": shows.actors,
        },
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{
        MediaServer, Shown, collection_label, details_item, play_time, statistics,
        year_in_brackets, year_word,
    };
    use crate::emby::Kind;

    const SHOWN: Shown = Shown {
        days: 30,
        movies: true,
        shows: true,
        music: true,
        most: 10,
    };

    fn video(seconds: u32) -> Value {
        json!({"MediaStreams": [{"Type": "Video"}], "PlaybackDuration": seconds})
    }

    fn row(more: &Value, base: &Value) -> Value {
        let mut row = base.clone();
        if let (Some(row), Some(more)) = (row.as_object_mut(), more.as_object()) {
            row.extend(more.clone());
        }
        row
    }

    #[test]
    fn time_of_play_reads_as_the_php_application_writes_it() {
        assert_eq!(play_time(0.0), "Unknown");
        assert_eq!(play_time(900_000_000.0), "01:30");
        assert_eq!(play_time(36_610_000_000.0), "01:01:01");
        // The hours start again at 24.
        assert_eq!(play_time(900_000_000_000.0), "01:00:00");
    }

    #[test]
    fn type_of_a_library_has_a_word() {
        assert_eq!(collection_label("tvshows"), "TV Shows");
        assert_eq!(collection_label("unknown"), "Other");
        assert_eq!(collection_label("books"), "Books");
        assert_eq!(collection_label(""), "");
    }

    #[test]
    fn year_is_found_in_a_title() {
        assert_eq!(
            year_in_brackets("The Show (2019)"),
            Some(("2019".to_owned(), "The Show".to_owned()))
        );
        assert_eq!(
            year_in_brackets("(US) The Office (2005) again"),
            Some(("2005".to_owned(), "(US) The Office  again".to_owned()))
        );
        assert_eq!(year_in_brackets("The Show (1819)"), None);
        assert_eq!(year_in_brackets("The Show 2019"), None);
        assert_eq!(year_word("Special of 2021!").as_deref(), Some("2021"));
        assert_eq!(year_word("Episode 12021"), None);
        assert_eq!(year_word("20215 A"), None);
        assert_eq!(year_word("1999"), Some("1999".to_owned()));
    }

    #[test]
    fn libraries_are_listed_and_summed() {
        let libraries = [
            json!({"Name": "Films", "CollectionType": "movies", "item_count": 120, "total_play_time": 36_610_000_000_i64, "ServerId": "made-up-server"}),
            json!({"Name": "Series", "CollectionType": "tvshows", "item_count": "40", "season_count": 90, "episode_count": 800, "total_play_time": 0}),
            json!({"CollectionType": "movies", "item_count": 5}),
        ];
        let stats = statistics(Some(&libraries), &[], SHOWN, "2026-11-20T10:00:00+00:00");
        assert_eq!(stats["period"], "30 days");
        assert_eq!(
            stats["libraries"][0],
            json!({
                "name": "Films", "type": "Movies", "item_count": 120, "season_count": 0,
                "episode_count": 0, "total_play_time": "01:01:01",
                "play_time_raw": 36_610_000_000_i64, "collection_type": "movies",
            })
        );
        assert_eq!(stats["libraries"][1]["total_play_time"], "0 min");
        assert_eq!(stats["libraries"][2]["name"], "Unknown Library");
        assert_eq!(
            stats["library_totals"],
            json!({
                "total_libraries": 3, "total_items": 165, "total_seasons": 90,
                "total_episodes": 800, "total_play_time": "01:01:01",
                "total_play_time_raw": 36_610_000_000_i64,
                "type_breakdown": {
                    "movies": {"count": 2, "items": 125, "play_time": 36_610_000_000_i64, "label": "Movies"},
                    "tvshows": {"count": 1, "items": 40, "play_time": 0, "label": "TV Shows"},
                },
            })
        );
        assert_eq!(
            stats["server_info"],
            json!({"server_id": "made-up-server", "last_updated": "2026-11-20T10:00:00+00:00"})
        );

        // Without libraries, the three parts are empty lists, as PHP writes them.
        let none = statistics(None, &[], SHOWN, "now");
        assert_eq!(none["libraries"], json!([]));
        assert_eq!(none["library_totals"], json!([]));
        assert_eq!(none["server_info"], json!([]));
        assert_eq!(none["users"], json!([]));
    }

    #[test]
    fn history_is_counted_for_each_film_series_and_user() {
        let film = video(5400);
        let history = [
            row(
                &json!({"NowPlayingItemName": "The Film (2021)", "NowPlayingItemId": "f1", "UserName": "robin", "ActivityDateInserted": "2026-11-18T20:00:00Z", "ServerId": "s1"}),
                &film,
            ),
            row(
                &json!({"SeriesName": "The Show", "SeriesId": "s-9", "NowPlayingItemId": "e1", "EpisodeName": "Pilot", "UserName": "robin", "ActivityDateInserted": "2026-11-19T20:00:00Z", "ProductionYear": 2019}),
                &video(1800),
            ),
            row(
                &json!({"SeriesName": "The Show", "NowPlayingItemId": "e2", "UserName": "kim", "ActivityDateInserted": "2026-11-17T20:00:00Z"}),
                &video(1700),
            ),
            row(
                &json!({"NowPlayingItemName": "The Film (2021)", "NowPlayingItemId": "f1", "UserName": "kim", "ActivityDateInserted": "2026-11-16T20:00:00Z"}),
                &film,
            ),
            row(
                &json!({"SeriesName": "The Show", "NowPlayingItemId": "e3", "UserName": "robin", "ActivityDateInserted": "2026-11-20T08:00:00Z"}),
                &video(1750),
            ),
            json!({"NowPlayingItemName": "A Song", "NowPlayingItemId": "m1", "PlaybackDuration": 200, "ActivityDateInserted": "2026-11-15T20:00:00Z"}),
            json!({"IsPaused": false}),
        ];
        let stats = statistics(None, &history, SHOWN, "now");

        assert_eq!(
            stats["most_watched_shows"],
            json!([{
                "id": "s-9", "title": "The Show", "type": "show", "play_count": 3,
                "total_duration": 5250, "year": "2019", "server_id": null, "poster_path": null,
                "first_played": "2026-11-17T20:00:00Z", "last_played": "2026-11-20T08:00:00Z",
            }])
        );
        assert_eq!(
            stats["most_watched_movies"],
            json!([{
                "id": "f1", "title": "The Film", "type": "movie", "play_count": 2,
                "total_duration": 10800, "year": "2021", "server_id": "s1", "poster_path": null,
                "first_played": "2026-11-16T20:00:00Z", "last_played": "2026-11-18T20:00:00Z",
            }])
        );
        assert_eq!(stats["most_listened_music"][0]["title"], "A Song");
        assert_eq!(stats["most_listened_music"][0]["play_count"], 1);
        assert_eq!(
            stats["users"],
            json!([
                {"name": "robin", "play_count": 3, "last_activity": "2026-11-20T08:00:00Z"},
                {"name": "kim", "play_count": 2, "last_activity": "2026-11-17T20:00:00Z"},
                {"name": "Unknown User", "play_count": 2, "last_activity": "2026-11-15T20:00:00Z"},
            ])
        );

        // A list that is switched off is empty, and a list is cut at the limit.
        let few = Shown {
            movies: false,
            most: 0,
            ..SHOWN
        };
        let stats = statistics(None, &history, few, "now");
        assert_eq!(stats["most_watched_movies"], json!([]));
        assert_eq!(stats["most_watched_shows"], json!([]));
    }

    #[test]
    fn details_of_an_item_name_no_key_of_the_media_server() {
        let details = json!({
            "Id": "abc123", "Name": "Pilot", "SeriesName": "The Show", "Type": "Episode",
            "ServerId": "srv1", "Overview": "It starts.", "Taglines": ["Here we go"],
            "PremiereDate": "2019-03-01T00:00:00.0000000Z", "Genres": ["Drama"],
            "CommunityRating": 8.5, "RunTimeTicks": 27_000_000_000_i64,
            "ImageTags": {"Thumb": "t1"},
            "People": [
                {"Name": "Kim Actor", "Role": "Lead", "Id": "p1"},
                {"Name": "No Role", "Role": "", "Id": "p2"},
            ],
        });
        let server = MediaServer {
            kind: Kind::Jellyfin,
            url: "https://jellyfin.example.com",
            link: "",
            tab_name: "Jellyfin",
            open_tab: true,
        };
        let item = details_item(
            "abc123",
            Some(&details),
            Some(&server),
            "https://stats.example.com",
        );
        assert_eq!(item["title"], "The Show");
        assert_eq!(item["type"], "tv");
        assert_eq!(item["key"], "abc123-list");
        assert_eq!(
            item["imageURL"],
            "api/v2/homepage/image?source=jellyfin&type=Thumb&img=abc123&height=600&width=400&key=abc123-jellystat"
        );
        assert_eq!(
            item["address"],
            "https://jellyfin.example.com/web/index.html#!/item?id=abc123&serverId=srv1"
        );
        assert_eq!(item["tabName"], "Jellyfin");
        assert_eq!(item["openTab"], true);
        assert_eq!(
            item["metadata"],
            json!({
                "guid": "abc123", "summary": "It starts.", "rating": "8.5", "duration": "2700000",
                "originallyAvailableAt": "", "year": "2019", "tagline": "Here we go",
                "genres": ["Drama"],
                "actors": [{
                    "name": "Kim Actor", "role": "Lead",
                    "thumb": "api/v2/homepage/image?source=jellyfin&type=Primary&img=p1&height=450&width=300&key=p1-cast",
                }],
            })
        );

        // With a link of the Admin.
        let linked = MediaServer {
            link: "https://app.example.com/#!/item?id={id}&serverId={serverId}",
            ..server
        };
        let item = details_item("abc123", Some(&details), Some(&linked), "");
        assert_eq!(
            item["address"],
            "https://app.example.com/#!/item?id=abc123&serverId=srv1"
        );
    }

    #[test]
    fn details_without_a_media_server_use_the_pictures_of_jellystat() {
        let details = json!({"Id": "abc123", "Name": "The Film", "Type": "Movie", "ImageTags": {"Primary": "tag 1"}, "ProductionYear": 2021});
        let item = details_item("abc123", Some(&details), None, "https://stats.example.com/");
        assert_eq!(item["title"], "The Film");
        assert_eq!(item["metadata"]["year"], "2021");
        assert_eq!(
            item["imageURL"],
            "https://stats.example.com/proxy/Items/abc123/Images/Primary?tag=tag%201&fillWidth=400&quality=90"
        );
        assert_eq!(item["address"], "");
        assert_eq!(item["openTab"], false);

        // Nothing is known about the item.
        let item = details_item("abc123", None, None, "https://stats.example.com");
        assert_eq!(item["title"], "Unknown Item");
        assert_eq!(item["metadata"]["tagline"], "View in JellyStat");
        assert_eq!(
            item["imageURL"],
            "https://stats.example.com/proxy/Items/Images/Primary?id=abc123&fillWidth=200&quality=90"
        );
        let item = details_item("abc123", Some(&json!({})), None, "");
        assert_eq!(item["imageURL"], "plugins/images/homepage/no-np.png");
    }
}
