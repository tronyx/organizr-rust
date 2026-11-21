//! The login with an OpenID Connect provider, such as Authentik, Keycloak, PocketID or
//! Zitadel.
//!
//! The flow is the authorization code flow with PKCE (S256), a `state` and a `nonce`:
//!
//! 1. [`discover`] reads the discovery document of the provider, and the keys that sign
//!    its ID tokens.
//! 2. [`start`] makes the address of the login page of the provider, with a new
//!    `state`, `nonce` and PKCE verifier. The caller keeps the three values until the
//!    browser comes back.
//! 3. [`finish`] trades the code of the answer for the tokens, and checks the ID token:
//!    its signature against the keys of the provider, the issuer, the audience, the
//!    time and the nonce. The library `openidconnect` makes these checks.
//!
//! The requests go through the client of this application, [`Outbound`], without
//! redirects. No error text of this module holds a token, a code, a secret or an
//! address with one of them.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use openidconnect::core::{
    CoreAuthDisplay, CoreAuthPrompt, CoreAuthenticationFlow, CoreErrorResponseType,
    CoreGenderClaim, CoreJsonWebKey, CoreJweContentEncryptionAlgorithm, CoreJwsSigningAlgorithm,
    CoreProviderMetadata, CoreRevocableToken, CoreRevocationErrorResponse,
    CoreTokenIntrospectionResponse, CoreTokenType,
};
use openidconnect::{
    AccessTokenHash, AdditionalClaims, AsyncHttpClient, AuthType, AuthorizationCode,
    ClaimsVerificationError, Client, ClientId, ClientSecret, CsrfToken, EmptyExtraTokenFields,
    EndpointMaybeSet, EndpointNotSet, EndpointSet, HttpRequest, HttpResponse, IdTokenFields,
    JsonWebKeySet, Nonce, OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl,
    RequestTokenError, Scope, StandardErrorResponse, StandardTokenResponse, TokenResponse, http,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::http::{Outbound, without_address};

/// The largest answer of a provider that is read.
const LARGEST_ANSWER: usize = 1024 * 1024;

/// How long one request to a provider may take.
const REQUEST_TIME: Duration = Duration::from_secs(15);

/// The claims of a token that the standard does not name, such as the groups.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OtherClaims {
    #[serde(flatten)]
    pub claims: Map<String, Value>,
}

impl AdditionalClaims for OtherClaims {}

type Fields = IdTokenFields<
    OtherClaims,
    EmptyExtraTokenFields,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJwsSigningAlgorithm,
>;

type Tokens = StandardTokenResponse<Fields, CoreTokenType>;

/// The client of the library, with the endpoints that a discovery document gives.
type ProviderClient = Client<
    OtherClaims,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<CoreErrorResponseType>,
    Tokens,
    CoreTokenIntrospectionResponse,
    CoreRevocableToken,
    CoreRevocationErrorResponse,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

/// What went wrong in a login. The texts hold no token, code, secret or address.
#[derive(Debug, Error)]
pub enum OidcError {
    /// The discovery document could not be read.
    #[error("the discovery document could not be read: {0}")]
    Discovery(String),
    /// The discovery document lacks a field that the login needs.
    #[error("the discovery document has no `{0}`")]
    Missing(&'static str),
    /// The discovery document is not one of OpenID Connect.
    #[error("the discovery document is not valid: {0}")]
    InvalidDiscovery(String),
    /// The keys of the provider could not be read.
    #[error("the signing keys of the provider could not be read: {0}")]
    Keys(String),
    /// A value of the settings or of the login cannot be used.
    #[error("the login cannot be made: {0}")]
    Setup(String),
    /// The token endpoint refused the code, or could not be reached.
    #[error("the code could not be traded for tokens: {0}")]
    Token(String),
    /// The provider sent no ID token.
    #[error("the provider sent no ID token")]
    NoIdToken,
    /// The nonce of the ID token is not the one of this login.
    #[error("the nonce of the ID token is wrong")]
    WrongNonce,
    /// The ID token failed a check: signature, issuer, audience, time or hash.
    #[error("the ID token is not valid: {0}")]
    InvalidIdToken(String),
}

/// An error of a request to a provider, without its address.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct RequestError(String);

/// The HTTP client of the library: the requests go through [`Outbound`], with a time
/// limit, without redirects, and with a limit on the size of the answer.
#[derive(Debug, Clone, Copy)]
pub struct ProviderHttp<'a> {
    outbound: &'a Outbound,
}

impl<'a> ProviderHttp<'a> {
    pub const fn new(outbound: &'a Outbound) -> Self {
        Self { outbound }
    }

