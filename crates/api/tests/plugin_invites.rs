//! The routes of the invites plugin, against a stand-in for `plex.tv`.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_api::router;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::outbox::Outbox;
use organizr_testkit::services::{
    self, KOMGA_API_KEY, NEXTCLOUD_ADMIN, OWNER_TOKEN, SERVER_ID, StandIn,
};
use organizr_testkit::{NO_LOGIN_LIMIT, installed_settings, seed, state_with};
use serde_json::{Value, json};

const SHARE_PATH: &str = "/api/servers/made-up-server-one/shared_servers/";

/// The settings of an install that uses the plugin with Plex.
fn plugin_settings(more: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut settings = vec![
        ("INVITES-enabled", json!(true)),
        ("INVITES-Auth-include", json!("4")),
        ("plexToken", json!(OWNER_TOKEN)),
        ("plexID", json!(SERVER_ID)),
        ("loginAttempts", json!(NO_LOGIN_LIMIT)),
    ];
    settings.extend(more.iter().cloned());
    settings
}

/// The application with a stand-in in the place of `plex.tv`, and its outbox.
fn app_with(
    database: &Database,
    plex: &StandIn,
    more: &[(&'static str, Value)],
) -> (Router, Outbox) {
    let (mailer, outbox) = Outbox::new();
    let state = state_with(database, installed_settings(&plugin_settings(more)))
        .with_plex_tv(&plex.url())
        .with_mailer(mailer);
    (router(state).expect("the route table is valid"), outbox)
}

async fn invite(app: &Router, who: As<'_>, code: &str) -> Answer {
    let body = json!({"code": code, "username": "Sam", "email": "sam@example.com"});
    call(app, who, "POST", "/plugins/invites", Some(body)).await
}

async fn use_code(app: &Router, code: &str, account: &str) -> Answer {
    let body = json!({"usedby": account});
    call(
        app,
        As::Guest,
        "POST",
        &format!("/plugins/invites/{code}"),
        Some(body),
    )
    .await
}

async fn verify(app: &Router, code: &str) -> Answer {
    call(
        app,
        As::Guest,
        "GET",
        &format!("/plugins/invites/{code}"),
        None,
    )
    .await
}

fn codes(answer: &Answer) -> Vec<String> {
    answer
        .data()
        .as_array()
        .expect("the data is a list")
        .iter()
        .map(|row| row["code"].as_str().expect("a code is a text").to_owned())
        .collect()
}

/// The requests for a share that the stand-in got.
fn shares(plex: &StandIn) -> Vec<services::Seen> {
    plex.seen()
        .into_iter()
        .filter(|seen| seen.path == SHARE_PATH)
        .collect()
}

#[tokio::test]
async fn invite_is_made_listed_and_used_once() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (app, _outbox) = app_with(
            database,
            &plex,
            &[
                ("INVITES-plexLibraries", json!("3,5")),
                ("INVITES-plex-tv-labels", json!("kids")),
            ],
        );

        let made = invite(&app, As::Admin, "AbC123").await;
        assert_eq!(made.status, StatusCode::OK, "{made:?}");
        assert_eq!(
            made.message(),
            json!("Invite Code: AbC123 has been created")
        );
        let again = invite(&app, As::Admin, "abc123").await;
        assert_eq!(again.status, StatusCode::CONFLICT);
        assert_eq!(again.message(), json!("Invite Code: abc123 already exists"));

        let list = call(&app, As::Admin, "GET", "/plugins/invites", None).await;
        let row = &list.data()[0];
        assert_eq!(row["code"], json!("AbC123"));
        assert_eq!(row["username"], json!("Sam"));
        assert_eq!(row["email"], json!("sam@example.com"));
        assert_eq!(row["valid"], json!("Yes"));
        assert_eq!(row["type"], json!("plex"));
        assert_eq!(row["dateused"], Value::Null);
        assert_eq!(row["date"].as_str().map(str::len), Some(19));

        // A guest asks whether the code is good, in any letter case, and uses it.
        let good = verify(&app, "ABC123").await;
        assert_eq!(good.status, StatusCode::OK);
        assert_eq!(good.message(), json!("Code has been verified"));
        let used = use_code(&app, "abc123", "sam@example.com").await;
        assert_eq!(used.status, StatusCode::OK, "{used:?}");
        assert_eq!(used.message(), json!("Plex User now has access to system"));

        let asked = shares(&plex);
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].header("x-plex-token"), Some(OWNER_TOKEN));
        assert_eq!(
            asked[0].json(),
            json!({
                "server_id": SERVER_ID,
                "shared_server": {
                    "library_section_ids": ["3", "5"],
                    "invited_email": "sam@example.com",
                },
                "sharing_settings": {
                    "filterTelevision": "label=kids",
                    "filterMovies": "",
                    "filterMusic": "",
                },
            })
        );

        let list = call(&app, As::Admin, "GET", "/plugins/invites", None).await;
        let row = &list.data()[0];
        assert_eq!(row["valid"], json!("No"));
        assert_eq!(row["usedby"], json!("sam@example.com"));
        assert_eq!(row["dateused"].as_str().map(str::len), Some(19));
        assert!(row["ip"].is_string());

        // A used code is as wrong as a code that never was.
        for code in ["AbC123", "nothing", "bad%20code"] {
            let answer = verify(&app, code).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{code}");
            assert_eq!(answer.message(), json!("Code is invalid"));
            let answer = use_code(&app, code, "sam@example.com").await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{code}");
        }
        assert_eq!(shares(&plex).len(), 1);
    })
    .await;
}

