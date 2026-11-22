//! The clients of the integrations against stand-ins for the services: what goes over
//! the wire, and what comes back.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::time::Duration;

use organizr_integrations::http::{Outbound, user_agent};
use organizr_integrations::plex::{PlexError, PlexTv};
use organizr_integrations::sso::{
    Fallback, JellyfinSession, Login, TautulliSession, jellyfin_session, ombi_token,
    overseerr_session, petio_token, tautulli_sessions,
};
use organizr_testkit::services::{
    self, JELLYFIN_SERVER_ID, LOCAL_ACCOUNT, OWNER_TOKEN, PLEX_USERS, SERVER_ID, StandIn,
};

const TIMEOUT: Duration = Duration::from_secs(10);
const CLIENT_ID: &str = "5d0c6f0e-made-up-install";

fn outbound() -> Outbound {
    Outbound::new().expect("the client can be built")
}

async fn plex() -> (PlexTv, StandIn) {
    let stand_in = services::plex_tv().await;
    (PlexTv::new(outbound(), &stand_in.url()), stand_in)
}

fn login(plex_token: Option<&str>) -> Login<'_> {
    Login {
        username: "robin",
        email: "robin@example.com",
        password: "the password of robin in organizr",
        plex_token,
        client_ip: "203.0.113.9",
        user_agent: Some("Browser/1.0"),
    }
}

const FALLBACK: Fallback<'static> = Fallback {
    user: LOCAL_ACCOUNT.0,
    password: LOCAL_ACCOUNT.2,
};

#[tokio::test]
async fn plex_login_sends_the_form_of_the_service_and_reads_the_account() {
    let (plex, stand_in) = plex().await;
    let robin = &PLEX_USERS[1];

    let account = plex
        .sign_in(CLIENT_ID, "ROBIN@example.com", robin.password)
        .await
        .expect("the service answers")
        .expect("the login is right");

    assert_eq!(account.username, "Robin");
    assert_eq!(account.email, "robin@example.com");
    assert_eq!(account.token, robin.token);
    assert_eq!(
        account.image.as_deref(),
        Some("https://plex.tv/users/robin/avatar")
    );
    let seen = &stand_in.seen()[0];
    assert_eq!(
        (seen.method.as_str(), seen.path.as_str()),
        ("POST", "/users/sign_in.json")
    );
    assert_eq!(seen.header("x-plex-client-identifier"), Some(CLIENT_ID));
    assert_eq!(seen.header("x-plex-product"), Some("Organizr"));
    assert_eq!(seen.header("user-agent"), Some(user_agent().as_str()));
    assert_eq!(seen.form()["user[login]"], "ROBIN@example.com");
    assert_eq!(seen.form()["user[password]"], robin.password);

    let wrong = plex.sign_in(CLIENT_ID, "robin", "not the password").await;
    assert_eq!(wrong.expect("the service answers"), None);
}

#[tokio::test]
async fn plex_token_names_its_account() {
    let (plex, stand_in) = plex().await;

    let account = plex
        .account(PLEX_USERS[2].token)
        .await
        .expect("the service answers")
        .expect("the token is known");

    assert_eq!(account.username, "pat");
    assert_eq!(account.email, "Pat@Example.com");
    assert_eq!(
        stand_in.seen()[0].header("x-plex-token"),
        Some(PLEX_USERS[2].token)
    );
    assert_eq!(
        plex.account("no-such-token").await.expect("it answers"),
        None
    );
    // An empty token is not sent.
    assert_eq!(plex.account("").await.expect("no request is needed"), None);
    assert_eq!(stand_in.seen().len(), 2);
}

#[tokio::test]
async fn friend_of_the_owner_is_found_with_the_token_of_the_owner() {
    let (plex, stand_in) = plex().await;

    assert!(
        plex.is_friend(OWNER_TOKEN, "robin", None)
            .await
            .expect("it answers")
    );
    assert!(
        plex.is_friend(OWNER_TOKEN, "pat@example.com", None)
            .await
            .expect("it answers")
    );
    assert!(
        !plex
            .is_friend(OWNER_TOKEN, "stranger", None)
            .await
            .expect("it answers")
    );
    // The strict check: the server of the install must be shared with the friend.
    assert!(
        plex.is_friend(OWNER_TOKEN, "robin", Some(SERVER_ID))
            .await
            .expect("it answers")
    );
    assert!(
        !plex
            .is_friend(OWNER_TOKEN, "pat", Some(SERVER_ID))
            .await
            .expect("it answers")
    );
    assert_eq!(stand_in.seen()[0].path, "/api/users");
    assert_eq!(stand_in.seen()[0].header("x-plex-token"), Some(OWNER_TOKEN));

    // A token that is not the one of the owner, and no token at all.
    let refused = plex.is_friend("another-token", "robin", None).await;
    assert!(
        matches!(refused, Err(PlexError::Status(401))),
        "{refused:?}"
    );
    assert!(
        !plex
            .is_friend("", "robin", None)
            .await
            .expect("no request is needed")
    );
}

