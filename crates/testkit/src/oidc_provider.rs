//! A stand-in for an OpenID Connect provider, for the tests of the login.
//!
//! It is a small web server on a port of this machine. It serves the discovery
//! document, the key set, the login page (which approves every login at once and sends
//! the browser back with a code), the token endpoint and the user info. It signs its ID
//! tokens with an Ed25519 key that it makes when it starts. A test can make it answer
//! wrongly in each of the ways that the login must refuse.

#![allow(
    clippy::indexing_slicing,
    clippy::struct_excessive_bools,
    reason = "a stand-in for a test reads JSON by key, and each switch is one way to answer wrongly"
)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{RawQuery, State};
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use openidconnect::JsonWebKeyId;
use openidconnect::PrivateSigningKey as _;
use openidconnect::core::{CoreEdDsaPrivateSigningKey, CoreJwsSigningAlgorithm};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use crate::services::StandIn;

/// The client of Organizr at the stand-in.
pub const CLIENT_ID: &str = "organizr-client";
/// Its secret.
pub const CLIENT_SECRET: &str = "the secret of the organizr client";

/// What the stand-in does wrong, and the person it vouches for. A test changes it with
/// [`OidcProvider::set`].
#[derive(Debug, Clone)]
pub struct Behavior {
    /// The claims of the person, for the ID token and the user info: `sub`,
    /// `preferred_username`, `email`, `groups` and others.
    pub person: Map<String, Value>,
    /// Whether the discovery document names a user info endpoint.
    pub user_info: bool,
    /// Another issuer in the ID token.
    pub issuer: Option<String>,
    /// Another audience in the ID token.
    pub audience: Option<String>,
    /// Another nonce in the ID token.
    pub nonce: Option<String>,
    /// The ID token expired an hour ago.
    pub expired: bool,
    /// The ID token is signed with another key of the same id.
    pub bad_signature: bool,
    /// The token endpoint refuses the code.
    pub token_error: bool,
}

impl Default for Behavior {
    fn default() -> Self {
        let person = json!({
            "sub": "subject-1",
            "preferred_username": "robin",
            "email": "robin@example.com",
            "email_verified": true,
            "groups": ["users"],
        });
        Self {
            person: person.as_object().cloned().unwrap_or_default(),
            user_info: true,
            issuer: None,
            audience: None,
            nonce: None,
            expired: false,
            bad_signature: false,
            token_error: false,
        }
    }
}

/// A login that the login page approved, until the code comes to the token endpoint.
#[derive(Debug, Clone)]
struct Pending {
    nonce: String,
    challenge: String,
    redirect: String,
}

struct Inner {
    url: String,
    key: CoreEdDsaPrivateSigningKey,
    other_key: CoreEdDsaPrivateSigningKey,
    behavior: Behavior,
    pending: HashMap<String, Pending>,
    /// What the login page got last: the query of the address.
    authorized: Vec<HashMap<String, String>>,
    /// The bodies of the requests to the token endpoint.
    token_requests: Vec<HashMap<String, String>>,
}

type Shared = Arc<Mutex<Inner>>;

/// A running stand-in provider. It stops when the value is dropped.
pub struct OidcProvider {
    stand_in: StandIn,
    routes: Router,
    shared: Shared,
}

impl std::fmt::Debug for OidcProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("OidcProvider")
    }
}

/// A new Ed25519 key, with the id that the key set names.
fn new_key(id: &str) -> CoreEdDsaPrivateSigningKey {
    let mut seed = [0_u8; 32];
    getrandom::fill(&mut seed).expect("random bytes");
    // The PKCS #8 form of an Ed25519 private key: a fixed start, then the 32 bytes.
    let mut der = vec![
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
        0x20,
    ];
    der.extend_from_slice(&seed);
    let pem = format!(
        "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
        STANDARD.encode(der)
    );
    CoreEdDsaPrivateSigningKey::from_ed25519_pem(&pem, Some(JsonWebKeyId::new(id.to_owned())))
        .expect("the key can be read")
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}