#[tokio::test]
async fn share_without_libraries_sends_the_empty_text_as_php_does() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (app, _outbox) = app_with(database, &plex, &[]);
        invite(&app, As::Admin, "one").await;
        let used = use_code(&app, "one", " Sam ").await;
        assert_eq!(used.status, StatusCode::OK, "{used:?}");
        let body = shares(&plex)[0].json();
        assert_eq!(body["shared_server"]["library_section_ids"], json!(""));
        assert_eq!(body["shared_server"]["invited_email"], json!("Sam"));
    })
    .await;
}

#[tokio::test]
async fn used_invite_makes_a_komga_account_for_the_address_of_the_invite() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let komga = services::komga().await;
        let (app, _outbox) = app_with(
            database,
            &plex,
            &[
                ("INVITES-komga-enabled", json!(true)),
                ("INVITES-komga-uri", json!(komga.url())),
                ("INVITES-komga-api-key", json!(KOMGA_API_KEY)),
                (
                    "INVITES-komga-default-user-password",
                    json!("first password"),
                ),
                // The form joins a list with commas; PHP splits it at semicolons.
                ("INVITES-komga-roles", json!("FILE_DOWNLOAD,PAGE_STREAMING")),
                ("INVITES-komga-libraryIds", json!("0A1B")),
            ],
        );
        invite(&app, As::Admin, "one").await;

        let used = use_code(&app, "one", "Sam").await;

        assert_eq!(used.status, StatusCode::OK, "{used:?}");
        let made = komga
            .seen()
            .into_iter()
            .find(|seen| seen.path == "/api/v2/users")
            .expect("Komga was asked for an account");
        assert_eq!(
            made.json(),
            json!({
                "email": "sam@example.com",
                "password": "first password",
                "roles": ["FILE_DOWNLOAD", "PAGE_STREAMING"],
                "sharedLibraries": {"all": false, "libraryIds": ["0A1B"]},
            })
        );
    })
    .await;
}

/// The settings of Nextcloud in the invites plugin, for the stand-in at `url`.
fn nextcloud_settings(url: &str, plex_sso: bool) -> Vec<(&'static str, Value)> {
    vec![
        ("INVITES-nextcloud-enabled", json!(true)),
        ("INVITES-nextcloud-plex-sso", json!(plex_sso)),
        ("INVITES-nextcloud-url", json!(url)),
        ("INVITES-nextcloud-admin-user", json!(NEXTCLOUD_ADMIN.0)),
        ("INVITES-nextcloud-admin-password", json!(NEXTCLOUD_ADMIN.1)),
        ("INVITES-nextcloud-groups-member", json!("family,friends")),
        ("INVITES-nextcloud-quota", json!("10GB")),
    ]
}

/// The form that the Nextcloud stand-in got for a new user.
fn nextcloud_form(nextcloud: &StandIn) -> Vec<(String, String)> {
    let made = nextcloud
        .seen()
        .into_iter()
        .find(|seen| seen.path == "/ocs/v1.php/cloud/users")
        .expect("Nextcloud was asked for a user");
    form_urlencoded::parse(made.body.as_bytes())
        .into_owned()
        .collect()
}

