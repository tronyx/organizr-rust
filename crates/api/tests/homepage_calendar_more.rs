//! SickRage, CouchPotato and calendars in the iCalendar format in the calendar of the
//! homepage, against stand-ins for them.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, ARR_KEY, COUCHPOTATO_KEY, OMBI_KEY, SICKRAGE_KEY};
use organizr_testkit::http::{As, call};
use organizr_testkit::services::StandIn;
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const CALENDAR: &str = "/homepage/calendar";
const SICKRAGE: &str = "/homepage/sickrage/calendar";
const COUCHPOTATO: &str = "/homepage/couchpotato/calendar";
const ICAL: &str = "/homepage/ical/calendar";

type Changed = Vec<(&'static str, Value)>;

fn sickrage_settings(service: &StandIn) -> Changed {
    vec![
        ("homepageSickrageEnabled", json!(true)),
        ("homepageSickrageAuth", json!("4")),
        ("sickrageURL", json!(service.url())),
        ("sickrageToken", json!(SICKRAGE_KEY)),
    ]
}

fn couchpotato_settings(service: &StandIn) -> Changed {
    vec![
        ("homepageCouchpotatoEnabled", json!(true)),
        ("homepageCouchpotatoAuth", json!("4")),
        ("couchpotatoURL", json!(service.url())),
        ("couchpotatoToken", json!(COUCHPOTATO_KEY)),
    ]
}

fn ical_settings(service: &StandIn) -> Changed {
    vec![
        ("homepageCalendarEnabled", json!(true)),
        ("homepageCalendarAuth", json!("4")),
        (
            "calendariCal",
            json!(format!("{}/calendar.ics", service.url())),
        ),
    ]
}

/// The settings with another value for one of them.
fn with(mut settings: Changed, key: &'static str, value: Value) -> Changed {
    settings.retain(|(known, _)| *known != key);
    settings.push((key, value));
    settings
}

fn texts(events: &Value, field: &str) -> Vec<String> {
    events
        .as_array()
        .expect("a list of events")
        .iter()
        .map(|event| event[field].as_str().unwrap_or("?").to_owned())
        .collect()
}

/// A day as text, a number of days after today in UTC, as the stand-ins count.
fn day_from_today(days: i64) -> String {
    let today = jiff::Timestamp::now()
        .to_zoned_datetime(jiff::tz::TimeZone::UTC)
        .date();
    today
        .checked_add(jiff::Span::new().days(days))
        .expect("a day near today")
        .strftime("%Y-%m-%d")
        .to_string()
}

/// The kinds of the items that the homepage lists for a caller.
async fn kinds(app: &Router, who: As<'_>) -> Vec<String> {
    texts(
        &call(app, who, "GET", "/homepage/items", None).await.data(),
        "kind",
    )
}

#[tokio::test]
async fn sickrage_gives_the_missed_the_coming_and_the_downloaded_episodes() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::sickrage().await;
        // Two installs: the second one has a wrong key and is left out.
        let mut settings = sickrage_settings(&service);
        settings = with(
            settings,
            "sickrageURL",
            json!(format!("{},{}", service.url(), service.url())),
        );
        settings = with(
            settings,
            "sickrageToken",
            json!(format!("{SICKRAGE_KEY},a-wrong-key")),
        );
        let app = installed_app_with(database, &settings);

        let episodes = call(&app, As::Admin, "GET", SICKRAGE, None).await;

        assert_eq!(episodes.status, StatusCode::OK, "{}", episodes.body);
        let events = episodes.data();
        assert_eq!(
            texts(&events, "id"),
            ["Sick-0-Miss-1", "Sick-0-Later-2", "Sick-0-History-1"]
        );
        assert_eq!(
            texts(&events, "downloadFilter"),
            ["text-danger", "text-info", "text-success"]
        );
        assert_eq!(
            texts(&events, "start"),
            [
                "2020-01-05 21:00:00",
                "2099-01-04 21:00:00",
                "2020-01-06 22:10"
            ]
        );
        assert_eq!(events[0]["imagetypeFilter"], "tv");
        assert_eq!(
            events[0]["className"],
            "inline-popups bg-calendar calendar-item tvID--4711"
        );
        assert_eq!(
            events[0]["details"]["bottomTitle"],
            "S02E03 - The Missed One"
        );
        assert_eq!(events[2]["details"]["bottomTitle"], "S02E02");
        // The key is a part of the path, as SickRage wants it.
        let seen = service.seen();
        assert_eq!(
            seen[0].path,
            format!("/api/{SICKRAGE_KEY}/?cmd=future&sort=date&type=missed%7Ctoday%7Csoon%7Clater")
        );
        assert_eq!(
            seen[1].path,
            format!("/api/{SICKRAGE_KEY}/?cmd=history&limit=100&type=downloaded")
        );
    })
    .await;
}