#[tokio::test]
async fn servers_of_the_owner_are_listed() {
    let (plex, stand_in) = plex().await;

    let servers = plex
        .servers(OWNER_TOKEN)
        .await
        .expect("the service answers");

    assert_eq!(servers.len(), 2);
    assert_eq!(servers[0].machine_identifier, SERVER_ID);
    assert!(servers[0].owned && !servers[1].owned);
    assert_eq!(
        stand_in.seen()[0].header("x-plex-client-identifier"),
        Some("01010101-10101010")
    );
}

#[tokio::test]
async fn service_that_is_away_is_an_error_and_no_wrong_login() {
    let stand_in = services::plex_tv().await;
    let url = stand_in.url();
    drop(stand_in);
    tokio::time::sleep(Duration::from_millis(50)).await;
    let plex = PlexTv::new(outbound(), &url);

    let result = plex.sign_in(CLIENT_ID, "robin", "a password").await;

    assert!(
        matches!(result, Err(PlexError::Unreachable(_))),
        "{result:?}"
    );
}

#[tokio::test]
async fn ombi_gives_a_token_for_a_plex_token_or_for_the_fallback_account() {
    let stand_in = services::ombi().await;
    let http = outbound();
    let url = stand_in.url();

    let token = ombi_token(&http, &url, login(Some(PLEX_USERS[1].token)), None, TIMEOUT).await;
    assert_eq!(token.as_deref(), Some("ombi-token-of-robin"));
    let seen = &stand_in.seen()[0];
    assert_eq!(seen.path, "/api/v1/Token/plextoken");
    assert_eq!(seen.json()["plexToken"], PLEX_USERS[1].token);
    // The password of the user is not sent with a Plex token.
    assert_eq!(seen.json()["password"], "");
    assert_eq!(seen.header("x-forwarded-for"), Some("203.0.113.9"));

    // Without a Plex token: the name and the password, which Ombi does not know here.
    let none = ombi_token(&http, &url, login(None), None, TIMEOUT).await;
    assert_eq!(none, None);
    let seen = &stand_in.seen()[1];
    assert_eq!(seen.path, "/api/v1/Token");
    assert_eq!(seen.json()["username"], "robin");
    assert_eq!(seen.json()["password"], "the password of robin in organizr");

    // The fallback account is tried after the user.
    let stranger = login(Some(PLEX_USERS[3].token));
    let token = ombi_token(&http, &url, stranger, Some(FALLBACK), TIMEOUT).await;
    assert_eq!(token.as_deref(), Some("ombi-token-of-fallback"));
    let last = stand_in.seen().pop().expect("a request was sent");
    assert_eq!(last.path, "/api/v1/Token");
    assert_eq!(last.json()["username"], LOCAL_ACCOUNT.0);
}

#[tokio::test]
async fn jellyfin_gives_the_login_of_its_web_client() {
    let stand_in = services::jellyfin().await;
    let http = outbound();
    let url = stand_in.url();
    let address = "https://organizr.example.com/jellyfin";

    let session = jellyfin_session(&http, &url, address, login(None), TIMEOUT)
        .await
        .expect("Jellyfin knows robin");

    let seen = &stand_in.seen()[0];
    assert_eq!(seen.json()["Username"], "robin");
    assert_eq!(seen.json()["Pw"], "the password of robin in organizr");
    assert_eq!(seen.header("x-forwarded-for"), Some("203.0.113.9"));
    // Jellyfin 12 refuses the device in `X-Emby-Authorization` (D-99).
    let device = seen.header("authorization").expect("the device is named");
    assert!(device.contains("DeviceId=\"Organizr_SSO\""), "{device}");
    let JellyfinSession {
        user_key,
        user,
        credentials,
    } = session;
    assert_eq!(
        user_key,
        format!("user-jellyfin-user-robin-{JELLYFIN_SERVER_ID}")
    );
    let user: serde_json::Value = serde_json::from_str(&user).expect("JSON");
    assert_eq!(user["Name"], "Robin");
    let credentials: serde_json::Value = serde_json::from_str(&credentials).expect("JSON");
    assert_eq!(
        credentials,
        serde_json::json!({"Servers": [{
            "ManualAddress": address,
            "Id": JELLYFIN_SERVER_ID,
            "UserId": "jellyfin-user-robin",
            "AccessToken": "jellyfin-token-of-robin",
        }]})
    );

    // A password that Jellyfin does not take gives no session.
    let wrong = Login {
        password: "another password",
        ..login(None)
    };
    assert_eq!(
        jellyfin_session(&http, &url, address, wrong, TIMEOUT).await,
        None
    );
}