#[tokio::test]
async fn used_invite_makes_a_nextcloud_user_in_each_group() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let nextcloud = services::nextcloud().await;
        let (app, _outbox) = app_with(
            database,
            &plex,
            &nextcloud_settings(&nextcloud.url(), false),
        );
        invite(&app, As::Admin, "one").await;

        let used = use_code(&app, "one", "Sam").await;

        assert_eq!(used.status, StatusCode::OK, "{used:?}");
        let form = nextcloud_form(&nextcloud);
        let field = |key: &str| -> Vec<&str> {
            form.iter()
                .filter(|(name, _)| name == key)
                .map(|(_, value)| value.as_str())
                .collect()
        };
        assert_eq!(field("userid"), ["sam@example.com"]);
        assert_eq!(field("email"), ["sam@example.com"]);
        assert_eq!(field("displayName"), ["Sam"]);
        assert_eq!(field("groups[]"), ["family", "friends"]);
        assert_eq!(field("quota"), ["10GB"]);
        assert_eq!(field("password")[0].len(), 32);
    })
    .await;
}

#[tokio::test]
async fn nextcloud_user_of_the_plex_app_has_the_id_of_the_plex_invite() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let nextcloud = services::nextcloud().await;
        let (app, _outbox) = app_with(database, &plex, &nextcloud_settings(&nextcloud.url(), true));
        // The stand-in of plex.tv has a waiting invite for robin@example.com.
        let body = json!({"code": "two", "username": "Robin", "email": "robin@example.com"});
        call(&app, As::Admin, "POST", "/plugins/invites", Some(body)).await;

        // The person who uses the code names another Plex account; the id comes from
        // the address of the invite.
        let used = use_code(&app, "two", "Sam").await;

        assert_eq!(used.status, StatusCode::OK, "{used:?}");
        let form = nextcloud_form(&nextcloud);
        assert!(
            form.contains(&("userid".to_owned(), "PlexTv-5550001".to_owned())),
            "{form:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn komga_account_is_not_made_without_its_settings() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let komga = services::komga().await;
        // Switched on, but without roles: PHP's `_checkKomgaVar` stops here.
        let (app, _outbox) = app_with(
            database,
            &plex,
            &[
                ("INVITES-komga-enabled", json!(true)),
                ("INVITES-komga-uri", json!(komga.url())),
                ("INVITES-komga-api-key", json!(KOMGA_API_KEY)),
                (
                    "INVITES-komga-default-user-password",
                    json!("first password"),
                ),
                ("INVITES-komga-libraryIds", json!("0A1B")),
            ],
        );
        invite(&app, As::Admin, "one").await;

        let used = use_code(&app, "one", "Sam").await;

        assert_eq!(used.status, StatusCode::OK, "{used:?}");
        assert!(komga.seen().is_empty(), "Komga is not asked");
    })
    .await;
}

#[tokio::test]
async fn refusal_of_plex_leaves_the_code_for_another_try() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        // A setting that is wrong, the account, and what the answer says.
        let cases = [
            (Some(("plexToken", "wrong")), "sam", "Incorrect Token"),
            (
                Some(("plexID", "another-server")),
                "sam",
                "Libraries not setup correct",
            ),
            (
                Some(("INVITES-plexLibraries", "404")),
                "sam",
                "Libraries not setup correct",
            ),
            (None, "broken", "An Error Occurred"),
        ];
        for (number, (wrong, account, message)) in cases.into_iter().enumerate() {
            let settings: Vec<(&'static str, Value)> = wrong
                .map(|(key, value)| (key, json!(value)))
                .into_iter()
                .collect();
            let (app, _outbox) = app_with(database, &plex, &settings);
            let code = format!("code{number}");
            invite(&app, As::Admin, &code).await;
            let answer = use_code(&app, &code, account).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{message}");
            assert_eq!(answer.message(), json!(message));
            // The code is still good.
            assert_eq!(
                verify(&app, &code).await.status,
                StatusCode::OK,
                "{message}"
            );
            let list = call(&app, As::Admin, "GET", "/plugins/invites", None).await;
            let row = &list.data()[number];
            assert_eq!(row["valid"], json!("Yes"));
            assert_eq!(row["usedby"], Value::Null);
            assert_eq!(row["ip"], Value::Null);
        }

        // An account that has the access uses the code up.
        let (app, _outbox) = app_with(database, &plex, &[]);
        invite(&app, As::Admin, "last").await;
        let answer = use_code(&app, "last", "ROBIN@example.com").await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), json!("Plex User already has access"));
        assert_eq!(verify(&app, "last").await.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn code_is_not_used_when_the_request_cannot_be_served() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (app, _outbox) = app_with(database, &plex, &[]);
        invite(&app, As::Admin, "one").await;
        for body in [json!({}), json!({"usedby": "  "})] {
            let answer = call(&app, As::Guest, "POST", "/plugins/invites/one", Some(body)).await;
            assert_eq!(answer.status, StatusCode::CONFLICT);
            assert_eq!(
                answer.message(),
                json!("Plex username or email not supplied")
            );
        }
        for missing in ["plexToken", "plexID"] {
            let (app, _outbox) = app_with(database, &plex, &[(missing, json!(""))]);
            let answer = use_code(&app, "one", "sam").await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{missing}");
            assert_eq!(answer.message(), json!("Plex Token/ID not set"));
        }
        assert_eq!(verify(&app, "one").await.status, StatusCode::OK);
        assert_eq!(shares(&plex).len(), 0);

        // For a server that is not Plex, the code is used and nothing is asked.
        let (app, _outbox) = app_with(database, &plex, &[("INVITES-type-include", json!("emby"))]);
        let answer = call(
            &app,
            As::Guest,
            "POST",
            "/plugins/invites/one",
            Some(json!({})),
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{answer:?}");
        assert_eq!(answer.message(), json!("User now has access to system"));
        assert_eq!(verify(&app, "one").await.status, StatusCode::UNAUTHORIZED);
        assert_eq!(shares(&plex).len(), 0);
    })
    .await;
}

