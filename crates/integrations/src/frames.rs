//! Whether a page lets another site show it in a frame, read from the header
//! `X-Frame-Options` of its answer.

use std::time::Duration;

use crate::http::{Outbound, without_address};
use crate::probe::host_of;

/// How long the server of a page has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(15);

/// Whether the header lets a page of the host `organizr` show the page at `url` in a
/// frame. `header` is the value of `X-Frame-Options`, when the answer has one.
///
/// A page without the header can be framed. `DENY` forbids it. `SAMEORIGIN` allows it
/// when the page is on the host of Organizr, and `ALLOW-FROM` when it names that host.
/// Every other value forbids it, as in the PHP application.
pub fn may_frame(header: Option<&str>, url: &str, organizr: &str) -> bool {
    let Some(header) = header else {
        return true;
    };
    let header = header.trim().to_lowercase();
    let organizr = organizr.to_lowercase();
    let is_organizr =
        |address: &str| host_of(address).is_some_and(|host| host.to_lowercase() == organizr);
    if header == "sameorigin" {
        is_organizr(url)
    } else if header.starts_with("allow-from") {
        header.split_whitespace().skip(1).any(is_organizr)
    } else {
        false
    }
}

/// The host of a `Host` header, without its port.
pub fn host_name(host: &str) -> Option<String> {
    host_of(&format!("http://{host}"))
}

/// Asks for the page at `url`, and says whether a page of the host `organizr` can show
/// it in a frame. Fails when the server cannot be reached. The text of the error
/// names no address.
pub async fn test(http: &Outbound, url: &str, organizr: &str) -> Result<bool, String> {
    let response = http
        .client_for(url)
        .get(url)
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
    let header = response
        .headers()
        .get("x-frame-options")
        .and_then(|value| value.to_str().ok());
    Ok(may_frame(header, url, organizr))
}

#[cfg(test)]
mod tests {
    use super::may_frame;

    #[test]
    fn header_says_who_can_frame_a_page() {
        let organizr = "organizr.example.com";
        let frame = |header: Option<&str>, url: &str| may_frame(header, url, organizr);

        assert!(frame(None, "https://sonarr.example.com/"));
        assert!(!frame(Some("DENY"), "https://sonarr.example.com/"));
        assert!(!frame(Some("SAMEORIGIN"), "https://sonarr.example.com/"));
        assert!(frame(
            Some("SAMEORIGIN"),
            "https://organizr.example.com/sonarr"
        ));
        assert!(frame(
            Some(" sameorigin "),
            "https://Organizr.Example.com:8443/sonarr"
        ));
        assert!(frame(
            Some("ALLOW-FROM https://organizr.example.com"),
            "https://sonarr.example.com/"
        ));
        assert!(!frame(
            Some("ALLOW-FROM https://other.example.com"),
            "https://sonarr.example.com/"
        ));
        assert!(!frame(
            Some("something else"),
            "https://organizr.example.com/"
        ));
    }
}