#[tokio::test]
async fn sickrage_is_refused_when_it_is_off_or_not_for_the_caller_or_not_set_up() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::sickrage().await;
        let settings = sickrage_settings(&service);

        let off = installed_app_with(database, &[]);
        let refused = call(&off, As::Admin, "GET", SICKRAGE, None).await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert_eq!(
            refused.message(),
            json!("homepageSickrageEnabled module is not enabled")
        );

        // At the start, SickRage is for an Admin and a Co-Admin.
        let app = installed_app_with(
            database,
            &with(settings.clone(), "homepageSickrageAuth", json!("1")),
        );
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let refused = call(&app, As::User(&user), "GET", SICKRAGE, None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            refused.message(),
            json!("User not approved to view this homepage item")
        );
        assert_eq!(kinds(&app, As::User(&user)).await, Vec::<String>::new());
        assert_eq!(kinds(&app, As::Admin).await, ["calendar"]);

        let no_key = installed_app_with(
            database,
            &with(settings.clone(), "sickrageToken", json!("")),
        );
        let refused = call(&no_key, As::Admin, "GET", SICKRAGE, None).await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(refused.message(), json!("sickrageToken was not supplied"));
        let no_address =
            installed_app_with(database, &with(settings.clone(), "sickrageURL", json!("")));
        let refused = call(&no_address, As::Admin, "GET", SICKRAGE, None).await;
        assert_eq!(refused.message(), json!("sickrageURL was not supplied"));
        assert_eq!(service.seen().len(), 0);

        // A SickRage that cannot be reached is left out, and the answer is a success.
        let down = installed_app_with(
            database,
            &with(settings, "sickrageURL", json!("http://127.0.0.1:9")),
        );
        let empty = call(&down, As::Admin, "GET", SICKRAGE, None).await;
        assert_eq!(empty.status, StatusCode::OK, "{}", empty.body);
        assert_eq!(empty.data(), json!([]));
    })
    .await;
}

#[tokio::test]
async fn couchpotato_gives_the_films_of_the_days_around_today() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::couchpotato().await;
        let settings = couchpotato_settings(&service);
        let app = installed_app_with(database, &settings);
        let user = seed::logged_in(database, &app, "zed", 4).await;

        let films = call(&app, As::User(&user), "GET", COUCHPOTATO, None).await;

        assert_eq!(films.status, StatusCode::OK, "{}", films.body);
        let events = films.data();
        // The film from long ago is left out, and it still counts.
        assert_eq!(texts(&events, "id"), ["CouchPotato-0-2", "CouchPotato-0-3"]);
        assert_eq!(texts(&events, "title"), ["The Film", "The Next Film"]);
        assert_eq!(
            texts(&events, "start"),
            [day_from_today(-2), day_from_today(5)]
        );
        assert_eq!(
            texts(&events, "downloadFilter"),
            ["text-success", "text-info"]
        );
        assert_eq!(events[0]["imagetypeFilter"], "film");
        assert_eq!(
            events[0]["className"],
            "inline-popups bg-calendar calendar-item movieID--815"
        );
        assert_eq!(events[0]["details"]["topTitle"], "The Film");
        assert_eq!(
            events[0]["details"]["image"],
            "/plugins/images/homepage/no-np.png"
        );
        assert_eq!(
            service.seen()[0].path,
            format!("/api/{COUCHPOTATO_KEY}/media.list?status=active%2Cdone")
        );

        // The days of the settings say which films show.
        let narrow =
            installed_app_with(database, &with(settings.clone(), "calendarEnd", json!("1")));
        let films = call(&narrow, As::Admin, "GET", COUCHPOTATO, None).await;
        assert_eq!(texts(&films.data(), "title"), ["The Film"]);

        // The refusals of an item, and an install that refuses its key.
        let refused = call(&app, As::Guest, "GET", COUCHPOTATO, None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        let off = installed_app_with(database, &[]);
        let refused = call(&off, As::Admin, "GET", COUCHPOTATO, None).await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert_eq!(
            refused.message(),
            json!("homepageCouchpotatoEnabled module is not enabled")
        );
        let no_address = installed_app_with(
            database,
            &with(settings.clone(), "couchpotatoURL", json!("")),
        );
        let refused = call(&no_address, As::Admin, "GET", COUCHPOTATO, None).await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(refused.message(), json!("couchpotatoURL was not supplied"));
        let wrong = installed_app_with(
            database,
            &with(settings, "couchpotatoToken", json!("a-wrong-key")),
        );
        let empty = call(&wrong, As::Admin, "GET", COUCHPOTATO, None).await;
        assert_eq!(empty.status, StatusCode::OK, "{}", empty.body);
        assert_eq!(empty.data(), json!([]));
    })
    .await;
}

