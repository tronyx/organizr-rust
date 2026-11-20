//! Netdata: one number for each chart of the homepage item, read from the data API
//! of Netdata.

use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::http::{Outbound, without_address};

/// How long Netdata has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(10);

/// What a chart shows: the values of the setting `netdata<N>Data`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// What the disks read.
    DiskRead,
    /// What the disks write.
    DiskWrite,
    /// The load of the processors.
    Cpu,
    /// What the network receives.
    NetIn,
    /// What the network sends.
    NetOut,
    /// The memory in use.
    RamUsed,
    /// The swap space in use.
    SwapUsed,
    /// The disk space that is free.
    DiskAvail,
    /// The disk space in use.
    DiskUsed,
    /// A request that the Admin wrote, see [`custom_definition`].
    Custom,
}

impl Source {
    /// The source of a value of the setting, or nothing for a value that names none.
    pub fn of(setting: &str) -> Option<Self> {
        Some(match setting {
            "disk-read" => Self::DiskRead,
            "disk-write" => Self::DiskWrite,
            "cpu" => Self::Cpu,
            "net-in" => Self::NetIn,
            "net-out" => Self::NetOut,
            "ram-used" => Self::RamUsed,
            "swap-used" => Self::SwapUsed,
            "disk-avail" => Self::DiskAvail,
            "disk-used" => Self::DiskUsed,
            "custom" => Self::Custom,
            _ => return None,
        })
    }
}

/// An install of Netdata, and how it is asked.
#[derive(Debug, Clone, Copy)]
pub struct Install<'a> {
    /// The address, complete.
    pub url: &'a str,
    /// How long the install has to answer.
    pub timeout: Option<Duration>,
    /// Whether the certificate of the install is left unchecked.
    pub never_check_certificate: bool,
}

/// A number as PHP sends it: a whole number without a fraction.
fn number(value: f64) -> Value {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the number is whole and within the range where every whole number is exact"
    )]
    if value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0 {
        json!(value as i64)
    } else {
        json!(value)
    }
}

/// The number of a JSON value, as PHP takes one for a calculation.
fn as_number(value: &Value) -> f64 {
    match value {
        Value::Number(number) => number.as_f64().unwrap_or(0.0),
        Value::String(text) => text.trim().parse().unwrap_or(0.0),
        Value::Bool(true) => 1.0,
        _ => 0.0,
    }
}

fn percent_of(value: f64, max: f64) -> f64 {
    if max == 0.0 { 0.0 } else { value / max * 100.0 }
}

/// The data of a chart of a rate, from an answer with `latest_values` and `max`.
fn rate(answer: &Value, units: &str) -> Map<String, Value> {
    let latest = answer
        .get("latest_values")
        .and_then(|values| values.get(0))
        .map_or(0.0, as_number);
    let max = answer.get("max").cloned().unwrap_or(Value::Null);
    let mut data = Map::new();
    data.insert("value".to_owned(), number(latest / 1000.0));
    data.insert(
        "percent".to_owned(),
        number(percent_of(latest, as_number(&max))),
    );
    data.insert("units".to_owned(), json!(units));
    data.insert("max".to_owned(), max);
    data
}

/// The data of a chart whose value is a part of 100.
fn part(value: Option<&Value>) -> Map<String, Value> {
    let value = value.cloned().unwrap_or(Value::Null);
    let mut data = Map::new();
    data.insert("value".to_owned(), value.clone());
    data.insert("percent".to_owned(), value);
    data.insert("max".to_owned(), json!(100));
    data.insert("units".to_owned(), json!("%"));
    data
}

/// Makes `value` and `percent` positive. A chart without them gets 0 for both.
fn positive(mut data: Map<String, Value>) -> Map<String, Value> {
    for field in ["value", "percent"] {
        let size = data.get(field).map_or(0.0, as_number).abs();
        data.insert(field.to_owned(), number(size));
    }
    data
}

/// A request of the Admin for a chart: one entry of the setting `netdataCustom`.
#[derive(Debug, Clone, PartialEq)]
pub struct Custom {
    /// The path of the request, which is put after the address and a slash.
    pub url: String,
    /// The way to the number in the answer: keys and places, with commas between them.
    pub value: String,
    /// The unit that the chart shows.
    pub units: Value,
    /// The highest value.
    pub max: Value,
    /// Calculations on the number, for example `/1000,*8`.
    pub mutator: Option<String>,
    /// The address of another install to ask, when the entry names one.
    pub netdata: Option<String>,
}

