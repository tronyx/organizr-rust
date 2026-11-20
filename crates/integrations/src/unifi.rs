//! UniFi: the health of the parts of a network, from a UniFi Network controller.
//!
//! There are two kinds of controller. A console with UniFi OS answers the request for
//! its start page with a header `x-csrf-token`, takes the login at `api/auth/login`,
//! and has the API of the network under `proxy/network`. The classic controller sends
//! no such header, takes the login at `api/login`, and has the API at its root. This is
//! how the PHP application tells them apart.
//!
//! The login gives cookies. They are used for the one request that follows, and are
//! not kept.

use std::time::Duration;

use reqwest::{Response, Url, header};
use serde_json::{Value, json};

use crate::http::{Outbound, without_address};

/// How long the controller has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// Why the controller gave no data.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UnifiError {
    /// The controller cannot be reached, or it did not answer in time. The text names
    /// no address.
    #[error("{0}")]
    Unreachable(String),
    /// The controller refused the login.
    #[error("Unifi response error - Check Credentials")]
    Credentials,
    /// The controller refused the request that came after the login.
    #[error("Unifi response error3")]
    Refused,
    /// A console with UniFi OS was asked for its sites.
    #[error("Unifi OS does not support Multi Site")]
    NoSites,
}

/// A controller, and how it is asked.
#[derive(Debug, Clone, Copy)]
pub struct Controller<'a> {
    /// The address, complete.
    pub url: &'a str,
    /// The user name of the login.
    pub username: &'a str,
    /// The password of the login.
    pub password: &'a str,
    /// The name of the site, for the classic controller.
    pub site: &'a str,
    /// How long the controller has for each answer.
    pub timeout: Option<Duration>,
    /// Whether the certificate of the controller is left unchecked.
    pub never_check_certificate: bool,
}

/// What a login gave.
struct Session {
    /// The token of a console with UniFi OS, or nothing for the classic controller.
    token: Option<String>,
    /// The cookies, in the form of a `Cookie` header.
    cookies: String,
}

/// The cookies that an answer sets, in the form of a `Cookie` header.
fn cookies_of(response: &Response) -> String {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|cookie| cookie.split(';').next())
        .map(str::trim)
        .filter(|pair| !pair.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

fn unreachable(error: reqwest::Error) -> UnifiError {
    UnifiError::Unreachable(without_address(error))
}

impl Controller<'_> {
    fn base(&self) -> &str {
        self.url.trim_end_matches('/')
    }

    fn limit(&self) -> Duration {
        self.timeout.unwrap_or(TIME_LIMIT)
    }

    /// The token of a console with UniFi OS, or nothing for the classic controller and
    /// for a controller that refuses the request for its start page.
    async fn token(&self, http: &Outbound) -> Result<Option<String>, UnifiError> {
        let response = http
            .client(self.url, self.never_check_certificate)
            .get(self.url)
            .timeout(self.limit())
            .send()
            .await
            .map_err(unreachable)?;
        if !response.status().is_success() {
            return Ok(None);
        }
        Ok(response
            .headers()
            .get("x-csrf-token")
            .and_then(|value| value.to_str().ok())
            .filter(|token| !token.is_empty())
            .map(str::to_owned))
    }

    async fn login(&self, http: &Outbound) -> Result<Session, UnifiError> {
        let token = self.token(http).await?;
        let client = http.client(self.url, self.never_check_certificate);
        let request = match &token {
            Some(token) => client
                .post(format!("{}/api/auth/login", self.base()))
                .header("x-csrf-token", token)
                .form(&[
                    ("username", self.username),
                    ("password", self.password),
                    ("remember", "1"),
                    ("strict", "1"),
                ]),
            None => client
                .post(format!("{}/api/login", self.base()))
                .json(&json!({
                    "username": self.username,
                    "password": self.password,
                    "remember": true,
                    "strict": true,
                })),
        };
        let response = request
            .timeout(self.limit())
            .send()
            .await
            .map_err(unreachable)?;
        if !response.status().is_success() {
            return Err(UnifiError::Credentials);
        }
        Ok(Session {
            token,
            cookies: cookies_of(&response),
        })
    }

    /// The JSON of an address, asked with the cookies of a login.
    async fn read(
        &self,
        http: &Outbound,
        session: &Session,
        url: Url,
    ) -> Result<Value, UnifiError> {
        let mut request = http
            .client(self.url, self.never_check_certificate)
            .get(url)
            .header(header::COOKIE, &session.cookies)
            .timeout(self.limit());
        if let Some(token) = &session.token {
            request = request.header("x-csrf-token", token);
        }
        let response = request.send().await.map_err(unreachable)?;
        if !response.status().is_success() {
            return Err(UnifiError::Refused);
        }
        Ok(response.json().await.unwrap_or(Value::Null))
    }

    /// The address of the health of the site: of the site `default` on a console with
    /// UniFi OS, and of the site of the settings on the classic controller.
    fn health_address(&self, is_unifi_os: bool) -> Result<Url, UnifiError> {
        let no_address = || UnifiError::Unreachable("the address cannot be read".to_owned());
        let mut url = Url::parse(self.base()).map_err(|_| no_address())?;
        {
            let mut path = url.path_segments_mut().map_err(|()| no_address())?;
            path.pop_if_empty();
            if is_unifi_os {
                path.extend(["proxy", "network", "api", "s", "default"]);
            } else {
                // The name is one part of the path, whatever it holds.
                path.extend(["api", "s", self.site]);
            }
            path.extend(["stat", "health"]);
        }
        Ok(url)
    }

    /// The health of the parts of the network, as `stat/health` gives it.
    pub async fn health(&self, http: &Outbound) -> Result<Value, UnifiError> {
        let session = self.login(http).await?;
        let url = self.health_address(session.token.is_some())?;
        self.read(http, &session, url).await
    }

    /// The sites of a classic controller, as `api/self/sites` gives them.
    pub async fn sites(&self, http: &Outbound) -> Result<Value, UnifiError> {
        let session = self.login(http).await?;
        if session.token.is_some() {
            return Err(UnifiError::NoSites);
        }
        let url = Url::parse(&format!("{}/api/self/sites", self.base()))
            .map_err(|_| UnifiError::Unreachable("the address cannot be read".to_owned()))?;
        self.read(http, &session, url).await
    }
}

#[cfg(test)]
mod tests {
    use super::Controller;

    fn controller<'a>(url: &'a str, site: &'a str) -> Controller<'a> {
        Controller {
            url,
            username: "",
            password: "",
            site,
            timeout: None,
            never_check_certificate: false,
        }
    }

    #[test]
    fn address_of_the_health_depends_on_the_kind_of_controller() {
        let address = |url, site, is_unifi_os| {
            controller(url, site)
                .health_address(is_unifi_os)
                .map(String::from)
        };
        assert_eq!(
            address("https://unifi.example.com:8443/", "home", false).as_deref(),
            Ok("https://unifi.example.com:8443/api/s/home/stat/health")
        );
        assert_eq!(
            address("https://192.0.2.1", "home", true).as_deref(),
            Ok("https://192.0.2.1/proxy/network/api/s/default/stat/health")
        );
        assert_eq!(
            address("https://example.com/unifi", "default", false).as_deref(),
            Ok("https://example.com/unifi/api/s/default/stat/health")
        );
        // A name of a site cannot lead to another path.
        assert_eq!(
            address("https://example.com", "../../admin?x=1", false).as_deref(),
            Ok("https://example.com/api/s/..%2F..%2Fadmin%3Fx=1/stat/health")
        );
        assert!(address("no address", "default", false).is_err());
    }
}
