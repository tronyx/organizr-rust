//! The part of the `check` command for the applications of the homepage that came
//! after the first ones: the media servers Emby and Jellyfin, Ombi, the calendars,
//! four more download clients, the indexers, and the monitors.
//!
//! Each line asks what the homepage item asks. An application without an address
//! (or, where the address also serves a login, without a key) has no line.

use jiff::Timestamp;
use organizr_integrations::couchpotato::Install as CouchPotato;
use organizr_integrations::downloaders::{Client as Downloader, Login};
use organizr_integrations::emby::{Kind as MediaServer, Server as MediaServerInstall};
use organizr_integrations::http::Outbound;
use organizr_integrations::indexers::{Install as IndexerInstall, Kind as Indexer};
use organizr_integrations::jellystat::Install as JellyStat;
use organizr_integrations::monitorr::{self, Install as Monitorr};
use organizr_integrations::netdata::{Install as Netdata, Source};
use organizr_integrations::octoprint::Install as OctoPrint;
use organizr_integrations::ombi::Install as Ombi;
use organizr_integrations::sickrage::Install as SickRage;
use organizr_integrations::trakt::{self, Account, Calendar};
use organizr_integrations::unifi::Controller;
use organizr_integrations::{
    adguard, downloaders_more, emby_live_tv, netronome, pihole, speedtest_tracker,
};
use organizr_settings::Settings;
use serde_json::Value;

use crate::check::{Line, Outcome, failed, line};
use crate::check_homepage::{TIME_LIMIT, asked, today};

/// The text of a search that no release has, as the connection test sends it.
const NO_RELEASE: &str = "this-is-just-a-test-for-organizr";

fn refused(what: &str) -> Outcome {
    failed(format!(
        "the application refused the request; look at {what}"
    ))
}

async fn ask_media_servers(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    for (kind, url, token, never_check) in [
        (
            MediaServer::Emby,
            &settings.emby_url,
            &settings.emby_token,
            settings.emby_disable_cert_check,
        ),
        (
            MediaServer::Jellyfin,
            &settings.jellyfin_url,
            &settings.jellyfin_token,
            settings.jellyfin_disable_cert_check,
        ),
    ] {
        // The address also serves the login through the media server.
        if token.is_empty() {
            continue;
        }
        let question = format!("The {} server lists what plays now", kind.name());
        asked(question, url, lines, async |url| {
            let server = MediaServerInstall {
                kind,
                url,
                token,
                timeout: Some(TIME_LIMIT),
                never_check_certificate: never_check,
            };
            match server.sessions(http).await {
                Ok(Some(Value::Array(_))) => Outcome::Passed,
                Ok(Some(_)) => failed("the answer is no list of sessions"),
                Ok(None) => refused("the API key"),
                Err(error) => failed(error),
            }
        })
        .await;
    }
    if settings.homepage_emby_live_tv_tracker_enabled && !settings.emby_token.is_empty() {
        let question = "The Emby server gives its name and its live television".to_owned();
        asked(question, &settings.emby_url, lines, async |url| {
            let server = MediaServerInstall {
                kind: MediaServer::Emby,
                url,
                token: &settings.emby_token,
                timeout: Some(TIME_LIMIT),
                never_check_certificate: settings.emby_disable_cert_check,
            };
            match emby_live_tv::test(http, &server).await {
                Ok(_) => Outcome::Passed,
                Err(error) => failed(error),
            }
        })
        .await;
    }
    ask_jellystat(settings, http, lines).await;
}

