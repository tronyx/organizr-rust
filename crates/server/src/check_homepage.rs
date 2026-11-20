//! The part of the `check` command for the applications of the homepage: each one is
//! asked what its homepage item asks it.
//!
//! An application without an address in the settings has no line in the report.

use std::time::{Duration, SystemTime};

use organizr_integrations::arr::{Install as ArrInstall, Kind};
use organizr_integrations::downloaders::{self, Client as Downloader, TorrentQuery};
use organizr_integrations::http::Outbound;
use organizr_integrations::overseerr::Install as Overseerr;
use organizr_integrations::plex_server::Server as PlexServer;
use organizr_integrations::tautulli::Install as Tautulli;
use organizr_integrations::weather::{self, Part, Place};
use organizr_integrations::{health_checks, uptime_kuma};
use organizr_settings::Settings;
use serde_json::Value;

use crate::check::{Line, Outcome, complete, failed, line};

/// How long each application has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(20);

/// A day as the calendars take it: `2026-11-20`.
fn today() -> String {
    let seconds = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    // The date of a number of days since 1970, by the usual rules of the calendar.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = if month_part < 10 {
        month_part + 3
    } else {
        month_part - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// The first address of a setting that can hold several, with commas between them.
fn first(addresses: &str) -> &str {
    addresses.split(',').next().unwrap_or_default()
}

/// A line for an application: skipped when its address is not complete, and else
/// what the question brought.
async fn asked<F>(question: String, address: &str, lines: &mut Vec<Line>, ask: F)
where
    F: AsyncFnOnce(&str) -> Outcome,
{
    let outcome = match complete(address) {
        Ok(url) => ask(url).await,
        Err(why) => why,
    };
    lines.push(line(question, outcome));
}

async fn ask_arr(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    let today = today();
    for (kind, name, urls, keys, never_check) in [
        (
            Kind::Sonarr,
            "Sonarr",
            &settings.sonarr_url,
            &settings.sonarr_token,
            settings.sonarr_disable_cert_check,
        ),
        (
            Kind::Radarr,
            "Radarr",
            &settings.radarr_url,
            &settings.radarr_token,
            settings.radarr_disable_cert_check,
        ),
        (
            Kind::Lidarr,
            "Lidarr",
            &settings.lidarr_url,
            &settings.lidarr_token,
            settings.lidarr_disable_cert_check,
        ),
    ] {
        if urls.is_empty() {
            continue;
        }
        let keys: Vec<&str> = keys.split(',').collect();
        let installs: Vec<&str> = urls.split(',').collect();
        for (number, address) in installs.iter().enumerate() {
            let question = format!(
                "{name} install {} of {} gives its calendar",
                number + 1,
                installs.len()
            );
            let key = keys.get(number).copied().unwrap_or_default();
            asked(question, address, lines, async |url| {
                let install = ArrInstall {
                    kind,
                    url,
                    key,
                    timeout: Some(TIME_LIMIT),
                    never_check_certificate: never_check,
                };
                match install.calendar(http, &today, &today, false).await {
                    Ok(_) => Outcome::Passed,
                    Err(error) => failed(error),
                }
            })
            .await;
        }
    }
}

async fn ask_downloaders(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    if !settings.sabnzbd_url.is_empty() {
        let question = "SABnzbd gives its queue".to_owned();
        asked(question, &settings.sabnzbd_url, lines, async |url| {
            let client = Downloader {
                url,
                timeout: Some(TIME_LIMIT),
                never_check_certificate: settings.sabnzbd_disable_cert_check,
            };
            let fields = [("mode", "queue")];
            match downloaders::sabnzbd(http, client, &settings.sabnzbd_token, &fields).await {
                Ok(Some(answer)) if answer.get("queue").is_some() => Outcome::Passed,
                Ok(_) => failed("the answer has no queue; look at the API key"),
                Err(error) => failed(error),
            }
        })
        .await;
    }
    if !settings.qbittorrent_url.is_empty() {
        let question = "qBittorrent takes the login and gives its torrents".to_owned();
        asked(question, &settings.qbittorrent_url, lines, async |url| {
            let client = Downloader {
                url,
                timeout: Some(TIME_LIMIT),
                never_check_certificate: settings.qbittorrent_disable_cert_check,
            };
            let query = TorrentQuery {
                username: &settings.qbittorrent_username,
                password: &settings.qbittorrent_password,
                first_api: settings.qbittorrent_api_version == 1,
                sort: &settings.qbittorrent_sort_order,
                reverse: settings.qbittorrent_reverse_sorting,
            };
            match downloaders::qbittorrent(http, client, query).await {
                Ok(_) => Outcome::Passed,
                Err(error) => failed(error),
            }
        })
        .await;
    }
}

async fn ask_monitors(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    // The address of Healthchecks has a default. Only an install with a key uses it.
    if !settings.health_checks_token.is_empty() {
        let question = "Healthchecks gives the checks of each project".to_owned();
        asked(question, &settings.health_checks_url, lines, async |url| {
            for (number, key) in settings.health_checks_token.split(',').enumerate() {
                let checks = health_checks::checks(
                    http,
                    url,
                    key,
                    "",
                    Some(TIME_LIMIT),
                    settings.health_checks_disable_cert_check,
                )
                .await;
                if checks.is_empty() {
                    return failed(format!(
                        "no checks for key {}; a wrong key, a wrong address, or a project without checks",
                        number + 1
                    ));
                }
            }
            Outcome::Passed
        })
        .await;
    }
    if !settings.uptime_kuma_url.is_empty() {
        let question = "Uptime Kuma gives its monitors".to_owned();
        asked(
            question,
            &settings.uptime_kuma_url,
            lines,
            async |url| match uptime_kuma::metrics(http, url, &settings.uptime_kuma_token).await {
                Ok(page) if uptime_kuma::monitors(&page).is_empty() => {
                    failed("the page has no monitors that were checked")
                }
                Ok(_) => Outcome::Passed,
                Err(error) => failed(error),
            },
        )
        .await;
    }
}

async fn ask_weather(settings: &Settings, http: &Outbound, api: &str, lines: &mut Vec<Line>) {
    if settings.homepage_weather_and_air_latitude.is_empty() {
        return;
    }
    let place = Place {
        api,
        latitude: &settings.homepage_weather_and_air_latitude,
        longitude: &settings.homepage_weather_and_air_longitude,
        units: &settings.homepage_weather_and_air_units,
        key: &settings.breezometer_token,
        timeout: Some(TIME_LIMIT),
    };
    for (part, name, is_on) in [
        (
            Part::Weather,
            "the weather",
            settings.homepage_weather_and_air_weather_enabled,
        ),
        (
            Part::Air,
            "the air quality",
            settings.homepage_weather_and_air_air_quality_enabled,
        ),
        (
            Part::Pollen,
            "the pollen",
            settings.homepage_weather_and_air_pollen_enabled,
        ),
    ] {
        if !is_on {
            continue;
        }
        let outcome = match weather::part(http, place, part).await {
            Ok(Value::Bool(false)) => {
                failed("the service refused the request; look at the API key")
            }
            Ok(_) => Outcome::Passed,
            Err(error) => failed(error),
        };
        lines.push(line(format!("The weather service gives {name}"), outcome));
    }
}

async fn ask_plex_and_friends(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    if !settings.plex_url.is_empty() {
        let question = "The Plex server lists what plays now".to_owned();
        asked(question, &settings.plex_url, lines, async |url| {
            let server = PlexServer {
                url,
                token: &settings.plex_token,
                timeout: Some(TIME_LIMIT),
                never_check_certificate: settings.plex_disable_cert_check,
            };
            match server.xml(http, "/status/sessions", &[]).await {
                Ok(Some(xml)) if xml.contains("<MediaContainer") => Outcome::Passed,
                Ok(Some(_)) => failed("the answer is no list of sessions"),
                Ok(None) => failed("the server refused the request; look at the Plex token"),
                Err(error) => failed(error),
            }
        })
        .await;
    }
    if !settings.tautulli_apikey.is_empty() {
        let question = "Tautulli gives its statistics".to_owned();
        asked(
            question,
            first(&settings.tautulli_url),
            lines,
            async |url| {
                let install = Tautulli {
                    url,
                    key: &settings.tautulli_apikey,
                    timeout: Some(TIME_LIMIT),
                    never_check_certificate: settings.tautulli_disable_cert_check,
                };
                let more = [("grouping", "1")];
                match install.command(http, "get_home_stats", &more).await {
                    Ok(Some(answer)) if answer.get("data").is_some_and(Value::is_array) => {
                        Outcome::Passed
                    }
                    Ok(_) => failed("the answer has no statistics; look at the API key"),
                    Err(error) => failed(error),
                }
            },
        )
        .await;
    }
    if !settings.overseerr_token.is_empty() {
        let question = "Overseerr gives its requests".to_owned();
        asked(question, &settings.overseerr_url, lines, async |url| {
            let install = Overseerr {
                url: url.to_owned(),
                key: settings.overseerr_token.clone(),
                timeout: Some(TIME_LIMIT),
                never_check_certificate: settings.overseerr_disable_cert_check,
            };
            match install.get(http, "request?take=1&skip=0").await {
                Ok(reply) if reply.is_success && reply.body.get("results").is_some() => {
                    Outcome::Passed
                }
                Ok(_) => failed("the answer has no requests; look at the API key"),
                Err(error) => failed(error),
            }
        })
        .await;
    }
}

/// Asks each application of the homepage that the settings have an address for.
/// `weather_api` is the address of the weather API, or of a stand-in for a test.
pub async fn ask(settings: &Settings, http: &Outbound, weather_api: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    ask_plex_and_friends(settings, http, &mut lines).await;
    ask_arr(settings, http, &mut lines).await;
    ask_downloaders(settings, http, &mut lines).await;
    ask_monitors(settings, http, &mut lines).await;
    ask_weather(settings, http, weather_api, &mut lines).await;
    lines
}

#[cfg(test)]
mod tests {
    use organizr_integrations::http::Outbound;
    use organizr_testkit::homepage_services::{
        self, ARR_KEY, HEALTH_KEYS, KUMA_KEY, OVERSEERR_KEY, PLEX_SERVER_TOKEN, QBITTORRENT_LOGIN,
        SABNZBD_KEY, TAUTULLI_KEY, WEATHER_KEY,
    };
    use organizr_testkit::installed_settings;
    use serde_json::{Value, json};

    use super::{ask, today};
    use crate::check::Outcome;

    #[test]
    fn today_is_a_date() {
        let today = today();
        let parts: Vec<&str> = today.split('-').collect();
        assert_eq!(parts.len(), 3, "{today}");
        assert!(
            parts[0].parse::<u32>().is_ok_and(|year| year >= 2026),
            "{today}"
        );
        assert!(
            parts[1]
                .parse::<u32>()
                .is_ok_and(|month| (1..=12).contains(&month))
        );
        assert!(
            parts[2]
                .parse::<u32>()
                .is_ok_and(|day| (1..=31).contains(&day))
        );
    }

    #[tokio::test]
    async fn install_without_addresses_has_no_lines() {
        let http = Outbound::new().expect("the client can be made");
        let lines = ask(&installed_settings(&[]), &http, "http://127.0.0.1:1").await;
        assert_eq!(lines, []);
    }

    #[tokio::test]
    async fn every_application_that_answers_as_expected_passes() {
        let plex = homepage_services::plex_server().await;
        let tautulli = homepage_services::tautulli_statistics().await;
        let overseerr = homepage_services::overseerr_requests().await;
        let (sonarr, radarr, lidarr) = (
            homepage_services::sonarr().await,
            homepage_services::radarr().await,
            homepage_services::lidarr().await,
        );
        let (sabnzbd, qbittorrent) = (
            homepage_services::sabnzbd().await,
            homepage_services::qbittorrent().await,
        );
        let (healthchecks, kuma) = (
            homepage_services::healthchecks().await,
            homepage_services::uptime_kuma().await,
        );
        let weather = homepage_services::breezometer().await;
        let settings: Vec<(&str, Value)> = vec![
            ("plexURL", json!(plex.url())),
            ("plexToken", json!(PLEX_SERVER_TOKEN)),
            ("tautulliURL", json!(tautulli.url())),
            ("tautulliApikey", json!(TAUTULLI_KEY)),
            ("overseerrURL", json!(overseerr.url())),
            ("overseerrToken", json!(OVERSEERR_KEY)),
            ("sonarrURL", json!(sonarr.url())),
            ("sonarrToken", json!(ARR_KEY)),
            (
                "radarrURL",
                json!(format!("{},{}", radarr.url(), radarr.url())),
            ),
            ("radarrToken", json!(format!("{ARR_KEY},a-wrong-key"))),
            ("lidarrURL", json!(lidarr.url())),
            ("lidarrToken", json!(ARR_KEY)),
            ("sabnzbdURL", json!(sabnzbd.url())),
            ("sabnzbdToken", json!(SABNZBD_KEY)),
            ("qBittorrentURL", json!(qbittorrent.url())),
            ("qBittorrentUsername", json!(QBITTORRENT_LOGIN.0)),
            ("qBittorrentPassword", json!(QBITTORRENT_LOGIN.1)),
            ("qBittorrentApiVersion", json!("2")),
            (
                "healthChecksURL",
                json!(format!("{}/api/v1/checks", healthchecks.url())),
            ),
            ("healthChecksToken", json!(HEALTH_KEYS.0)),
            ("uptimeKumaURL", json!(kuma.url())),
            ("uptimeKumaToken", json!(KUMA_KEY)),
            ("homepageWeatherAndAirLatitude", json!("52.52")),
            ("homepageWeatherAndAirLongitude", json!("13.4")),
            ("homepageWeatherAndAirWeatherEnabled", json!(true)),
            ("homepageWeatherAndAirPollenEnabled", json!(true)),
            ("breezometerToken", json!(WEATHER_KEY)),
        ];
        let http = Outbound::new().expect("the client can be made");

        let lines = ask(&installed_settings(&settings), &http, &weather.url()).await;

        let report: Vec<String> = lines
            .iter()
            .map(|line| {
                let result = match &line.outcome {
                    Outcome::Passed => "ok",
                    Outcome::Failed(_) => "FAILED",
                    Outcome::Skipped(_) => "skipped",
                };
                format!("{result} {}", line.question)
            })
            .collect();
        assert_eq!(
            report,
            [
                "ok The Plex server lists what plays now",
                "ok Tautulli gives its statistics",
                "ok Overseerr gives its requests",
                "ok Sonarr install 1 of 1 gives its calendar",
                "ok Radarr install 1 of 2 gives its calendar",
                // The second install of Radarr has a wrong key.
                "FAILED Radarr install 2 of 2 gives its calendar",
                "ok Lidarr install 1 of 1 gives its calendar",
                "ok SABnzbd gives its queue",
                "ok qBittorrent takes the login and gives its torrents",
                "ok Healthchecks gives the checks of each project",
                "ok Uptime Kuma gives its monitors",
                "ok The weather service gives the weather",
                "ok The weather service gives the pollen",
            ]
        );
        // The report names no key and no address.
        let text = lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        for secret in [
            ARR_KEY,
            SABNZBD_KEY,
            TAUTULLI_KEY,
            WEATHER_KEY,
            PLEX_SERVER_TOKEN,
            "127.0.0.1",
        ] {
            assert!(!text.contains(secret), "{secret} is in the report: {text}");
        }
    }

    #[tokio::test]
    async fn wrong_keys_fail_and_an_address_without_a_scheme_is_skipped() {
        let (sabnzbd, qbittorrent, kuma) = (
            homepage_services::sabnzbd().await,
            homepage_services::qbittorrent().await,
            homepage_services::uptime_kuma().await,
        );
        let tautulli = homepage_services::tautulli_statistics().await;
        let weather = homepage_services::breezometer().await;
        let settings: Vec<(&str, Value)> = vec![
            ("plexURL", json!("/plex")),
            ("plexToken", json!(PLEX_SERVER_TOKEN)),
            ("tautulliURL", json!(tautulli.url())),
            ("tautulliApikey", json!("a-wrong-key")),
            ("sabnzbdURL", json!(sabnzbd.url())),
            ("sabnzbdToken", json!("a-wrong-key")),
            ("qBittorrentURL", json!(qbittorrent.url())),
            ("qBittorrentUsername", json!(QBITTORRENT_LOGIN.0)),
            ("qBittorrentPassword", json!("a wrong password")),
            ("qBittorrentApiVersion", json!("2")),
            ("uptimeKumaURL", json!(kuma.url())),
            ("uptimeKumaToken", json!("a-wrong-key")),
            ("homepageWeatherAndAirLatitude", json!("52.52")),
            ("homepageWeatherAndAirLongitude", json!("13.4")),
            ("homepageWeatherAndAirAirQualityEnabled", json!(true)),
            ("breezometerToken", json!("a-wrong-key")),
        ];
        let http = Outbound::new().expect("the client can be made");

        let lines = ask(&installed_settings(&settings), &http, &weather.url()).await;

        assert_eq!(lines.len(), 6, "{lines:?}");
        assert!(
            matches!(lines[0].outcome, Outcome::Skipped(_)),
            "{:?}",
            lines[0]
        );
        for line in &lines[1..] {
            assert!(matches!(line.outcome, Outcome::Failed(_)), "{line:?}");
        }
        assert_eq!(
            lines[5].question,
            "The weather service gives the air quality"
        );
    }
}
