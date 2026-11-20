//! PromPage: the services of a PromPage status page.

use std::time::Duration;

use serde_json::Value;

use crate::http::{Outbound, without_address};

/// How long PromPage has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// Why PromPage gave no services.
#[derive(Debug, thiserror::Error)]
pub enum PromPageError {
    /// PromPage cannot be reached, or it did not answer in time. The text names no
    /// address.
    #[error("PromPage cannot be reached: {0}")]
    Unreachable(String),
    /// PromPage answered with a status that is no success.
    #[error("PromPage answered with the status {0}")]
    Status(u16),
}

/// The services of `api/services`, as PromPage lists them: each one with its name,
/// whether it is up, and its uptime in percent. `null` when the answer names none.
pub async fn services(http: &Outbound, url: &str) -> Result<Value, PromPageError> {
    let url = format!("{}/api/services", url.trim_end_matches('/'));
    let response = http
        .client(&url, false)
        .get(&url)
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(|error| PromPageError::Unreachable(without_address(error)))?;
    if !response.status().is_success() {
        return Err(PromPageError::Status(response.status().as_u16()));
    }
    let answer: Value = response.json().await.unwrap_or(Value::Null);
    Ok(services_of(answer))
}

fn services_of(mut answer: Value) -> Value {
    answer.get_mut("services").map_or(Value::Null, Value::take)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::services_of;

    #[test]
    fn services_are_the_list_of_the_answer() {
        let answer = json!({"services": [{"name": "Blog", "status": true, "uptime": 99.95}]});
        assert_eq!(
            services_of(answer),
            json!([{"name": "Blog", "status": true, "uptime": 99.95}])
        );
        assert_eq!(services_of(json!({"other": 1})), Value::Null);
        assert_eq!(services_of(json!("no object")), Value::Null);
    }
}
