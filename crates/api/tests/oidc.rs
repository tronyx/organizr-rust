//! The login with an OpenID Connect provider, against the stand-in provider of the
//! testkit: the list of providers, the start of a login, the callback with its checks,
//! the accounts and groups that a login gives, and the test of the settings page.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::{StatusCode, header};
use organizr_auth::secrets::PREFIX;
use organizr_db::Database;
use organizr_domain::{NewUser, Timestamp, User};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call, request, send};
use organizr_testkit::login_servers::closed_port;
use organizr_testkit::oidc_provider::{Behaviour, CLIENT_ID, CLIENT_SECRET, OidcProvider};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const LOGIN_COOKIE: &str = "organizr_oidc_login";
const CALLBACK: &str = "http://organizr.example.com/api/v2/oidc/keycloak/callback";

/// The settings of an install with the stand-in as its Keycloak provider.
fn settings_for(discovery_url: &str, more: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut settings = vec![
        ("oidcEnabled", json!(true)),
        ("oidcKeycloakEnabled", json!(true)),
        ("oidcKeycloakName", json!("Company login")),
        ("oidcKeycloakClientId", json!(CLIENT_ID)),
        ("oidcKeycloakClientSecret", json!(CLIENT_SECRET)),
        ("oidcKeycloakDiscoveryUrl", json!(discovery_url)),
        (
            "oidcGroupMappings",
            json!(r#"{"users": 4, "power": "3", "admins": 0}"#),
        ),
    ];
    settings.extend(more.iter().cloned());
    settings
}

fn app(database: &Database, provider: &OidcProvider, more: &[(&'static str, Value)]) -> Router {
    installed_app_with(database, &settings_for(&provider.discovery_url(), more))
}

async fn authorize(app: &Router) -> Answer {
    call(app, As::Guest, "GET", "/oidc/keycloak/authorize", None).await
}

fn location(answer: &Answer) -> String {
    answer.headers[header::LOCATION]
        .to_str()
        .expect("the address is text")
        .to_owned()
}

fn session_cookie(answer: &Answer) -> Option<String> {
    answer
        .cookies
        .iter()
        .find(|cookie| cookie.starts_with("organizr_token_"))
        .and_then(|cookie| cookie.split(';').next())
        .map(str::to_owned)
}

/// Starts a login, lets the stand-in approve it, and sends the browser back to the
/// callback with the login cookie.
async fn log_in(app: &Router, provider: &OidcProvider) -> Answer {
    let started = authorize(app).await;
    assert_eq!(started.status, StatusCode::FOUND, "{}", started.body);
    let cookie = started
        .cookie(LOGIN_COOKIE)
        .expect("the login cookie is set");
    let back = provider.approve(&location(&started)).await;
    send(app, request("GET", &back, &[("cookie", &cookie)], "")).await
}

/// Asserts that a callback failed with the page and the text, and made no session.
fn assert_failed(answer: &Answer, text: &str) {
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(
        answer.headers[header::CONTENT_TYPE],
        "text/html;charset=UTF-8"
    );
    assert!(
        answer.body.contains("<h3>Login Failed</h3>"),
        "{}",
        answer.body
    );
    assert!(
        answer.body.contains(&format!("<p>{text}</p>")),
        "{}",
        answer.body
    );
    assert_eq!(
        session_cookie(answer),
        None,
        "a failed login made a session"
    );
    assert_login_cookie_dropped(answer);
}

fn assert_login_cookie_dropped(answer: &Answer) {
    let dropped = answer.cookies_named(LOGIN_COOKIE);
    assert_eq!(dropped.len(), 1, "{:?}", answer.cookies);
    assert!(dropped[0].starts_with(&format!("{LOGIN_COOKIE}=;")));
    assert!(dropped[0].contains("Max-Age=0"));
}

async fn user(database: &Database, name: &str) -> Option<User> {
    database
        .users()
        .find_by_username(name)
        .await
        .expect("the lookup works")
}

async fn level_of(database: &Database, user: &User) -> u16 {
    database
        .groups()
        .find(user.group)
        .await
        .expect("the lookup works")
        .expect("the group exists")
        .level
        .get()
}

async fn sessions(database: &Database) -> u64 {
    let mut count = 0;
    for user in database
        .users()
        .list()
        .await
        .expect("the users can be listed")
    {
        let list = database
            .sessions()
            .list_valid_of_user(user.id, Timestamp::from_unix_seconds(0))
            .await
            .expect("the sessions can be listed");
        count += u64::try_from(list.len()).expect("a count");
    }
    count
}

async fn store_user(database: &Database, name: &str, email: &str, level: u16) -> User {
    let group = database
        .groups()
        .find_by_level(organizr_domain::GroupLevel::new(level).expect("a level"))
        .await
        .expect("the lookup works")
        .expect("the group exists");
    database
        .users()
        .insert(NewUser {
            username: name.to_owned(),
            email: Some(email.to_owned()),
            password_hash: organizr_auth::password::hash("a password").expect("a hash"),
            group: group.id,
            image: None,
            created_at: Timestamp::from_unix_seconds(1_800_000_000),
        })
        .await
        .expect("the user can be stored")
}

async fn auth_service(app: &Router, name: &str) -> Value {
    let users = call(app, As::Admin, "GET", "/users", None).await;
    users
        .data()
        .as_array()
        .expect("a list")
        .iter()
        .find(|user| user["username"] == json!(name))
        .map_or_else(
            || panic!("the user {name} is listed"),
            |user| user["auth_service"].clone(),
        )
}

#[tokio::test]
async fn providers_lists_the_providers_that_are_on_to_anybody() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let off = installed_app_with(database, &[("oidcKeycloakEnabled", json!(true))]);
        let answer = call(&off, As::Guest, "GET", "/oidc/providers", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.data(), json!([]));

        let on = installed_app_with(
            database,
            &[
                ("oidcEnabled", json!(true)),
                ("oidcKeycloakEnabled", json!(true)),
                ("oidcZitadelEnabled", json!(true)),
                ("oidcZitadelName", json!("Zitadel of the office")),
            ],
        );
        let answer = call(&on, As::Guest, "GET", "/oidc/providers", None).await;
        assert_eq!(
            answer.data(),
            json!({
                "keycloak": {"name": "Keycloak", "enabled": true},
                "zitadel": {"name": "Zitadel of the office", "enabled": true},
            })
        );
    })
    .await;
}

