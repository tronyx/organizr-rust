//! The items of the homepage that are made from the XML of a Plex Media Server.

// One expected item has more fields than the `json!` macro takes at the usual limit.
#![recursion_limit = "256"]
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_integrations::plex_server::{
    ItemContext, Playlist, items, picture_name, playlist_videos, playlists,
};
use serde_json::{Map, Value, json};

const SESSIONS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<MediaContainer size="2">
  <Video type="episode" ratingKey="301" parentRatingKey="300" grandparentRatingKey="30"
         title="The One" parentTitle="Season 2" grandparentTitle="The Show" index="3" parentIndex="2"
         parentThumb="/library/metadata/300/thumb/1" grandparentThumb="/library/metadata/30/thumb/1"
         grandparentArt="/library/metadata/30/art/1" librarySectionID="2" viewOffset="600000"
         duration="2400000" addedAt="1577880000" summary="What happens." year="2020" guid="plex://episode/abc">
    <Media videoResolution="1080" duration="2400000">
      <Part decision="transcode"><Stream decision="transcode"/></Part>
    </Media>
    <User id="1" title="robin" thumb="https://plex.tv/users/abc/avatar"/>
    <Player address="192.0.2.50" machineIdentifier="player-&quot;one&quot;" platform="Chrome" product="Plex Web" device="Windows" state="paused"/>
    <Session id="session-one" bandwidth="4000" location="lan"/>
    <TranscodeSession progress="55.5" throttled="1" videoDecision="transcode" audioDecision="copy"
                      sourceVideoCodec="hevc" videoCodec="h264" sourceAudioCodec="eac3" audioCodec="aac" container="mkv" audioChannels="2"/>
  </Video>
  <Video type="movie" ratingKey="815" title="The Film" year="2019" thumb="/library/metadata/815/thumb/1"
         art="/library/metadata/815/art/1" librarySectionID="1" duration="6000000" summary="A film." rating="7.2" studio="Studio" tagline="Watch it.">
    <Media videoResolution="4k"><Part decision="directplay"/></Media>
    <Genre tag="Drama"/><Genre tag="Crime"/>
    <Role tag="An Actor" role="The Hero" thumb="https://art.example.net/actor.jpg"/>
    <Role tag="Another Actor" role="The Friend"/>
    <User id="2" title="pat"/>
    <Player address="198.51.100.7" machineIdentifier="player-two" platform="Roku" product="Plex for Roku" device="Roku" state="playing"/>
    <Session id="session-two" bandwidth="20000" location="wan"/>
  </Video>
  <Video type="movie" ratingKey="900" title="No library"/>
</MediaContainer>"#;

fn context<'a>(
    names: Option<&'a Map<String, Value>>,
    is_cached: &'a (dyn Fn(&str) -> bool + Sync),
) -> ItemContext<'a> {
    ItemContext {
        show_names: true,
        show_addresses: true,
        custom_names: names,
        tab_url: "",
        tab_name: "",
        server_id: "made-up-server-one",
        poster: (200, 300),
        wide: (1200, 675),
        now: 1_600_000_000,
        fresh: "fresh",
        is_cached,
    }
}

const NOTHING_CACHED: &(dyn Fn(&str) -> bool + Sync) = &|_| false;