#[tokio::test]
async fn member_sees_and_deletes_the_own_invites_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (app, _outbox) = app_with(database, &plex, &[("INVITES-maximum-invites", json!("2"))]);
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        let zoe = seed::logged_in(database, &app, "zoe", 4).await;
        let boss = seed::logged_in(database, &app, "pat", 1).await;
        let guest = seed::logged_in(database, &app, "zed", 999).await;

        assert_eq!(
            invite(&app, As::User(&robin), "r1").await.status,
            StatusCode::OK
        );
        assert_eq!(
            invite(&app, As::User(&robin), "r2").await.status,
            StatusCode::OK
        );
        assert_eq!(
            invite(&app, As::User(&zoe), "z1").await.status,
            StatusCode::OK
        );
        // The limit is for each member, and not for a Co-Admin.
        let over = invite(&app, As::User(&robin), "r3").await;
        assert_eq!(over.status, StatusCode::CONFLICT);
        assert_eq!(over.message(), json!("Maximum number of invites reached"));
        for code in ["p1", "p2", "p3"] {
            assert_eq!(
                invite(&app, As::User(&boss), code).await.status,
                StatusCode::OK
            );
        }

        let list = |who| call(&app, who, "GET", "/plugins/invites", None);
        assert_eq!(codes(&list(As::User(&robin)).await), ["r1", "r2"]);
        assert_eq!(codes(&list(As::User(&zoe)).await), ["z1"]);
        assert_eq!(codes(&list(As::User(&boss)).await).len(), 6);
        let rows = list(As::User(&robin)).await.data();
        assert_eq!(rows[0]["invitedby"], json!("robin"));

        // The routes of the members are closed for a level with less access.
        for who in [As::Guest, As::User(&guest)] {
            assert_eq!(list(who).await.status, StatusCode::UNAUTHORIZED);
            assert_eq!(
                invite(&app, who, "g1").await.status,
                StatusCode::UNAUTHORIZED
            );
            let answer = call(&app, who, "DELETE", "/plugins/invites/r1", None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        }

        // A member deletes nothing until the setting allows it.
        let refused = call(
            &app,
            As::User(&robin),
            "DELETE",
            "/plugins/invites/r1",
            None,
        )
        .await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert_eq!(
            refused.message(),
            json!("You are not permitted to delete invites.")
        );
        let (app, _outbox) = app_with(
            database,
            &plex,
            &[("INVITES-allow-delete-include", json!(true))],
        );
        let other = call(
            &app,
            As::User(&robin),
            "DELETE",
            "/plugins/invites/z1",
            None,
        )
        .await;
        assert_eq!(other.status, StatusCode::NOT_FOUND);
        assert_eq!(other.message(), json!("Code not found"));
        let own = call(
            &app,
            As::User(&robin),
            "DELETE",
            "/plugins/invites/R1",
            None,
        )
        .await;
        assert_eq!(own.status, StatusCode::OK, "{own:?}");
        assert_eq!(own.message(), json!("Code has been deleted"));
        let any = call(&app, As::User(&boss), "DELETE", "/plugins/invites/z1", None).await;
        assert_eq!(any.status, StatusCode::OK);
        let gone = call(&app, As::User(&boss), "DELETE", "/plugins/invites/z1", None).await;
        assert_eq!(gone.status, StatusCode::NOT_FOUND);
        let left = call(&app, As::Admin, "GET", "/plugins/invites", None).await;
        assert_eq!(codes(&left), ["r2", "p1", "p2", "p3"]);
    })
    .await;
}