#[tokio::test]
async fn authorize_refuses_when_the_login_or_the_provider_is_off() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let off = installed_app_with(database, &[("oidcKeycloakEnabled", json!(true))]);
        let answer = authorize(&off).await;
        assert_eq!(answer.status, StatusCode::FORBIDDEN);
        assert_eq!(answer.message(), json!("OIDC is not enabled"));

        let on = installed_app_with(database, &[("oidcEnabled", json!(true))]);
        let answer = authorize(&on).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND);
        assert_eq!(answer.message(), json!("Provider not enabled: keycloak"));
        let answer = call(&on, As::Guest, "GET", "/oidc/other/authorize", None).await;
        assert_eq!(answer.message(), json!("Provider not enabled: other"));
    })
    .await;
}

#[tokio::test]
async fn authorize_sends_the_browser_to_the_provider_with_pkce_state_and_nonce() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        let app = app(database, &provider, &[]);

        let answer = authorize(&app).await;

        assert_eq!(answer.status, StatusCode::FOUND);
        let address = location(&answer);
        assert!(address.starts_with(&format!("{}/authorize?", provider.issuer())));
        provider.approve(&address).await;
        let query = provider.last_authorize();
        assert_eq!(query["response_type"], "code");
        assert_eq!(query["client_id"], CLIENT_ID);
        assert_eq!(query["redirect_uri"], CALLBACK);
        assert_eq!(query["scope"], "openid profile email");
        assert_eq!(query["code_challenge_method"], "S256");
        assert!(query["code_challenge"].len() >= 43);
        assert!(query["state"].len() >= 16);
        assert!(query["nonce"].len() >= 16);
        assert!(!address.contains(CLIENT_SECRET));

        let cookies = answer.cookies_named(LOGIN_COOKIE);
        assert_eq!(cookies.len(), 1);
        let cookie = cookies[0];
        assert!(
            cookie.ends_with("; Path=/api/v2/oidc; Max-Age=600; HttpOnly; SameSite=Lax"),
            "{cookie}"
        );
        // The cookie is sealed: it shows none of its values.
        for value in [&query["state"], &query["nonce"], &"keycloak".to_owned()] {
            assert!(!cookie.contains(value.as_str()));
        }

        let secure = send(
            &app,
            request(
                "GET",
                "/api/v2/oidc/keycloak/authorize",
                &[("x-forwarded-proto", "https")],
                "",
            ),
        )
        .await;
        // The proxy is not trusted, so the request did not come over HTTPS.
        assert!(!secure.cookies_named(LOGIN_COOKIE)[0].contains("Secure"));
    })
    .await;
}

#[tokio::test]
async fn login_makes_the_account_and_the_session_and_goes_to_the_start_page() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        let app = app(database, &provider, &[]);

        let answer = log_in(&app, &provider).await;

        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        assert_eq!(location(&answer), "/");
        assert_login_cookie_dropped(&answer);
        let session = session_cookie(&answer).expect("the session cookie is set");
        let me = call(&app, As::User(&session), "GET", "/token/me", None).await;
        assert_eq!(me.data()["username"], json!("robin"));
        assert_eq!(me.data()["groupID"], json!(4));

        let robin = user(database, "robin").await.expect("the account was made");
        assert_eq!(robin.email.as_deref(), Some("robin@example.com"));
        assert_eq!(auth_service(&app, "robin").await, json!("oidc::keycloak"));

        let sent = provider.token_requests();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0]["client_secret"], CLIENT_SECRET);
        assert_eq!(sent[0]["redirect_uri"], CALLBACK);
        assert!(sent[0]["code_verifier"].len() >= 43);
    })
    .await;
}

