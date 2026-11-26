//! OctoPrint: the print job of a printer, and the webcam of its settings.

use std::time::Duration;

use serde_json::{Value, json};

use crate::http::{Outbound, without_address};

/// How long OctoPrint has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(10);

/// An install of OctoPrint, and how it is asked.
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
    /// The JSON of `api/job` or `api/settings`, or nothing when OctoPrint refuses the
    /// request. An answer that is no JSON is `null`.
    ///
    /// Fails when OctoPrint cannot be reached. The text of the error names no address.
    pub async fn read(&self, http: &Outbound, endpoint: &str) -> Result<Option<Value>, String> {
        let url = format!("{}/api/{endpoint}", self.url.trim_end_matches('/'));
        let response = http
            .keyed_client(&url, self.never_check_certificate)
            .get(&url)
            .header("X-Api-Key", self.key)
            .timeout(self.timeout.unwrap_or(TIME_LIMIT))
            .send()
            .await
            .map_err(without_address)?;
        if !response.status().is_success() {
            return Ok(None);
        }
        Ok(Some(response.json().await.unwrap_or(Value::Null)))
    }
}

/// The part of the settings of OctoPrint that the homepage shows: whether the webcam is
/// on, and the address of its stream. The settings hold API keys and passwords, which
/// stay on the server.
pub fn webcam_of(settings: &Value) -> Value {
    let webcam = settings.get("webcam");
    let field = |name: &str| {
        webcam
            .and_then(|webcam| webcam.get(name))
            .cloned()
            .unwrap_or(Value::Null)
    };
    json!({
        "webcam": {
            "webcamEnabled": field("webcamEnabled"),
            "streamUrl": field("streamUrl"),
        },
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::webcam_of;

    #[test]
    fn only_the_webcam_of_the_settings_is_passed_on() {
        let settings = json!({
            "api": {"key": "made-up-key-of-octoprint"},
            "webcam": {
                "webcamEnabled": true,
                "streamUrl": "/webcam/?action=stream",
                "snapshotUrl": "http://127.0.0.1:8080/?action=snapshot",
            },
        });
        assert_eq!(
            webcam_of(&settings),
            json!({"webcam": {"webcamEnabled": true, "streamUrl": "/webcam/?action=stream"}})
        );
        assert_eq!(
            webcam_of(&json!(null)),
            json!({"webcam": {"webcamEnabled": null, "streamUrl": null}})
        );
    }
}
