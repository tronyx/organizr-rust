//! The health checks plugin: asks the addresses of a list of services, and tells
//! Healthchecks for each service whether it answered.
//!
//! Each service has a check at Healthchecks, with an id of its own. A run pings the
//! address `<ping address>/<id>/start`, asks the service, and then pings
//! `<ping address>/<id>` for a service that answered, or `.../<id>/fail`.

use std::time::Duration;

use organizr_integrations::http::Outbound;
use organizr_settings::Settings;
use serde_json::{Map, Value, json};

/// How long a service has to answer.
const SERVICE_TIME_LIMIT: Duration = Duration::from_secs(60);

/// How long Healthchecks has to take a ping.
const PING_TIME_LIMIT: Duration = Duration::from_secs(30);

/// One service of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Service {
    /// The key of the service in the setting.
    pub key: String,
    /// The fields of the service as the setting holds them: `Service Name`, `UUID`,
    /// `External URL`, `Internal URL`, `Enabled`, and `Send API Key`, which PHP does not
    /// have.
    pub fields: Map<String, Value>,
}

/// Whether PHP takes the value for true in a loose comparison.
fn is_on(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(on)) => *on,
        Some(Value::String(text)) => !text.is_empty() && text != "0" && text != "false",
        Some(Value::Number(number)) => number.as_f64() != Some(0.0),
        _ => false,
    }
}

impl Service {
    fn text(&self, field: &str) -> &str {
        self.fields
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    /// The id of the check at Healthchecks.
    pub fn id(&self) -> &str {
        self.text("UUID")
    }

    /// The address inside the network.
    pub fn internal(&self) -> &str {
        self.text("Internal URL")
    }

    /// The address from the internet.
    pub fn external(&self) -> &str {
        self.text("External URL")
    }

    /// Whether a request to the service carries the API key of Organizr. A service
    /// without the switch, as every service of PHP Organizr, gets the key, as in PHP.
    /// A new service has the switch, off (D-130).
    pub fn sends_key(&self) -> bool {
        self.fields
            .get(SEND_KEY)
            .is_none_or(|value| is_on(Some(value)))
    }
}

/// The label of the switch that sends the API key of Organizr to a service.
pub const SEND_KEY: &str = "Send API Key";

/// The services of the `HEALTHCHECKS-all-items` setting that are switched on and have
/// an id. The setting holds, for each service, a list of fields with a `label` and a
/// `value`.
pub fn services(setting: &Value) -> Vec<Service> {
    let entries: Vec<(String, &Value)> = match setting {
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| (key.clone(), value))
            .collect(),
        Value::Array(list) => list
            .iter()
            .enumerate()
            .map(|(place, value)| (place.to_string(), value))
            .collect(),
        _ => Vec::new(),
    };
    entries
        .into_iter()
        .filter_map(|(key, fields)| {
            // PHP stores a list as a map with the places as keys. The import can
            // bring either form.
            let listed: Vec<&Value> = match fields {
                Value::Array(list) => list.iter().collect(),
                Value::Object(map) => map.values().collect(),
                _ => return None,
            };
            let fields: Map<String, Value> = listed
                .into_iter()
                .filter_map(|field| {
                    let label = field.get("label")?.as_str()?;
                    Some((
                        label.to_owned(),
                        field.get("value").cloned().unwrap_or(Value::Null),
                    ))
                })
                .collect();
            Some(Service { key, fields })
        })
        .filter(|service| is_on(service.fields.get("Enabled")) && !service.id().is_empty())
        .collect()
}

/// The address that pings go to. For an install of one's own, the address ends in
/// `/ping`. The service of healthchecks.io has a host of its own for pings.
pub fn ping_address(setting: &str) -> String {
    let address = setting.trim().trim_end_matches('/');
    let lower = address.to_lowercase();
    if lower.contains("hc-ping.com") || lower.contains("/ping") {
        address.to_owned()
    } else {
        format!("{address}/ping")
    }
}

/// Which answers of a service count as "it works".
#[derive(Debug, Clone, Copy)]
pub struct Rules {
    /// Whether `401 Unauthorized` counts.
    pub unauthorized_passes: bool,
    /// Whether `403 Forbidden` counts.
    pub forbidden_passes: bool,
}

impl Rules {
    /// Whether a status counts as "it works": a success always, and `401` and `403`
    /// when the settings say so.
    pub const fn passes(self, status: u16) -> bool {
        (status >= 200 && status < 300)
            || (status == 401 && self.unauthorized_passes)
            || (status == 403 && self.forbidden_passes)
    }
}

fn is_web_address(address: &str) -> bool {
    address.starts_with("http://") || address.starts_with("https://")
}

/// Asks one address of a service. With a key, the request carries the API key of
/// Organizr in the header `Token`, so a service behind the forward auth of Organizr lets
/// it in. The certificate of the service is not checked, as in the PHP application.
async fn answers(http: &Outbound, address: &str, api_key: Option<&str>, rules: Rules) -> bool {
    let address = address.trim();
    if !is_web_address(address) {
        tracing::warn!("an address of a health check has no http:// or https://");
        return false;
    }
    let mut request = http
        .client(address, true)
        .get(address)
        .timeout(SERVICE_TIME_LIMIT);
    if let Some(key) = api_key {
        request = request.header("Token", key);
    }
    match request.send().await {
        Ok(response) => rules.passes(response.status().as_u16()),
        Err(error) => {
            tracing::warn!(error = %error.without_url(), "a service of a health check did not answer");
            false
        }
    }
}