fn query_map(query: Option<&str>) -> HashMap<String, String> {
    form_urlencoded::parse(query.unwrap_or_default().as_bytes())
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}

impl OidcProvider {
    /// Starts a stand-in that approves every login of the person of
    /// [`Behavior::default`].
    pub async fn start() -> Self {
        let shared: Shared = Arc::new(Mutex::new(Inner {
            url: String::new(),
            key: new_key("key-1"),
            other_key: new_key("key-1"),
            behavior: Behavior::default(),
            pending: HashMap::new(),
            authorized: Vec::new(),
            token_requests: Vec::new(),
        }));
        let routes = Router::new()
            .route("/.well-known/openid-configuration", get(discovery))
            .route("/jwks", get(keys))
            .route("/authorize", get(authorize))
            .route("/token", post(token))
            .route("/userinfo", get(user_info))
            .with_state(Arc::clone(&shared));
        let stand_in = StandIn::start(routes.clone()).await;
        shared.lock().expect("the state can be locked").url = stand_in.url();
        Self {
            stand_in,
            routes,
            shared,
        }
    }

    /// The address of the discovery document.
    pub fn discovery_url(&self) -> String {
        format!("{}/.well-known/openid-configuration", self.stand_in.url())
    }

    /// The issuer: the address of the stand-in.
    pub fn issuer(&self) -> String {
        self.stand_in.url()
    }

    /// Changes what the stand-in does.
    pub fn set(&self, change: impl FnOnce(&mut Behavior)) {
        change(
            &mut self
                .shared
                .lock()
                .expect("the state can be locked")
                .behavior,
        );
    }

    /// The query of the last request to the login page.
    pub fn last_authorize(&self) -> HashMap<String, String> {
        self.shared
            .lock()
            .expect("the state can be locked")
            .authorized
            .last()
            .cloned()
            .unwrap_or_default()
    }

    /// The bodies of the requests to the token endpoint, oldest first.
    pub fn token_requests(&self) -> Vec<HashMap<String, String>> {
        self.shared
            .lock()
            .expect("the state can be locked")
            .token_requests
            .clone()
    }

    /// Sends the browser to the login page at `address`, which the authorize route of
    /// Organizr named, and returns where the stand-in sends the browser back: the path
    /// and the query of the callback, below the host.
    pub async fn approve(&self, address: &str) -> String {
        let path = address
            .strip_prefix(&self.stand_in.url())
            .unwrap_or_else(|| panic!("the address is not one of the stand-in: {address}"));
        let request = Request::builder()
            .uri(path)
            .body(Body::empty())
            .expect("the request is valid");
        let response = self
            .routes
            .clone()
            .oneshot(request)
            .await
            .expect("the stand-in answers");
        assert_eq!(response.status(), StatusCode::FOUND);
        let back = response.headers()[header::LOCATION]
            .to_str()
            .expect("the address is text")
            .to_owned();
        let after_scheme = back
            .split_once("://")
            .map_or(back.as_str(), |(_, rest)| rest);
        after_scheme
            .find('/')
            .map_or_else(|| "/".to_owned(), |at| after_scheme[at..].to_owned())
    }
}

