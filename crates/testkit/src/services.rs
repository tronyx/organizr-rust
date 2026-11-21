//! Stand-ins for the services that Organizr talks to: the account service of Plex, Ombi,
//! Overseerr and Tautulli.
//!
//! Each stand-in is a small web server on a port of this machine. It answers the
//! requests that Organizr sends in the form of the real service, as its API documents
//! and the PHP application describe it, and it keeps every request for the test. All
//! accounts and tokens are made up.

#![allow(
    clippy::indexing_slicing,
    clippy::format_push_string,
    reason = "a stand-in for a test reads JSON by key and builds its answers as text"
)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde_json::{Value, json};
use tokio::net::TcpListener;

/// One request that a stand-in got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    /// The method.
    pub method: String,
    /// The path, with the query.
    pub path: String,
    /// The headers, with their names in lower case.
    pub headers: HashMap<String, String>,
    /// The body as text.
    pub body: String,
}

impl Seen {
    /// A header of the request.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }

    /// The body as JSON.
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }

    /// The body as the fields of a form.
    pub fn form(&self) -> HashMap<String, String> {
        form_urlencoded_pairs(&self.body)
    }
}

fn form_urlencoded_pairs(body: &str) -> HashMap<String, String> {
    body.split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(name, value)| (decoded(name), decoded(value)))
        .collect()
}