async fn ask_jellystat(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    // The server asks the internal address when the settings have one.
    let address = if settings.jelly_stat_internal_url.is_empty() {
        &settings.jelly_stat_url
    } else {
        &settings.jelly_stat_internal_url
    };
    if address.is_empty() {
        return;
    }
    let in_frame = settings.homepage_jelly_stat_display_mode == "iframe";
    let question = if in_frame {
        "JellyStat sends its page"
    } else {
        "JellyStat gives its libraries"
    };
    asked(question.to_owned(), address, lines, async |url| {
        let install = JellyStat {
            url,
            key: &settings.jelly_stat_apikey,
            never_check_certificate: settings.jelly_stat_disable_cert_check,
        };
        if in_frame {
            return match install.get(http, "", &[]).await {
                Ok(answer) if answer.is_success() => Outcome::Passed,
                Ok(answer) => failed(format!("the answer has the status {}", answer.status)),
                Err(error) => failed(error),
            };
        }
        match install.libraries(http).await {
            Ok(Some(_)) => Outcome::Passed,
            Ok(None) => refused("the API key"),
            Err(error) => failed(error),
        }
    })
    .await;
}

async fn ask_ombi(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    // The address also serves the login through Ombi.
    if settings.ombi_token.is_empty() {
        return;
    }
    let question = "Ombi gives its requests".to_owned();
    asked(question, &settings.ombi_url, lines, async |url| {
        let install = Ombi {
            url: url.to_owned(),
            key: settings.ombi_token.clone(),
            timeout: Some(TIME_LIMIT),
            never_check_certificate: settings.ombi_disable_cert_check,
        };
        match install.get(http, "v1/Request/movie").await {
            Ok(reply) if reply.is_success && reply.body.is_array() => Outcome::Passed,
            Ok(_) => failed("the answer has no requests; look at the API key"),
            Err(error) => failed(error),
        }
    })
    .await;
}

async fn ask_calendars(
    settings: &Settings,
    http: &Outbound,
    trakt_api: &str,
    lines: &mut Vec<Line>,
) {
    let installs = pihole::installs(&settings.sickrage_url, &settings.sickrage_token);
    for (number, (address, key)) in installs.iter().enumerate() {
        if settings.sickrage_url.is_empty() {
            break;
        }
        let question = format!(
            "SickRage install {} of {} gives its calendar",
            number + 1,
            installs.len()
        );
        asked(question, address, lines, async |url| {
            let install = SickRage {
                url,
                key,
                never_check_certificate: settings.sickrage_disable_cert_check,
            };
            match install.future(http).await {
                // SickRage answers a wrong key with the status 200.
                Ok(answer) => match sickrage_fault(&answer) {
                    None => Outcome::Passed,
                    Some(why) => failed(why),
                },
                Err(error) => failed(error),
            }
        })
        .await;
    }
    let installs = pihole::installs(&settings.couchpotato_url, &settings.couchpotato_token);
    for (number, (address, key)) in installs.iter().enumerate() {
        if settings.couchpotato_url.is_empty() {
            break;
        }
        let question = format!(
            "CouchPotato install {} of {} gives its films",
            number + 1,
            installs.len()
        );
        asked(question, address, lines, async |url| {
            let install = CouchPotato {
                url,
                key,
                never_check_certificate: settings.couchpotato_disable_cert_check,
            };
            match install.films(http).await {
                Ok(_) => Outcome::Passed,
                Err(error) => failed(error),
            }
        })
        .await;
    }
    ask_trakt(settings, http, trakt_api, lines).await;
}

/// Why an answer of SickRage is no calendar, or nothing.
fn sickrage_fault(answer: &Value) -> Option<String> {
    let field = |name: &str| answer.get(name).and_then(Value::as_str).unwrap_or_default();
    match field("result") {
        "success" => None,
        result => Some(match field("message") {
            "" if result.is_empty() => "the answer has no result".to_owned(),
            "" => result.to_owned(),
            message => message.to_owned(),
        }),
    }
}