#[tokio::test]
async fn login_without_user_info_takes_the_claims_of_the_id_token() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        provider.set(|behaviour| {
            behaviour.user_info = false;
            behaviour
                .person
                .insert("preferred_username".to_owned(), json!("sam"));
            behaviour
                .person
                .insert("email".to_owned(), json!("sam@example.com"));
        });
        let app = app(database, &provider, &[]);

        let answer = log_in(&app, &provider).await;

        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        assert!(user(database, "sam").await.is_some());
    })
    .await;
}

#[tokio::test]
async fn callback_refuses_a_state_that_does_not_match() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        let app = app(database, &provider, &[]);
        let started = authorize(&app).await;
        let cookie = started
            .cookie(LOGIN_COOKIE)
            .expect("the login cookie is set");
        let back = provider.approve(&location(&started)).await;
        let (path, query) = back.split_once('?').expect("a query");
        let changed: String = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(
                form_urlencoded::parse(query.as_bytes()).map(|(key, value)| {
                    let value = if key == "state" {
                        "another state".into()
                    } else {
                        value
                    };
                    (key, value)
                }),
            )
            .finish();

        let answer = send(
            &app,
            request(
                "GET",
                &format!("{path}?{changed}"),
                &[("cookie", &cookie)],
                "",
            ),
        )
        .await;

        assert_failed(&answer, "Invalid or expired state parameter");
        assert!(provider.token_requests().is_empty(), "the code was traded");
        assert_eq!(sessions(database).await, 0);
    })
    .await;
}

#[tokio::test]
async fn callback_refuses_a_missing_a_forged_or_an_expired_login_cookie() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        let app = app(database, &provider, &[]);
        let started = authorize(&app).await;
        let back = provider.approve(&location(&started)).await;
        let query = provider.last_authorize();

        let missing = send(&app, request("GET", &back, &[], "")).await;
        assert_failed(&missing, "Invalid or expired state parameter");

        let forged = format!("{LOGIN_COOKIE}={}", "ab".repeat(80));
        let forged = send(&app, request("GET", &back, &[("cookie", &forged)], "")).await;
        assert_failed(&forged, "Invalid or expired state parameter");

        // The values of the login, sealed with the key of the install, ten minutes too late.
        let expired = json!({
            "provider": "keycloak",
            "state": query["state"],
            "nonce": query["nonce"],
            "verifier": "a verifier",
            "back": "/",
            "expires": 1_000_000,
        });
        let sealed = seed::secrets()
            .seal("oidc/login", &expired.to_string())
            .expect("it seals");
        let value = sealed.strip_prefix(PREFIX).expect("the prefix");
        let expired = send(
            &app,
            request(
                "GET",
                &back,
                &[("cookie", &format!("{LOGIN_COOKIE}={value}"))],
                "",
            ),
        )
        .await;
        assert_failed(&expired, "Invalid or expired state parameter");

        assert!(provider.token_requests().is_empty(), "the code was traded");
        assert_eq!(sessions(database).await, 0);
    })
    .await;
}

#[tokio::test]
async fn callback_refuses_a_cookie_of_another_provider() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        let app = app(database, &provider, &[]);
        let started = authorize(&app).await;
        let cookie = started
            .cookie(LOGIN_COOKIE)
            .expect("the login cookie is set");
        let back = provider.approve(&location(&started)).await;
        let other = back.replace("/oidc/keycloak/", "/oidc/authentik/");

        let answer = send(&app, request("GET", &other, &[("cookie", &cookie)], "")).await;

        assert_failed(&answer, "Invalid or expired state parameter");
    })
    .await;
}

#[tokio::test]
async fn callback_refuses_an_answer_without_a_code_or_with_an_error() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        let app = app(database, &provider, &[]);

        let answer = call(
            &app,
            As::Guest,
            "GET",
            "/oidc/keycloak/callback?state=abc",
            None,
        )
        .await;
        assert_failed(&answer, "Missing code or state parameter");

        let path =
            "/oidc/keycloak/callback?error=access_denied&error_description=%3Cb%3EHi%3C%2Fb%3E";
        let answer = call(&app, As::Guest, "GET", path, None).await;
        assert_failed(&answer, "The provider refused the login: access_denied");
        assert!(!answer.body.contains("Hi"));

        let path = "/oidc/keycloak/callback?error=%3Cscript%3E";
        let answer = call(&app, As::Guest, "GET", path, None).await;
        assert_failed(&answer, "The provider refused the login");
    })
    .await;
}

/// A way in which the stand-in answers wrongly, how to make it, and the text of the page.
type Case = (&'static str, fn(&mut Behaviour), &'static str);