    async fn send(self, request: HttpRequest) -> Result<HttpResponse, RequestError> {
        let url = request.uri().to_string();
        let client = self.outbound.direct_client_for(&url);
        let mut request = reqwest::Request::try_from(request)
            .map_err(|error| RequestError(without_address(error)))?;
        *request.timeout_mut() = Some(REQUEST_TIME);
        let mut response = client
            .execute(request)
            .await
            .map_err(|error| RequestError(without_address(error)))?;
        let status = response.status();
        let headers = response.headers().clone();
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| RequestError(without_address(error)))?
        {
            if body.len().saturating_add(chunk.len()) > LARGEST_ANSWER {
                return Err(RequestError("the answer is too large".to_owned()));
            }
            body.extend_from_slice(&chunk);
        }
        let mut answer = http::Response::new(body);
        *answer.status_mut() = status;
        *answer.headers_mut() = headers;
        Ok(answer)
    }
}

impl<'c> AsyncHttpClient<'c> for ProviderHttp<'_> {
    type Error = RequestError;
    type Future = Pin<Box<dyn Future<Output = Result<HttpResponse, RequestError>> + Send + 'c>>;

    fn call(&'c self, request: HttpRequest) -> Self::Future {
        Box::pin(self.send(request))
    }
}

/// The fields of a discovery document that the PHP application asks for, in its order.
const NEEDED_FIELDS: [&str; 3] = ["authorization_endpoint", "token_endpoint", "issuer"];

/// What the discovery of a provider found.
#[derive(Debug, Clone)]
pub struct Discovered {
    metadata: CoreProviderMetadata,
}

impl Discovered {
    /// The issuer of the provider.
    pub fn issuer(&self) -> &str {
        self.metadata.issuer().as_str()
    }
}

/// Reads the discovery document at `address`, and the keys that sign the ID tokens of
/// the provider.
pub async fn discover(outbound: &Outbound, address: &str) -> Result<Discovered, OidcError> {
    let http = ProviderHttp::new(outbound);
    let request = http::Request::get(address)
        .header(http::header::ACCEPT, "application/json")
        .body(Vec::new())
        .map_err(|_| OidcError::Discovery("the address is not valid".to_owned()))?;
    let answer = http
        .send(request)
        .await
        .map_err(|error| OidcError::Discovery(error.0))?;
    if !answer.status().is_success() {
        return Err(OidcError::Discovery(format!(
            "the answer has the status {}",
            answer.status().as_u16()
        )));
    }
    let document: Value = serde_json::from_slice(answer.body())
        .map_err(|_| OidcError::Discovery("the answer is no JSON".to_owned()))?;
    for field in NEEDED_FIELDS {
        let present = document
            .get(field)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty());
        if !present {
            return Err(OidcError::Missing(field));
        }
    }
    let metadata = metadata_of(document)?;
    let keys = JsonWebKeySet::fetch_async(metadata.jwks_uri(), &http)
        .await
        .map_err(|error| OidcError::Keys(discovery_error_text(&error)))?;
    Ok(Discovered {
        metadata: metadata.set_jwks(keys),
    })
}

/// Reads the metadata of a discovery document, with the name of the field that is
/// wrong when it cannot be read.
fn metadata_of(document: Value) -> Result<CoreProviderMetadata, OidcError> {
    serde_json::from_value(document).map_err(|error| OidcError::InvalidDiscovery(error.to_string()))
}

/// The text of an error of the library that reads the keys, without the answer.
fn discovery_error_text<E: std::error::Error>(error: &openidconnect::DiscoveryError<E>) -> String {
    match error {
        openidconnect::DiscoveryError::Request(error) => error.to_string(),
        openidconnect::DiscoveryError::Response(status, _, _) => {
            format!("the answer has the status {}", status.as_u16())
        }
        openidconnect::DiscoveryError::Parse(_) => "the answer is not a key set".to_owned(),
        _ => "the keys could not be read".to_owned(),
    }
}

/// The client of an install at a provider.
#[derive(Debug, Clone, Copy)]
pub struct Application<'a> {
    pub client_id: &'a str,
    /// Empty for a public client.
    pub client_secret: &'a str,
    /// The address of the callback of this install.
    pub redirect: &'a str,
}