#[test]
fn episode_that_plays_becomes_an_item_with_its_stream() {
    let made =
        items(SESSIONS, &context(None, NOTHING_CACHED), &[""], &[]).expect("the XML is valid");

    // The entry without a library is left out.
    assert_eq!(made.len(), 2);
    // The key of a picture is the name of the picture in the cache, made from the
    // picture and its size (D-153). The item keeps its keys as in PHP.
    let wide_name = picture_name("/library/metadata/30/art/1", (1200, 675), "30-np");
    let poster_name = picture_name("/library/metadata/300/thumb/1", (200, 300), "301-list");
    let wide = format!(
        "api/v2/homepage/image?source=plex&img=/library/metadata/30/art/1&height=675&width=1200&key={wide_name}"
    );
    let poster = format!(
        "api/v2/homepage/image?source=plex&img=/library/metadata/300/thumb/1&height=300&width=200&key={poster_name}"
    );
    assert_eq!(
        made[0],
        json!({
            "type": "tv",
            "title": "The Show",
            "secondaryTitle": "Season 2 - Episode 3",
            "summary": "The One",
            "ratingKey": "300",
            "thumb": "/library/metadata/300/thumb/1",
            "key": "301-list",
            "nowPlayingThumb": "/library/metadata/30/art/1",
            "nowPlayingKey": "30-np",
            "nowPlayingTitle": "The Show - The One",
            "nowPlayingBottom": "S2 · E3",
            "metadataKey": "30",
            "originalType": "episode",
            "uid": "301",
            "elapsed": 600_000,
            "duration": 2_400_000,
            "addedAt": 1_577_880_000,
            "watched": 25,
            "transcoded": 30,
            "stream": "transcode",
            "id": "player-one",
            "session": "session-one",
            "bandwidth": "4000",
            "bandwidthType": "lan",
            "sessionType": "Transcoding",
            "state": "pause",
            "user": "robin",
            "userThumb": "https://plex.tv/users/abc/avatar",
            "userAddress": "192.0.2.50",
            "address": "https://app.plex.tv/web/app#!/server/made-up-server-one/details?key=/library/metadata/301",
            "nowPlayingOriginalImage": format!("{wide}$fresh"),
            "originalImage": format!("{poster}$fresh"),
            "openTab": false,
            "tabName": "",
            "userStream": {
                "platform": "Chrome",
                "product": "Plex Web",
                "device": "Windows",
                "stream": "transcode (Throttled)",
                "videoResolution": "1080",
                "throttled": true,
                "sourceVideoCodec": "hevc",
                "videoCodec": "h264",
                "audioCodec": "aac",
                "sourceAudioCodec": "eac3",
                "videoDecision": "Transcode",
                "audioDecision": "Direct Stream",
                "container": "mkv",
                "audioChannels": "2",
            },
            "metadata": {
                "guid": "plex://episode/abc",
                "summary": "What happens.",
                "rating": "",
                "duration": "2400000",
                "originallyAvailableAt": "",
                "year": "2020",
                "studio": "",
                "tagline": "",
                "genres": "",
                "actors": "",
            },
            "nowPlayingImageURL": wide,
            "imageURL": poster,
        })
    );
}

#[test]
fn film_has_its_genres_and_the_actors_with_a_picture() {
    let made =
        items(SESSIONS, &context(None, NOTHING_CACHED), &[""], &[]).expect("the XML is valid");
    let film = &made[1];

    assert_eq!(film["type"], "movie");
    assert_eq!(film["secondaryTitle"], "2019");
    assert_eq!(film["elapsed"], Value::Null);
    assert_eq!(film["watched"], 0);
    assert_eq!(film["transcoded"], "");
    assert_eq!(film["sessionType"], "Direct Playing");
    assert_eq!(film["state"], "play");
    assert_eq!(film["userStream"]["stream"], "directplay");
    assert_eq!(film["userStream"]["videoDecision"], "Direct Play");
    assert_eq!(film["metadata"]["genres"], json!(["Drama", "Crime"]));
    assert_eq!(
        film["metadata"]["actors"],
        json!([{"name": "An Actor", "role": "The Hero", "thumb": "https://art.example.net/actor.jpg"}])
    );
    assert_eq!(film["metadata"]["tagline"], "Watch it.");
}

#[test]
fn caller_without_the_level_sees_no_name_and_no_address() {
    let hidden = ItemContext {
        show_names: false,
        show_addresses: false,
        ..context(None, NOTHING_CACHED)
    };

    let made = items(SESSIONS, &hidden, &[""], &[]).expect("the XML is valid");

    assert_eq!(made[0]["user"], "");
    assert_eq!(made[0]["userThumb"], "");
    assert_eq!(made[0]["userAddress"], "x.x.x.x");
}

#[test]
fn settings_change_the_name_the_link_and_the_source_of_a_picture() {
    let mut names = Map::new();
    names.insert("robin".to_owned(), json!("Robin R."));
    let wide_name = picture_name("/library/metadata/30/art/1", (1200, 675), "30-np");
    let cached = wide_name.clone();
    let is_cached: &(dyn Fn(&str) -> bool + Sync) = &move |key| key == cached;
    let changed = ItemContext {
        tab_url: "https://plex.example.com",
        tab_name: "Plex",
        ..context(Some(&names), is_cached)
    };

    let made = items(SESSIONS, &changed, &[""], &[]).expect("the XML is valid");

    assert_eq!(made[0]["user"], "Robin R.");
    assert_eq!(made[1]["user"], "pat");
    assert_eq!(
        made[0]["address"],
        "https://plex.example.com/web/index.html#!/server/made-up-server-one/details?key=/library/metadata/301"
    );
    assert_eq!(made[0]["openTab"], true);
    assert_eq!(made[0]["tabName"], "Plex");
    // The wide picture is in the cache. The poster is not.
    assert_eq!(
        made[0]["nowPlayingImageURL"],
        format!("data/cache/plex-{wide_name}.jpg")
    );
    assert!(
        made[0]["imageURL"]
            .as_str()
            .is_some_and(|address| address.starts_with("api/v2/homepage/image?"))
    );
}

