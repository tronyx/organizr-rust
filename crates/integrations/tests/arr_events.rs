//! The calendar events that are made from the answers of Sonarr, Radarr and Lidarr.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use jiff::Timestamp;
use jiff::tz::TimeZone;
use organizr_integrations::arr::{
    Links, NO_PICTURE, Picture, Releases, lidarr_events, radarr_events, sonarr_events,
};
use serde_json::{Value, json};

const LINKS: Links<'static> = Links {
    base: "https://Sonarr.example.com/",
    frame: "Sonarr",
    show_link: true,
};

fn now() -> Timestamp {
    "2026-11-10T12:00:00Z".parse().expect("the time is valid")
}

fn episode(number: i64, aired: &str, has_file: bool, monitored: bool) -> Value {
    let file = if has_file {
        json!({
            "size": 1_234_567,
            "quality": {"quality": {"name": "WEBDL-1080p"}},
            "mediaInfo": {"audioChannels": 5.1, "audioCodec": "EAC3", "videoCodec": "h264"},
        })
    } else {
        Value::Null
    };
    json!({
        "seasonNumber": 2,
        "episodeNumber": number,
        "title": "The One",
        "airDateUtc": aired,
        "overview": "What happens.",
        "hasFile": has_file,
        "monitored": monitored,
        "episodeFile": file,
        "series": {
            "title": "Tom & Jerry:  The Show!",
            "tvdbId": 4711,
            "status": "continuing",
            "runtime": 45,
            "ratings": {"votes": 10, "value": 8.1},
            "genres": ["Comedy"],
            "seasons": [{"seasonNumber": 1}, {"seasonNumber": 2}],
            "images": [
                {"coverType": "poster", "url": "/MediaCover/1/poster.jpg", "remoteUrl": "https://art.example.net/poster.jpg"},
                {"coverType": "fanart", "url": "/MediaCover/1/fanart.jpg", "remoteUrl": "https://art.example.net/fanart.jpg"},
            ],
        },
    })
}

#[test]
fn sonarr_episode_becomes_an_event_of_the_calendar() {
    let entries = [episode(3, "2026-11-09T20:00:00Z", true, true)];

    let made = sonarr_events(&entries, 0, LINKS, now());

    assert_eq!(
        made.events,
        [json!({
            "id": "Sonarr-0-1",
            "title": "Tom & Jerry:  The Show!",
            // Not the first episode of its season: one second later.
            "start": "2026-11-09T20:00:01Z",
            "className": "inline-popups bg-calendar calendar-item tvID--4711",
            "imagetype": "tv text-success",
            "imagetypeFilter": "tv",
            "downloadFilter": "text-success",
            "bgColor": "bg-success",
            "details": {
                "seasonCount": 2,
                "status": "continuing",
                "topTitle": "Tom & Jerry:  The Show!",
                "bottomTitle": "S02E03 - The One",
                "overview": "What happens.",
                "runtime": 45,
                "image": "data/cache/tv-4711.jpg",
                "ratings": 8.1,
                "videoQuality": "WEBDL-1080p",
                "audioChannels": 5.1,
                "audioCodec": "EAC3",
                "videoCodec": "h264",
                "size": 1_234_567,
                "genres": ["Comedy"],
                "href": "https://sonarr.example.com/series/tom-and-jerry-the-show",
                "icon": "/plugins/images/tabs/sonarr.png",
                "frame": "Sonarr",
                "showLink": true,
            },
        })]
    );
    // The fan art on the internet is fetched, not the path in Sonarr.
    assert_eq!(
        made.pictures,
        [Picture {
            name: "tv-4711".to_owned(),
            source: "https://art.example.net/fanart.jpg".to_owned(),
        }]
    );
}

#[test]
fn sonarr_state_follows_the_file_the_air_time_and_the_episode_number() {
    let state = |entry: Value| {
        sonarr_events(&[entry], 0, LINKS, now()).events[0]["downloadFilter"]
            .as_str()
            .expect("the event has a state")
            .to_owned()
    };
    let (past, future) = ("2026-11-09T20:00:00Z", "2026-11-11T20:00:00Z");

    assert_eq!(
        state(episode(1, future, false, true)),
        "text-primary animated flash"
    );
    assert_eq!(state(episode(2, future, false, false)), "text-dark");
    assert_eq!(state(episode(2, future, false, true)), "text-info");
    assert_eq!(state(episode(2, past, false, true)), "text-danger");
    assert_eq!(state(episode(1, past, true, true)), "text-success");
    // One episode that was not aired yet says nothing about the next one.
    let both = sonarr_events(
        &[
            episode(2, future, false, true),
            episode(2, past, false, true),
        ],
        0,
        LINKS,
        now(),
    );
    assert_eq!(both.events[1]["downloadFilter"], "text-danger");
    assert_eq!(both.events[1]["id"], "Sonarr-0-2");
}