/// Whether PHP takes a value of an entry for "not given".
fn is_not_given(value: Option<&Value>) -> bool {
    matches!(value, None | Some(Value::Null)) || value.and_then(Value::as_str) == Some("")
}

/// The entry of the setting `netdataCustom` for the chart with a number. The error is
/// the text that the PHP application sends for it.
pub fn custom_definition(setting: &str, chart: usize) -> Result<Custom, &'static str> {
    let all = match serde_json::from_str::<Value>(setting) {
        Ok(Value::Object(map)) if !map.is_empty() => Value::Object(map),
        Ok(Value::Array(list)) if !list.is_empty() => Value::Array(list),
        _ => return Err("unable to parse custom JSON"),
    };
    let entry = match &all {
        Value::Array(list) => list.get(chart),
        other => other.get(chart.to_string()),
    };
    let Some(entry) = entry.filter(|entry| !entry.is_null()) else {
        return Err("custom definition not found");
    };
    let text = |field: &str| entry.get(field).and_then(Value::as_str).map(str::to_owned);
    let (Some(url), Some(value)) = (text("url"), text("value")) else {
        return Err("custom definition incomplete");
    };
    let or = |field: &str, default: Value| {
        if is_not_given(entry.get(field)) {
            default
        } else {
            entry.get(field).cloned().unwrap_or(default)
        }
    };
    Ok(Custom {
        url,
        value,
        units: or("units", json!("%")),
        max: or("max", json!(100)),
        mutator: text("mutator"),
        netdata: text("netdata").filter(|address| !address.is_empty()),
    })
}

/// The value at the end of a way through an answer, or nothing when the way ends
/// before.
fn selected<'a>(answer: &'a Value, way: &str) -> Option<&'a Value> {
    way.split(',').try_fold(answer, |at, step| match at {
        Value::Array(list) => list.get(step.trim().parse::<usize>().ok()?),
        Value::Object(map) => map.get(step),
        _ => None,
    })
}

/// The number at the start of a text, as PHP reads one, or 0.
fn leading_number(text: &str) -> f64 {
    let text = text.trim_start();
    (1..=text.len())
        .rev()
        .filter(|end| text.is_char_boundary(*end))
        .find_map(|end| text.get(..end)?.parse::<f64>().ok())
        .filter(|number| number.is_finite())
        .unwrap_or(0.0)
}

/// A number after the calculations of a mutator. A division by 0 is left out.
fn mutated(mut value: f64, mutators: &str) -> f64 {
    for mutator in mutators.split(',') {
        let mut letters = mutator.chars();
        let Some(operation) = letters.next() else {
            continue;
        };
        let by = leading_number(letters.as_str());
        match operation {
            '+' => value += by,
            '-' => value -= by,
            '*' => value *= by,
            '/' if by != 0.0 => value /= by,
            _ => {}
        }
    }
    value
}

/// The data of a custom chart, from the answer to its request.
fn custom_data(answer: &Value, custom: &Custom) -> Map<String, Value> {
    let mut data = Map::new();
    data.insert("max".to_owned(), custom.max.clone());
    data.insert("units".to_owned(), custom.units.clone());
    let found = selected(answer, &custom.value);
    let is_number = found.is_some_and(|value| match value {
        Value::Number(_) => true,
        Value::String(text) => text.trim().parse::<f64>().is_ok(),
        _ => false,
    });
    let max = as_number(&custom.max);
    if let (Some(found), true) = (found, is_number) {
        let value = match &custom.mutator {
            Some(mutators) => mutated(as_number(found), mutators),
            None => as_number(found),
        };
        data.insert("value".to_owned(), number(value));
        data.insert("percent".to_owned(), number(percent_of(value, max)));
    } else {
        // Something that is no number cannot be calculated with.
        data.insert("value".to_owned(), Value::Null);
        data.insert("percent".to_owned(), json!(0));
    }
    data
}

