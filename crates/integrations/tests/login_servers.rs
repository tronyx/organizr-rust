//! The LDAP, FTP, Emby and Jellyfin back ends of the login against stand-ins for the
//! servers: what goes over the wire, and what the answer means.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use organizr_integrations::ftp::{self, FtpFailure, FtpServer};
use organizr_integrations::http::Outbound;
use organizr_integrations::ldap::{self, Directory, LdapError, Login};
use organizr_integrations::media_login::{self, ConnectAccount};
use organizr_integrations::tls;
use organizr_testkit::login_servers::{
    self, FTP_USERS, LDAP_PEOPLE, LDAP_SERVICE, closed_port, ldap_server,
};
use organizr_testkit::services::StandIn;
use serde_json::{Value, json};

const TIME: Duration = Duration::from_secs(5);

fn directory<'a>(hosts: &'a [String], port: u16, bind: (&'a str, &'a str)) -> Directory<'a> {
    Directory {
        hosts,
        port,
        ssl: false,
        start_tls: false,
        bind,
        timeout: TIME,
        tls: tls::client_config().expect("the TLS settings can be made"),
    }
}

fn local() -> Vec<String> {
    vec!["127.0.0.1".to_owned()]
}

#[tokio::test]
async fn ldap_login_binds_as_the_application_then_as_the_person_and_reads_the_mail() {
    let server = ldap_server(false).await;
    let hosts = local();
    let directory = directory(&hosts, server.port(), LDAP_SERVICE);
    let robin = &LDAP_PEOPLE[0];

    let login = ldap::log_in(&directory, robin.dn, robin.password).await;

    assert_eq!(
        login,
        Ok(Login::Accepted {
            email: Some("robin@ldap.example.com".to_owned())
        })
    );
    // The unbind goes out after the answer, so it may not have arrived yet.
    assert_eq!(
        server.seen()[..4],
        [
            format!("bind {}", LDAP_SERVICE.0),
            format!("bind {}", robin.dn),
            format!("bind {}", LDAP_SERVICE.0),
            format!("search {}", robin.dn),
        ]
    );
}

#[tokio::test]
async fn ldap_login_without_mail_has_no_address() {
    let server = ldap_server(false).await;
    let hosts = local();
    let directory = directory(&hosts, server.port(), LDAP_SERVICE);
    let kim = &LDAP_PEOPLE[1];

    let login = ldap::log_in(&directory, kim.dn, kim.password).await;
    assert_eq!(login, Ok(Login::Accepted { email: None }));

    // A name that the server takes, but that is no entry, gives no address either.
    let server = ldap_server(true).await;
    let directory = Directory {
        port: server.port(),
        bind: ("", ""),
        ..directory
    };
    assert_eq!(
        ldap::log_in(&directory, LDAP_SERVICE.0, LDAP_SERVICE.1).await,
        Ok(Login::Accepted { email: None })
    );
    assert_eq!(server.seen()[0], "bind ");
}

#[tokio::test]
async fn ldap_login_with_a_wrong_or_empty_password_is_refused() {
    let server = ldap_server(true).await;
    let hosts = local();
    let directory = directory(&hosts, server.port(), LDAP_SERVICE);
    let robin = &LDAP_PEOPLE[0];

    assert_eq!(
        ldap::log_in(&directory, robin.dn, "wrong").await,
        Ok(Login::Refused)
    );
    let before = server.seen().len();
    // An empty password would be a bind without a name, which this server takes: it
    // is not sent at all.
    assert_eq!(
        ldap::log_in(&directory, robin.dn, "").await,
        Ok(Login::Refused)
    );
    assert_eq!(server.seen().len(), before);
}

#[tokio::test]
async fn ldap_bind_account_that_the_server_refuses_fails_the_login_and_the_test() {
    let server = ldap_server(false).await;
    let hosts = local();
    let robin = &LDAP_PEOPLE[0];

    let wrong = directory(&hosts, server.port(), (LDAP_SERVICE.0, "wrong"));
    assert_eq!(
        ldap::log_in(&wrong, robin.dn, robin.password).await,
        Err(LdapError::Refused(49))
    );
    assert_eq!(
        ldap::test_connection(&wrong)
            .await
            .map_err(|error| error.to_string()),
        Err("Invalid credentials".to_owned())
    );
    // Without a name, the server of this test refuses the anonymous bind.
    let anonymous = directory(&hosts, server.port(), ("", ""));
    assert_eq!(
        ldap::test_connection(&anonymous).await,
        Err(LdapError::Refused(48))
    );
    // A name without a password is not sent.
    let no_password = directory(&hosts, server.port(), (LDAP_SERVICE.0, ""));
    assert_eq!(
        ldap::test_connection(&no_password).await,
        Err(LdapError::NoBindPassword)
    );
    assert_eq!(
        ldap::test_connection(&directory(&hosts, server.port(), LDAP_SERVICE)).await,
        Ok(())
    );
}