#[test]
fn sonarr_event_without_a_file_or_fan_art_says_so() {
    let mut entry = episode(1, "2026-11-11T20:00:00Z", false, true);
    entry["series"]["images"] = json!([]);
    entry.as_object_mut().expect("an object").remove("overview");
    let no_link = Links { base: "", ..LINKS };

    let made = sonarr_events(&[entry], 1, no_link, now());

    let event = &made.events[0];
    // The first episode of a season keeps its time.
    assert_eq!(event["start"], "2026-11-11T20:00:00Z");
    assert_eq!(event["id"], "Sonarr-1-1");
    let details = &event["details"];
    assert_eq!(details["image"], NO_PICTURE);
    assert_eq!(details["overview"], "");
    assert_eq!(details["href"], "");
    for field in [
        "videoQuality",
        "audioChannels",
        "audioCodec",
        "videoCodec",
        "size",
    ] {
        assert_eq!(details[field], "unknown", "{field}");
    }
    assert_eq!(made.pictures, []);
}

fn film() -> Value {
    json!({
        "title": "The Film",
        "tmdbId": 815,
        "status": "released",
        "overview": "A film.",
        "runtime": 101,
        "hasFile": false,
        "year": 2026,
        "studio": "Studio",
        "genres": ["Drama"],
        "ratings": {"value": 7.2},
        "alternateTitles": [{"title": "Der Film"}, {"title": "Le Film"}],
        "inCinemas": "2026-10-01T00:00:00Z",
        "digitalRelease": "2026-11-20T00:00:00Z",
        "physicalRelease": "2026-11-20T00:00:00Z",
        "images": [
            {"coverType": "poster", "url": "/MediaCover/5/poster.jpg"},
            {"coverType": "fanart", "url": "/MediaCover/5/fanart.jpg", "remoteUrl": "https://art.example.net/film.jpg"},
        ],
    })
}

const ALL: Releases = Releases {
    physical: true,
    digital: true,
    cinema: true,
};

fn radarr(entries: &[Value], releases: Releases) -> Vec<Value> {
    let links = Links {
        base: "https://radarr.example.com",
        frame: "",
        show_link: false,
    };
    radarr_events(entries, 0, links, releases, now(), &TimeZone::UTC).events
}

#[test]
fn radarr_film_has_one_event_for_each_day_of_release() {
    let events = radarr(&[film()], ALL);

    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        json!({
            "id": "Radarr-0-1",
            "title": "The Film",
            "start": "2026-11-20",
            "className": "inline-popups bg-calendar movieID--815",
            // Not released yet, and no file.
            "imagetype": "film text-info",
            "imagetypeFilter": "film",
            "downloadFilter": "text-info",
            // The disc and the download come on the same day.
            "releaseType": ["physical", "digital"],
            "bgColor": "bg-info",
            "details": {
                "topTitle": "The Film",
                "bottomTitle": "Der Film, Le Film",
                "status": "released",
                "overview": "A film.",
                "runtime": 101,
                "image": "https://art.example.net/film.jpg",
                "ratings": 7.2,
                "videoQuality": "unknown",
                "audioChannels": "unknown",
                "audioCodec": "unknown",
                "videoCodec": "unknown",
                "size": "unknown",
                "genres": ["Drama"],
                "year": 2026,
                "studio": "Studio",
                "href": "https://radarr.example.com/movie/815",
                "icon": "/plugins/images/tabs/radarr.png",
                "frame": "",
                "showLink": false,
            },
        })
    );
    assert_eq!(events[1]["id"], "Radarr-0-2");
    assert_eq!(events[1]["start"], "2026-10-01");
    assert_eq!(events[1]["releaseType"], json!(["cinema"]));
    // Released, and no file.
    assert_eq!(events[1]["downloadFilter"], "text-danger");
}