/// Trakt: the calendar of the account of the stored access token. The check asks for
/// no new tokens, because it stores nothing.
async fn ask_trakt(settings: &Settings, http: &Outbound, api: &str, lines: &mut Vec<Line>) {
    if settings.trakt_client_id.is_empty() || settings.trakt_access_token.is_empty() {
        return;
    }
    let question = "Trakt gives the calendar of the account";
    let now = Timestamp::now();
    if trakt::is_over(&settings.trakt_access_token_expires, now) {
        lines.push(line(
            question,
            Outcome::Skipped(
                "the access token is over; the homepage asks Trakt for new tokens".to_owned(),
            ),
        ));
        return;
    }
    let account = Account {
        api,
        client_id: &settings.trakt_client_id,
        access_token: &settings.trakt_access_token,
        timeout: Some(TIME_LIMIT),
    };
    let outcome = match account.calendar(http, Calendar::Shows, &today(), 1).await {
        Ok(Some(_)) => Outcome::Passed,
        Ok(None) => refused("the login at Trakt in the settings"),
        Err(error) => failed(error),
    };
    lines.push(line(question, outcome));
}

/// The address of the XML-RPC endpoint of rTorrent, as the homepage makes it from the
/// address and the second address of the settings.
fn rtorrent_endpoint(address: &str, other: &str) -> String {
    let cut = |address: &str| {
        let end = address.find(['?', '#']).unwrap_or(address.len());
        address[..end].trim_end_matches('/').to_owned()
    };
    if other.is_empty() {
        let endpoint = cut(address);
        if address.contains(".php") {
            endpoint
        } else {
            format!("{endpoint}/RPC2")
        }
    } else if other.contains(address) {
        cut(other)
    } else {
        cut(&format!("{}{other}", address.trim_end_matches('/')))
    }
}

async fn ask_downloaders(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    if !settings.deluge_url.is_empty() {
        let question = "Deluge takes the login and gives its torrents".to_owned();
        asked(question, &settings.deluge_url, lines, async |url| {
            let client = Downloader {
                url,
                timeout: Some(TIME_LIMIT),
                never_check_certificate: settings.deluge_disable_cert_check,
            };
            match downloaders_more::deluge(http, client, &settings.deluge_password).await {
                Ok(_) => Outcome::Passed,
                Err(error) => failed(error),
            }
        })
        .await;
    }
    if !settings.rtorrent_url.is_empty() {
        let question = "rTorrent gives its torrents".to_owned();
        asked(question, &settings.rtorrent_url, lines, async |url| {
            let endpoint = rtorrent_endpoint(url, &settings.rtorrent_url_override);
            let client = Downloader {
                url: &endpoint,
                timeout: Some(TIME_LIMIT),
                never_check_certificate: settings.rtorrent_disable_cert_check,
            };
            let login = Login {
                username: &settings.rtorrent_username,
                password: &settings.rtorrent_password,
            };
            match downloaders_more::rtorrent(http, client, login).await {
                Ok(Some(_)) => Outcome::Passed,
                Ok(None) => failed("rTorrent gave no list of torrents"),
                Err(error) => failed(error),
            }
        })
        .await;
    }
    for (name, address, username, password, never_check) in [
        (
            "uTorrent",
            &settings.utorrent_url,
            &settings.utorrent_username,
            &settings.utorrent_password,
            settings.utorrent_disable_cert_check,
        ),
        (
            "JDownloader",
            &settings.jdownloader_url,
            &settings.jdownloader_username,
            &settings.jdownloader_password,
            settings.jdownloader_disable_cert_check,
        ),
    ] {
        if address.is_empty() {
            continue;
        }
        let question = format!("{name} gives its downloads");
        asked(question, address, lines, async |url| {
            // The address of uTorrent can hold its token in the query. The homepage
            // asks it without the query.
            let without_query = url.find(['?', '#']).map_or(url, |end| &url[..end]);
            let client = Downloader {
                url: if name == "uTorrent" {
                    without_query
                } else {
                    url
                },
                timeout: Some(TIME_LIMIT),
                never_check_certificate: never_check,
            };
            let login = Login { username, password };
            let answer = if name == "uTorrent" {
                downloaders_more::utorrent(http, client, login)
                    .await
                    .map(|torrents| torrents.is_some())
            } else {
                downloaders_more::jdownloader(http, client, login)
                    .await
                    .map(|answer| answer.is_some())
            };
            match answer {
                Ok(true) => Outcome::Passed,
                Ok(false) => refused("the user name and the password"),
                Err(error) => failed(error),
            }
        })
        .await;
    }
}