async fn discovery(State(shared): State<Shared>) -> Response {
    let inner = shared.lock().expect("the state can be locked");
    let url = &inner.url;
    let mut document = json!({
        "issuer": url,
        "authorization_endpoint": format!("{url}/authorize"),
        "token_endpoint": format!("{url}/token"),
        "jwks_uri": format!("{url}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["EdDSA"],
        "code_challenge_methods_supported": ["S256"],
    });
    if inner.behavior.user_info {
        document["userinfo_endpoint"] = json!(format!("{url}/userinfo"));
    }
    axum::Json(document).into_response()
}

async fn keys(State(shared): State<Shared>) -> Response {
    let inner = shared.lock().expect("the state can be locked");
    let key = inner.key.as_verification_key();
    axum::Json(json!({ "keys": [key] })).into_response()
}

/// The login page: approves the login at once, and sends the browser back.
async fn authorize(State(shared): State<Shared>, RawQuery(query): RawQuery) -> Response {
    let fields = query_map(query.as_deref());
    let mut inner = shared.lock().expect("the state can be locked");
    inner.authorized.push(fields.clone());
    let mut code = [0_u8; 16];
    getrandom::fill(&mut code).expect("random bytes");
    let code = URL_SAFE_NO_PAD.encode(code);
    let redirect = fields.get("redirect_uri").cloned().unwrap_or_default();
    inner.pending.insert(
        code.clone(),
        Pending {
            nonce: fields.get("nonce").cloned().unwrap_or_default(),
            challenge: fields.get("code_challenge").cloned().unwrap_or_default(),
            redirect: redirect.clone(),
        },
    );
    let state = fields.get("state").cloned().unwrap_or_default();
    let back: String = form_urlencoded::Serializer::new(String::new())
        .append_pair("code", &code)
        .append_pair("state", &state)
        .finish();
    (
        StatusCode::FOUND,
        [(header::LOCATION, format!("{redirect}?{back}"))],
    )
        .into_response()
}

fn signed(key: &CoreEdDsaPrivateSigningKey, claims: &Value) -> String {
    let header = json!({"alg": "EdDSA", "kid": "key-1", "typ": "JWT"});
    let message = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header.to_string()),
        URL_SAFE_NO_PAD.encode(claims.to_string())
    );
    let signature = key
        .sign(&CoreJwsSigningAlgorithm::EdDsa, message.as_bytes())
        .expect("the key signs");
    format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature))
}

fn token_error(error: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(json!({ "error": error })),
    )
        .into_response()
}

/// The token endpoint: trades a code of the login page for an access token and an ID
/// token, after the checks of PKCE, the redirect address and the client.
async fn token(State(shared): State<Shared>, body: Bytes) -> Response {
    let fields: HashMap<String, String> = form_urlencoded::parse(&body)
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    let mut inner = shared.lock().expect("the state can be locked");
    inner.token_requests.push(fields.clone());
    if inner.behavior.token_error {
        return token_error("invalid_grant");
    }
    let field = |name: &str| fields.get(name).map_or("", String::as_str);
    if field("client_id") != CLIENT_ID || field("client_secret") != CLIENT_SECRET {
        return token_error("invalid_client");
    }
    let Some(pending) = inner.pending.remove(field("code")) else {
        return token_error("invalid_grant");
    };
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(field("code_verifier").as_bytes()));
    if field("grant_type") != "authorization_code"
        || challenge != pending.challenge
        || field("redirect_uri") != pending.redirect
    {
        return token_error("invalid_grant");
    }

    let behavior = &inner.behavior;
    let issued = now();
    let mut claims = Value::Object(behavior.person.clone());
    claims["iss"] = json!(behavior.issuer.clone().unwrap_or_else(|| inner.url.clone()));
    claims["aud"] = json!(behavior.audience.as_deref().unwrap_or(CLIENT_ID));
    claims["iat"] = json!(if behavior.expired {
        issued - 7200
    } else {
        issued
    });
    claims["exp"] = json!(if behavior.expired {
        issued - 3600
    } else {
        issued + 300
    });
    claims["nonce"] = json!(behavior.nonce.clone().unwrap_or(pending.nonce));
    let key = if behavior.bad_signature {
        &inner.other_key
    } else {
        &inner.key
    };
    let id_token = signed(key, &claims);
    axum::Json(json!({
        "access_token": "an access token of the stand-in",
        "token_type": "Bearer",
        "expires_in": 300,
        "id_token": id_token,
    }))
    .into_response()
}

/// The user info of the person, for the access token of the stand-in.
async fn user_info(State(shared): State<Shared>, headers: HeaderMap) -> Response {
    let authorized = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        == Some("Bearer an access token of the stand-in");
    if !authorized {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let inner = shared.lock().expect("the state can be locked");
    axum::Json(Value::Object(inner.behavior.person.clone())).into_response()
}