impl Install<'_> {
    /// The JSON of a path with a query, or nothing when Netdata cannot be reached or
    /// refuses the request. The item then shows the chart without a value, as in the
    /// PHP application.
    async fn read(&self, http: &Outbound, base: &str, path: &str) -> Option<Value> {
        let url = format!("{}/{path}", base.trim_end_matches('/'));
        let sent = http
            .client(&url, self.never_check_certificate)
            .get(&url)
            .timeout(self.timeout.unwrap_or(TIME_LIMIT))
            .send()
            .await;
        let response = match sent {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(error = without_address(error), "Netdata cannot be reached");
                return None;
            }
        };
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "Netdata refused the request");
            return None;
        }
        Some(response.json().await.unwrap_or(Value::Null))
    }

    /// The data of a chart of a fixed source: `value`, `percent`, `units` and `max`.
    /// Without an answer of Netdata, the data is empty. [`Source::Custom`] has no fixed
    /// request, see [`Install::custom`].
    pub async fn chart(&self, http: &Outbound, source: Source) -> Map<String, Value> {
        const RATE: &str = "&format=array&points=540&group=average&gtime=0&options=absolute|jsonwrap|nonzero&after=-540";
        const PART: &str = "&format=array&points=540&group=average&gtime=0&options=absolute|percentage|jsonwrap|nonzero&after=-540";
        const SPACE: &str = "api/v1/data?chart=disk_space._&format=json&points=509&group=average&gtime=0&options=ms|jsonwrap|nonzero&after=-540&dimension=";
        let rate_of = |chart: &str, dimension: &str| {
            format!("api/v1/data?chart={chart}&dimensions={dimension}{RATE}")
        };
        let (path, units) = match source {
            Source::DiskRead => (rate_of("system.io", "in"), "MiB/s"),
            Source::DiskWrite => (rate_of("system.io", "out"), "MiB/s"),
            Source::NetIn => (rate_of("system.net", "received"), "Mbit/s"),
            Source::NetOut => (rate_of("system.net", "sent"), "Mbit/s"),
            Source::Cpu => ("api/v1/data?chart=system.cpu&format=array".to_owned(), ""),
            Source::RamUsed => (
                format!("api/v1/data?chart=system.ram{PART}&dimensions=used|buffers|active|wired"),
                "",
            ),
            Source::SwapUsed => (
                format!("api/v1/data?chart=system.swap{PART}&dimensions=used"),
                "",
            ),
            Source::DiskAvail => (format!("{SPACE}avail"), ""),
            Source::DiskUsed => (format!("{SPACE}used"), ""),
            Source::Custom => return Map::new(),
        };
        let answer = self.read(http, self.url, &path).await;
        let data = answer.as_ref().map(|answer| match source {
            Source::DiskRead | Source::DiskWrite | Source::NetIn | Source::NetOut => {
                rate(answer, units)
            }
            Source::Cpu => part(answer.get(0)),
            Source::RamUsed | Source::SwapUsed => {
                part(answer.get("result").and_then(|result| result.get(0)))
            }
            _ => part(answer.pointer("/result/data/0/1")),
        });
        match source {
            // What goes out is counted below 0.
            Source::DiskWrite | Source::NetOut => positive(data.unwrap_or_default()),
            _ => data.unwrap_or_default(),
        }
    }

    /// The data of a custom chart. `base` is the address to ask: the one of the
    /// install, or the one that the entry names.
    pub async fn custom(&self, http: &Outbound, base: &str, custom: &Custom) -> Map<String, Value> {
        // The path of an entry may start with a slash. One slash goes to Netdata.
        match self
            .read(http, base, custom.url.trim_start_matches('/'))
            .await
        {
            Some(answer) => custom_data(&answer, custom),
            None => Map::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{
        Custom, Source, custom_data, custom_definition, mutated, part, positive, rate, selected,
    };

    #[test]
    fn rate_is_in_thousands_with_its_part_of_the_highest_value() {
        let data = rate(&json!({"latest_values": [1500.0], "max": 6000}), "MiB/s");
        assert_eq!(
            Value::Object(data),
            json!({"value": 1.5, "percent": 25, "units": "MiB/s", "max": 6000})
        );
        // Without a highest value, the part is 0.
        let data = rate(&json!({"latest_values": [-3000], "max": 0}), "Mbit/s");
        assert_eq!(data["value"], json!(-3));
        assert_eq!(data["percent"], json!(0));
        let data = positive(data);
        assert_eq!(data["value"], json!(3));
        // An answer without the fields counts as 0.
        assert_eq!(rate(&json!({}), "MiB/s")["value"], json!(0));
        assert_eq!(
            Value::Object(positive(serde_json::Map::new())),
            json!({"value": 0, "percent": 0})
        );
    }

    #[test]
    fn part_of_one_hundred_is_its_own_percent() {
        assert_eq!(
            Value::Object(part(Some(&json!(42.5)))),
            json!({"value": 42.5, "percent": 42.5, "max": 100, "units": "%"})
        );
        assert_eq!(part(None)["value"], Value::Null);
    }

    #[test]
    fn source_comes_from_the_value_of_the_setting() {
        assert_eq!(Source::of("disk-read"), Some(Source::DiskRead));
        assert_eq!(Source::of("custom"), Some(Source::Custom));
        // PHP names these, and has no code for them.
        assert_eq!(Source::of("cpu-temp-c"), None);
        assert_eq!(Source::of(""), None);
    }

    #[test]
    fn custom_definition_is_read_with_the_texts_of_php_for_what_is_wrong() {
        let setting = r#"{"1": {"url": "/api/v1/data?chart=system.ram", "value": "result,0"},
            "2": {"url": "x", "value": "a", "units": "GB", "max": "", "mutator": "/1000",
                  "netdata": "http://other:19999"},
            "3": {"url": "x"}}"#;
        assert_eq!(
            custom_definition(setting, 1),
            Ok(Custom {
                url: "/api/v1/data?chart=system.ram".to_owned(),
                value: "result,0".to_owned(),
                units: json!("%"),
                max: json!(100),
                mutator: None,
                netdata: None,
            })
        );
        let second = custom_definition(setting, 2).expect("the entry is complete");
        assert_eq!(second.units, json!("GB"));
        assert_eq!(second.max, json!(100));
        assert_eq!(second.mutator.as_deref(), Some("/1000"));
        assert_eq!(second.netdata.as_deref(), Some("http://other:19999"));
        assert_eq!(
            custom_definition(setting, 3),
            Err("custom definition incomplete")
        );
        assert_eq!(
            custom_definition(setting, 4),
            Err("custom definition not found")
        );
        for unreadable in ["{}", "", "not JSON", "[]", "5"] {
            assert_eq!(
                custom_definition(unreadable, 1),
                Err("unable to parse custom JSON"),
                "{unreadable}"
            );
        }
    }

    #[test]
    fn way_through_an_answer_takes_keys_and_places() {
        let answer = json!({"result": [7, {"deep": 9}], "0": "zero"});
        assert_eq!(selected(&answer, "result,0"), Some(&json!(7)));
        assert_eq!(selected(&answer, "result,1,deep"), Some(&json!(9)));
        assert_eq!(selected(&answer, "0"), Some(&json!("zero")));
        assert_eq!(selected(&answer, "result,5"), None);
        assert_eq!(selected(&answer, "result,0,more"), None);
    }

    #[test]
    fn mutators_calculate_in_their_order() {
        assert!((mutated(2000.0, "/1000,*8,+1,-3") - 14.0).abs() < 1e-9);
        // A division by 0, an unknown sign and an empty part change nothing.
        assert!((mutated(5.0, "/0,%2,,/nothing") - 5.0).abs() < 1e-9);
        assert!((mutated(5.0, "*2 times") - 10.0).abs() < 1e-9);
    }

    #[test]
    fn custom_chart_has_its_value_and_its_part_of_the_highest_value() {
        let custom = Custom {
            url: String::new(),
            value: "result,0".to_owned(),
            units: json!("GB"),
            max: json!("200"),
            mutator: Some("/1000".to_owned()),
            netdata: None,
        };
        assert_eq!(
            Value::Object(custom_data(&json!({"result": [50000]}), &custom)),
            json!({"max": "200", "units": "GB", "value": 50, "percent": 25})
        );
        // A value that is no number is no value.
        assert_eq!(
            Value::Object(custom_data(&json!({"result": [[1]]}), &custom)),
            json!({"max": "200", "units": "GB", "value": null, "percent": 0})
        );
    }
}