/// Each way in which the ID token is wrong, with the text of the page.
#[tokio::test]
async fn callback_refuses_an_id_token_that_fails_a_check() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let cases: [Case; 5] = [
            (
                "wrong nonce",
                |b| b.nonce = Some("another nonce".to_owned()),
                "Invalid nonce",
            ),
            (
                "wrong issuer",
                |b| b.issuer = Some("https://evil.example.com".to_owned()),
                "Invalid ID token",
            ),
            (
                "wrong audience",
                |b| b.audience = Some("another-client".to_owned()),
                "Invalid ID token",
            ),
            (
                "bad signature",
                |b| b.bad_signature = true,
                "Invalid ID token",
            ),
            ("expired token", |b| b.expired = true, "Invalid ID token"),
        ];
        for (case, change, text) in cases {
            let provider = OidcProvider::start().await;
            provider.set(change);
            let app = app(database, &provider, &[]);

            let answer = log_in(&app, &provider).await;

            assert_eq!(answer.status, StatusCode::OK, "{case}");
            assert_failed(&answer, text);
            assert!(
                user(database, "robin").await.is_none(),
                "{case}: an account was made"
            );
        }
        assert_eq!(sessions(database).await, 0);
    })
    .await;
}

#[tokio::test]
async fn callback_refuses_when_the_token_endpoint_refuses_the_code() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        provider.set(|behaviour| behaviour.token_error = true);
        let app = app(database, &provider, &[]);

        let answer = log_in(&app, &provider).await;

        assert_failed(&answer, "Failed to exchange authorization code");
        assert_eq!(sessions(database).await, 0);
    })
    .await;
}

#[tokio::test]
async fn unreachable_provider_stops_the_start_and_the_end_of_a_login() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let port = closed_port().await;
        let away = installed_app_with(
            database,
            &settings_for(
                &format!("http://127.0.0.1:{port}/.well-known/openid-configuration"),
                &[],
            ),
        );
        let answer = authorize(&away).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            answer.message(),
            json!("Failed to generate authorization URL")
        );
        assert_eq!(answer.cookies_named(LOGIN_COOKIE), Vec::<&str>::new());

        // The provider goes away while the person logs in there.
        let provider = OidcProvider::start().await;
        let app = app(database, &provider, &[]);
        let started = authorize(&app).await;
        let cookie = started
            .cookie(LOGIN_COOKIE)
            .expect("the login cookie is set");
        let back = provider.approve(&location(&started)).await;
        drop(provider);

        let answer = send(&app, request("GET", &back, &[("cookie", &cookie)], "")).await;

        assert_failed(&answer, "Failed to exchange authorization code");
        assert_eq!(sessions(database).await, 0);
    })
    .await;
}

#[tokio::test]
async fn group_mapping_takes_the_first_the_highest_or_the_lowest_privilege() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        for (number, mode, level) in [
            (1, "first", 3),
            (2, "highest_privilege", 0),
            (3, "lowest_privilege", 4),
        ] {
            let provider = OidcProvider::start().await;
            let name = format!("person{number}");
            provider.set(|behaviour| {
                behaviour
                    .person
                    .insert("preferred_username".to_owned(), json!(name));
                behaviour
                    .person
                    .insert("email".to_owned(), json!(format!("{name}@example.com")));
                behaviour.person.insert(
                    "groups".to_owned(),
                    json!(["power", "/org/admins", "users"]),
                );
            });
            let app = app(
                database,
                &provider,
                &[("oidcGroupMappingMode", json!(mode))],
            );

            let answer = log_in(&app, &provider).await;

            assert_eq!(answer.status, StatusCode::FOUND, "{mode}: {}", answer.body);
            let person = user(database, &name).await.expect("the account was made");
            assert_eq!(level_of(database, &person).await, level, "{mode}");
        }

        // No group of the person has a mapping: the default group of the settings.
        let provider = OidcProvider::start().await;
        provider.set(|behaviour| {
            behaviour
                .person
                .insert("preferred_username".to_owned(), json!("nomap"));
            behaviour
                .person
                .insert("email".to_owned(), json!("nomap@example.com"));
            behaviour
                .person
                .insert("groups".to_owned(), json!(["strangers"]));
        });
        let app = app(database, &provider, &[("oidcDefaultGroupId", json!("2"))]);
        log_in(&app, &provider).await;
        let person = user(database, "nomap").await.expect("the account was made");
        assert_eq!(level_of(database, &person).await, 2);
    })
    .await;
}