#[test]
fn libraries_and_types_can_be_left_out() {
    let all = context(None, NOTHING_CACHED);

    let without_films = items(SESSIONS, &all, &["1"], &[]).expect("the XML is valid");
    assert_eq!(without_films.len(), 1);
    assert_eq!(without_films[0]["type"], "tv");

    let without_episodes = items(SESSIONS, &all, &[""], &["episode"]).expect("the XML is valid");
    assert_eq!(without_episodes.len(), 1);
    assert_eq!(without_episodes[0]["type"], "movie");

    assert!(items("<html>", &all, &[""], &[]).is_err());
}

#[test]
fn item_without_pictures_or_added_a_moment_ago_shows_the_stand_in_pictures() {
    let xml = r#"<MediaContainer>
      <Video type="movie" ratingKey="1" title="Bare" librarySectionID="1"/>
      <Video type="movie" ratingKey="2" title="New" librarySectionID="1" thumb="/t" art="/a" addedAt="1599999900"/>
      <Directory type="season" ratingKey="41" parentRatingKey="40" title="Season 1" parentTitle="A Show" thumb="/s" art="/sa" librarySectionID="2"/>
      <Track type="track" ratingKey="52" parentRatingKey="51" grandparentRatingKey="50" title="A Song" parentTitle="An Album" grandparentTitle="A Band" thumb="/tt" parentThumb="/pt" librarySectionID="3"/>
      <Video type="clip" title="News" live="1" librarySectionID="4" thumb="/c" art="/ca"/>
    </MediaContainer>"#;

    let made = items(xml, &context(None, NOTHING_CACHED), &[""], &[]).expect("the XML is valid");

    assert_eq!(made[0]["nowPlayingKey"], "no-np");
    assert_eq!(
        made[0]["nowPlayingImageURL"],
        "plugins/images/homepage/no-np.png"
    );
    assert_eq!(made[0]["key"], "no-list");
    assert_eq!(made[0]["imageURL"], "plugins/images/homepage/no-list.png");
    // Added 100 seconds ago: Plex has no poster yet.
    assert_eq!(made[1]["key"], "no-list");
    assert_eq!(made[1]["nowPlayingKey"], "2-np");

    let season = &made[2];
    assert_eq!(season["type"], "tv");
    assert_eq!(season["title"], "A Show");
    assert_eq!(season["secondaryTitle"], "Season 1");
    assert_eq!(season["metadataKey"], "40");
    assert_eq!(season.get("nowPlayingTitle"), None);

    let track = &made[3];
    assert_eq!(track["type"], "music");
    assert_eq!(track["nowPlayingTitle"], "A Band - A Song");
    assert_eq!(track["nowPlayingThumb"], "/pt");
    assert_eq!(track["metadataKey"], "50");

    let live = &made[4];
    assert_eq!(live["type"], "clip");
    assert_eq!(live["title"], "Live TV");
    assert_eq!(live["nowPlayingKey"], "livetv.png");
    assert_eq!(live["nowPlayingBottom"], "Live TV");
    assert_eq!(live["useImage"], "plugins/images/homepage/livetv.png");
    assert_eq!(live.get("metadataKey"), None);
}

#[test]
fn video_playlists_are_listed_without_the_private_ones() {
    let xml = r#"<MediaContainer>
      <Playlist title="Film night!" playlistType="video" key="/playlists/7/items"/>
      <Playlist title="My Private list" playlistType="video" key="/playlists/8/items"/>
      <Playlist title="Songs" playlistType="audio" key="/playlists/9/items"/>
    </MediaContainer>"#;

    assert_eq!(
        playlists(xml).expect("the XML is valid"),
        [Playlist {
            title: "Film night!".to_owned(),
            key: "/playlists/7/items".to_owned(),
        }]
    );

    let videos = r#"<MediaContainer>
      <Video type="movie" ratingKey="815" title="The Film" thumb="/t" art="/a"/>
      <Track type="track" ratingKey="1" title="A Song"/>
    </MediaContainer>"#;
    let made = playlist_videos(videos, &context(None, NOTHING_CACHED)).expect("the XML is valid");
    assert_eq!(made.len(), 1);
    assert_eq!(made[0]["title"], "The Film");
}