#[tokio::test]
async fn ldap_test_login_says_whether_the_person_binds() {
    let server = ldap_server(false).await;
    let hosts = local();
    let directory = directory(&hosts, server.port(), LDAP_SERVICE);
    let robin = &LDAP_PEOPLE[0];
    assert_eq!(
        ldap::test_login(&directory, robin.dn, robin.password).await,
        Ok(true)
    );
    assert_eq!(
        ldap::test_login(&directory, robin.dn, "wrong").await,
        Ok(false)
    );
}

#[tokio::test]
async fn ldap_tries_the_hosts_in_turn_and_says_when_none_answers() {
    let server = ldap_server(false).await;
    let robin = &LDAP_PEOPLE[0];
    // The first host has nothing on the port; the second one is the server.
    let hosts = vec!["127.0.0.2".to_owned(), "127.0.0.1".to_owned()];
    let both = Directory {
        timeout: Duration::from_secs(2),
        ..directory(&hosts, server.port(), LDAP_SERVICE)
    };
    assert!(matches!(
        ldap::log_in(&both, robin.dn, robin.password).await,
        Ok(Login::Accepted { .. })
    ));

    let closed = closed_port().await;
    let hosts = local();
    let nobody = directory(&hosts, closed, LDAP_SERVICE);
    let error = ldap::log_in(&nobody, robin.dn, robin.password)
        .await
        .expect_err("nobody answers");
    assert_eq!(error.to_string(), "Can't contact LDAP server");
}

fn ftp<'a>(port: u16, secure: bool) -> FtpServer<'a> {
    FtpServer {
        host: "127.0.0.1",
        port,
        secure,
        timeout: TIME,
        tls: tls::client_config().expect("the TLS settings can be made"),
    }
}

#[tokio::test]
async fn ftp_login_sends_user_pass_and_quit() {
    let server = login_servers::ftp_server().await;
    let (user, password) = FTP_USERS[0];

    assert_eq!(
        ftp::log_in(&ftp(server.port(), false), user, password).await,
        Ok(true)
    );
    assert_eq!(
        server.seen(),
        [
            format!("USER {user}"),
            format!("PASS {password}"),
            "QUIT".to_owned()
        ]
    );
    assert_eq!(
        ftp::log_in(&ftp(server.port(), false), user, "wrong").await,
        Ok(false)
    );
    assert_eq!(server.seen().last().map(String::as_str), Some("QUIT"));
}

#[tokio::test]
async fn ftp_refuses_an_empty_password_and_a_line_break_without_asking() {
    let server = login_servers::ftp_server().await;
    let port = server.port();
    assert_eq!(ftp::log_in(&ftp(port, false), "robin", "").await, Ok(false));
    assert_eq!(
        ftp::log_in(&ftp(port, false), "robin\r\nDELE x", "pw").await,
        Ok(false)
    );
    assert_eq!(server.seen(), Vec::<String>::new());
}

#[tokio::test]
async fn ftp_refuses_the_names_of_anonymous_ftp_without_asking() {
    let server = login_servers::ftp_server().await;
    let port = server.port();
    for name in ["anonymous", "Anonymous", " FTP "] {
        assert_eq!(
            ftp::log_in(&ftp(port, false), name, "any@example.com").await,
            Ok(false)
        );
    }
    assert_eq!(server.seen(), Vec::<String>::new());
}

#[tokio::test]
async fn ftps_needs_tls_and_an_unreachable_server_is_an_error() {
    let server = login_servers::ftp_server().await;
    let (user, password) = FTP_USERS[0];
    assert_eq!(
        ftp::log_in(&ftp(server.port(), true), user, password).await,
        Err(FtpFailure::NoTls)
    );
    // The password never went to the server.
    assert_eq!(server.seen(), ["AUTH TLS"]);

    let closed = closed_port().await;
    assert_eq!(
        ftp::log_in(&ftp(closed, false), user, password).await,
        Err(FtpFailure::Unreachable)
    );
}

/// What a media server stand-in got: the path, the headers and the body.
type Seen = Arc<Mutex<Vec<(String, HeaderMap, String)>>>;

fn record(seen: &Seen, path: &str, headers: &HeaderMap, body: &str) {
    seen.lock()
        .expect("it locks")
        .push((path.to_owned(), headers.clone(), body.to_owned()));
}