#[tokio::test]
async fn zitadel_roles_are_the_keys_of_their_claim() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        provider.set(|behaviour| {
            behaviour.person.insert(
                "urn:zitadel:iam:org:project:roles".to_owned(),
                json!({"admins": {"1234": "org.example.com"}}),
            );
        });
        let app = installed_app_with(
            database,
            &[
                ("oidcEnabled", json!(true)),
                ("oidcZitadelEnabled", json!(true)),
                ("oidcZitadelClientId", json!(CLIENT_ID)),
                ("oidcZitadelClientSecret", json!(CLIENT_SECRET)),
                ("oidcZitadelDiscoveryUrl", json!(provider.discovery_url())),
                ("oidcGroupMappings", json!(r#"{"admins": 1}"#)),
            ],
        );
        let started = call(&app, As::Guest, "GET", "/oidc/zitadel/authorize", None).await;
        let cookie = started
            .cookie(LOGIN_COOKIE)
            .expect("the login cookie is set");
        let back = provider.approve(&location(&started)).await;
        assert!(back.starts_with("/api/v2/oidc/zitadel/callback?"));

        let answer = send(&app, request("GET", &back, &[("cookie", &cookie)], "")).await;

        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        let robin = user(database, "robin").await.expect("the account was made");
        assert_eq!(level_of(database, &robin).await, 1);
    })
    .await;
}

#[tokio::test]
async fn without_automatic_accounts_an_unknown_person_gets_no_account() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        let app = app(
            database,
            &provider,
            &[("oidcAutoCreateUsers", json!(false))],
        );

        let answer = log_in(&app, &provider).await;

        assert_failed(&answer, "Failed to create or link user");
        assert!(user(database, "robin").await.is_none());
        assert_eq!(sessions(database).await, 0);
    })
    .await;
}

#[tokio::test]
async fn existing_account_is_linked_by_its_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_user(database, "robin.smith", "Robin@Example.com", 2).await;
        let provider = OidcProvider::start().await;
        let app = app(
            database,
            &provider,
            &[("oidcAutoCreateUsers", json!(false))],
        );

        let answer = log_in(&app, &provider).await;

        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        let session = session_cookie(&answer).expect("the session cookie is set");
        let me = call(&app, As::User(&session), "GET", "/token/me", None).await;
        assert_eq!(me.data()["username"], json!("robin.smith"));
        // Without `oidcUpdateGroupsOnLogin`, the account keeps its group.
        assert_eq!(me.data()["groupID"], json!(2));
        assert_eq!(
            auth_service(&app, "robin.smith").await,
            json!("oidc::keycloak")
        );
        assert!(user(database, "robin").await.is_none());
    })
    .await;
}

#[tokio::test]
async fn address_that_the_provider_did_not_check_links_no_account() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_user(database, "robin.smith", "robin@example.com", 2).await;
        let provider = OidcProvider::start().await;
        provider.set(|behaviour| {
            behaviour
                .person
                .insert("email_verified".to_owned(), json!(false));
        });
        let app = app(database, &provider, &[]);

        let answer = log_in(&app, &provider).await;

        // The address is taken, so no account can be made either.
        assert_failed(&answer, "Failed to create or link user");
    })
    .await;
}

#[tokio::test]
async fn without_linking_only_accounts_of_the_provider_are_used() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_user(database, "robin", "other@example.com", 2).await;
        let provider = OidcProvider::start().await;
        let no_link = [("oidcLinkExistingUsers", json!(false))];

        let answer = log_in(&app(database, &provider, &no_link), &provider).await;
        assert_failed(&answer, "Failed to create or link user");

        // With linking, the account of the same name is linked, and then counts as an
        // account of the provider.
        let answer = log_in(&app(database, &provider, &[]), &provider).await;
        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        let answer = log_in(&app(database, &provider, &no_link), &provider).await;
        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        assert!(session_cookie(&answer).is_some());
    })
    .await;
}

#[tokio::test]
async fn group_of_an_account_follows_the_provider_only_with_the_setting() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_user(database, "robin", "robin@example.com", 2).await;
        let provider = OidcProvider::start().await;
        provider.set(|behaviour| {
            behaviour
                .person
                .insert("groups".to_owned(), json!(["admins"]));
        });

        log_in(&app(database, &provider, &[]), &provider).await;
        let robin = user(database, "robin").await.expect("the account");
        assert_eq!(level_of(database, &robin).await, 2);

        let update = [("oidcUpdateGroupsOnLogin", json!(true))];
        let answer = log_in(&app(database, &provider, &update), &provider).await;
        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        let robin = user(database, "robin").await.expect("the account");
        assert_eq!(level_of(database, &robin).await, 0);
    })
    .await;
}

