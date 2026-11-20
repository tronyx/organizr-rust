//! The `check` command: asks the services of the stored settings the questions that a
//! login asks them, and prints for each one whether the answer was usable.
//!
//! The tests of this application talk to stand-ins of plex.tv, Ombi, Overseerr and
//! Tautulli. This command shows whether the real services answer in the same way. It
//! prints no token, no password, no address and no name of an account.

use std::fmt;
use std::time::Duration;

use anyhow::Context;
use organizr_integrations::http::Outbound;
use organizr_integrations::plex::{PLEX_TV, PlexAccount, PlexTv};
use organizr_integrations::sso::{self, Fallback, Login};
use organizr_integrations::trakt::TRAKT_API;
use organizr_integrations::weather::BREEZOMETER;
use organizr_settings::Settings;
use tracing_subscriber::EnvFilter;

use crate::serve::{StorageArgs, load_settings, open_database};

/// How long each service has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(20);

/// The options of `organizr check`.
#[derive(Debug, clap::Args)]
pub struct Args {
    #[command(flatten)]
    storage: StorageArgs,
}

/// What one question brought.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The service gave an answer that the application can use.
    Passed,
    /// The service gave no usable answer. The text says what went wrong.
    Failed(String),
    /// The question was not asked. The text says why.
    Skipped(String),
}

/// One line of the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// What was asked.
    pub question: String,
    /// What it brought.
    pub outcome: Outcome,
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.outcome {
            Outcome::Passed => write!(f, "ok       {}", self.question),
            Outcome::Failed(why) => write!(f, "FAILED   {}: {why}", self.question),
            Outcome::Skipped(why) => write!(f, "skipped  {}: {why}", self.question),
        }
    }
}

pub fn line(question: impl Into<String>, outcome: Outcome) -> Line {
    Line {
        question: question.into(),
        outcome,
    }
}

pub fn failed(why: impl fmt::Display) -> Outcome {
    Outcome::Failed(why.to_string())
}

fn skipped(why: &str) -> Outcome {
    Outcome::Skipped(why.to_owned())
}

