//! Jackett and Prowlarr: the search of the indexers, and the hand-over of a release
//! that was found.
//!
//! Jackett takes its API key only in the address of a request. Prowlarr takes it in a
//! header. Both services put their key into the download links of their answers, so
//! [`without_key`] cuts it out before an answer goes to a browser.

use std::time::Duration;

use reqwest::Url;
use serde_json::{Value, json};

use crate::http::{Outbound, without_address};

/// How long a service has to answer, as in the PHP application.
const TIME_LIMIT: Duration = Duration::from_secs(120);

/// The field of the address of a black hole link that holds the key of Jackett.
const JACKETT_KEY_FIELD: &str = "jackett_apikey";

/// Which of the two services.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Jackett.
    Jackett,
    /// Prowlarr.
    Prowlarr,
}

impl Kind {
    /// The name of the service.
    pub fn name(self) -> &'static str {
        match self {
            Self::Jackett => "Jackett",
            Self::Prowlarr => "Prowlarr",
        }
    }
}

/// An install of one of the two services.
#[derive(Debug, Clone, Copy)]
pub struct Install<'a> {
    /// The address, complete.
    pub url: &'a str,
    /// The API key.
    pub key: &'a str,
    /// Whether the certificate of the install is left unchecked.
    pub never_check_certificate: bool,
}

/// What a service answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// Whether the status is a success.
    pub is_success: bool,
    /// The body as text.
    pub body: String,
}

impl Reply {
    /// The body as JSON without the key of the install, or nothing when the body is no
    /// JSON list and no JSON object.
    pub fn json(&self, key: &str) -> Option<Value> {
        match serde_json::from_str(&self.body) {
            Ok(value @ (Value::Object(_) | Value::Array(_))) => Some(without_key(value, key)),
            _ => None,
        }
    }
}

async fn reply(request: reqwest::RequestBuilder) -> Result<Reply, String> {
    let response = request
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
    let is_success = response.status().is_success();
    let body = response.text().await.map_err(without_address)?;
    Ok(Reply { is_success, body })
}

impl Install<'_> {
    fn base(&self) -> &str {
        self.url.trim_end_matches('/')
    }

    /// Searches every indexer for a text.
    ///
    /// Fails when the service cannot be reached. The text of the error names no
    /// address, because the address of Jackett holds the key.
    pub async fn search(&self, http: &Outbound, kind: Kind, query: &str) -> Result<Reply, String> {
        let request = match kind {
            Kind::Jackett => {
                let url = format!("{}/api/v2.0/indexers/all/results", self.base());
                http.client(&url, self.never_check_certificate)
                    .get(&url)
                    .query(&[("apikey", self.key), ("Query", query)])
            }
            Kind::Prowlarr => {
                let url = format!("{}/api/v1/search", self.base());
                http.client(&url, self.never_check_certificate)
                    .get(&url)
                    .header("X-Api-Key", self.key)
                    .query(&[("query", query)])
            }
        };
        reply(request).await
    }

    /// Asks Jackett to put the torrent file of a release into its black hole folder.
    /// `address` is the address that [`black_hole_address`] made.
    pub async fn black_hole(&self, http: &Outbound, address: &str) -> Result<Reply, String> {
        reply(
            http.client(address, self.never_check_certificate)
                .get(address),
        )
        .await
    }

    /// Asks Prowlarr to send a release of its last search to its download client.
    pub async fn grab(&self, http: &Outbound, guid: &str, indexer: i64) -> Result<Reply, String> {
        let url = format!("{}/api/v1/search", self.base());
        let request = http
            .client(&url, self.never_check_certificate)
            .post(&url)
            .header("X-Api-Key", self.key)
            .json(&json!({ "guid": guid, "indexerId": indexer }));
        reply(request).await
    }
}