/// Decodes a part of a form: `+` is a space, and `%xx` is a byte.
fn decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(byte) = bytes.get(index) {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (byte, hex) {
            (b'%', Some(value)) => {
                result.push(value);
                index += 3;
            }
            (b'+', _) => {
                result.push(b' ');
                index += 1;
            }
            _ => {
                result.push(*byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}

type Log = Arc<Mutex<Vec<Seen>>>;

/// A running stand-in. It stops when the value is dropped.
#[derive(Debug)]
pub struct StandIn {
    address: SocketAddr,
    seen: Log,
    task: tokio::task::JoinHandle<()>,
}

impl StandIn {
    /// Starts a server with the given routes.
    pub async fn start(routes: Router) -> Self {
        Self::start_on(routes, None).await
    }

    /// Starts a server with the given routes over HTTPS, with the certificate of
    /// [`crate::tls::LOCALHOST_CERTIFICATE`] for the name `localhost`, which the
    /// authority of [`crate::tls::AUTHORITY`] signed. [`StandIn::https_url`] is its
    /// address.
    pub async fn start_tls(routes: Router) -> Self {
        Self::start_on(routes, Some(crate::tls::acceptor())).await
    }

    async fn start_on(routes: Router, tls: Option<tokio_rustls::TlsAcceptor>) -> Self {
        let seen = Log::default();
        let log = Arc::clone(&seen);
        let app = routes.layer(axum::middleware::from_fn(
            move |request: Request, next: Next| {
                let log = Arc::clone(&log);
                async move {
                    let (parts, body) = request.into_parts();
                    let bytes = axum::body::to_bytes(body, 1024 * 1024)
                        .await
                        .unwrap_or_default();
                    let headers = parts
                        .headers
                        .iter()
                        .map(|(name, value)| {
                            (
                                name.as_str().to_owned(),
                                value.to_str().unwrap_or_default().to_owned(),
                            )
                        })
                        .collect();
                    log.lock().expect("the log can be locked").push(Seen {
                        method: parts.method.to_string(),
                        path: parts
                            .uri
                            .path_and_query()
                            .map_or_else(String::new, ToString::to_string),
                        headers,
                        body: String::from_utf8_lossy(&bytes).into_owned(),
                    });
                    next.run(Request::from_parts(parts, axum::body::Body::from(bytes)))
                        .await
                }
            },
        ));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a local port can be opened");
        let address = listener.local_addr().expect("the port has an address");
        let task = tokio::spawn(async move {
            match tls {
                Some(acceptor) => {
                    let _ =
                        axum::serve(crate::tls::TlsListener::new(listener, acceptor), app).await;
                }
                None => {
                    let _ = axum::serve(listener, app).await;
                }
            }
        });
        Self {
            address,
            seen,
            task,
        }
    }

    /// The address of the stand-in, without a slash at the end.
    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    /// The address of a stand-in over HTTPS, by the name `localhost`, without a slash at
    /// the end.
    pub fn https_url(&self) -> String {
        format!("https://localhost:{}", self.address.port())
    }

    /// The requests so far, oldest first.
    pub fn seen(&self) -> Vec<Seen> {
        self.seen.lock().expect("the log can be locked").clone()
    }
}

impl Drop for StandIn {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// A Plex account of the stand-in for `plex.tv`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexUser {
    /// The user name.
    pub username: &'static str,
    /// The e-mail address.
    pub email: &'static str,
    /// The password.
    pub password: &'static str,
    /// The token of the account.
    pub token: &'static str,
    /// The servers of the owner that are shared with the account. An account without
    /// any is no friend of the owner.
    pub shared_servers: &'static [&'static str],
}

/// The token of the owner of the Plex server in the stand-in.
pub const OWNER_TOKEN: &str = "made-up-token-of-the-owner";

/// The id of the Plex server of the owner.
pub const SERVER_ID: &str = "made-up-server-one";

/// The accounts of the stand-in: the owner, two friends, a stranger, and a stranger
/// whose account has no e-mail address.
pub const PLEX_USERS: [PlexUser; 5] = [
    PlexUser {
        username: "Owner",
        email: "owner@example.com",
        password: "the password of the owner",
        token: OWNER_TOKEN,
        shared_servers: &[],
    },
    PlexUser {
        username: "Robin",
        email: "robin@example.com",
        password: "the plex password of robin",
        token: "made-up-token-of-robin",
        shared_servers: &[SERVER_ID],
    },
    PlexUser {
        username: "pat",
        email: "Pat@Example.com",
        password: "the plex password of pat",
        token: "made-up-token-of-pat",
        shared_servers: &["made-up-server-two"],
    },
    PlexUser {
        username: "stranger",
        email: "stranger@example.com",
        password: "the plex password of a stranger",
        token: "made-up-token-of-a-stranger",
        shared_servers: &[],
    },
    PlexUser {
        username: "nomail",
        email: "",
        password: "the plex password of nomail",
        token: "made-up-token-of-nomail",
        shared_servers: &[],
    },
];

fn plex_account(user: &PlexUser) -> Value {
    json!({
        "user": {
            "id": 1,
            "uuid": "made-up",
            "username": user.username,
            "title": user.username,
            "email": user.email,
            "thumb": format!("https://plex.tv/users/{}/avatar", user.username.to_lowercase()),
            "authToken": user.token,
            "authentication_token": user.token,
        }
    })
}

async fn plex_sign_in(body: Bytes) -> Response {
    let form = form_urlencoded_pairs(&String::from_utf8_lossy(&body));
    let login = form.get("user[login]").map(|login| login.to_lowercase());
    let password = form.get("user[password]");
    let found = PLEX_USERS.iter().find(|user| {
        let is_user = login.as_deref() == Some(&user.username.to_lowercase())
            || login.as_deref() == Some(&user.email.to_lowercase());
        is_user && password.map(String::as_str) == Some(user.password)
    });
    match found {
        Some(user) => (StatusCode::CREATED, axum::Json(plex_account(user))).into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({"error": "Invalid email, username, or password."})),
        )
            .into_response(),
    }
}

fn plex_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-plex-token")
        .and_then(|value| value.to_str().ok())
}

async fn plex_account_of_token(headers: HeaderMap) -> Response {
    match PLEX_USERS
        .iter()
        .find(|user| plex_token(&headers) == Some(user.token))
    {
        Some(user) => axum::Json(plex_account(user)).into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({"error": "Invalid authentication token."})),
        )
            .into_response(),
    }
}