async fn ping(http: &Outbound, address: &str) {
    let request = http
        .client_for(address)
        .get(address)
        .timeout(PING_TIME_LIMIT);
    if let Err(error) = request.send().await {
        tracing::warn!(error = %error.without_url(), "a ping to Healthchecks failed");
    }
}

/// Runs the checks of every service, and returns the services with their results, as
/// the PHP application does: the fields of each service, and `results` with
/// `internal` and `external`.
///
/// A service with both addresses is asked at the address from the internet only when
/// the one inside the network did not answer.
pub async fn run(http: &Outbound, settings: &Settings) -> Value {
    let base = ping_address(&settings.health_checks_ping_url);
    let rules = Rules {
        unauthorized_passes: settings.health_checks_401_enabled,
        forbidden_passes: settings.health_checks_403_enabled,
    };
    let mut all = Map::new();
    for service in services(&settings.health_checks_all_items) {
        let (inside, outside) = (
            !service.internal().is_empty(),
            !service.external().is_empty(),
        );
        let key = service.sends_key().then_some(settings.api_key.as_str());
        let mut results = Map::new();
        let mut passed = false;
        if inside || outside {
            ping(http, &format!("{base}/{}/start", service.id())).await;
        }
        if inside {
            passed = answers(http, service.internal(), key, rules).await;
            results.insert(
                "internal".to_owned(),
                json!(if passed { "Success" } else { "Error" }),
            );
        }
        if outside {
            let result = if inside && passed {
                "Not needed"
            } else {
                passed = answers(http, service.external(), key, rules).await;
                if passed { "Success" } else { "Error" }
            };
            results.insert("external".to_owned(), json!(result));
        }
        let end = if passed { "" } else { "/fail" };
        ping(http, &format!("{base}/{}{end}", service.id())).await;

        let mut fields = service.fields;
        if !results.is_empty() {
            fields.insert("results".to_owned(), Value::Object(results));
        }
        all.insert(service.key, Value::Object(fields));
    }
    Value::Object(all)
}

/// Whether the plugin can run at all: it is switched on, it has services, and it has
/// an address for the pings.
pub fn is_ready(settings: &Settings) -> bool {
    let has_items = match &settings.health_checks_all_items {
        Value::Object(map) => !map.is_empty(),
        Value::Array(list) => !list.is_empty(),
        _ => false,
    };
    settings.health_checks_enabled && has_items && !settings.health_checks_ping_url.is_empty()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Rules, ping_address, services};

    fn item(name: &str, id: &str, enabled: &serde_json::Value) -> serde_json::Value {
        json!([
            {"type": "input", "label": "Service Name", "value": name},
            {"type": "input", "label": "UUID", "value": id},
            {"type": "input", "label": "External URL", "value": "https://plex.example.com"},
            {"type": "input", "label": "Internal URL", "value": ""},
            {"type": "switch", "label": "Enabled", "value": enabled},
        ])
    }

    #[test]
    fn services_are_the_ones_that_are_switched_on_and_have_an_id() {
        let setting = json!({
            "0": item("Plex", "made-up-id-one", &json!(true)),
            "1": item("Off", "made-up-id-two", &json!(false)),
            "2": item("No id", "", &json!(true)),
            "3": item("Text switch", "made-up-id-four", &json!("true")),
            "4": "not a list",
        });

        let found = services(&setting);

        let keys: Vec<&str> = found.iter().map(|service| service.key.as_str()).collect();
        assert_eq!(keys, ["0", "3"]);
        assert_eq!(found[0].id(), "made-up-id-one");
        assert_eq!(found[0].external(), "https://plex.example.com");
        assert_eq!(found[0].internal(), "");
        assert_eq!(found[0].fields["Service Name"], "Plex");

        // The fields of a service can be a map with the places as keys.
        let mapped = services(&json!({"0": {
            "0": {"label": "UUID", "value": "made-up-id-five"},
            "1": {"label": "Enabled", "value": true},
        }}));
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].id(), "made-up-id-five");

        // A list is read as an object whose keys are the places.
        let listed = services(&json!([item("Plex", "made-up-id-one", &json!(true))]));
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].key, "0");
        assert_eq!(services(&json!("")), []);
    }

    #[test]
    fn ping_address_of_an_own_install_ends_in_ping() {
        assert_eq!(ping_address("https://hc-ping.com/"), "https://hc-ping.com");
        assert_eq!(
            ping_address("https://checks.example.com"),
            "https://checks.example.com/ping"
        );
        assert_eq!(
            ping_address("https://checks.example.com/ping/"),
            "https://checks.example.com/ping"
        );
    }

    #[test]
    fn refusals_pass_only_when_the_settings_say_so() {
        let strict = Rules {
            unauthorized_passes: false,
            forbidden_passes: false,
        };
        let loose = Rules {
            unauthorized_passes: true,
            forbidden_passes: true,
        };

        for status in [200, 204, 299] {
            assert!(strict.passes(status), "{status}");
        }
        for status in [301, 401, 403, 404, 500, 199] {
            assert!(!strict.passes(status), "{status}");
        }
        assert!(loose.passes(401));
        assert!(loose.passes(403));
        assert!(!loose.passes(404));
    }
}
