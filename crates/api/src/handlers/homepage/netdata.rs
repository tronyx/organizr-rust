//! The Netdata item: up to seven charts, each with one number of a Netdata install.

use axum::Extension;
use axum::extract::State;
use axum::http::HeaderMap;
use organizr_integrations::http::{Outbound, time_limit};
use organizr_integrations::netdata::{self, Install, Source};
use organizr_settings::Settings;
use serde_json::{Map, Value, json};

use super::{Permissions, is_empty, no_requests, service_url};
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The numbers of the charts.
const CHARTS: std::ops::RangeInclusive<usize> = 1..=7;

/// What must hold before a caller gets the item.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageNetdataEnabled".to_owned()],
        auth: vec!["homepageNetdataAuth".to_owned()],
        not_empty: vec!["netdataURL".to_owned()],
    }
}

/// A setting of a chart, for example `netdata3Title`.
fn of_chart(settings: &Settings, chart: usize, name: &str) -> Value {
    settings
        .get(&format!("netdata{chart}{name}"))
        .unwrap_or(Value::Null)
}

/// What a request for the charts needs.
struct Asked<'a> {
    settings: &'a Settings,
    headers: &'a HeaderMap,
    client: &'a Client,
    http: &'a Outbound,
}

impl Asked<'_> {
    /// The data of a chart from Netdata, without the settings of the chart.
    async fn data(&self, install: Install<'_>, chart: usize) -> Map<String, Value> {
        let source = of_chart(self.settings, chart, "Data");
        match Source::of(source.as_str().unwrap_or_default()) {
            Some(Source::Custom) => {
                let custom = match netdata::custom_definition(&self.settings.netdata_custom, chart)
                {
                    Ok(custom) => custom,
                    Err(error) => {
                        let mut data = Map::new();
                        data.insert("error".to_owned(), json!(error));
                        return data;
                    }
                };
                // An entry can name another install.
                let base = custom.netdata.as_deref().map_or_else(
                    || install.url.to_owned(),
                    |address| service_url(address, self.headers, self.client),
                );
                install.custom(self.http, &base, &custom).await
            }
            Some(source) => install.chart(self.http, source).await,
            // A source that is not known, as the PHP application answers it.
            None => {
                let mut data = Map::new();
                data.insert("value".to_owned(), json!(0));
                data.insert("units".to_owned(), json!("N/A"));
                data.insert("max".to_owned(), json!(100));
                data
            }
        }
    }
}

/// `GET /homepage/netdata/data`: the charts that are switched on, in their order.
pub async fn data(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = &state
        .outbound_for(settings.netdata_use_custom_certificate)
        .ok_or_else(no_requests)?;

    let url = service_url(&settings.netdata_url, &headers, &client);
    let install = Install {
        url: &url,
        timeout: time_limit(&settings.homepage_netdata_refresh.to_string()),
        never_check_certificate: settings.netdata_disable_cert_check,
    };
    let asked = Asked {
        settings: &settings,
        headers: &headers,
        client: &client,
        http,
    };
    let mut charts = Vec::new();
    for chart in CHARTS {
        if is_empty(Some(&of_chart(&settings, chart, "Enabled"))) {
            continue;
        }
        let mut data = asked.data(install, chart).await;
        for (field, name) in [
            ("title", "Title"),
            ("colour", "Colour"),
            ("chart", "Chart"),
            ("size", "Size"),
            ("lg", "lg"),
            ("md", "md"),
            ("sm", "sm"),
        ] {
            data.insert(field.to_owned(), of_chart(&settings, chart, name));
        }
        charts.push(Value::Object(data));
    }
    Ok(Envelope::success(json!({
        "data": charts,
        "url": settings.netdata_url,
    })))
}