fn xml(text: String) -> Response {
    ([(header::CONTENT_TYPE, "application/xml")], text).into_response()
}

async fn plex_friends(headers: HeaderMap) -> Response {
    if plex_token(&headers) != Some(OWNER_TOKEN) {
        return (
            StatusCode::UNAUTHORIZED,
            "<errors><error>Invalid token</error></errors>",
        )
            .into_response();
    }
    let mut users = String::new();
    for user in PLEX_USERS
        .iter()
        .filter(|user| !user.shared_servers.is_empty())
    {
        users.push_str(&format!(
            "  <User id=\"1\" title=\"{0}\" username=\"{0}\" email=\"{1}\" home=\"0\">\n",
            user.username, user.email
        ));
        for server in user.shared_servers {
            users.push_str(&format!(
                "    <Server id=\"1\" machineIdentifier=\"{server}\" name=\"Media\" owned=\"0\"/>\n"
            ));
        }
        users.push_str("  </User>\n");
    }
    xml(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<MediaContainer friendlyName=\"myPlex\" size=\"2\">\n{users}</MediaContainer>\n"
    ))
}

async fn plex_servers(headers: HeaderMap) -> Response {
    if plex_token(&headers) != Some(OWNER_TOKEN) {
        return (
            StatusCode::UNAUTHORIZED,
            "<errors><error>Invalid token</error></errors>",
        )
            .into_response();
    }
    xml(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<MediaContainer friendlyName=\"myPlex\" size=\"2\">\n  \
         <Server name=\"Media\" address=\"203.0.113.5\" port=\"32400\" machineIdentifier=\"{SERVER_ID}\" owned=\"1\"/>\n  \
         <Server name=\"Shared\" address=\"198.51.100.9\" port=\"32400\" machineIdentifier=\"made-up-server-nine\" owned=\"0\"/>\n\
         </MediaContainer>\n"
    ))
}

/// The share of a server with an account. The stand-in knows the server of the owner,
/// and refuses an account that has that server.
async fn plex_share(
    Path(server): Path<String>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<Value>,
) -> Response {
    if plex_token(&headers) != Some(OWNER_TOKEN) {
        return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
    }
    let libraries = &body["shared_server"]["library_section_ids"];
    let unknown_library = libraries
        .as_array()
        .is_some_and(|ids| ids.iter().any(|id| id == "404"));
    if server != SERVER_ID || unknown_library {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    }
    let invited = body["shared_server"]["invited_email"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();
    if invited == "broken" {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Trouble").into_response();
    }
    let has_access = PLEX_USERS.iter().any(|user| {
        user.shared_servers.contains(&SERVER_ID)
            && (user.username.to_lowercase() == invited || user.email.to_lowercase() == invited)
    });
    if has_access {
        return (StatusCode::BAD_REQUEST, "Already shared").into_response();
    }
    axum::Json(json!({"id": 7, "invited_email": invited})).into_response()
}

/// Starts a stand-in for `plex.tv` with the accounts of [`PLEX_USERS`].
pub async fn plex_tv() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/users/sign_in.json", post(plex_sign_in))
            .route("/users/account.json", get(plex_account_of_token))
            .route("/api/users", get(plex_friends))
            .route("/pms/servers", get(plex_servers))
            .route("/api/servers/{server}/shared_servers/", post(plex_share)),
    )
    .await
}

/// The Plex account of a token that an application of the stand-ins knows. The owner
/// and the friends have an account in each application. The stranger has none.
fn known_by_token(token: Option<&str>) -> Option<&'static PlexUser> {
    PLEX_USERS
        .iter()
        .filter(|user| user.username != "stranger")
        .find(|user| token == Some(user.token))
}

/// The local account of the applications: the name, the e-mail address and the password.
pub const LOCAL_ACCOUNT: (&str, &str, &str) = (
    "fallback",
    "fallback@example.com",
    "the password of the fallback account",
);

