//! Pi-hole 6: the statistics of the last 24 hours of one or more installs, for the
//! homepage, and the login that the connection test checks.
//!
//! As in the PHP application 2.1.5000, Organizr logs in to the API of each install with
//! its password (the app password of Pi-hole), removes the older sessions of Organizr,
//! and asks for the summary and the blocked domains of the last 24 hours. The `api.php`
//! of Pi-hole 5 is not asked any more, as in PHP.

use std::time::Duration;

use reqwest::RequestBuilder;
use serde_json::{Map, Value, json};

use crate::http::{Outbound, without_address};
use crate::probe::host_of;

/// How long an install has to answer each request.
const TIME_LIMIT: Duration = Duration::from_secs(10);

/// The `User-Agent` of each request. PHP sends it, and removes only the sessions of
/// Pi-hole that have it.
const USER_AGENT: &str = "Organizr";

/// The 24 hours of the statistics, in seconds.
const DAY: i64 = 24 * 60 * 60;

/// How many blocked domains Pi-hole is asked for, as in PHP.
const DOMAIN_COUNT: u32 = 1000;

/// The installs of the settings: each address with its token. The addresses and the
/// tokens come with commas between them, and belong together by their place. An
/// address without a token at its place is left out, as in the PHP application: so
/// the first address always counts, and each further one needs a comma in the tokens.
pub fn installs<'a>(addresses: &'a str, tokens: &'a str) -> Vec<(&'a str, &'a str)> {
    addresses.split(',').zip(tokens.split(',')).collect()
}

/// The answer of Pi-hole to a request, or `None` when it refused the request. Fails
/// when the install cannot be reached; the text names no address.
async fn send(request: RequestBuilder) -> Result<Option<Value>, String> {
    let response = request
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "Pi-hole refused the request");
        return Ok(None);
    }
    Ok(Some(response.json().await.unwrap_or(Value::Null)))
}

/// Logs in to the install at `base` with its password, and removes the older sessions
/// of Organizr, as PHP does. The id of the new session, or `None` when Pi-hole refused
/// the password.
pub async fn log_in(http: &Outbound, base: &str, password: &str) -> Result<Option<String>, String> {
    let url = format!("{base}/api/auth");
    let request = http
        .client_for(&url)
        .post(&url)
        .json(&json!({ "password": password }));
    let answer = send(request).await?;
    let Some(sid) = answer
        .as_ref()
        .and_then(|answer| answer.pointer("/session/sid"))
        .and_then(Value::as_str)
        .filter(|sid| !sid.is_empty())
        .map(str::to_owned)
    else {
        return Ok(None);
    };
    remove_older_sessions(http, base, &sid).await?;
    Ok(Some(sid))
}

/// Removes every session of Organizr but the current one. Pi-hole keeps only a few
/// sessions, and Organizr logs in each time it loads the item.
async fn remove_older_sessions(http: &Outbound, base: &str, sid: &str) -> Result<(), String> {
    let url = format!("{base}/api/auth/sessions");
    let Some(answer) = send(http.client_for(&url).get(&url).header("sid", sid)).await? else {
        return Ok(());
    };
    let sessions = answer
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for session in sessions {
        let current = matches!(session.get("current_session"), Some(Value::Bool(true)))
            || session.get("current_session").and_then(Value::as_i64) == Some(1);
        if current || session.get("user_agent").and_then(Value::as_str) != Some(USER_AGENT) {
            continue;
        }
        let Some(id) = session.get("id").and_then(Value::as_i64) else {
            continue;
        };
        let url = format!("{base}/api/auth/session/{id}");
        send(http.client_for(&url).delete(&url).header("sid", sid)).await?;
    }
    Ok(())
}

/// The statistics of each install, by its address. The address is the completed address
/// of the settings, without a slash at the end, as PHP names it. An install that refused
/// a request has what it sent: an install that refused the login has only an empty list
/// of blocked domains, as in PHP.
///
/// `now` is the time in seconds since 1970. Fails when an install has no password, or
/// cannot be reached; the text names the host of an install without a password, and no
/// address otherwise.
pub async fn statistics(
    http: &Outbound,
    installs: &[(String, &str)],
    now: i64,
) -> Result<Map<String, Value>, String> {
    let mut all = Map::new();
    for (address, password) in installs {
        let base = address.trim_end_matches('/');
        if password.is_empty() {
            return Err(format!(
                "{}: Missing API Token",
                host_of(base).unwrap_or_default()
            ));
        }
        let Some(sid) = log_in(http, base, password).await? else {
            all.insert(base.to_owned(), json!({ "domains_being_blocked": [] }));
            continue;
        };
        let range = format!("from={}&until={now}", now - DAY);
        let url = format!("{base}/api/stats/database/summary?{range}");
        let summary = send(http.client_for(&url).get(&url).header("sid", &sid)).await?;
        let mut stats = match summary {
            Some(Value::Object(stats)) => stats,
            _ => Map::new(),
        };
        let url = format!(
            "{base}/api/stats/database/top_domains?{range}&blocked=true&count={DOMAIN_COUNT}"
        );
        let domains: Vec<Value> = send(http.client_for(&url).get(&url).header("sid", &sid))
            .await?
            .as_ref()
            .and_then(|answer| answer.get("domains"))
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |domains| {
                domains
                    .iter()
                    .filter_map(|domain| domain.get("domain").cloned())
                    .collect()
            });
        stats.insert("domains_being_blocked".to_owned(), Value::Array(domains));
        all.insert(base.to_owned(), Value::Object(stats));
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::{host_of, installs};

    #[test]
    fn address_and_token_belong_together_by_their_place() {
        assert_eq!(installs("http://a", ""), [("http://a", "")]);
        assert_eq!(
            installs("http://a,http://b", "one,two"),
            [("http://a", "one"), ("http://b", "two")]
        );
        // An address without a token at its place is left out.
        assert_eq!(installs("http://a,http://b", "one"), [("http://a", "one")]);
        assert_eq!(
            installs("http://a,http://b", ","),
            [("http://a", ""), ("http://b", "")]
        );
    }

    #[test]
    fn host_names_the_install() {
        assert_eq!(
            host_of("http://192.0.2.5:8080/admin").as_deref(),
            Some("192.0.2.5")
        );
        assert_eq!(
            host_of("https://pihole.example.com/admin").as_deref(),
            Some("pihole.example.com")
        );
        assert_eq!(host_of("no address"), None);
    }
}