#[tokio::test]
async fn test_route_is_for_admins_and_co_admins_and_reads_the_discovery() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        let app = app(database, &provider, &[]);
        let co_admin = seed::logged_in(database, &app, "co", 1).await;
        let power_user = seed::logged_in(database, &app, "power", 2).await;

        for who in [As::Guest, As::User(&power_user)] {
            let answer = call(&app, who, "GET", "/oidc/keycloak/test", None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{who:?}");
        }
        let expected = format!("OIDC connection successful - Issuer: {}", provider.issuer());
        for who in [As::Admin, As::User(&co_admin)] {
            let answer = call(&app, who, "GET", "/oidc/keycloak/test", None).await;
            assert_eq!(answer.status, StatusCode::OK, "{who:?}: {}", answer.body);
            assert_eq!(answer.message(), json!(expected));
        }

        let answer = call(&app, As::Admin, "GET", "/oidc/other/test", None).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND);
        assert_eq!(answer.message(), json!("Provider not found"));
        let answer = call(&app, As::Admin, "GET", "/oidc/authentik/test", None).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(answer.message(), json!("Discovery URL not configured"));
        let no_client = installed_app_with(
            database,
            &[("oidcPocketidDiscoveryUrl", json!(provider.discovery_url()))],
        );
        let answer = call(&no_client, As::Admin, "GET", "/oidc/pocketid/test", None).await;
        assert_eq!(answer.message(), json!("Client ID not configured"));

        let port = closed_port().await;
        let away = installed_app_with(
            database,
            &settings_for(&format!("http://127.0.0.1:{port}/discovery"), &[]),
        );
        let answer = call(&away, As::Admin, "GET", "/oidc/keycloak/test", None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            answer.message(),
            json!("Failed to fetch discovery document")
        );
    })
    .await;
}

#[tokio::test]
async fn launch_names_the_provider_of_the_automatic_redirect_and_the_logout_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plain = installed_app_with(database, &[]);
        let launch = call(&plain, As::Guest, "GET", "/launch", None).await;
        assert_eq!(
            launch.data()["settings"]["login"]["oidc"],
            json!({"enabled": false, "autoRedirect": "", "logoutURL": ""})
        );

        let app = installed_app_with(
            database,
            &[
                ("oidcEnabled", json!(true)),
                ("oidcKeycloakEnabled", json!(true)),
                ("oidcAutoRedirect", json!(true)),
                ("oidcAutoRedirectProvider", json!("keycloak")),
                (
                    "oidcAutoRedirectLogoutUrl",
                    json!("https://id.example.com/logout"),
                ),
            ],
        );
        let launch = call(&app, As::Guest, "GET", "/launch", None).await;
        assert_eq!(
            launch.data()["settings"]["login"]["oidc"],
            json!({
                "enabled": true,
                "autoRedirect": "keycloak",
                "logoutURL": "https://id.example.com/logout",
            })
        );

        // A provider that is off is no target of the redirect.
        let off = installed_app_with(
            database,
            &[
                ("oidcEnabled", json!(true)),
                ("oidcAutoRedirect", json!(true)),
                ("oidcAutoRedirectProvider", json!("keycloak")),
            ],
        );
        let launch = call(&off, As::Guest, "GET", "/launch", None).await;
        assert_eq!(
            launch.data()["settings"]["login"]["oidc"]["autoRedirect"],
            json!("")
        );
    })
    .await;
}

/// Sets the person of the stand-in: the user name and a checked e-mail address.
fn person(provider: &OidcProvider, name: &str, email: &str) {
    provider.set(|behaviour| {
        behaviour
            .person
            .insert("preferred_username".to_owned(), json!(name));
        behaviour.person.insert("email".to_owned(), json!(email));
        behaviour
            .person
            .insert("email_verified".to_owned(), json!(true));
    });
}

#[tokio::test]
async fn admin_account_is_not_linked_by_its_name() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        person(&provider, seed::ADMIN_NAME, "someone@example.com");
        let app = app(database, &provider, &[]);

        let answer = log_in(&app, &provider).await;

        assert_failed(&answer, "Failed to create or link user");
        assert_eq!(sessions(database).await, 0);
        assert_eq!(
            auth_service(&app, seed::ADMIN_NAME).await,
            json!("internal")
        );
    })
    .await;
}

#[tokio::test]
async fn co_admin_account_is_not_linked_by_its_checked_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_user(database, "boss", "boss@example.com", 1).await;
        let provider = OidcProvider::start().await;
        person(&provider, "boss.at.the.provider", "boss@example.com");
        let app = app(database, &provider, &[]);

        let answer = log_in(&app, &provider).await;

        assert_failed(&answer, "Failed to create or link user");
        assert_eq!(sessions(database).await, 0);
        assert!(user(database, "boss.at.the.provider").await.is_none());
        assert_eq!(auth_service(&app, "boss").await, json!("internal"));
    })
    .await;
}

#[tokio::test]
async fn admin_account_that_this_provider_linked_logs_in() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let chief = store_user(database, "chief", "chief@example.com", 0).await;
        database
            .users()
            .set_auth_service(chief.id, "oidc", Some("keycloak"))
            .await
            .expect("the service can be stored");
        let provider = OidcProvider::start().await;
        person(&provider, "chief", "chief@example.com");

        let answer = log_in(&app(database, &provider, &[]), &provider).await;
        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        let session = session_cookie(&answer).expect("the session cookie is set");
        let app = app(database, &provider, &[]);
        let me = call(&app, As::User(&session), "GET", "/token/me", None).await;
        assert_eq!(me.data()["username"], json!("chief"));
        assert_eq!(me.data()["groupID"], json!(0));

        // A link to another provider does not count for this one.
        database
            .users()
            .set_auth_service(chief.id, "oidc", Some("authentik"))
            .await
            .expect("the service can be stored");
        let answer = log_in(&app, &provider).await;
        assert_failed(&answer, "Failed to create or link user");
    })
    .await;
}