fn is_local_account(name: Option<&str>, password: Option<&str>) -> bool {
    (name == Some(LOCAL_ACCOUNT.0) || name == Some(LOCAL_ACCOUNT.1))
        && password == Some(LOCAL_ACCOUNT.2)
}

async fn ombi_login(axum::Json(body): axum::Json<Value>) -> Response {
    if is_local_account(body["username"].as_str(), body["password"].as_str()) {
        axum::Json(
            json!({"access_token": "ombi-token-of-fallback", "expiration": "2031-01-01T00:00:00Z"}),
        )
        .into_response()
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

async fn ombi_plex_login(axum::Json(body): axum::Json<Value>) -> Response {
    match known_by_token(body["plexToken"].as_str()) {
        Some(user) => axum::Json(json!({
            "access_token": format!("ombi-token-of-{}", user.username.to_lowercase()),
            "expiration": "2031-01-01T00:00:00Z",
        }))
        .into_response(),
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}

/// Starts a stand-in for Ombi.
pub async fn ombi() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/api/v1/Token", post(ombi_login))
            .route("/api/v1/Token/plextoken", post(ombi_plex_login)),
    )
    .await
}

fn overseerr_session(name: &str) -> Response {
    // The real application signs its session id. The value holds characters that a
    // cookie writes with `%`.
    let cookie = format!(
        "connect.sid=s%3Aoverseerr-session-of-{name}.sig%2Bnature; Path=/; Expires=Wed, 01 Jan 2031 00:00:00 GMT; HttpOnly; SameSite=Lax"
    );
    (
        [(header::SET_COOKIE, cookie)],
        axum::Json(json!({"id": 1, "email": format!("{name}@example.com"), "plexUsername": name})),
    )
        .into_response()
}

async fn overseerr_plex_login(axum::Json(body): axum::Json<Value>) -> Response {
    match known_by_token(body["authToken"].as_str()) {
        Some(user) => overseerr_session(&user.username.to_lowercase()),
        None => (
            StatusCode::FORBIDDEN,
            axum::Json(json!({"message": "Access denied."})),
        )
            .into_response(),
    }
}

async fn overseerr_local_login(axum::Json(body): axum::Json<Value>) -> Response {
    if is_local_account(body["email"].as_str(), body["password"].as_str()) {
        overseerr_session("fallback")
    } else {
        (
            StatusCode::FORBIDDEN,
            axum::Json(json!({"message": "Access denied."})),
        )
            .into_response()
    }
}

/// Starts a stand-in for Overseerr.
pub async fn overseerr() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/api/v1/auth/plex", post(overseerr_plex_login))
            .route("/api/v1/auth/local", post(overseerr_local_login)),
    )
    .await
}

async fn tautulli_login(State(uuid): State<&'static str>, body: Bytes) -> Response {
    let form = form_urlencoded_pairs(&String::from_utf8_lossy(&body));
    let field = |name: &str| {
        form.get(name)
            .map(String::as_str)
            .filter(|text| !text.is_empty())
    };
    let name = match known_by_token(field("token")) {
        Some(user) => Some(user.username.to_lowercase()),
        None if is_local_account(field("username"), field("password")) => {
            Some(LOCAL_ACCOUNT.0.to_owned())
        }
        None => None,
    };
    match name {
        Some(name) => axum::Json(json!({
            "status": "success",
            "token": format!("tautulli-session-of-{name}"),
            "uuid": uuid,
        }))
        .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({"status": "error", "message": "Invalid login."})),
        )
            .into_response(),
    }
}

/// Starts a stand-in for a Tautulli install with the given id, at the path `prefix`
/// (empty for the root of the host).
pub async fn tautulli(uuid: &'static str, prefix: &str) -> StandIn {
    StandIn::start(
        Router::new()
            .route(&format!("{prefix}/auth/signin"), post(tautulli_login))
            .with_state(uuid),
    )
    .await
}