#[tokio::test]
async fn invite_needs_a_code_a_name_and_an_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (app, _outbox) = app_with(database, &plex, &[]);
        let long = "a".repeat(256);
        let cases = [
            (
                json!({"username": "Sam", "email": "sam@example.com"}),
                "Code not supplied",
            ),
            (
                json!({"code": "0", "username": "Sam", "email": "sam@example.com"}),
                "Code not supplied",
            ),
            (
                json!({"code": "one", "email": "sam@example.com"}),
                "Username not supplied",
            ),
            (
                json!({"code": "one", "username": "Sam", "email": " "}),
                "Email not supplied",
            ),
            (
                json!({"code": "a/b", "username": "Sam", "email": "sam@example.com"}),
                "Code is not valid",
            ),
            (
                json!({"code": "a".repeat(51), "username": "Sam", "email": "sam@example.com"}),
                "Code is not valid",
            ),
            (
                json!({"code": "one", "username": long, "email": "sam@example.com"}),
                "String is over limit of: 255",
            ),
            (
                json!({"code": "one", "username": "Sam", "email": "no address"}),
                "Email is not a valid email",
            ),
        ];
        for (body, message) in cases {
            let answer = call(&app, As::Admin, "POST", "/plugins/invites", Some(body)).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{message}");
            assert_eq!(answer.message(), json!(message));
        }
        let list = call(&app, As::Admin, "GET", "/plugins/invites", None).await;
        assert_eq!(list.data(), json!([]));
    })
    .await;
}

#[tokio::test]
async fn invited_person_gets_an_e_mail_with_the_code_when_the_mailer_is_on() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let mailer = [
            ("PHPMAILER-enabled", json!(true)),
            ("PHPMAILER-smtpHost", json!("smtp.example.com")),
            ("PHPMAILER-domain", json!("https://organizr.example.com")),
        ];
        let (app, outbox) = app_with(database, &plex, &mailer);
        assert_eq!(
            invite(&app, As::Admin, "AbC123").await.status,
            StatusCode::OK
        );
        let mails = outbox.wait_for(1).await;
        let to = mails[0].to.as_ref().expect("the e-mail has a recipient");
        assert_eq!(to.address, "sam@example.com");
        assert_eq!(to.name.as_deref(), Some("Sam"));
        assert!(mails[0].html.contains("AbC123"), "{}", mails[0].html);
        assert!(
            mails[0].html.contains("?invite=AbC123"),
            "{}",
            mails[0].html
        );

        // Without the mailer, the invite is made and no e-mail is sent, also when
        // an SMTP server is set.
        let (app, outbox) = app_with(database, &plex, &mailer[1..]);
        assert_eq!(invite(&app, As::Admin, "two").await.status, StatusCode::OK);
        assert_eq!(outbox.after_a_moment().await.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn wrong_codes_lock_the_client_out_as_failed_logins_do() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let limit = [
            ("loginAttempts", json!("3")),
            ("loginLockout", json!("60000")),
        ];
        let (app, _outbox) = app_with(database, &plex, &limit);
        invite(&app, As::Admin, "good").await;

        assert_eq!(verify(&app, "bad1").await.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            use_code(&app, "bad2", "sam").await.status,
            StatusCode::UNAUTHORIZED
        );
        // The third wrong code locks, and then the good code is refused too.
        let third = verify(&app, "bad3").await;
        assert_eq!(third.status, StatusCode::FORBIDDEN);
        assert_eq!(third.message(), json!("User is locked out"));
        assert_eq!(verify(&app, "good").await.status, StatusCode::FORBIDDEN);
        assert_eq!(
            use_code(&app, "good", "sam").await.status,
            StatusCode::FORBIDDEN
        );
        assert_eq!(shares(&plex).len(), 0);
    })
    .await;
}

#[tokio::test]
async fn member_has_no_limit_of_invites_when_the_setting_is_zero() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (app, _outbox) = app_with(database, &plex, &[]);
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        for code in ["r1", "r2", "r3"] {
            let made = invite(&app, As::User(&robin), code).await;
            assert_eq!(made.status, StatusCode::OK, "{code}");
        }
    })
    .await;
}

#[tokio::test]
async fn plugin_that_is_off_answers_no_route() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (app, _outbox) = app_with(database, &plex, &[("INVITES-enabled", json!(false))]);
        let requests = [
            ("GET", "/plugins/invites"),
            ("POST", "/plugins/invites"),
            ("GET", "/plugins/invites/one"),
            ("POST", "/plugins/invites/one"),
            ("DELETE", "/plugins/invites/one"),
        ];
        for (method, path) in requests {
            let answer = call(&app, As::Admin, method, path, Some(json!({}))).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{method} {path}");
            assert_eq!(
                answer.message(),
                json!("INVITES-enabled module is not enabled")
            );
        }
    })
    .await;
}
