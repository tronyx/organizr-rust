//! Pi-hole: the summary of one or more installs, for the homepage.
//!
//! The summary comes from `api.php?summaryRaw` of Pi-hole 5. Pi-hole 6 has another
//! API, which the PHP application does not know either.

use std::time::Duration;

use serde_json::{Map, Value};

use crate::http::{Outbound, without_address};
use crate::probe::host_of;

/// How long an install has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(10);

/// The installs of the settings: each address with its token. The addresses and the
/// tokens come with commas between them, and belong together by their place. An
/// address without a token at its place is left out, as in the PHP application: so
/// the first address always counts, and each further one needs a comma in the tokens.
pub fn installs<'a>(addresses: &'a str, tokens: &'a str) -> Vec<(&'a str, &'a str)> {
    addresses.split(',').zip(tokens.split(',')).collect()
}

/// The summary of each install, by its host. An install that refuses the request, or
/// answers with something that is no list and no object, is left out.
///
/// Fails when an install cannot be reached. The text names no address, because the
/// address holds the token.
pub async fn summaries(
    http: &Outbound,
    installs: &[(String, &str)],
) -> Result<Map<String, Value>, String> {
    let mut all = Map::new();
    for (address, token) in installs {
        let url = format!("{}/api.php", address.trim_end_matches('/'));
        let mut request = http
            .client_for(&url)
            .get(format!("{url}?summaryRaw"))
            .timeout(TIME_LIMIT);
        if !token.is_empty() {
            request = request.query(&[("auth", token)]);
        }
        let response = request.send().await.map_err(without_address)?;
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "Pi-hole refused the request");
            continue;
        }
        let Some(host) = host_of(address) else {
            continue;
        };
        if let Ok(summary @ (Value::Object(_) | Value::Array(_))) = response.json().await {
            all.insert(host, summary);
        } else {
            tracing::warn!("the answer of Pi-hole cannot be read");
        }
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