fn client(
    discovered: &Discovered,
    application: Application<'_>,
) -> Result<ProviderClient, OidcError> {
    let secret = (!application.client_secret.is_empty())
        .then(|| ClientSecret::new(application.client_secret.to_owned()));
    let redirect = RedirectUrl::new(application.redirect.to_owned())
        .map_err(|_| OidcError::Setup("the redirect address is not valid".to_owned()))?;
    // The client secret goes in the body of the token request, as the PHP application
    // sends it.
    Ok(ProviderClient::from_provider_metadata(
        discovered.metadata.clone(),
        ClientId::new(application.client_id.to_owned()),
        secret,
    )
    .set_auth_type(AuthType::RequestBody)
    .set_redirect_uri(redirect))
}

/// A login that was started: the address of the login page of the provider, and the
/// values to keep until the browser comes back.
#[derive(Debug, Clone)]
pub struct Started {
    pub address: String,
    pub state: String,
    pub nonce: String,
    pub verifier: String,
}

/// Starts a login. `scopes` is the setting of the provider: scopes with commas or
/// spaces between them. `openid` is always asked for.
pub fn start(
    discovered: &Discovered,
    application: Application<'_>,
    scopes: &str,
) -> Result<Started, OidcError> {
    let client = client(discovered, application)?;
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let mut request = client.authorize_url(
        CoreAuthenticationFlow::AuthorizationCode,
        CsrfToken::new_random,
        Nonce::new_random,
    );
    for scope in scopes
        .split([',', ' '])
        .map(str::trim)
        .filter(|scope| !scope.is_empty() && *scope != "openid")
    {
        request = request.add_scope(Scope::new(scope.to_owned()));
    }
    let (address, state, nonce) = request.set_pkce_challenge(challenge).url();
    Ok(Started {
        address: address.to_string(),
        state: state.secret().clone(),
        nonce: nonce.secret().clone(),
        verifier: verifier.secret().clone(),
    })
}

/// The answer of the provider, and what this install kept for it.
#[derive(Debug, Clone, Copy)]
pub struct Callback<'a> {
    pub code: &'a str,
    pub nonce: &'a str,
    pub verifier: &'a str,
}

/// The person whom the provider vouches for, from the user info of the provider or,
/// when the provider gives none, from the ID token.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Person {
    /// The subject (`sub`) of the checked ID token, also when the other claims come from
    /// the user info.
    pub subject: String,
    pub preferred_username: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub picture: Option<String>,
    /// The claims that the standard does not name, such as the groups.
    pub other: Map<String, Value>,
}

impl Person {
    /// The user name for Organizr: the preferred user name, or else the name, or else
    /// the subject, as in the PHP application. An empty value counts as none.
    pub fn username(&self) -> Option<&str> {
        [
            self.preferred_username.as_deref(),
            self.name.as_deref(),
            Some(self.subject.as_str()),
        ]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|name| !name.is_empty())
    }
}

/// Finishes a login: trades the code for the tokens, checks the ID token and reads the
/// user info.
pub async fn finish(
    outbound: &Outbound,
    discovered: &Discovered,
    application: Application<'_>,
    callback: Callback<'_>,
) -> Result<Person, OidcError> {
    let http = ProviderHttp::new(outbound);
    let client = client(discovered, application)?;
    let tokens = client
        .exchange_code(AuthorizationCode::new(callback.code.to_owned()))
        .map_err(|_| OidcError::Setup("the provider has no token endpoint".to_owned()))?
        .set_pkce_verifier(PkceCodeVerifier::new(callback.verifier.to_owned()))
        .request_async(&http)
        .await
        .map_err(|error| OidcError::Token(token_error_text(&error)))?;

    let id_token = tokens.id_token().ok_or(OidcError::NoIdToken)?;
    let verifier = client.id_token_verifier();
    let claims = id_token
        .claims(&verifier, &Nonce::new(callback.nonce.to_owned()))
        .map_err(|error| match error {
            ClaimsVerificationError::InvalidNonce(_) => OidcError::WrongNonce,
            other => OidcError::InvalidIdToken(claims_error_text(&other)),
        })?;
    if let Some(expected) = claims.access_token_hash() {
        let invalid = || OidcError::InvalidIdToken("the access token hash is wrong".to_owned());
        let algorithm = id_token.signing_alg().map_err(|_| invalid())?;
        let key = id_token.signing_key(&verifier).map_err(|_| invalid())?;
        let actual = AccessTokenHash::from_token(tokens.access_token(), algorithm, key)
            .map_err(|_| invalid())?;
        if actual != *expected {
            return Err(invalid());
        }
    }

    let from_token = Person {
        subject: claims.subject().to_string(),
        preferred_username: claims.preferred_username().map(|name| name.to_string()),
        name: claims
            .name()
            .and_then(|name| name.get(None))
            .map(|name| name.to_string()),
        email: claims.email().map(|email| email.to_string()),
        email_verified: claims.email_verified(),
        picture: claims
            .picture()
            .and_then(|picture| picture.get(None))
            .map(|picture| picture.to_string()),
        other: claims.additional_claims().claims.clone(),
    };

    // As in the PHP application, the user info comes first. The library checks that it
    // describes the subject of the ID token.
    let Ok(request) = client.user_info(
        tokens.access_token().clone(),
        Some(claims.subject().clone()),
    ) else {
        return Ok(from_token);
    };
    let info: Result<openidconnect::UserInfoClaims<OtherClaims, CoreGenderClaim>, _> =
        request.request_async(&http).await;
    match info {
        Ok(info) => Ok(Person {
            subject: claims.subject().to_string(),
            preferred_username: info.preferred_username().map(|name| name.to_string()),
            name: info
                .name()
                .and_then(|name| name.get(None))
                .map(|name| name.to_string()),
            email: info.email().map(|email| email.to_string()),
            email_verified: info.email_verified(),
            picture: info
                .picture()
                .and_then(|picture| picture.get(None))
                .map(|picture| picture.to_string()),
            other: info.additional_claims().claims.clone(),
        }),
        Err(error) => {
            tracing::warn!(error = %user_info_error_text(&error), "the user info of the provider could not be read; the claims of the ID token count");
            Ok(from_token)
        }
    }
}

