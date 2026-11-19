//! The account service of Plex, `plex.tv`.
//!
//! Organizr asks it three things: whether a name and a password are those of a Plex
//! account, whose account a token belongs to, and whether an account is a friend of the
//! owner of the Plex server, which means that the owner shared the server with it.

use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;

use crate::http::Outbound;

/// The address of the service.
pub const PLEX_TV: &str = "https://plex.tv";

/// How long a login at the service may take.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(30);

/// How long every other request may take.
const TIMEOUT: Duration = Duration::from_secs(10);

/// The name and the version under which Organizr shows up in the device list of a Plex
/// account.
const PRODUCT: &str = "Organizr";
const PRODUCT_VERSION: &str = "2.0";

/// The client id that the PHP application uses when it lists servers.
const SERVER_LIST_CLIENT: &str = "01010101-10101010";

/// Why the service gave no answer that can be used.
#[derive(Debug, Error)]
pub enum PlexError {
    /// The service cannot be reached, or it did not answer in time.
    #[error("plex.tv cannot be reached: {0}")]
    Unreachable(#[source] reqwest::Error),
    /// The service answered with a status that is no success.
    #[error("plex.tv answered with the status {0}")]
    Status(u16),
    /// The answer is not what the service usually sends.
    #[error("the answer of plex.tv cannot be read: {0}")]
    Unreadable(String),
}

/// A Plex account, as the service describes it.
#[derive(Clone, PartialEq, Eq)]
pub struct PlexAccount {
    /// The user name.
    pub username: String,
    /// The e-mail address.
    pub email: String,
    /// The address of the picture.
    pub image: Option<String>,
    /// The token that acts for the account.
    pub token: String,
}

impl std::fmt::Debug for PlexAccount {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The token never goes into a log line.
        formatter
            .debug_struct("PlexAccount")
            .field("username", &self.username)
            .field("email", &self.email)
            .finish_non_exhaustive()
    }
}

/// A Plex server of an account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexServer {
    /// The name that the owner gave the server.
    pub name: String,
    /// The address of the server.
    pub address: String,
    /// The id of the server, which the `plexID` setting holds.
    pub machine_identifier: String,
    /// Whether the account owns the server.
    pub owned: bool,
}

#[derive(Deserialize)]
struct AccountAnswer {
    user: AccountFields,
}

#[derive(Deserialize)]
struct AccountFields {
    username: Option<String>,
    email: Option<String>,
    thumb: Option<String>,
    // The service sends the token under both names in one answer.
    #[serde(rename = "authToken")]
    auth_token: Option<String>,
    authentication_token: Option<String>,
}

impl AccountAnswer {
    fn into_account(self, token: Option<&str>) -> Option<PlexAccount> {
        let fields = self.user;
        Some(PlexAccount {
            username: fields.username.filter(|name| !name.is_empty())?,
            email: fields.email.unwrap_or_default(),
            image: fields.thumb.filter(|thumb| !thumb.is_empty()),
            token: fields
                .auth_token
                .or(fields.authentication_token)
                .filter(|token| !token.is_empty())
                .or_else(|| token.map(str::to_owned))?,
        })
    }
}

/// The client for the account service.
#[derive(Debug, Clone)]
pub struct PlexTv {
    base: String,
    http: Outbound,
}