#[tokio::test]
async fn petio_gives_a_token_for_a_plex_token_or_for_the_fallback_account() {
    let stand_in = services::petio().await;
    let http = outbound();
    let url = stand_in.url();

    let token = petio_token(&http, &url, login(Some(PLEX_USERS[1].token)), None, TIMEOUT).await;
    assert_eq!(token.as_deref(), Some("petio-token-of-robin"));
    let seen = &stand_in.seen()[0];
    assert_eq!(seen.path, "/api/login/plex_login");
    assert_eq!(seen.json()["token"], PLEX_USERS[1].token);
    // The name and the password are empty with a Plex token, as in PHP.
    assert_eq!(seen.json()["user"]["password"], "");
    assert_eq!(seen.json()["user"]["type"], 1);
    assert_eq!(seen.json()["authToken"], false);
    assert_eq!(seen.header("x-forwarded-for"), Some("203.0.113.9"));

    // Without a Plex token: the name and the password, which Petio does not know here.
    let none = petio_token(&http, &url, login(None), None, TIMEOUT).await;
    assert_eq!(none, None);
    let seen = &stand_in.seen()[1];
    assert_eq!(seen.path, "/api/login");
    assert_eq!(seen.json()["user"]["username"], "robin");
    assert_eq!(
        seen.json()["user"]["password"],
        "the password of robin in organizr"
    );

    // The fallback account is tried after the user, with its name and password.
    let token = petio_token(&http, &url, login(None), Some(FALLBACK), TIMEOUT).await;
    assert_eq!(token.as_deref(), Some("petio-token-of-fallback"));
    let last = stand_in.seen().pop().expect("a request was sent");
    assert_eq!(last.path, "/api/login");
    assert_eq!(last.json()["token"], serde_json::Value::Null);
}

#[tokio::test]
async fn overseerr_gives_its_session_cookie() {
    let stand_in = services::overseerr().await;
    let http = outbound();
    let url = stand_in.url();

    let session =
        overseerr_session(&http, &url, login(Some(PLEX_USERS[1].token)), None, TIMEOUT).await;
    // The value of the cookie, decoded, as the browser must send it back.
    assert_eq!(
        session.as_deref(),
        Some("s:overseerr-session-of-robin.sig+nature")
    );
    let seen = &stand_in.seen()[0];
    assert_eq!(seen.path, "/api/v1/auth/plex");
    assert_eq!(seen.json()["authToken"], PLEX_USERS[1].token);

    // Without a Plex token, the e-mail address and the password are sent.
    let none = overseerr_session(&http, &url, login(None), None, TIMEOUT).await;
    assert_eq!(none, None);
    let seen = &stand_in.seen()[1];
    assert_eq!(seen.path, "/api/v1/auth/local");
    assert_eq!(seen.json()["email"], "robin@example.com");

    let fallback = Fallback {
        user: LOCAL_ACCOUNT.1,
        password: LOCAL_ACCOUNT.2,
    };
    let session = overseerr_session(&http, &url, login(None), Some(fallback), TIMEOUT).await;
    assert_eq!(
        session.as_deref(),
        Some("s:overseerr-session-of-fallback.sig+nature")
    );
}

#[tokio::test]
async fn each_tautulli_install_gives_a_session_with_its_id_and_path() {
    let first = services::tautulli("made-up-uuid-one", "").await;
    let second = services::tautulli("made-up-uuid-two", "/tautulli").await;
    let gone = services::tautulli("made-up-uuid-three", "").await;
    let gone_url = gone.url();
    drop(gone);
    let http = outbound();
    let installs = [
        (first.url(), "/".to_owned()),
        (gone_url, "/".to_owned()),
        (format!("{}/tautulli", second.url()), "/tautulli".to_owned()),
    ];

    let sessions =
        tautulli_sessions(&http, &installs, login(Some(PLEX_USERS[1].token)), TIMEOUT).await;

    assert_eq!(
        sessions,
        [
            TautulliSession {
                uuid: "made-up-uuid-one".to_owned(),
                token: "tautulli-session-of-robin".to_owned(),
                path: "/".to_owned(),
            },
            TautulliSession {
                uuid: "made-up-uuid-two".to_owned(),
                token: "tautulli-session-of-robin".to_owned(),
                path: "/tautulli".to_owned(),
            },
        ]
    );
    let seen = &first.seen()[0];
    assert_eq!(seen.path, "/auth/signin");
    assert_eq!(seen.form()["token"], PLEX_USERS[1].token);
    assert_eq!(seen.form()["username"], "");
    assert_eq!(seen.form()["remember_me"], "1");
    assert_eq!(seen.header("user-agent"), Some("Browser/1.0"));
    assert_eq!(second.seen()[0].path, "/tautulli/auth/signin");

    // A user whom Tautulli does not know gets no session.
    let stranger = login(Some(PLEX_USERS[3].token));
    assert_eq!(
        tautulli_sessions(&http, &installs[..1], stranger, TIMEOUT).await,
        []
    );
}