#[tokio::test]
async fn account_of_a_normal_level_is_still_linked_by_its_name() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_user(database, "sam", "sam@example.com", 3).await;
        let provider = OidcProvider::start().await;
        person(&provider, "sam", "another@example.com");
        let app = app(database, &provider, &[]);

        let answer = log_in(&app, &provider).await;

        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        assert!(session_cookie(&answer).is_some());
        assert_eq!(auth_service(&app, "sam").await, json!("oidc::keycloak"));
    })
    .await;
}

/// Starts a link of the account of `session` to the stand-in, lets the stand-in approve
/// it, and returns the callback path and the login cookie.
async fn start_link(app: &Router, provider: &OidcProvider, session: &str) -> (String, String) {
    let started = call(app, As::User(session), "GET", "/oidc/keycloak/link", None).await;
    assert_eq!(started.status, StatusCode::FOUND, "{}", started.body);
    let cookie = started
        .cookie(LOGIN_COOKIE)
        .expect("the login cookie is set");
    let back = provider.approve(&location(&started)).await;
    (back, cookie)
}

async fn finish_link(app: &Router, back: &str, cookies: &[&str]) -> Answer {
    let cookies = cookies.join("; ");
    send(app, request("GET", back, &[("cookie", &cookies)], "")).await
}

#[tokio::test]
async fn admin_links_the_own_account_and_then_logs_in_through_the_provider() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        person(&provider, "chief", "chief@provider.example.com");
        let app = app(database, &provider, &[]);
        let session = seed::logged_in(database, &app, "chief", 0).await;
        let users_before = database.users().count().await.expect("a count");

        let (back, cookie) = start_link(&app, &provider, &session).await;
        let answer = finish_link(&app, &back, &[&cookie, &session]).await;

        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        assert_eq!(location(&answer), "/?oidcLinked=keycloak#/account");
        assert_eq!(session_cookie(&answer), None, "the link made a new session");
        assert_login_cookie_dropped(&answer);
        assert_eq!(auth_service(&app, "chief").await, json!("oidc::keycloak"));
        assert_eq!(
            database.users().count().await.expect("a count"),
            users_before
        );
        let linked = call(&app, As::User(&session), "GET", "/oidc/link", None).await;
        assert_eq!(
            linked.data(),
            json!({"provider": "keycloak", "name": "Company login"})
        );

        // The link counts for the login through the provider, also with linking off.
        let no_link = installed_app_with(
            database,
            &settings_for(
                &provider.discovery_url(),
                &[("oidcLinkExistingUsers", json!(false))],
            ),
        );
        let answer = log_in(&no_link, &provider).await;
        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        let new_session = session_cookie(&answer).expect("the session cookie is set");
        let me = call(&no_link, As::User(&new_session), "GET", "/token/me", None).await;
        assert_eq!(me.data()["username"], json!("chief"));
        assert_eq!(me.data()["groupID"], json!(0));
    })
    .await;
}

#[tokio::test]
async fn link_is_found_by_the_checked_address_when_the_names_differ() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        let app = app(
            database,
            &provider,
            &[("oidcLinkExistingUsers", json!(false))],
        );
        let session = seed::logged_in(database, &app, "deputy", 1).await;
        let deputy = user(database, "deputy").await.expect("the account");
        database
            .users()
            .update(
                deputy.id,
                organizr_domain::UserChanges {
                    email: organizr_domain::Change::Set("deputy@example.com".to_owned()),
                    ..organizr_domain::UserChanges::none()
                },
            )
            .await
            .expect("the address can be stored");
        person(&provider, "dep.at.the.provider", "deputy@example.com");

        let (back, cookie) = start_link(&app, &provider, &session).await;
        let answer = finish_link(&app, &back, &[&cookie, &session]).await;
        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);

        let answer = log_in(&app, &provider).await;
        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
        let new_session = session_cookie(&answer).expect("the session cookie is set");
        let me = call(&app, As::User(&new_session), "GET", "/token/me", None).await;
        assert_eq!(me.data()["username"], json!("deputy"));
    })
    .await;
}

#[tokio::test]
async fn link_must_come_back_to_the_session_that_started_it() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        person(&provider, "chief", "chief@provider.example.com");
        let app = app(database, &provider, &[]);
        let chief = seed::logged_in(database, &app, "chief", 0).await;
        let other = seed::logged_in(database, &app, "other", 4).await;

        let (back, cookie) = start_link(&app, &provider, &chief).await;
        let answer = finish_link(&app, &back, &[&cookie, &other]).await;
        assert_failed(
            &answer,
            "The link must be finished in the session that started it",
        );

        let (back, cookie) = start_link(&app, &provider, &chief).await;
        let answer = finish_link(&app, &back, &[&cookie]).await;
        assert_failed(
            &answer,
            "The link must be finished in the session that started it",
        );

        assert!(provider.token_requests().is_empty(), "the code was traded");
        assert_eq!(auth_service(&app, "chief").await, json!("internal"));
    })
    .await;
}