async fn ask_indexers(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    for (kind, address, key, never_check) in [
        (
            Indexer::Jackett,
            &settings.jackett_url,
            &settings.jackett_token,
            settings.jackett_disable_cert_check,
        ),
        (
            Indexer::Prowlarr,
            &settings.prowlarr_url,
            &settings.prowlarr_token,
            settings.prowlarr_disable_cert_check,
        ),
    ] {
        if address.is_empty() {
            continue;
        }
        let question = format!("{} answers a search", kind.name());
        asked(question, address, lines, async |url| {
            let install = IndexerInstall {
                url,
                key,
                never_check_certificate: never_check,
            };
            match install.search(http, kind, NO_RELEASE).await {
                Ok(reply) if reply.is_success => Outcome::Passed,
                Ok(_) => refused("the API key"),
                Err(error) => failed(error),
            }
        })
        .await;
    }
}

async fn ask_monitors(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    let installs: Vec<&str> = settings.adguard_url.split(',').collect();
    let login = adguard::authorization(&settings.adguard_username, &settings.adguard_password);
    for (number, address) in installs.iter().enumerate() {
        if settings.adguard_url.is_empty() {
            break;
        }
        let question = format!(
            "AdGuard Home install {} of {} gives its statistics",
            number + 1,
            installs.len()
        );
        asked(
            question,
            address,
            lines,
            async |url| match adguard::statistics(http, &[url.to_owned()], login.as_deref()).await {
                Ok(statistics) if statistics.data.is_empty() => failed(
                    "the application refused the request; look at the user name and the password",
                ),
                Ok(_) => Outcome::Passed,
                Err(error) => failed(error),
            },
        )
        .await;
    }
    if !settings.netdata_url.is_empty() {
        let question = "Netdata gives the load of the processors".to_owned();
        asked(question, &settings.netdata_url, lines, async |url| {
            let install = Netdata {
                url,
                timeout: Some(TIME_LIMIT),
                never_check_certificate: settings.netdata_disable_cert_check,
            };
            if install.chart(http, Source::Cpu).await.is_empty() {
                failed("no answer that the chart can use")
            } else {
                Outcome::Passed
            }
        })
        .await;
    }
    if !settings.monitorr_url.is_empty() {
        let question = "Monitorr gives the states of its services".to_owned();
        asked(question, &settings.monitorr_url, lines, async |url| {
            let install = Monitorr {
                url,
                timeout: Some(TIME_LIMIT),
                never_check_certificate: settings.monitorr_disable_cert_check,
            };
            match install.page(http).await {
                Ok(page) if monitorr::services(&page).is_empty() => {
                    failed("the page has no services")
                }
                Ok(_) => Outcome::Passed,
                Err(error) => failed(error),
            }
        })
        .await;
    }
    if !settings.speedtest_url.is_empty() {
        let question = "Speedtest Tracker gives its last result".to_owned();
        asked(
            question,
            &settings.speedtest_url,
            lines,
            async |url| match speedtest_tracker::latest(
                http,
                url,
                settings.speedtest_disable_cert_check,
            )
            .await
            {
                Ok(Some(_)) => Outcome::Passed,
                Ok(None) => failed("the application refused the request"),
                Err(error) => failed(error),
            },
        )
        .await;
    }
    ask_netronome(settings, http, lines).await;
    ask_octoprint_and_unifi(settings, http, lines).await;
}