/// Runs the check, prints the report, and fails when a service gave no usable answer.
pub async fn run(args: Args) -> anyhow::Result<()> {
    // The clients say in the log why a service gave no usable answer. The log goes to
    // the standard error stream, so the report stays on its own.
    let filter = EnvFilter::try_from_env("ORGANIZR_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let database = open_database(&args.storage).await?;
    let secrets = args.storage.secrets()?;
    let settings = load_settings(&database, &secrets).await?;
    database
        .close()
        .await
        .context("the database did not close cleanly")?;
    let http = Outbound::new().context("cannot make the client for the requests")?;

    let mut lines = ask(&settings, &http, PLEX_TV).await;
    lines.extend(crate::check_homepage::ask(&settings, &http, BREEZOMETER).await);
    lines.extend(crate::check_homepage_more::ask(&settings, &http, TRAKT_API).await);
    for line in &lines {
        println!("{line}");
    }
    let failures = lines
        .iter()
        .filter(|line| matches!(line.outcome, Outcome::Failed(_)))
        .count();
    anyhow::ensure!(
        failures == 0,
        "{failures} of {} questions got no usable answer",
        lines.len()
    );
    Ok(())
}

/// The address of a service as the settings hold it, when the check can ask it: the
/// address must say `http://` or `https://`.
pub fn complete(address: &str) -> Result<&str, Outcome> {
    let address = address.trim();
    if address.is_empty() {
        Err(skipped("the settings hold no address"))
    } else if address.starts_with("http://") || address.starts_with("https://") {
        Ok(address)
    } else {
        Err(skipped(
            "the address does not start with http:// or https://, so only a browser knows the host",
        ))
    }
}

/// Asks every service. `plex_tv` is the address of plex.tv, or of a stand-in for a test.
pub async fn ask(settings: &Settings, http: &Outbound, plex_tv: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    if let Some(account) = ask_plex(settings, http, plex_tv, &mut lines).await {
        ask_applications(settings, http, &account, &mut lines).await;
    }
    lines
}

/// Asks plex.tv about the stored token. Returns the account of the token, or nothing
/// when the applications cannot be asked without it.
async fn ask_plex(
    settings: &Settings,
    http: &Outbound,
    plex_tv: &str,
    lines: &mut Vec<Line>,
) -> Option<PlexAccount> {
    let token = settings.plex_token.as_str();
    if token.is_empty() {
        lines.push(line(
            "plex.tv knows the stored Plex token",
            skipped("the settings hold no Plex token"),
        ));
        return None;
    }

    let plex = PlexTv::new(http.clone(), plex_tv);
    let account = match plex.account(token).await {
        Ok(Some(account)) => {
            lines.push(line("plex.tv knows the stored Plex token", Outcome::Passed));
            account
        }
        Ok(None) => {
            lines.push(line(
                "plex.tv knows the stored Plex token",
                failed("plex.tv refused the token"),
            ));
            return None;
        }
        Err(error) => {
            lines.push(line("plex.tv knows the stored Plex token", failed(error)));
            return None;
        }
    };

    lines.push(line(
        "plex.tv lists the Plex servers, and the stored id is one of them",
        match plex.servers(token).await {
            Ok(servers) if settings.plex_id.is_empty() => skipped(&format!(
                "{} servers, and the settings hold no id",
                servers.len()
            )),
            Ok(servers) => {
                if servers
                    .iter()
                    .any(|server| server.machine_identifier == settings.plex_id)
                {
                    Outcome::Passed
                } else {
                    failed(format!(
                        "none of the {} servers has the stored id",
                        servers.len()
                    ))
                }
            }
            Err(error) => failed(error),
        },
    ));
    lines.push(line(
        "plex.tv gives the list of the accounts that the server is shared with",
        // The owner is on no such list. The question is whether the list can be read.
        match plex.is_friend(token, &account.username, None).await {
            Ok(_) => Outcome::Passed,
            Err(error) => failed(error),
        },
    ));
    Some(account)
}

/// Logs the owner of the token in to each application, as at a login through Plex.
async fn ask_applications(
    settings: &Settings,
    http: &Outbound,
    account: &PlexAccount,
    lines: &mut Vec<Line>,
) {
    let token = settings.plex_token.as_str();
    let owner = Login {
        username: &account.username,
        email: &account.email,
        password: "",
        plex_token: Some(token),
        client_ip: "127.0.0.1",
        user_agent: None,
    };

    lines.push(line(
        "Ombi gives a session for the Plex token",
        match complete(&settings.ombi_url) {
            Ok(url) => match sso::ombi_token(http, url, owner, None, TIME_LIMIT).await {
                Some(_) => Outcome::Passed,
                None => no_session(),
            },
            Err(why) => why,
        },
    ));
    lines.push(line(
        "Overseerr gives a session for the Plex token",
        match complete(&settings.overseerr_url) {
            Ok(url) => match sso::overseerr_session(http, url, owner, None, TIME_LIMIT).await {
                Some(_) => Outcome::Passed,
                None => no_session(),
            },
            Err(why) => why,
        },
    ));

    // A user whom the application does not know gets the session of the fallback account.
    let stranger = Login {
        username: "",
        email: "",
        plex_token: None,
        ..owner
    };
    for (name, address, user, password) in [
        (
            "Ombi",
            &settings.ombi_url,
            &settings.ombi_fallback_user,
            &settings.ombi_fallback_password,
        ),
        (
            "Overseerr",
            &settings.overseerr_url,
            &settings.overseerr_fallback_user,
            &settings.overseerr_fallback_password,
        ),
    ] {
        if user.is_empty() || password.is_empty() {
            continue;
        }
        let fallback = Some(Fallback { user, password });
        let outcome = match complete(address) {
            Ok(url) => {
                let session = if name == "Ombi" {
                    sso::ombi_token(http, url, stranger, fallback, TIME_LIMIT).await
                } else {
                    sso::overseerr_session(http, url, stranger, fallback, TIME_LIMIT).await
                };
                session.map_or_else(no_session, |_| Outcome::Passed)
            }
            Err(why) => why,
        };
        lines.push(line(
            format!("{name} gives a session for the fallback user"),
            outcome,
        ));
    }
    ask_tautulli(settings, http, owner, lines).await;
}

/// The application gave no session. Its client wrote the reason to the log.
fn no_session() -> Outcome {
    failed("no session")
}

/// Asks each Tautulli install of the settings.
async fn ask_tautulli(
    settings: &Settings,
    http: &Outbound,
    owner: Login<'_>,
    lines: &mut Vec<Line>,
) {
    let installs: Vec<&str> = settings
        .tautulli_url
        .split(',')
        .map(str::trim)
        .filter(|address| !address.is_empty())
        .collect();
    if installs.is_empty() {
        lines.push(line(
            "Tautulli gives a session for the Plex token",
            skipped("the settings hold no address"),
        ));
    }
    for (number, address) in installs.iter().enumerate() {
        let question = format!(
            "Tautulli install {} of {} gives a session for the Plex token",
            number + 1,
            installs.len()
        );
        let outcome = match complete(address) {
            Ok(url) => {
                let install = [(url.to_owned(), "/".to_owned())];
                if sso::tautulli_sessions(http, &install, owner, TIME_LIMIT)
                    .await
                    .is_empty()
                {
                    no_session()
                } else {
                    Outcome::Passed
                }
            }
            Err(why) => why,
        };
        lines.push(line(question, outcome));
    }
}

#[cfg(test)]
mod tests {
    use organizr_integrations::http::Outbound;
    use organizr_testkit::installed_settings;
    use organizr_testkit::services::{self, LOCAL_ACCOUNT, OWNER_TOKEN, SERVER_ID};
    use serde_json::{Value, json};

    use super::{Outcome, ask};

    async fn outcomes(settings: &[(&str, Value)], plex: &str) -> Vec<(String, Outcome)> {
        let http = Outbound::new().expect("the client can be made");
        ask(&installed_settings(settings), &http, plex)
            .await
            .into_iter()
            .map(|line| (line.question, line.outcome))
            .collect()
    }

    fn is_failed(outcome: &Outcome) -> bool {
        matches!(outcome, Outcome::Failed(_))
    }

    fn is_skipped(outcome: &Outcome) -> bool {
        matches!(outcome, Outcome::Skipped(_))
    }

    #[tokio::test]
    async fn every_service_that_answers_as_expected_passes() {
        let plex = services::plex_tv().await;
        let (ombi, overseerr) = (services::ombi().await, services::overseerr().await);
        let one = services::tautulli("made-up-uuid-one", "").await;
        let two = services::tautulli("made-up-uuid-two", "/tautulli").await;
        let settings = [
            ("plexToken", json!(OWNER_TOKEN)),
            ("plexID", json!(SERVER_ID)),
            ("ombiURL", json!(ombi.url())),
            ("ombiFallbackUser", json!(LOCAL_ACCOUNT.0)),
            ("ombiFallbackPassword", json!(LOCAL_ACCOUNT.2)),
            ("overseerrURL", json!(overseerr.url())),
            ("overseerrFallbackUser", json!(LOCAL_ACCOUNT.1)),
            ("overseerrFallbackPassword", json!(LOCAL_ACCOUNT.2)),
            (
                "tautulliURL",
                json!(format!("{}, {}/tautulli", one.url(), two.url())),
            ),
        ];

        let lines = outcomes(&settings, &plex.url()).await;

        let questions: Vec<&str> = lines
            .iter()
            .map(|(question, _)| question.as_str())
            .collect();
        assert_eq!(
            questions,
            [
                "plex.tv knows the stored Plex token",
                "plex.tv lists the Plex servers, and the stored id is one of them",
                "plex.tv gives the list of the accounts that the server is shared with",
                "Ombi gives a session for the Plex token",
                "Overseerr gives a session for the Plex token",
                "Ombi gives a session for the fallback user",
                "Overseerr gives a session for the fallback user",
                "Tautulli install 1 of 2 gives a session for the Plex token",
                "Tautulli install 2 of 2 gives a session for the Plex token",
            ]
        );
        for (question, outcome) in &lines {
            assert_eq!(outcome, &Outcome::Passed, "{question}");
        }
    }

    #[tokio::test]
    async fn token_that_plex_refuses_ends_the_check() {
        let plex = services::plex_tv().await;

        let refused = outcomes(&[("plexToken", json!("another-token"))], &plex.url()).await;
        assert_eq!(refused.len(), 1);
        assert!(is_failed(&refused[0].1), "{refused:?}");

        let none = outcomes(&[], &plex.url()).await;
        assert_eq!(none.len(), 1);
        assert!(is_skipped(&none[0].1), "{none:?}");

        // Nothing listens at this address.
        let unreachable =
            outcomes(&[("plexToken", json!(OWNER_TOKEN))], "http://127.0.0.1:1").await;
        assert_eq!(unreachable.len(), 1);
        assert!(is_failed(&unreachable[0].1), "{unreachable:?}");
    }

    #[tokio::test]
    async fn service_with_a_wrong_answer_fails_and_one_without_an_address_is_skipped() {
        let plex = services::plex_tv().await;
        // Ombi is asked at the address of Overseerr, which does not know the path.
        let overseerr = services::overseerr().await;
        let settings = [
            ("plexToken", json!(OWNER_TOKEN)),
            ("plexID", json!("an id of no server")),
            ("ombiURL", json!(overseerr.url())),
            ("ombiFallbackUser", json!("nobody")),
            ("ombiFallbackPassword", json!("a wrong password")),
            ("overseerrURL", json!("/overseerr")),
            ("tautulliURL", json!(overseerr.url())),
        ];

        let lines = outcomes(&settings, &plex.url()).await;

        let of = |question: &str| {
            lines
                .iter()
                .find(|(asked, _)| asked == question)
                .map_or_else(
                    || panic!("`{question}` was not asked: {lines:?}"),
                    |(_, outcome)| outcome.clone(),
                )
        };
        assert_eq!(
            of("plex.tv lists the Plex servers, and the stored id is one of them"),
            Outcome::Failed("none of the 2 servers has the stored id".to_owned())
        );
        assert!(is_failed(&of("Ombi gives a session for the Plex token")));
        assert!(is_failed(&of("Ombi gives a session for the fallback user")));
        assert!(is_skipped(&of(
            "Overseerr gives a session for the Plex token"
        )));
        assert!(is_failed(&of(
            "Tautulli install 1 of 1 gives a session for the Plex token"
        )));
        // Overseerr has no fallback user, so that question is not on the list.
        assert_eq!(lines.len(), 7, "{lines:?}");
    }

    #[tokio::test]
    async fn lines_are_printed_with_their_outcome_first() {
        let plex = services::plex_tv().await;
        let settings = [("plexToken", json!(OWNER_TOKEN))];
        let http = Outbound::new().expect("the client can be made");

        let lines = ask(&installed_settings(&settings), &http, &plex.url()).await;

        let printed: Vec<String> = lines.iter().map(ToString::to_string).collect();
        assert_eq!(
            printed,
            [
                "ok       plex.tv knows the stored Plex token",
                "skipped  plex.tv lists the Plex servers, and the stored id is one of them: 2 servers, and the settings hold no id",
                "ok       plex.tv gives the list of the accounts that the server is shared with",
                "skipped  Ombi gives a session for the Plex token: the settings hold no address",
                "skipped  Overseerr gives a session for the Plex token: the settings hold no address",
                "skipped  Tautulli gives a session for the Plex token: the settings hold no address",
            ]
        );
        // The report names no token.
        assert!(!printed.join("\n").contains(OWNER_TOKEN));
    }
}