#[tokio::test]
async fn link_refuses_a_forged_or_an_expired_login_cookie() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        let app = app(database, &provider, &[]);
        let chief = seed::logged_in(database, &app, "chief", 0).await;
        let chief_id = user(database, "chief").await.expect("the account").id;
        let (back, _) = start_link(&app, &provider, &chief).await;
        let query = provider.last_authorize();

        let forged = format!("{LOGIN_COOKIE}={}", "cd".repeat(90));
        let answer = finish_link(&app, &back, &[&forged, &chief]).await;
        assert_failed(&answer, "Invalid or expired state parameter");

        let expired = json!({
            "provider": "keycloak",
            "state": query["state"],
            "nonce": query["nonce"],
            "verifier": "a verifier",
            "back": "/#/account",
            "expires": 1_000_000,
            "link_user": chief_id.get(),
        });
        let sealed = seed::secrets()
            .seal("oidc/login", &expired.to_string())
            .expect("it seals");
        let value = sealed.strip_prefix(PREFIX).expect("the prefix");
        let expired = format!("{LOGIN_COOKIE}={value}");
        let answer = finish_link(&app, &back, &[&expired, &chief]).await;
        assert_failed(&answer, "Invalid or expired state parameter");
        assert_eq!(auth_service(&app, "chief").await, json!("internal"));
    })
    .await;
}

#[tokio::test]
async fn link_never_makes_an_account() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        person(&provider, "stranger", "stranger@example.com");
        let app = app(database, &provider, &[]);
        let chief = seed::logged_in(database, &app, "chief", 0).await;
        let users_before = database.users().count().await.expect("a count");

        let (back, cookie) = start_link(&app, &provider, &chief).await;
        let answer = finish_link(&app, &back, &[&cookie, &chief]).await;

        assert_failed(
            &answer,
            "The provider names the user stranger, whose user name and e-mail address are not those of this account",
        );
        assert_eq!(database.users().count().await.expect("a count"), users_before);
        assert!(user(database, "stranger").await.is_none());
        assert_eq!(auth_service(&app, "chief").await, json!("internal"));
    })
    .await;
}

#[tokio::test]
async fn second_account_cannot_link_an_identity_that_belongs_to_another() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        person(&provider, "chief", "chief@provider.example.com");
        let app = app(database, &provider, &[]);
        let chief = seed::logged_in(database, &app, "chief", 0).await;
        let deputy = seed::logged_in(database, &app, "deputy", 1).await;

        let (back, cookie) = start_link(&app, &provider, &chief).await;
        let answer = finish_link(&app, &back, &[&cookie, &chief]).await;
        assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);

        // The provider vouches for "chief", which is the name of another account.
        let (back, cookie) = start_link(&app, &provider, &deputy).await;
        let answer = finish_link(&app, &back, &[&cookie, &deputy]).await;
        assert_failed(
            &answer,
            "Another account already has the user name chief of the provider",
        );
        assert_eq!(auth_service(&app, "deputy").await, json!("internal"));
        assert_eq!(auth_service(&app, "chief").await, json!("oidc::keycloak"));
    })
    .await;
}

#[tokio::test]
async fn link_route_needs_a_session_of_this_site() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let provider = OidcProvider::start().await;
        let app = app(database, &provider, &[]);
        let chief = seed::logged_in(database, &app, "chief", 0).await;

        for who in [As::Guest, As::Admin] {
            let answer = call(&app, who, "GET", "/oidc/keycloak/link", None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{who:?}");
            assert_eq!(answer.cookies_named(LOGIN_COOKIE), Vec::<&str>::new());
        }
        let answer = call(&app, As::Guest, "GET", "/oidc/link", None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);

        let cross_site = send(
            &app,
            request(
                "GET",
                "/api/v2/oidc/keycloak/link",
                &[("cookie", &chief), ("sec-fetch-site", "cross-site")],
                "",
            ),
        )
        .await;
        assert_ne!(cross_site.status, StatusCode::FOUND);
        assert_eq!(cross_site.cookies_named(LOGIN_COOKIE), Vec::<&str>::new());

        let answer = call(&app, As::User(&chief), "GET", "/oidc/zitadel/link", None).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND);
        assert_eq!(answer.message(), json!("Provider not enabled: zitadel"));

        let linked = call(&app, As::User(&chief), "GET", "/oidc/link", None).await;
        assert_eq!(linked.data(), json!({"provider": null, "name": null}));
    })
    .await;
}