async fn ask_netronome(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    if settings.netronome_url.is_empty() {
        return;
    }
    let question = "Netronome gives its speed tests".to_owned();
    asked(
        question,
        &settings.netronome_url,
        lines,
        async |url| match netronome::results(
            http,
            url,
            netronome::time_range(&settings.homepage_netronome_time_range),
            settings.netronome_disable_cert_check,
        )
        .await
        {
            Ok(Some(_)) => Outcome::Passed,
            Ok(None) => failed("the application refused the request"),
            Err(error) => failed(error),
        },
    )
    .await;
}

async fn ask_octoprint_and_unifi(settings: &Settings, http: &Outbound, lines: &mut Vec<Line>) {
    if !settings.octoprint_url.is_empty() {
        let question = "OctoPrint gives its job".to_owned();
        asked(question, &settings.octoprint_url, lines, async |url| {
            let install = OctoPrint {
                url,
                key: &settings.octoprint_token,
                timeout: Some(TIME_LIMIT),
                never_check_certificate: settings.octoprint_disable_cert_check,
            };
            match install.read(http, "job").await {
                Ok(Some(_)) => Outcome::Passed,
                Ok(None) => refused("the API key"),
                Err(error) => failed(error),
            }
        })
        .await;
    }
    if !settings.unifi_url.is_empty() {
        let question = "UniFi takes the login and gives the health of the site".to_owned();
        asked(question, &settings.unifi_url, lines, async |url| {
            let controller = Controller {
                url,
                username: &settings.unifi_username,
                password: &settings.unifi_password,
                site: &settings.unifi_site_name,
                timeout: Some(TIME_LIMIT),
                never_check_certificate: settings.unifi_disable_cert_check,
            };
            match controller.health(http).await {
                Ok(_) => Outcome::Passed,
                Err(error) => failed(error),
            }
        })
        .await;
    }
}

/// Asks each of these applications that the settings have an address for. `trakt_api`
/// is the address of the API of Trakt, or of a stand-in for a test.
pub async fn ask(settings: &Settings, http: &Outbound, trakt_api: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    ask_media_servers(settings, http, &mut lines).await;
    ask_ombi(settings, http, &mut lines).await;
    ask_calendars(settings, http, trakt_api, &mut lines).await;
    ask_downloaders(settings, http, &mut lines).await;
    ask_indexers(settings, http, &mut lines).await;
    ask_monitors(settings, http, &mut lines).await;
    lines
}

#[cfg(test)]
mod tests {
    use organizr_integrations::http::Outbound;
    use organizr_testkit::homepage_services::{
        self, COUCHPOTATO_KEY, DELUGE_PASSWORD, JACKETT_KEY, JDOWNLOADER_LOGIN, JELLYSTAT_KEY,
        MEDIA_SERVER_KEY, OCTOPRINT_KEY, OMBI_KEY, PROWLARR_KEY, RTORRENT_LOGIN, SICKRAGE_KEY,
        TRAKT_CLIENT, TRAKT_TOKENS, UNIFI_LOGIN, UTORRENT_LOGIN,
    };
    use organizr_testkit::installed_settings;
    use serde_json::{Value, json};

    use super::{ask, rtorrent_endpoint};
    use crate::check::{Line, Outcome};

