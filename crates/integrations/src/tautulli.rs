//! Tautulli: the statistics of a Plex server, for the homepage.

use std::time::Duration;

use serde_json::Value;

use crate::http::Outbound;

/// How long Tautulli has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// Tautulli cannot be reached, or it did not answer in time. The text names no
/// address, because the address holds the API key.
#[derive(Debug, thiserror::Error)]
#[error("Tautulli cannot be reached: {0}")]
pub struct TautulliError(String);

/// An install of Tautulli, and how it is asked.
#[derive(Debug, Clone, Copy)]
pub struct Install<'a> {
    /// The address, complete.
    pub url: &'a str,
    /// The API key.
    pub key: &'a str,
    /// How long the install has to answer.
    pub timeout: Option<Duration>,
    /// Whether the certificate of the install is left unchecked.
    pub never_check_certificate: bool,
}

impl Install<'_> {
    fn api(&self) -> String {
        format!("{}/api/v2", self.url.trim_end_matches('/'))
    }

    /// The address of a picture of the Plex server in the given size, which Tautulli
    /// hands through. The address holds the API key, so it stays on the server.
    pub fn picture_address(
        &self,
        picture: &str,
        rating_key: &str,
        width: u32,
        height: u32,
    ) -> String {
        let field = |text: &str| {
            let mut out = String::new();
            for byte in text.bytes() {
                if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
                    out.push(char::from(byte));
                } else {
                    out.push('%');
                    for digit in [byte >> 4, byte & 0x0F] {
                        out.push(
                            char::from_digit(u32::from(digit), 16)
                                .map_or('0', |hex| hex.to_ascii_uppercase()),
                        );
                    }
                }
            }
            out
        };
        format!(
            "{}?apikey={}&cmd=pms_image_proxy&img={}&rating_key={}&width={width}&height={height}",
            self.api(),
            field(self.key),
            field(picture),
            field(rating_key),
        )
    }

    /// Runs a command of the API, with more fields of the query. Gives the `response`
    /// part of the answer, or nothing when Tautulli refused the request or sent
    /// something else.
    pub async fn command(
        &self,
        http: &Outbound,
        command: &str,
        more: &[(&str, &str)],
    ) -> Result<Option<Value>, TautulliError> {
        let url = self.api();
        let mut query = vec![("apikey", self.key), ("cmd", command)];
        query.extend_from_slice(more);
        let response = http
            .client(&url, self.never_check_certificate)
            .get(&url)
            .query(&query)
            .timeout(self.timeout.unwrap_or(TIME_LIMIT))
            .send()
            .await
            .map_err(|error| TautulliError(error.without_url().to_string()))?;
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), command, "Tautulli refused the request");
            return Ok(None);
        }
        Ok(response
            .json::<Value>()
            .await
            .ok()
            .and_then(|mut body| body.get_mut("response").map(Value::take)))
    }
}

#[cfg(test)]
mod tests {
    use super::Install;

    #[test]
    fn picture_address_holds_each_value_as_one_field() {
        let install = Install {
            url: "http://tautulli.example.com/",
            key: "key&1",
            timeout: None,
            never_check_certificate: false,
        };

        assert_eq!(
            install.picture_address("/library/metadata/7/art/1?a=b", "7", 1200, 675),
            "http://tautulli.example.com/api/v2?apikey=key%261&cmd=pms_image_proxy&img=%2Flibrary%2Fmetadata%2F7%2Fart%2F1%3Fa%3Db&rating_key=7&width=1200&height=675"
        );
    }
}