#[test]
fn radarr_shows_only_the_releases_that_the_settings_name() {
    let only = |physical, digital, cinema| {
        radarr(
            &[film()],
            Releases {
                physical,
                digital,
                cinema,
            },
        )
        .iter()
        .map(|event| event["releaseType"].clone())
        .collect::<Vec<_>>()
    };

    assert_eq!(
        only(false, true, true),
        [json!(["digital"]), json!(["cinema"])]
    );
    assert_eq!(only(false, false, true), [json!(["cinema"])]);
    assert_eq!(only(true, false, false), [json!(["physical", "digital"])]);
    assert_eq!(only(false, false, false), Vec::<Value>::new());

    // All three on one day are one event.
    let mut same = film();
    same["inCinemas"] = same["physicalRelease"].clone();
    let events = radarr(&[same], ALL);
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0]["releaseType"],
        json!(["physical", "digital", "cinema"])
    );

    // A film without a date has no event.
    let mut none = film();
    for field in ["inCinemas", "digitalRelease", "physicalRelease"] {
        none[field] = Value::Null;
    }
    assert_eq!(radarr(&[none], ALL), Vec::<Value>::new());
}

#[test]
fn radarr_picture_that_only_radarr_has_goes_through_the_cache() {
    let mut entry = film();
    entry["hasFile"] = json!(true);
    entry["movieFile"] = json!({
        "size": 99,
        "quality": {"quality": {"name": "Bluray-1080p"}},
        "mediaInfo": {"audioChannels": 2, "audioFormat": "AAC", "videoCodec": "x265"},
    });
    entry["images"] = json!([{"coverType": "fanart", "url": "/MediaCover/5/fanart.jpg"}]);
    entry.as_object_mut().expect("an object").remove("ratings");
    let links = Links {
        base: "",
        frame: "",
        show_link: true,
    };

    let made = radarr_events(&[entry], 2, links, ALL, now(), &TimeZone::UTC);

    let details = &made.events[0]["details"];
    assert_eq!(details["image"], "data/cache/movie-815.jpg");
    assert_eq!(
        made.pictures[0],
        Picture {
            name: "movie-815".to_owned(),
            source: "/MediaCover/5/fanart.jpg".to_owned(),
        }
    );
    assert_eq!(made.events[0]["downloadFilter"], "text-success");
    assert_eq!(details["videoQuality"], "Bluray-1080p");
    assert_eq!(details["audioCodec"], "AAC");
    assert_eq!(details["size"], 99);
    assert_eq!(details["ratings"], 0);
    assert_eq!(made.events[0]["id"], "Radarr-2-1");
}

#[test]
fn lidarr_album_becomes_an_event_with_the_answer_of_lidarr() {
    let album = |percent: Value, released: &str| {
        json!({
            "title": "The Album",
            "releaseDate": released,
            "genres": ["Rock"],
            "statistics": {"percentOfTracks": percent},
            "artist": {
                "artistName": "The Band",
                "foreignArtistId": "ABC-123",
                "overview": "A band.",
                "ratings": {"value": 9},
                "images": [{"coverType": "fanart", "url": "http://art.example.net/band.jpg"}],
            },
        })
    };
    let links = Links {
        base: "https://lidarr.example.com",
        frame: "Lidarr",
        show_link: true,
    };
    let entries = [
        album(json!(100.0), "2026-11-01T00:00:00Z"),
        album(json!(50), "2026-11-01T00:00:00Z"),
        album(json!(0), "2026-12-01T00:00:00Z"),
    ];

    let made = lidarr_events(&entries, 0, links, now());

    let states: Vec<&str> = made
        .events
        .iter()
        .map(|event| event["downloadFilter"].as_str().unwrap_or("?"))
        .collect();
    assert_eq!(states, ["text-success", "text-danger", "text-info"]);
    let event = &made.events[0];
    assert_eq!(event["id"], "Lidarr-0-1");
    assert_eq!(event["title"], "The Band");
    assert_eq!(event["start"], "2026-11-01T00:00:00Z");
    assert_eq!(
        event["className"],
        "inline-popups bg-calendar calendar-item musicID--"
    );
    assert_eq!(event["imagetype"], "music text-success");
    assert_eq!(event["data"], entries[0]);
    let details = &event["details"];
    assert_eq!(details["topTitle"], "The Album");
    assert_eq!(details["bottomTitle"], "The Band");
    assert_eq!(details["image"], "https://art.example.net/band.jpg");
    assert_eq!(details["href"], "https://lidarr.example.com/artist/abc-123");
    assert_eq!(details["ratings"], 9);
    assert_eq!(made.pictures, []);
}