    fn report(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                let result = match &line.outcome {
                    Outcome::Passed => "ok",
                    Outcome::Failed(_) => "FAILED",
                    Outcome::Skipped(_) => "skipped",
                };
                format!("{result} {}", line.question)
            })
            .collect()
    }

    fn printed(lines: &[Line]) -> String {
        lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The questions of a report that asks every application of this part.
    const QUESTIONS: [&str; 20] = [
        "The Emby server lists what plays now",
        "The Jellyfin server lists what plays now",
        "JellyStat gives its libraries",
        "Ombi gives its requests",
        "SickRage install 1 of 1 gives its calendar",
        "CouchPotato install 1 of 1 gives its films",
        "Trakt gives the calendar of the account",
        "Deluge takes the login and gives its torrents",
        "rTorrent gives its torrents",
        "uTorrent gives its downloads",
        "JDownloader gives its downloads",
        "Jackett answers a search",
        "Prowlarr answers a search",
        "AdGuard Home install 1 of 1 gives its statistics",
        "Netdata gives the load of the processors",
        "Monitorr gives the states of its services",
        "Speedtest Tracker gives its last result",
        "Netronome gives its speed tests",
        "OctoPrint gives its job",
        "UniFi takes the login and gives the health of the site",
    ];

    fn all(result: &str) -> Vec<String> {
        QUESTIONS
            .iter()
            .map(|question| format!("{result} {question}"))
            .collect()
    }

    #[test]
    fn endpoint_of_rtorrent_is_made_as_the_homepage_makes_it() {
        assert_eq!(
            rtorrent_endpoint("http://host:8080/", ""),
            "http://host:8080/RPC2"
        );
        assert_eq!(
            rtorrent_endpoint("http://host/rutorrent/plugins/httprpc/action.php", ""),
            "http://host/rutorrent/plugins/httprpc/action.php"
        );
        assert_eq!(
            rtorrent_endpoint("http://host", "/digest/RPC2"),
            "http://host/digest/RPC2"
        );
        assert_eq!(
            rtorrent_endpoint("http://host", "http://host/other/RPC2?x=1"),
            "http://host/other/RPC2"
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
        let (emby, jellyfin, jellystat) = (
            homepage_services::emby().await,
            homepage_services::jellyfin().await,
            homepage_services::jellystat().await,
        );
        let (ombi, sickrage, couchpotato, trakt) = (
            homepage_services::ombi_requests().await,
            homepage_services::sickrage().await,
            homepage_services::couchpotato().await,
            homepage_services::trakt().await,
        );
        let (deluge, rtorrent, utorrent, jdownloader) = (
            homepage_services::deluge().await,
            homepage_services::rtorrent().await,
            homepage_services::utorrent().await,
            homepage_services::jdownloader().await,
        );
        let (jackett, prowlarr) = (
            homepage_services::jackett().await,
            homepage_services::prowlarr().await,
        );
        let (adguard, netdata, monitorr, speedtest) = (
            homepage_services::adguard().await,
            homepage_services::netdata().await,
            homepage_services::monitorr().await,
            homepage_services::speedtest_tracker().await,
        );
        let (octoprint, unifi, netronome) = (
            homepage_services::octoprint().await,
            homepage_services::unifi_classic().await,
            homepage_services::netronome().await,
        );
        let settings: Vec<(&str, Value)> = vec![
            ("embyURL", json!(emby.url())),
            ("embyToken", json!(MEDIA_SERVER_KEY)),
            ("jellyfinURL", json!(jellyfin.url())),
            ("jellyfinToken", json!(MEDIA_SERVER_KEY)),
            ("jellyStatURL", json!(jellystat.url())),
            ("jellyStatApikey", json!(JELLYSTAT_KEY)),
            ("ombiURL", json!(ombi.url())),
            ("ombiToken", json!(OMBI_KEY)),
            ("sickrageURL", json!(sickrage.url())),
            ("sickrageToken", json!(SICKRAGE_KEY)),
            ("couchpotatoURL", json!(couchpotato.url())),
            ("couchpotatoToken", json!(COUCHPOTATO_KEY)),
            ("traktClientId", json!(TRAKT_CLIENT.0)),
            ("traktAccessToken", json!(TRAKT_TOKENS.0)),
            ("traktAccessTokenExpires", json!("2099-01-01T00:00:00Z")),
            ("delugeURL", json!(deluge.url())),
            ("delugePassword", json!(DELUGE_PASSWORD)),
            ("rTorrentURL", json!(rtorrent.url())),
            ("rTorrentUsername", json!(RTORRENT_LOGIN.0)),
            ("rTorrentPassword", json!(RTORRENT_LOGIN.1)),
            ("uTorrentURL", json!(utorrent.url())),
            ("uTorrentUsername", json!(UTORRENT_LOGIN.0)),
            ("uTorrentPassword", json!(UTORRENT_LOGIN.1)),
            ("jdownloaderURL", json!(jdownloader.url())),
            ("jdownloaderUsername", json!(JDOWNLOADER_LOGIN.0)),
            ("jdownloaderPassword", json!(JDOWNLOADER_LOGIN.1)),
            ("jackettURL", json!(jackett.url())),
            ("jackettToken", json!(JACKETT_KEY)),
            ("prowlarrURL", json!(prowlarr.url())),
            ("prowlarrToken", json!(PROWLARR_KEY)),
            ("adguardURL", json!(adguard.url())),
            ("netdataURL", json!(netdata.url())),
            ("monitorrURL", json!(monitorr.url())),
            ("speedtestURL", json!(speedtest.url())),
            ("netronomeURL", json!(netronome.url())),
            ("octoprintURL", json!(octoprint.url())),
            ("octoprintToken", json!(OCTOPRINT_KEY)),
            ("unifiURL", json!(unifi.url())),
            ("unifiUsername", json!(UNIFI_LOGIN.0)),
            ("unifiPassword", json!(UNIFI_LOGIN.1)),
            ("unifiSiteName", json!("default")),
        ];
        let http = Outbound::new().expect("the client can be made");

        let lines = ask(&installed_settings(&settings), &http, &trakt.url()).await;

        assert_eq!(report(&lines), all("ok"), "{lines:?}");
        // The report names no key, no password and no address.
        let text = printed(&lines);
        for secret in [
            MEDIA_SERVER_KEY,
            JELLYSTAT_KEY,
            OMBI_KEY,
            SICKRAGE_KEY,
            COUCHPOTATO_KEY,
            TRAKT_TOKENS.0,
            DELUGE_PASSWORD,
            RTORRENT_LOGIN.1,
            UTORRENT_LOGIN.1,
            JDOWNLOADER_LOGIN.1,
            JACKETT_KEY,
            PROWLARR_KEY,
            OCTOPRINT_KEY,
            UNIFI_LOGIN.1,
            "127.0.0.1",
        ] {
            assert!(!text.contains(secret), "{secret} is in the report: {text}");
        }
    }

    #[tokio::test]
    async fn wrong_keys_and_wrong_addresses_fail() {
        let (emby, jellystat, ombi, sickrage, couchpotato, trakt) = (
            homepage_services::emby().await,
            homepage_services::jellystat().await,
            homepage_services::ombi_requests().await,
            homepage_services::sickrage().await,
            homepage_services::couchpotato().await,
            homepage_services::trakt().await,
        );
        let (deluge, rtorrent, utorrent, jdownloader) = (
            homepage_services::deluge().await,
            homepage_services::rtorrent().await,
            homepage_services::utorrent().await,
            homepage_services::jdownloader().await,
        );
        let (jackett, prowlarr, adguard) = (
            homepage_services::jackett().await,
            homepage_services::prowlarr().await,
            homepage_services::adguard_with_login().await,
        );
        let (netdata, monitorr, octoprint, unifi) = (
            homepage_services::netdata().await,
            homepage_services::monitorr().await,
            homepage_services::octoprint().await,
            homepage_services::unifi_classic().await,
        );
        let wrong = json!("a-wrong-key");
        let settings: Vec<(&str, Value)> = vec![
            ("embyURL", json!(emby.url())),
            ("embyToken", wrong.clone()),
            // Jellyfin sends its key in another header than Emby.
            ("jellyfinURL", json!(emby.url())),
            ("jellyfinToken", json!(MEDIA_SERVER_KEY)),
            ("jellyStatURL", json!(jellystat.url())),
            ("jellyStatApikey", wrong.clone()),
            ("ombiURL", json!(ombi.url())),
            ("ombiToken", wrong.clone()),
            ("sickrageURL", json!(sickrage.url())),
            ("sickrageToken", wrong.clone()),
            ("couchpotatoURL", json!(couchpotato.url())),
            ("couchpotatoToken", wrong.clone()),
            ("traktClientId", json!(TRAKT_CLIENT.0)),
            ("traktAccessToken", wrong.clone()),
            ("delugeURL", json!(deluge.url())),
            ("delugePassword", wrong.clone()),
            ("rTorrentURL", json!(rtorrent.url())),
            ("rTorrentUsername", json!(RTORRENT_LOGIN.0)),
            ("rTorrentPassword", wrong.clone()),
            ("uTorrentURL", json!(utorrent.url())),
            ("uTorrentUsername", json!(UTORRENT_LOGIN.0)),
            ("uTorrentPassword", wrong.clone()),
            ("jdownloaderURL", json!(jdownloader.url())),
            ("jdownloaderUsername", json!(JDOWNLOADER_LOGIN.0)),
            ("jdownloaderPassword", wrong.clone()),
            ("jackettURL", json!(jackett.url())),
            ("jackettToken", wrong.clone()),
            ("prowlarrURL", json!(prowlarr.url())),
            ("prowlarrToken", wrong.clone()),
            // This AdGuard Home needs a login, which the homepage does not send.
            ("adguardURL", json!(adguard.url())),
            // Each of these four is asked at an application that does not know the path.
            ("netdataURL", json!(monitorr.url())),
            ("monitorrURL", json!(netdata.url())),
            ("speedtestURL", json!(netdata.url())),
            ("netronomeURL", json!(netdata.url())),
            ("octoprintURL", json!(octoprint.url())),
            ("octoprintToken", wrong.clone()),
            ("unifiURL", json!(unifi.url())),
            ("unifiUsername", json!(UNIFI_LOGIN.0)),
            ("unifiPassword", json!("a wrong password")),
            ("unifiSiteName", json!("default")),
        ];
        let http = Outbound::new().expect("the client can be made");

        let lines = ask(&installed_settings(&settings), &http, &trakt.url()).await;

        assert_eq!(report(&lines), all("FAILED"), "{lines:?}");
        let text = printed(&lines);
        assert!(!text.contains("127.0.0.1"), "{text}");
    }

    #[tokio::test]
    async fn live_television_and_the_page_of_jellystat_are_asked_when_they_are_on() {
        let (live_tv, jellystat, trakt) = (
            homepage_services::emby_live_tv().await,
            homepage_services::jellystat().await,
            homepage_services::trakt().await,
        );
        let settings: Vec<(&str, Value)> = vec![
            ("embyURL", json!(live_tv.url())),
            ("embyToken", json!(MEDIA_SERVER_KEY)),
            ("homepageEmbyLiveTVTrackerEnabled", json!(true)),
            ("jellyStatURL", json!("https://jellystat.example.com")),
            ("jellyStatInternalURL", json!(jellystat.url())),
            ("homepageJellyStatDisplayMode", json!("iframe")),
            ("traktClientId", json!(TRAKT_CLIENT.0)),
            ("traktAccessToken", json!(TRAKT_TOKENS.0)),
            ("traktAccessTokenExpires", json!("2020-01-01T00:00:00Z")),
            ("rTorrentURL", json!("/rtorrent")),
        ];
        let http = Outbound::new().expect("the client can be made");

        let lines = ask(&installed_settings(&settings), &http, &trakt.url()).await;

        assert_eq!(
            report(&lines),
            [
                // The stand-in for the live television has no list of sessions.
                "FAILED The Emby server lists what plays now",
                "ok The Emby server gives its name and its live television",
                "ok JellyStat sends its page",
                // The check stores nothing, so it asks Trakt for no new tokens.
                "skipped Trakt gives the calendar of the account",
                // An address without a scheme names no host.
                "skipped rTorrent gives its torrents",
            ],
            "{lines:?}"
        );
    }
}
