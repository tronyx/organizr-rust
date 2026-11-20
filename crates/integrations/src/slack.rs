//! A message to a webhook of Slack or of Discord, for the test of the log settings.

use std::time::Duration;

use serde_json::{Value, json};

use crate::http::{Outbound, without_address};

/// How long the service has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(15);

/// Whether a webhook wants the form of Discord: an address of Discord that does not
/// end with `/slack`. Discord takes the form of Slack at an address with that ending.
fn is_discord(webhook: &str) -> bool {
    let address = webhook.split(['?', '#']).next().unwrap_or_default();
    webhook.to_lowercase().contains("discord") && !address.trim_end_matches('/').ends_with("/slack")
}

/// The body of the test message.
fn message(webhook: &str, channel: &str, user: &str) -> Value {
    let text = format!("Warning Test: the test of the log settings of Organizr, started by {user}");
    if is_discord(webhook) {
        return json!({ "username": "Organizr", "content": text });
    }
    let mut body = json!({
        "username": "Organizr",
        "text": text,
        "attachments": [{
            "color": "warning",
            "fields": [
                {"title": "Level", "value": "WARNING", "short": true},
                {"title": "test", "value": "success", "short": true},
            ],
        }],
    });
    if let (false, Some(fields)) = (channel.is_empty(), body.as_object_mut()) {
        fields.insert("channel".to_owned(), json!(channel));
    }
    body
}

/// Sends the test message. Fails when the service cannot be reached or refuses the
/// message. The text of the error names no address, because the address of a webhook
/// is a secret.
pub async fn send_test(
    http: &Outbound,
    webhook: &str,
    channel: &str,
    user: &str,
) -> Result<(), String> {
    let response = http
        .client(webhook, false)
        .post(webhook)
        .json(&message(webhook, channel, user))
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "The webhook answered with the status {}",
            response.status().as_u16()
        ))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{is_discord, message};

    #[test]
    fn message_has_the_form_of_the_service() {
        let slack = message("https://hooks.slack.com/services/T/B/x", "#logs", "admin");
        assert_eq!(slack["channel"], "#logs");
        assert!(
            slack["text"]
                .as_str()
                .is_some_and(|text| text.contains("admin"))
        );
        assert_eq!(slack["attachments"][0]["fields"][1]["value"], "success");
        assert_eq!(slack["content"], Value::Null);

        let discord = message("https://discord.com/api/webhooks/1/x", "", "admin");
        assert!(
            discord["content"]
                .as_str()
                .is_some_and(|text| text.starts_with("Warning Test"))
        );
        assert_eq!(discord["attachments"], Value::Null);

        // Discord takes the form of Slack at an address that ends with `/slack`.
        let compatible = message("https://discord.com/api/webhooks/1/x/slack", "", "admin");
        assert_eq!(compatible["channel"], Value::Null);
        assert!(compatible["text"].is_string());
    }

    #[test]
    fn address_of_discord_without_the_ending_of_slack_is_discord() {
        assert!(is_discord("https://discord.com/api/webhooks/1/x"));
        assert!(is_discord(
            "https://discordapp.com/api/webhooks/1/x?wait=true"
        ));
        assert!(!is_discord("https://discord.com/api/webhooks/1/x/slack"));
        assert!(!is_discord(
            "https://discord.com/api/webhooks/1/x/slack/?wait=true"
        ));
        assert!(!is_discord("https://hooks.slack.com/services/T/B/x"));
    }
}