/// A stand-in for an Emby or Jellyfin server: the user "robin" with the password
/// "robin-media-password", the user "nopass" without a password, and the users of the
/// key "the-admin-key".
async fn media_server(seen: Seen, login_path: &'static str, logout_works: bool) -> StandIn {
    let on_login = Arc::clone(&seen);
    let on_logout = Arc::clone(&seen);
    let on_users = Arc::clone(&seen);
    StandIn::start(
        Router::new()
            .route(
                login_path,
                post(move |headers: HeaderMap, body: String| {
                    record(&on_login, login_path, &headers, &body);
                    let sent: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                    let password = sent
                        .get("pw")
                        .or_else(|| sent.get("Pw"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    let user = sent["Username"].as_str().unwrap_or_default().to_owned();
                    async move {
                        match (user.as_str(), password.as_str()) {
                            ("robin", "robin-media-password") => (
                                StatusCode::OK,
                                axum::Json(json!({
                                    "SessionInfo": {"Id": "s1"},
                                    "User": {"Name": "robin", "HasPassword": true},
                                    "AccessToken": "session-token-of-robin",
                                })),
                            ),
                            ("nopass", _) => (
                                StatusCode::OK,
                                axum::Json(json!({
                                    "SessionInfo": {"Id": "s2"},
                                    "User": {"Name": "nopass", "HasPassword": false},
                                    "AccessToken": "session-token-of-nopass",
                                })),
                            ),
                            _ => (StatusCode::UNAUTHORIZED, axum::Json(Value::Null)),
                        }
                    }
                }),
            )
            .route(
                "/Sessions/Logout",
                post(move |headers: HeaderMap, body: String| {
                    record(&on_logout, "/Sessions/Logout", &headers, &body);
                    async move {
                        if logout_works {
                            StatusCode::NO_CONTENT
                        } else {
                            StatusCode::INTERNAL_SERVER_ERROR
                        }
                    }
                }),
            )
            .route(
                "/Users",
                get(move |headers: HeaderMap| {
                    record(&on_users, "/Users", &headers, "");
                    let allowed = headers
                        .get("x-emby-token")
                        .is_some_and(|value| value == "the-admin-key");
                    async move {
                        if !allowed {
                            return (StatusCode::UNAUTHORIZED, axum::Json(Value::Null));
                        }
                        (
                            StatusCode::OK,
                            axum::Json(json!([
                                {"Name": "robin", "ConnectUserName": "Robin@Connect.example.com", "ConnectLinkType": "LinkedUser"},
                                {"Name": "pat", "ConnectUserName": "pat-connect"},
                                {"Name": "zed"},
                            ])),
                        )
                    }
                }),
            ),
    )
    .await
}

fn outbound() -> Outbound {
    Outbound::new().expect("the client can be built")
}

#[tokio::test]
async fn emby_local_login_sends_what_php_sends_and_logs_the_session_out() {
    let seen = Seen::default();
    let emby = media_server(Arc::clone(&seen), "/Users/AuthenticateByName", true).await;
    let http = outbound();

    let ok = media_login::emby_local(&http, &emby.url(), "robin", "robin-media-password").await;
    assert!(ok.expect("the server answers"));

    let seen = seen.lock().expect("it locks").clone();
    let (path, headers, body) = &seen[0];
    assert_eq!(path, "/Users/AuthenticateByName");
    assert_eq!(
        headers["authorization"],
        "Emby UserId=\"e8837bc1-ad67-520e-8cd2-f629e3155721\", Client=\"None\", Device=\"Organizr\", DeviceId=\"xxx\", Version=\"1.0.0.0\""
    );
    let body: Value = serde_json::from_str(body).expect("JSON");
    assert_eq!(body["Username"], "robin");
    assert_eq!(body["pw"], "robin-media-password");
    // sha1("robin-media-password") and md5("robin-media-password").
    assert_eq!(body["Password"].as_str().map(str::len), Some(40));
    assert_eq!(body["PasswordMd5"].as_str().map(str::len), Some(32));
    let (path, headers, _) = &seen[1];
    assert_eq!(path, "/Sessions/Logout");
    assert_eq!(headers["x-emby-token"], "session-token-of-robin");
    assert_eq!(headers["x-mediabrowser-token"], "session-token-of-robin");
}

#[tokio::test]
async fn emby_local_refuses_a_user_without_a_password_and_a_failed_log_out() {
    let http = outbound();
    let seen = Seen::default();
    let emby = media_server(Arc::clone(&seen), "/Users/AuthenticateByName", true).await;
    assert!(
        !media_login::emby_local(&http, &emby.url(), "nopass", "x")
            .await
            .expect("answers")
    );
    assert!(
        !media_login::emby_local(&http, &emby.url(), "robin", "wrong")
            .await
            .expect("answers")
    );
    // No session was opened, so none was logged out.
    assert!(
        seen.lock()
            .expect("it locks")
            .iter()
            .all(|(path, ..)| path != "/Sessions/Logout")
    );

    let broken = media_server(Seen::default(), "/Users/AuthenticateByName", false).await;
    assert!(
        !media_login::emby_local(&http, &broken.url(), "robin", "robin-media-password")
            .await
            .expect("answers")
    );
}

#[tokio::test]
/// PHP sends the device in `X-Emby-Authorization`, which Jellyfin 12 refuses with
/// 400. The header `Authorization` carries the same text and works on every Jellyfin (D-99).
async fn jellyfin_login_sends_the_device_of_php_in_authorization() {
    let seen = Seen::default();
    let jellyfin = media_server(Arc::clone(&seen), "/Users/authenticatebyname", true).await;
    let http = outbound();

    let ok = media_login::jellyfin(&http, &jellyfin.url(), "robin", "robin-media-password").await;
    assert!(ok.expect("the server answers"));
    assert!(
        !media_login::jellyfin(&http, &jellyfin.url(), "robin", "wrong")
            .await
            .expect("answers")
    );

    let seen = seen.lock().expect("it locks").clone();
    let (_, headers, body) = &seen[0];
    let device = "MediaBrowser Client=\"Organizr Auth\", Device=\"Organizr\", DeviceId=\"orgv2\", Version=\"2.0\"";
    assert_eq!(headers["authorization"], device);
    assert!(headers.get("x-emby-authorization").is_none());
    assert_eq!(
        serde_json::from_str::<Value>(body).expect("JSON"),
        json!({"Username": "robin", "Pw": "robin-media-password"})
    );
    let (path, headers, _) = &seen[1];
    assert_eq!(path, "/Sessions/Logout");
    assert_eq!(
        headers["authorization"],
        format!("{device}, Token=\"session-token-of-robin\"")
    );
    assert!(headers.get("x-emby-authorization").is_none());
}

/// A stand-in for Emby Connect, with the account "Robin" of the given address.
async fn emby_connect(seen: Seen) -> StandIn {
    StandIn::start(Router::new().route(
        "/service/user/authenticate",
        post(move |headers: HeaderMap, body: String| {
            record(&seen, "/service/user/authenticate", &headers, &body);
            let fits = body == "nameOrEmail=robin&rawpw=robin-connect-password";
            async move {
                if fits {
                    (
                        StatusCode::OK,
                        axum::Json(json!({
                            "AccessToken": "connect-token",
                            "User": {"Name": "Robin", "Email": "robin@connect.example.com"},
                        })),
                    )
                } else {
                    (StatusCode::UNAUTHORIZED, axum::Json(Value::Null))
                }
            }
        }),
    ))
    .await
}

#[tokio::test]
async fn emby_connect_login_needs_a_linked_user_on_the_server() {
    let seen = Seen::default();
    let connect = emby_connect(Arc::clone(&seen)).await;
    let emby = media_server(Arc::clone(&seen), "/Users/AuthenticateByName", true).await;
    let http = outbound();
    let (connect_url, emby_url) = (connect.url(), emby.url());

    let found = media_login::emby_connect(
        &http,
        (&connect_url, &emby_url, "the-admin-key"),
        "robin",
        "robin-connect-password",
    )
    .await
    .expect("the servers answer");
    assert_eq!(
        found,
        Some(ConnectAccount {
            username: "Robin".to_owned(),
            email: "robin@connect.example.com".to_owned(),
        })
    );
    let log = seen.lock().expect("it locks").clone();
    assert_eq!(log[0].1["x-application"], "Organizr/2.0");
    assert_eq!(log[0].1["accept"], "application/json");
    // The key of the Admin went in the header, and not in the address.
    assert_eq!(log[1].0, "/Users");
    assert_eq!(log[1].1["x-emby-token"], "the-admin-key");

    // A wrong password, and a wrong key of the server, find nobody.
    let wrong = media_login::emby_connect(
        &http,
        (&connect_url, &emby_url, "the-admin-key"),
        "robin",
        "wrong",
    )
    .await;
    assert_eq!(wrong.expect("answers"), None);
    let wrong_key = media_login::emby_connect(
        &http,
        (&connect_url, &emby_url, "another-key"),
        "robin",
        "robin-connect-password",
    )
    .await;
    assert_eq!(wrong_key.expect("answers"), None);
}

#[tokio::test]
async fn an_unreachable_media_server_is_an_error_without_an_address() {
    let http = outbound();
    let closed = closed_port().await;
    let url = format!("http://127.0.0.1:{closed}");
    let error = media_login::emby_local(&http, &url, "robin", "robin-media-password")
        .await
        .expect_err("nobody answers")
        .to_string();
    assert!(!error.contains("127.0.0.1"), "{error}");
    assert!(!error.contains("robin-media-password"), "{error}");
}