#[tokio::test]
async fn ical_addresses_give_their_events_of_the_days_around_today() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::ics().await;
        let settings = ical_settings(&service);
        // The second address gives no calendar, and is left out.
        let two = format!("{0}/calendar.ics,{0}/missing.ics", service.url());
        let app = installed_app_with(
            database,
            &with(settings.clone(), "calendariCal", json!(two)),
        );
        let user = seed::logged_in(database, &app, "zed", 4).await;

        let answer = call(&app, As::User(&user), "GET", ICAL, None).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let events = answer.data();
        // The event of the year 2019 is left out.
        assert_eq!(texts(&events, "title"), ["Dinner", "A birthday"]);
        assert_eq!(texts(&events, "bgColor"), ["bg-info", "bg-success"]);
        assert_eq!(
            events[1],
            json!({
                "title": "A birthday",
                "imagetype": "calendar-o text-warning text-custom-calendar text-success",
                "imagetypeFilter": "ical",
                "className": "bg-calendar calendar-item bg-custom-calendar",
                "start": day_from_today(-3),
                "end": day_from_today(-2),
                "bgColor": "bg-success",
            })
        );
        assert_eq!(service.seen().len(), 2);
        assert_eq!(kinds(&app, As::User(&user)).await, ["calendar"]);

        // The refusals have the texts of the PHP application.
        let refused = call(&app, As::Guest, "GET", ICAL, None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            refused.message(),
            json!("User not approved to view this homepage item")
        );
        let off = installed_app_with(
            database,
            &with(settings.clone(), "homepageCalendarEnabled", json!(false)),
        );
        let refused = call(&off, As::Admin, "GET", ICAL, None).await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert_eq!(
            refused.message(),
            json!("iCal homepage item is not enabled")
        );
        let no_address = installed_app_with(database, &with(settings, "calendariCal", json!("")));
        let refused = call(&no_address, As::Admin, "GET", ICAL, None).await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(refused.message(), json!("iCal URL is not defined"));
        assert_eq!(kinds(&no_address, As::Admin).await, Vec::<String>::new());
    })
    .await;
}