fn token_error_text<E: std::error::Error>(
    error: &RequestTokenError<E, StandardErrorResponse<CoreErrorResponseType>>,
) -> String {
    match error {
        // Only the error code: its description could quote the code of the login.
        RequestTokenError::ServerResponse(answer) => {
            format!("the provider answered `{}`", answer.error().as_ref())
        }
        RequestTokenError::Request(error) => error.to_string(),
        RequestTokenError::Parse(_, _) => "the answer is not a token answer".to_owned(),
        RequestTokenError::Other(_) => "the answer of the provider is not valid".to_owned(),
    }
}

fn claims_error_text(error: &ClaimsVerificationError) -> String {
    match error {
        ClaimsVerificationError::Expired(_) => "it has expired".to_owned(),
        ClaimsVerificationError::InvalidAudience(_) => "the audience is wrong".to_owned(),
        ClaimsVerificationError::InvalidIssuer(_) => "the issuer is wrong".to_owned(),
        ClaimsVerificationError::InvalidSubject(_) => "the subject is wrong".to_owned(),
        ClaimsVerificationError::SignatureVerification(_) => {
            "the signature does not check out".to_owned()
        }
        other => other.to_string(),
    }
}

fn user_info_error_text<E: std::error::Error>(error: &openidconnect::UserInfoError<E>) -> String {
    match error {
        openidconnect::UserInfoError::Request(error) => error.to_string(),
        openidconnect::UserInfoError::Response(status, _, _) => {
            format!("the answer has the status {}", status.as_u16())
        }
        openidconnect::UserInfoError::ClaimsVerification(error) => claims_error_text(error),
        openidconnect::UserInfoError::Parse(_) => "the answer is not user info".to_owned(),
        _ => "the user info could not be read".to_owned(),
    }
}

/// How the groups of a person become one Organizr group, the setting
/// `oidcGroupMappingMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingMode {
    /// The first group of the claim that has a mapping.
    First,
    /// The mapped group with the most access: the lowest level.
    HighestPrivilege,
    /// The mapped group with the least access: the highest level.
    LowestPrivilege,
}

impl MappingMode {
    /// The mode of a setting. Any other text is `first`, as in the PHP application.
    pub fn from_setting(text: &str) -> Self {
        match text {
            "highest_privilege" => Self::HighestPrivilege,
            "lowest_privilege" => Self::LowestPrivilege,
            _ => Self::First,
        }
    }
}