/// The address of a black hole link of Jackett on the install at `base`, with the key
/// of the install. `link` is the part of the link from `/bh/` on, as a search gave it.
///
/// Nothing when the link is no black hole link, or when it leads away from the
/// install.
pub fn black_hole_address(base: &str, link: &str, key: &str) -> Option<String> {
    if !link.starts_with("/bh/") {
        return None;
    }
    let base = Url::parse(base.trim_end_matches('/')).ok()?;
    let mut address = Url::parse(&format!("{}{link}", base.as_str().trim_end_matches('/'))).ok()?;
    let folder = format!("{}/bh/", base.path().trim_end_matches('/'));
    // The address was read again, so `..` in the link is worked out by now.
    if address.origin() != base.origin() || !address.path().starts_with(&folder) {
        return None;
    }
    let fields: Vec<(String, String)> = address
        .query_pairs()
        .filter(|(name, _)| name != JACKETT_KEY_FIELD)
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();
    address
        .query_pairs_mut()
        .clear()
        .append_pair(JACKETT_KEY_FIELD, key)
        .extend_pairs(fields);
    address.set_fragment(None);
    Some(address.into())
}

/// A text without the key.
pub fn text_without_key(text: &str, key: &str) -> String {
    if key.is_empty() {
        text.to_owned()
    } else {
        text.replace(key, "")
    }
}

/// An answer of a service without the key of the install: the key is cut out of every
/// text, for example out of the download links.
pub fn without_key(mut value: Value, key: &str) -> Value {
    fn cut(value: &mut Value, key: &str) {
        match value {
            Value::String(text) if text.contains(key) => *text = text.replace(key, ""),
            Value::Array(list) => list.iter_mut().for_each(|entry| cut(entry, key)),
            Value::Object(map) => map.values_mut().for_each(|entry| cut(entry, key)),
            _ => {}
        }
    }
    if !key.is_empty() {
        cut(&mut value, key);
    }
    value
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Reply, black_hole_address, without_key};

    const KEY: &str = "made-up-key";

    #[test]
    fn black_hole_address_gets_the_key_of_the_install() {
        assert_eq!(
            black_hole_address(
                "http://jackett.example.com:9117/",
                "/bh/tracker/?jackett_apikey=&path=abc&file=A+film",
                KEY
            )
            .as_deref(),
            Some(
                "http://jackett.example.com:9117/bh/tracker/?jackett_apikey=made-up-key&path=abc&file=A+film"
            )
        );
        // A key that the caller sent is not used.
        assert_eq!(
            black_hole_address(
                "http://jackett.example.com/jackett",
                "/bh/tracker/?jackett_apikey=other&path=abc",
                KEY
            )
            .as_deref(),
            Some(
                "http://jackett.example.com/jackett/bh/tracker/?jackett_apikey=made-up-key&path=abc"
            )
        );
    }

    #[test]
    fn link_that_is_no_black_hole_link_has_no_address() {
        let base = "http://jackett.example.com:9117";
        for link in [
            "",
            "bh/tracker/",
            "/api/v2.0/server/config",
            "@other.example.com/bh/",
            ".other.example.com/bh/",
            "/bh/../api/v2.0/server/config",
            "/bh/%2e%2e/api/v2.0/server/config",
            "/dl/tracker/?path=abc",
        ] {
            assert_eq!(black_hole_address(base, link, KEY), None, "{link}");
        }
        assert_eq!(black_hole_address("no address", "/bh/tracker/", KEY), None);
    }

    #[test]
    fn key_is_cut_out_of_every_text_of_an_answer() {
        let answer = json!({
            "Results": [{
                "Title": "A film",
                "Size": 5,
                "Link": "http://jackett/dl/tracker/?jackett_apikey=made-up-key&path=abc",
                "BlackholeLink": "http://jackett/bh/tracker/?jackett_apikey=made-up-key&path=abc",
                "MagnetUri": null,
            }],
        });
        let cut = without_key(answer.clone(), KEY);

        assert!(!cut.to_string().contains(KEY));
        assert_eq!(
            cut["Results"][0]["BlackholeLink"],
            json!("http://jackett/bh/tracker/?jackett_apikey=&path=abc")
        );
        assert_eq!(cut["Results"][0]["Title"], json!("A film"));
        // Without a key, nothing is cut.
        assert_eq!(without_key(answer.clone(), ""), answer);
    }

    #[test]
    fn body_that_is_no_list_and_no_object_is_no_json() {
        let reply = |body: &str| Reply {
            is_success: true,
            body: body.to_owned(),
        };
        assert_eq!(reply("[]").json(KEY), Some(json!([])));
        assert_eq!(reply("<html>").json(KEY), None);
        assert_eq!(reply("5").json(KEY), None);
        assert_eq!(reply("").json(KEY), None);
    }
}