#[tokio::test]
async fn calendar_merges_the_new_sources_that_the_caller_sees() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (sonarr, sickrage, couchpotato, ics) = (
            homepage_services::sonarr().await,
            homepage_services::sickrage().await,
            homepage_services::couchpotato().await,
            homepage_services::ics().await,
        );
        let mut settings = vec![
            ("homepageSonarrEnabled", json!(true)),
            ("sonarrURL", json!(sonarr.url())),
            ("sonarrToken", json!(ARR_KEY)),
        ];
        // SickRage is for an Admin only, and the other two are for users too.
        settings.extend(with(
            sickrage_settings(&sickrage),
            "homepageSickrageAuth",
            json!("0"),
        ));
        settings.extend(couchpotato_settings(&couchpotato));
        settings.extend(ical_settings(&ics));
        let app = installed_app_with(database, &settings);
        let user = seed::logged_in(database, &app, "zed", 4).await;

        let all = call(&app, As::Admin, "GET", CALENDAR, None).await;

        assert_eq!(all.status, StatusCode::OK, "{}", all.body);
        // The applications in the order of the PHP application, and the iCal
        // addresses apart from them.
        assert_eq!(
            texts(&all.data()["events"], "id"),
            [
                "Sonarr-0-1",
                "Sick-0-Miss-1",
                "Sick-0-Later-2",
                "Sick-0-History-1",
                "CouchPotato-0-2",
                "CouchPotato-0-3",
            ]
        );
        assert_eq!(
            texts(&all.data()["ical"], "title"),
            ["Dinner", "A birthday"]
        );

        let some = call(&app, As::User(&user), "GET", CALENDAR, None).await;
        assert_eq!(
            texts(&some.data()["events"], "id"),
            ["CouchPotato-0-2", "CouchPotato-0-3"]
        );
        assert_eq!(texts(&some.data()["ical"], "title").len(), 2);

        // A guest has no level, and gets a success with nothing in it.
        let none = call(&app, As::Guest, "GET", CALENDAR, None).await;
        assert_eq!(none.status, StatusCode::OK);
        assert_eq!(none.data(), json!({"ical": false, "events": []}));
        assert_eq!(kinds(&app, As::Guest).await, Vec::<String>::new());
        assert_eq!(kinds(&app, As::User(&user)).await, ["calendar"]);

        // A source that fails does not take the others with it.
        let broken = with(
            with(settings, "sickrageURL", json!("http://127.0.0.1:9")),
            "calendariCal",
            json!("http://127.0.0.1:9/calendar.ics"),
        );
        let app = installed_app_with(database, &broken);
        let rest = call(&app, As::Admin, "GET", CALENDAR, None).await;
        assert_eq!(rest.status, StatusCode::OK, "{}", rest.body);
        assert_eq!(
            texts(&rest.data()["events"], "id"),
            ["Sonarr-0-1", "CouchPotato-0-2", "CouchPotato-0-3"]
        );
        assert_eq!(rest.data()["ical"], json!([]));
    })
    .await;
}

#[tokio::test]
async fn connection_tests_of_ombi_and_sickrage_answer_for_the_right_key_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let test = async |service: &str, settings: &[(&str, Value)]| {
            let app = installed_app_with(database, settings);
            call(&app, As::Admin, "POST", &format!("/test/{service}"), None).await
        };

        let ombi = homepage_services::ombi_requests().await;
        let settings = |key: &str| vec![("ombiURL", json!(ombi.url())), ("ombiToken", json!(key))];
        let passed = test("ombi", &settings(OMBI_KEY)).await;
        assert_eq!(passed.status, StatusCode::OK, "{}", passed.body);
        assert_eq!(passed.message(), json!("API Connection succeeded"));
        let seen = ombi.seen();
        assert_eq!(seen[0].path, "/api/v1/Settings/about");
        assert_eq!(seen[0].header("apikey"), Some(OMBI_KEY));
        let wrong = test("ombi", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
        assert_eq!(wrong.message(), json!("API Connection failed"));
        let none = test("ombi", &settings("")).await;
        assert_eq!(none.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(none.message(), json!("Ombi Token is not defined"));
        let none = test("ombi", &[]).await;
        assert_eq!(none.message(), json!("Ombi URL is not defined"));

        let sickrage = homepage_services::sickrage().await;
        let settings = |address: &str, key: &str| {
            vec![
                ("sickrageURL", json!(address)),
                ("sickrageToken", json!(key)),
            ]
        };
        let passed = test("sickrage", &settings(&sickrage.url(), SICKRAGE_KEY)).await;
        assert_eq!(passed.status, StatusCode::OK, "{}", passed.body);
        assert_eq!(
            sickrage.seen()[0].path,
            format!("/api/{SICKRAGE_KEY}/?cmd=sb")
        );
        // The install that failed is named by its address, which holds no key.
        let wrong = test("sickrage", &settings(&sickrage.url(), "a-wrong-key")).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            wrong.message(),
            json!(format!("{}: WRONG API KEY", sickrage.url()))
        );
        let down = test("sickrage", &settings("http://127.0.0.1:9", SICKRAGE_KEY)).await;
        assert_eq!(down.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!down.body.contains(SICKRAGE_KEY), "{}", down.body);
        let none = test("sickrage", &settings(&sickrage.url(), "")).await;
        assert_eq!(none.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(none.message(), json!("SickRage Token is not defined"));
        let none = test("sickrage", &[]).await;
        assert_eq!(none.message(), json!("SickRage URL is not defined"));

        // Only an Admin or a Co-Admin runs a test.
        let app = installed_app_with(database, &[]);
        for service in ["ombi", "sickrage"] {
            let refused = call(&app, As::Guest, "POST", &format!("/test/{service}"), None).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{service}");
        }
    })
    .await;
}