/// The groups in the claim `claim` of a person. A text is one group. With
/// `roles_as_object`, an object gives its keys, as Zitadel sends its roles.
pub fn groups_of(claims: &Map<String, Value>, claim: &str, roles_as_object: bool) -> Vec<String> {
    match claims.get(claim) {
        Some(Value::String(group)) => vec![group.clone()],
        Some(Value::Array(groups)) => groups
            .iter()
            .filter_map(|group| match group {
                Value::String(group) => Some(group.clone()),
                Value::Number(group) => Some(group.to_string()),
                _ => None,
            })
            .collect(),
        Some(Value::Object(roles)) if roles_as_object => roles.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

/// The highest level of a group, the level of Guest.
const HIGHEST_LEVEL: u16 = 999;

/// The level of a mapping: a whole number from 0 to 999, or a text of one. Other values
/// are left out.
fn level_of(value: &Value) -> Option<u16> {
    let level = match value {
        Value::Number(number) => number
            .as_u64()
            .and_then(|number| u16::try_from(number).ok()),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }?;
    (level <= HIGHEST_LEVEL).then_some(level)
}

/// The level of the Organizr group for the groups of a person, from the mappings of
/// the setting `oidcGroupMappings` (a JSON object from a group name to a level).
/// `None` when no group has a mapping.
///
/// As in the PHP application, a group matches by its full name, then without a slash
/// at the start (Keycloak writes `/parent/child`), then by its last segment.
pub fn mapped_level(groups: &[String], mappings: &str, mode: MappingMode) -> Option<u16> {
    let Ok(Value::Object(mappings)) = serde_json::from_str::<Value>(mappings) else {
        return None;
    };
    let matched: Vec<u16> = groups
        .iter()
        .filter_map(|group| {
            let stripped = group.trim_start_matches('/');
            let last = stripped.rsplit('/').next().unwrap_or(stripped);
            [group.as_str(), stripped, last]
                .into_iter()
                .find_map(|name| mappings.get(name))
                .and_then(level_of)
        })
        .collect();
    match mode {
        MappingMode::First => matched.first().copied(),
        MappingMode::HighestPrivilege => matched.iter().min().copied(),
        MappingMode::LowestPrivilege => matched.iter().max().copied(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{MappingMode, Person, groups_of, mapped_level};

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn groups_come_from_a_list_a_text_or_the_keys_of_an_object() {
        let claims = json!({
            "groups": ["admins", 7, {"x": 1}],
            "one": "users",
            "urn:zitadel:iam:org:project:roles": {"admin": {"1": "org"}, "user": {}},
        });
        let claims = claims.as_object().expect("an object");
        assert_eq!(groups_of(claims, "groups", false), names(&["admins", "7"]));
        assert_eq!(groups_of(claims, "one", false), names(&["users"]));
        assert_eq!(
            groups_of(claims, "urn:zitadel:iam:org:project:roles", true),
            names(&["admin", "user"])
        );
        assert_eq!(
            groups_of(claims, "urn:zitadel:iam:org:project:roles", false),
            Vec::<String>::new()
        );
        assert_eq!(groups_of(claims, "missing", false), Vec::<String>::new());
    }

    #[test]
    fn mapping_takes_the_first_the_highest_or_the_lowest_privilege() {
        let mappings = r#"{"users": 4, "power": "3", "admins": 0}"#;
        let groups = names(&["users", "admins", "power"]);
        assert_eq!(mapped_level(&groups, mappings, MappingMode::First), Some(4));
        assert_eq!(
            mapped_level(&groups, mappings, MappingMode::HighestPrivilege),
            Some(0)
        );
        assert_eq!(
            mapped_level(&groups, mappings, MappingMode::LowestPrivilege),
            Some(4)
        );
        assert_eq!(MappingMode::from_setting("anything"), MappingMode::First);
    }

    #[test]
    fn mapping_matches_without_the_slash_and_by_the_last_segment() {
        let mappings = r#"{"staff": 2, "team/leads": 1}"#;
        assert_eq!(
            mapped_level(&names(&["/team/leads"]), mappings, MappingMode::First),
            Some(1)
        );
        assert_eq!(
            mapped_level(&names(&["/org/staff"]), mappings, MappingMode::First),
            Some(2)
        );
    }

    #[test]
    fn mapping_leaves_out_values_that_are_no_level_and_bad_settings() {
        // PHP turns the text "admin" into 0, the level of the Admin.
        let mappings = r#"{"a": "admin", "b": -1, "c": 1.5, "d": 5000}"#;
        let groups = names(&["a", "b", "c", "d"]);
        assert_eq!(mapped_level(&groups, mappings, MappingMode::First), None);
        assert_eq!(mapped_level(&groups, "not json", MappingMode::First), None);
        assert_eq!(mapped_level(&groups, "[1, 2]", MappingMode::First), None);
    }

    #[test]
    fn username_is_the_first_name_that_is_not_empty() {
        let mut person = Person {
            subject: "sub-1".to_owned(),
            preferred_username: Some(" ".to_owned()),
            name: Some("Robin".to_owned()),
            ..Person::default()
        };
        assert_eq!(person.username(), Some("Robin"));
        person.name = None;
        assert_eq!(person.username(), Some("sub-1"));
        person.preferred_username = Some("robin".to_owned());
        assert_eq!(person.username(), Some("robin"));
    }
}