impl PlexTv {
    /// A client for the service at `base`: [`PLEX_TV`], or a stand-in for a test.
    pub fn new(http: Outbound, base: &str) -> Self {
        Self {
            base: base.trim_end_matches('/').to_owned(),
            http,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    /// Logs in with the name, or the e-mail address, and the password of a Plex account.
    /// `client_id` is the id of this install. Returns `None` when the service does not
    /// take the login.
    pub async fn sign_in(
        &self,
        client_id: &str,
        login: &str,
        password: &str,
    ) -> Result<Option<PlexAccount>, PlexError> {
        let url = self.url("/users/sign_in.json");
        let response = self
            .http
            .client_for(&url)
            .post(&url)
            .timeout(LOGIN_TIMEOUT)
            .header("Accept", "application/json")
            .header("X-Plex-Product", PRODUCT)
            .header("X-Plex-Version", PRODUCT_VERSION)
            .header("X-Plex-Client-Identifier", client_id)
            .form(&[("user[login]", login), ("user[password]", password)])
            .send()
            .await
            .map_err(PlexError::Unreachable)?;
        if !response.status().is_success() {
            return Ok(None);
        }
        let answer: AccountAnswer = response
            .json()
            .await
            .map_err(|error| PlexError::Unreadable(error.to_string()))?;
        Ok(answer.into_account(None))
    }

    /// The account that a token belongs to. Returns `None` when the service does not
    /// know the token.
    pub async fn account(&self, token: &str) -> Result<Option<PlexAccount>, PlexError> {
        if token.is_empty() {
            return Ok(None);
        }
        let url = self.url("/users/account.json");
        let response = self
            .http
            .client_for(&url)
            .get(&url)
            .timeout(TIMEOUT)
            .header("Accept", "application/json")
            .header("X-Plex-Token", token)
            .send()
            .await
            .map_err(PlexError::Unreachable)?;
        if !response.status().is_success() {
            return Ok(None);
        }
        let answer: AccountAnswer = response
            .json()
            .await
            .map_err(|error| PlexError::Unreadable(error.to_string()))?;
        Ok(answer.into_account(Some(token)))
    }

    async fn xml(&self, path: &str, headers: &[(&str, &str)]) -> Result<String, PlexError> {
        let url = self.url(path);
        let mut request = self.http.client_for(&url).get(&url).timeout(TIMEOUT);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let response = request.send().await.map_err(PlexError::Unreachable)?;
        if !response.status().is_success() {
            return Err(PlexError::Status(response.status().as_u16()));
        }
        response
            .text()
            .await
            .map_err(|error| PlexError::Unreadable(error.to_string()))
    }

    /// Whether an account is a friend of the owner of `owner_token`. `name` is the user
    /// name or the e-mail address of the account, in any letter case.
    ///
    /// With `server`, the id of a Plex server, the owner must also have shared that
    /// server with the account. This is the `plexStrictFriends` setting.
    pub async fn is_friend(
        &self,
        owner_token: &str,
        name: &str,
        server: Option<&str>,
    ) -> Result<bool, PlexError> {
        if owner_token.is_empty() {
            return Ok(false);
        }
        let text = self
            .xml("/api/users", &[("X-Plex-Token", owner_token)])
            .await?;
        friend_in(&text, name, server)
    }

    /// The Plex servers of the account of a token.
    pub async fn servers(&self, token: &str) -> Result<Vec<PlexServer>, PlexError> {
        let headers = [
            ("X-Plex-Product", PRODUCT),
            ("X-Plex-Version", PRODUCT_VERSION),
            ("X-Plex-Client-Identifier", SERVER_LIST_CLIENT),
            ("X-Plex-Token", token),
        ];
        let text = self.xml("/pms/servers", &headers).await?;
        servers_in(&text)
    }
}

fn parsed(text: &str) -> Result<roxmltree::Document<'_>, PlexError> {
    roxmltree::Document::parse(text).map_err(|error| PlexError::Unreadable(error.to_string()))
}

/// Looks for an account in the friend list of the service.
fn friend_in(xml: &str, name: &str, server: Option<&str>) -> Result<bool, PlexError> {
    let wanted = name.to_lowercase();
    let document = parsed(xml)?;
    let is_wanted = |value: Option<&str>| {
        value.is_some_and(|value| !value.is_empty() && value.to_lowercase() == wanted)
    };
    Ok(document
        .root_element()
        .children()
        .filter(roxmltree::Node::is_element)
        .filter(|user| is_wanted(user.attribute("username")) || is_wanted(user.attribute("email")))
        .any(|user| match server {
            None => true,
            Some(server) => user
                .children()
                .filter(|child| child.has_tag_name("Server"))
                .any(|shared| shared.attribute("machineIdentifier") == Some(server)),
        }))
}

/// Reads the server list of the service.
fn servers_in(xml: &str) -> Result<Vec<PlexServer>, PlexError> {
    let document = parsed(xml)?;
    Ok(document
        .root_element()
        .children()
        .filter(roxmltree::Node::is_element)
        .map(|server| {
            let text = |name: &str| server.attribute(name).unwrap_or_default().to_owned();
            PlexServer {
                name: text("name"),
                address: text("address"),
                machine_identifier: text("machineIdentifier"),
                owned: server.attribute("owned") == Some("1"),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{AccountAnswer, PlexAccount, friend_in, servers_in};

    /// The friend list as the service sends it, with made-up accounts.
    const FRIENDS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<MediaContainer friendlyName="myPlex" identifier="com.plexapp.plugins.myplex" machineIdentifier="abc" totalSize="3" size="3">
  <User id="101" title="Robin" username="Robin" email="robin@example.com" thumb="https://plex.tv/users/1/avatar" home="0">
    <Server id="1" serverId="11" machineIdentifier="server-one" name="Media" lastSeenAt="1" numLibraries="3" owned="0" pending="0"/>
  </User>
  <User id="102" title="pat" username="pat" email="Pat@Example.com" home="0">
    <Server id="2" serverId="12" machineIdentifier="server-two" name="Other" owned="0" pending="0"/>
    <Server id="3" serverId="11" machineIdentifier="server-one" name="Media" owned="0" pending="0"/>
  </User>
  <User id="103" title="Guest" username="" email="" home="1"/>
</MediaContainer>"#;

    #[test]
    fn friend_is_found_by_name_or_address_in_any_letter_case() {
        for name in [
            "robin",
            "ROBIN",
            "robin@example.com",
            "pat@example.com",
            "PAT",
        ] {
            assert_eq!(friend_in(FRIENDS, name, None).ok(), Some(true), "{name}");
        }
        for name in ["sam", "", "robin@example.org", "Guest"] {
            assert_eq!(friend_in(FRIENDS, name, None).ok(), Some(false), "{name}");
        }
    }

    #[test]
    fn strict_check_needs_the_server_to_be_shared_with_the_friend() {
        assert_eq!(
            friend_in(FRIENDS, "robin", Some("server-one")).ok(),
            Some(true)
        );
        assert_eq!(
            friend_in(FRIENDS, "robin", Some("server-two")).ok(),
            Some(false)
        );
        assert_eq!(
            friend_in(FRIENDS, "pat", Some("server-one")).ok(),
            Some(true)
        );
        assert_eq!(
            friend_in(FRIENDS, "pat", Some("server-two")).ok(),
            Some(true)
        );
        assert_eq!(
            friend_in(FRIENDS, "pat", Some("server-three")).ok(),
            Some(false)
        );
    }

    #[test]
    fn answer_that_is_no_xml_is_an_error() {
        assert!(friend_in("<html>Maintenance", "robin", None).is_err());
        assert!(servers_in("").is_err());
    }

    #[test]
    fn servers_are_read_with_their_ids() {
        let xml = r#"<MediaContainer friendlyName="myPlex" size="2">
  <Server accessToken="made-up" name="Media" address="203.0.113.5" port="32400" machineIdentifier="server-one" owned="1"/>
  <Server name="Friend's" address="198.51.100.9" machineIdentifier="server-two" owned="0"/>
</MediaContainer>"#;

        let servers = servers_in(xml).expect("the list can be read");

        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].name, "Media");
        assert_eq!(servers[0].address, "203.0.113.5");
        assert_eq!(servers[0].machine_identifier, "server-one");
        assert!(servers[0].owned && !servers[1].owned);
        assert_eq!(servers[1].name, "Friend's");
    }

    fn account(json: &str, token: Option<&str>) -> Option<PlexAccount> {
        serde_json::from_str::<AccountAnswer>(json)
            .expect("the answer is JSON")
            .into_account(token)
    }

    #[test]
    fn token_is_read_under_both_of_its_names() {
        // The service sends both names in one answer.
        let both = account(
            r#"{"user": {"username": "Robin", "authToken": "new-name", "authentication_token": "old-name"}}"#,
            None,
        );
        assert_eq!(
            both.map(|account| account.token).as_deref(),
            Some("new-name")
        );
        let older = account(
            r#"{"user": {"username": "Robin", "authentication_token": "old-name"}}"#,
            None,
        );
        assert_eq!(
            older.map(|account| account.token).as_deref(),
            Some("old-name")
        );
    }

    #[test]
    fn account_is_read_from_both_forms_of_the_answer() {
        let signed_in = account(
            r#"{"user": {"id": 1, "username": "Robin", "email": "robin@example.com", "thumb": "https://plex.tv/users/1/avatar", "authToken": "made-up-token"}}"#,
            None,
        )
        .expect("the answer holds an account");
        assert_eq!(signed_in.username, "Robin");
        assert_eq!(signed_in.email, "robin@example.com");
        assert_eq!(
            signed_in.image.as_deref(),
            Some("https://plex.tv/users/1/avatar")
        );
        assert_eq!(signed_in.token, "made-up-token");
        assert!(!format!("{signed_in:?}").contains("made-up-token"));

        // An answer without a token: the token is the one that was asked about.
        let by_token = account(
            r#"{"user": {"username": "Robin", "email": "robin@example.com", "thumb": ""}}"#,
            Some("the-asked-token"),
        )
        .expect("the answer holds an account");
        assert_eq!(by_token.token, "the-asked-token");
        assert_eq!(by_token.image, None);

        assert_eq!(
            account(r#"{"user": {"email": "robin@example.com"}}"#, Some("t")),
            None
        );
        assert_eq!(account(r#"{"user": {"username": "Robin"}}"#, None), None);
    }
}
