//! Logging in through Plex, and the sessions that a login gets from other applications,
//! against stand-ins for the services.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_api::router;
use organizr_auth::password::{self, PasswordCheck};
use organizr_auth::secrets::plex_token_name;
use organizr_auth::two_factor::code_at;
use organizr_db::Database;
use organizr_domain::{GroupLevel, LevelRange, NewTab, TabKind};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::outbox::Outbox;
use organizr_testkit::services::{
    self, LOCAL_ACCOUNT, OWNER_TOKEN, PLEX_USERS, SERVER_ID, StandIn,
};
use organizr_testkit::{installed_settings, seed, state_with};
use serde_json::{Value, json};

const ROBIN: &services::PlexUser = &PLEX_USERS[1];
const PAT: &services::PlexUser = &PLEX_USERS[2];
const STRANGER: &services::PlexUser = &PLEX_USERS[3];

/// The settings of an install that checks passwords with Plex.
fn plex_settings(more: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut settings = vec![
        ("authType", json!("external")),
        ("authBackend", json!("plex")),
        ("plexToken", json!(OWNER_TOKEN)),
        ("plexID", json!(SERVER_ID)),
        ("loginAttempts", json!("1000000")),
    ];
    settings.extend(more.iter().cloned());
    settings
}

/// The application with a stand-in in the place of `plex.tv`.
fn app_with(database: &Database, plex: &StandIn, settings: &[(&'static str, Value)]) -> Router {
    let state = state_with(database, installed_settings(settings)).with_plex_tv(&plex.url());
    router(state).expect("the route table is valid")
}

async fn log_in(app: &Router, body: Value) -> Answer {
    call(app, As::Guest, "POST", "/login", Some(body)).await
}

async fn log_in_with_password(app: &Router, name: &str, password: &str) -> Answer {
    log_in(app, json!({"username": name, "password": password})).await
}

async fn log_in_with_token(app: &Router, token: &str) -> Answer {
    log_in(app, json!({"oAuth": token, "oAuthType": "plex"})).await
}

/// The value of a cookie that an answer sets, and the whole `Set-Cookie` text.
fn cookie<'a>(answer: &'a Answer, name: &str) -> Option<&'a str> {
    answer
        .cookies
        .iter()
        .map(String::as_str)
        // An answer drops the session cookie at other paths before it sets it (D-145).
        .find(|cookie| {
            cookie.starts_with(&format!("{name}="))
                && !(name.starts_with("organizr_token_") && cookie.contains("=;"))
        })
}

async fn user(database: &Database, name: &str) -> Option<organizr_domain::User> {
    database
        .users()
        .find_by_username(name)
        .await
        .expect("the lookup works")
}

fn has_password(user: &organizr_domain::User, password: &str) -> bool {
    matches!(
        password::verify(password, &user.password_hash),
        PasswordCheck::Correct { .. }
    )
}

#[tokio::test]
async fn friend_of_the_server_logs_in_with_the_plex_password_and_gets_an_account() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let app = app_with(database, &plex, &plex_settings(&[]));

        let answer = log_in_with_password(&app, "robin@example.com", ROBIN.password).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert!(
            cookie(
                &answer,
                "organizr_token_5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21"
            )
            .is_some()
        );
        // The account has the name and the address of the Plex account, in the group of
        // new users.
        let robin = user(database, "Robin").await.expect("the account was made");
        assert_eq!(robin.email.as_deref(), Some("robin@example.com"));
        let group = database
            .groups()
            .find_default()
            .await
            .expect("it loads")
            .expect("one");
        assert_eq!(robin.group, group.id);
        // The Plex token of the user is kept, sealed.
        let login = database
            .users()
            .login_of(robin.id)
            .await
            .expect("it loads")
            .expect("the user is there");
        let sealed = login.sealed_plex_token.expect("a token is stored");
        assert!(!sealed.contains(ROBIN.token));
        assert_eq!(
            seed::secrets()
                .open(&plex_token_name(robin.id), &sealed)
                .expect("it opens"),
            ROBIN.token
        );

        // The next login finds the account, by the name in any letter case.
        let again = log_in_with_password(&app, "ROBIN", ROBIN.password).await;
        assert_eq!(again.status, StatusCode::OK, "{}", again.body);
        assert_eq!(database.users().list().await.expect("it loads").len(), 2);
        // What Plex asked: the id of this install names the device.
        let asked = &plex.seen()[0];
        assert_eq!(asked.path, "/users/sign_in.json");
        assert_eq!(
            asked.header("x-plex-client-identifier"),
            Some(seed::INSTALL_ID)
        );
    })
    .await;
}

#[tokio::test]
async fn only_accounts_that_the_owner_shared_the_server_with_log_in() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let strict = app_with(database, &plex, &plex_settings(&[]));

        // A Plex account that is no friend, and a friend of another server of the owner.
        for account in [STRANGER, PAT] {
            let refused = log_in_with_password(&strict, account.username, account.password).await;
            assert_eq!(
                refused.status,
                StatusCode::UNAUTHORIZED,
                "{}",
                account.username
            );
            assert_eq!(refused.message(), json!("User credentials incorrect"));
            assert_eq!(user(database, account.username).await, None);
        }
        // A wrong password of a friend.
        let refused = log_in_with_password(&strict, "robin", "not the password").await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);

        // Without the strict check, every friend of the owner logs in.
        let lenient = app_with(
            database,
            &plex,
            &plex_settings(&[("plexStrictFriends", json!(false))]),
        );
        let passed = log_in_with_password(&lenient, PAT.username, PAT.password).await;
        assert_eq!(passed.status, StatusCode::OK, "{}", passed.body);
        let refused = log_in_with_password(&lenient, STRANGER.username, STRANGER.password).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);

        // The owner is nobody's friend. The `plexAdmin` setting names the owner, by the
        // name or by the address.
        let owner = &PLEX_USERS[0];
        let refused = log_in_with_password(&strict, owner.username, owner.password).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        for admin in ["OWNER", "Owner@Example.com"] {
            let app = app_with(
                database,
                &plex,
                &plex_settings(&[("plexAdmin", json!(admin))]),
            );
            let passed = log_in_with_password(&app, owner.username, owner.password).await;
            assert_eq!(passed.status, StatusCode::OK, "{admin}: {}", passed.body);
        }
    })
    .await;
}

#[tokio::test]
async fn setting_decides_whether_the_stored_password_counts() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let admin = (seed::ADMIN_NAME, seed::ADMIN_PASSWORD);

        // `external`: only Plex counts. The Admin of the install has no Plex account.
        let external = app_with(database, &plex, &plex_settings(&[]));
        let refused = log_in_with_password(&external, admin.0, admin.1).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);

        // `both`: Plex first, then the stored password.
        let both = app_with(
            database,
            &plex,
            &plex_settings(&[("authType", json!("both"))]),
        );
        assert_eq!(
            log_in_with_password(&both, admin.0, admin.1).await.status,
            StatusCode::OK
        );
        assert_eq!(
            log_in_with_password(&both, "robin", ROBIN.password)
                .await
                .status,
            StatusCode::OK
        );
        // `internal`: Plex is not asked at all.
        let asked = plex.seen().len();
        let internal = app_with(
            database,
            &plex,
            &plex_settings(&[("authType", json!("internal"))]),
        );
        assert_eq!(
            log_in_with_password(&internal, admin.0, admin.1)
                .await
                .status,
            StatusCode::OK
        );
        assert_eq!(plex.seen().len(), asked);
        // The password that Plex took is the password of the account now, so the login
        // works without Plex too.
        let passed = log_in_with_password(&internal, "robin", ROBIN.password).await;
        assert_eq!(passed.status, StatusCode::OK, "{}", passed.body);

        // Plex is away. `both` still takes the stored passwords, and `external` nobody.
        let url = plex.url();
        drop(plex);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let away = |kind: &str| {
            let settings = plex_settings(&[("authType", json!(kind))]);
            let state = state_with(database, installed_settings(&settings)).with_plex_tv(&url);
            router(state).expect("the route table is valid")
        };
        let passed = log_in_with_password(&away("both"), "robin", ROBIN.password).await;
        assert_eq!(passed.status, StatusCode::OK, "{}", passed.body);
        let refused = log_in_with_password(&away("external"), "robin", ROBIN.password).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn account_of_organizr_is_found_by_the_address_of_the_plex_account() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        // The Admin of the install has the address of the Plex account of the owner, and
        // another name.
        let settings = plex_settings(&[("plexAdmin", json!("owner"))]);
        let app = app_with(database, &plex, &settings);
        let admin = user(database, seed::ADMIN_NAME).await.expect("is there");
        let new_address = json!({"email": "owner@example.com"});
        let path = format!("/users/{}", admin.id.get());
        let changed = call(&app, As::Admin, "PUT", &path, Some(new_address)).await;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);

        let owner = &PLEX_USERS[0];
        let answer = log_in_with_password(&app, owner.email, owner.password).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        // No second account: the login is the one of the Admin.
        assert_eq!(database.users().list().await.expect("it loads").len(), 1);
        let me = call(
            &app,
            As::User(
                cookie(
                    &answer,
                    "organizr_token_5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21",
                )
                .and_then(|cookie| cookie.split(';').next())
                .expect("the login sets the session cookie"),
            ),
            "GET",
            "/token/me",
            None,
        )
        .await;
        assert_eq!(me.data()["username"], json!(seed::ADMIN_NAME));
        assert_eq!(me.data()["groupID"], json!(0));
        // The Plex password does not replace the password of the Admin.
        let admin = user(database, seed::ADMIN_NAME).await.expect("is there");
        assert!(has_password(&admin, seed::ADMIN_PASSWORD));
        assert!(!has_password(&admin, owner.password));
    })
    .await;
}

#[tokio::test]
async fn plex_friend_does_not_get_an_admin_account_of_the_same_name() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let settings = plex_settings(&[("authType", json!("both")), ("plexoAuth", json!(true))]);
        let app = app_with(database, &plex, &settings);
        // The Admin happens to have the name of a Plex account that the server is shared with.
        let admin = user(database, seed::ADMIN_NAME).await.expect("is there");
        let path = format!("/users/{}", admin.id.get());
        let renamed = call(
            &app,
            As::Admin,
            "PUT",
            &path,
            Some(json!({"username": "Robin"})),
        )
        .await;
        assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);

        // Neither the Plex password nor the Plex token of that account opens the Admin.
        let with_password = log_in_with_password(&app, "robin", ROBIN.password).await;
        assert_eq!(
            with_password.status,
            StatusCode::UNAUTHORIZED,
            "{}",
            with_password.body
        );
        let with_token = log_in_with_token(&app, ROBIN.token).await;
        assert_eq!(
            with_token.status,
            StatusCode::UNAUTHORIZED,
            "{}",
            with_token.body
        );

        // The Admin keeps the own password, which still opens the account, and no
        // account was made.
        let admin = user(database, "Robin").await.expect("is there");
        assert!(has_password(&admin, seed::ADMIN_PASSWORD));
        assert_eq!(database.users().list().await.expect("it loads").len(), 1);
        let own = log_in_with_password(&app, "Robin", seed::ADMIN_PASSWORD).await;
        assert_eq!(own.status, StatusCode::OK, "{}", own.body);
    })
    .await;
}

#[tokio::test]
async fn plex_friend_does_not_get_a_co_admin_account_of_the_same_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let app = app_with(
            database,
            &plex,
            &plex_settings(&[("authType", json!("both"))]),
        );
        // A Co-Admin has the address of a Plex account that the server is shared with.
        let _ = seed::logged_in(database, &app, "kim", 1).await;
        let kim = user(database, "kim").await.expect("is there");
        let path = format!("/users/{}", kim.id.get());
        let changed = call(
            &app,
            As::Admin,
            "PUT",
            &path,
            Some(json!({"email": ROBIN.email})),
        )
        .await;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);

        let refused = log_in_with_password(&app, ROBIN.email, ROBIN.password).await;

        assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{}", refused.body);
        let kim = user(database, "kim").await.expect("is there");
        assert!(has_password(&kim, seed::USER_PASSWORD));
        assert_eq!(database.users().list().await.expect("it loads").len(), 2);
    })
    .await;
}

#[tokio::test]
async fn komga_master_password_goes_with_the_e_mail_of_the_plex_account() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let komga = services::komga().await;
        let settings = plex_settings(&[
            ("authType", json!("internal")),
            ("plexoAuth", json!(true)),
            ("plexAdmin", json!("Owner")),
            ("ssoKomga", json!(true)),
            ("ssoKomgaAuth", json!("999")),
            ("komgaURL", json!(komga.url())),
            (
                "komgaSSOMasterPassword",
                json!(services::KOMGA_MASTER_PASSWORD),
            ),
        ]);
        let app = app_with(database, &plex, &settings);
        let first = log_in_with_token(&app, OWNER_TOKEN).await;
        assert_eq!(first.status, StatusCode::OK, "{}", first.body);

        // The owner takes the e-mail of the Komga account of robin, who has no account
        // in Organizr. The master password goes with the e-mail that plex.tv vouches
        // for, so Komga is not asked for robin's account (D-150).
        let owner = user(database, "Owner")
            .await
            .expect("the login made the account");
        let id = owner.id;
        // A login with a token gave the account a random password, so the change of
        // the address goes through the API key (D-185).
        let body = json!({"email": services::KOMGA_ROBIN.0});
        let changed = call(&app, As::Admin, "PUT", &format!("/users/{id}"), Some(body)).await;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);

        let again = log_in_with_token(&app, OWNER_TOKEN).await;
        assert_eq!(again.status, StatusCode::OK, "{}", again.body);
        assert!(
            cookie(&again, "komga_token").is_none(),
            "{:?}",
            again.cookies
        );
    })
    .await;
}

#[tokio::test]
async fn plex_token_logs_in_without_a_password() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        // The stored passwords count, and a login with a Plex token is on.
        let settings = plex_settings(&[("authType", json!("internal")), ("plexoAuth", json!(true))]);
        let app = app_with(database, &plex, &settings);

        let answer = log_in_with_token(&app, ROBIN.token).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert!(cookie(&answer, "organizr_token_5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21").is_some());
        // The interface of PHP reads this cookie to know how the user logged in.
        assert!(cookie(&answer, "oAuth").is_some_and(|cookie| cookie.starts_with("oAuth=true;")));
        let robin = user(database, "Robin").await.expect("the account was made");
        // Nobody knows the password of the new account. It is a hash of a random text.
        assert!(robin.password_hash.expose().starts_with("$argon2id$"));
        assert_eq!(plex.seen()[0].path, "/users/account.json");
        assert_eq!(plex.seen()[0].header("x-plex-token"), Some(ROBIN.token));

        // A token that Plex does not know, and the token of a stranger.
        for token in ["no-such-token", STRANGER.token] {
            let refused = log_in_with_token(&app, token).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{token}");
            assert_eq!(refused.message(), json!("User credentials incorrect"));
        }
        // A kind of token that the application does not know.
        let refused = log_in(&app, json!({"oAuth": ROBIN.token, "oAuthType": "other"})).await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        // An empty token is a login with a password.
        let body = json!({"oAuth": "", "oAuthType": "plex", "username": seed::ADMIN_NAME, "password": seed::ADMIN_PASSWORD});
        assert_eq!(log_in(&app, body).await.status, StatusCode::OK);

        // With the setting off, no token logs in.
        let off = app_with(database, &plex, &plex_settings(&[]));
        let refused = log_in_with_token(&off, ROBIN.token).await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(refused.message(), json!("Plex oAuth is not setup"));
        assert!(cookie(&refused, "oAuth").is_none());
    })
    .await;
}

#[tokio::test]
async fn second_factor_of_organizr_is_asked_after_plex_unless_the_setting_says_no() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let settings = plex_settings(&[("plexoAuth", json!(true))]);
        let app = app_with(database, &plex, &settings);
        // The first login makes the account. Then the user adds a second factor.
        let first = log_in_with_token(&app, ROBIN.token).await;
        let session = cookie(
            &first,
            "organizr_token_5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21",
        )
        .and_then(|cookie| cookie.split(';').next())
        .expect("the login sets the session cookie")
        .to_owned();
        // A login with the Plex password makes it the password of the account, which
        // the second factor asks for (D-186).
        let mirrored = log_in_with_password(&app, "robin", ROBIN.password).await;
        assert_eq!(mirrored.status, StatusCode::OK, "{}", mirrored.body);
        let secret = "JBSWY3DPEHPK3PXP";
        let body = json!({"secret": secret, "type": "google", "password": ROBIN.password});
        let saved = call(&app, As::User(&session), "PUT", "/2fa", Some(body)).await;
        assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock is after 1970")
            .as_secs();
        // A code works for one login, so the second login takes the code of the next
        // step.
        for (step, body) in [
            json!({"oAuth": ROBIN.token, "oAuthType": "plex"}),
            json!({"username": "robin", "password": ROBIN.password}),
        ]
        .into_iter()
        .enumerate()
        {
            let needed = log_in(&app, body.clone()).await;
            assert_eq!(needed.message(), json!("2FA Code Needed"), "{body}");
            let step = u64::try_from(step).expect("the step fits");
            let code = code_at(secret, now + step * 30).expect("the secret is valid");
            let mut with_code = body;
            with_code["tfaCode"] = json!(code);
            assert_eq!(log_in(&app, with_code).await.status, StatusCode::OK);
        }

        // With the setting, a login with a Plex token needs no code. One with the Plex
        // password still does.
        let lenient = plex_settings(&[
            ("plexoAuth", json!(true)),
            ("ignoreTFAIfPlexOAuth", json!(true)),
        ]);
        let app = app_with(database, &plex, &lenient);
        assert_eq!(
            log_in_with_token(&app, ROBIN.token).await.status,
            StatusCode::OK
        );
        let needed = log_in_with_password(&app, "robin", ROBIN.password).await;
        assert_eq!(needed.message(), json!("2FA Code Needed"));
    })
    .await;
}

#[tokio::test]
async fn new_account_from_plex_gets_the_welcome_e_mail() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let settings = plex_settings(&[
            ("PHPMAILER-enabled", json!(true)),
            ("PHPMAILER-smtpHost", json!("smtp.example.com")),
        ]);
        let (mailer, outbox) = Outbox::new();
        let state = state_with(database, installed_settings(&settings))
            .with_plex_tv(&plex.url())
            .with_mailer(mailer);
        let app = router(state).expect("the route table is valid");

        assert_eq!(
            log_in_with_password(&app, "robin", ROBIN.password)
                .await
                .status,
            StatusCode::OK
        );

        let mails = outbox.wait_for(1).await;
        assert_eq!(mails[0].subject, "Thank you For Registering");
        assert_eq!(
            mails[0].to.clone().map(|to| to.address).as_deref(),
            Some("robin@example.com")
        );
        // The second login makes no account, so it sends no welcome.
        log_in_with_password(&app, "robin", ROBIN.password).await;
        assert_eq!(outbox.after_a_moment().await.len(), 1);
    })
    .await;
}

/// The settings that switch the single sign-on of all four applications on.
fn sso_settings(
    ombi: &StandIn,
    overseerr: &StandIn,
    tautulli: &[String],
) -> Vec<(&'static str, Value)> {
    vec![
        ("plexoAuth", json!(true)),
        ("ssoPlex", json!(true)),
        ("ssoOmbi", json!(true)),
        ("ombiURL", json!(ombi.url())),
        ("ssoOverseerr", json!(true)),
        ("overseerrURL", json!(overseerr.url())),
        ("ssoTautulli", json!(true)),
        ("tautulliURL", json!(tautulli.join(","))),
    ]
}

#[tokio::test]
async fn login_with_a_plex_token_gets_a_session_of_each_application() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (ombi, overseerr) = (services::ombi().await, services::overseerr().await);
        let first = services::tautulli("made-up-uuid-one", "").await;
        let second = services::tautulli("made-up-uuid-two", "/tautulli").await;
        let tautulli = [first.url(), format!(" {}/tautulli", second.url())];
        let settings = plex_settings(&sso_settings(&ombi, &overseerr, &tautulli));
        let app = app_with(database, &plex, &settings);

        let answer = log_in_with_token(&app, ROBIN.token).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        // The cookie of Plex Web: the token itself. A script of Plex reads it.
        let plex_cookie = cookie(&answer, "mpt").expect("the Plex cookie is set");
        assert!(
            plex_cookie.starts_with(&format!("mpt={};", ROBIN.token)),
            "{plex_cookie}"
        );
        assert!(!plex_cookie.contains("HttpOnly"));
        assert!(plex_cookie.contains("domain=.example.com"), "{plex_cookie}");
        // Ombi reads its token from this cookie.
        let ombi_cookie = cookie(&answer, "Auth").expect("the Ombi cookie is set");
        assert!(
            ombi_cookie.starts_with("Auth=ombi-token-of-robin;"),
            "{ombi_cookie}"
        );
        // The session of Overseerr, written as a browser stores it.
        let overseerr_cookie = cookie(&answer, "connect.sid").expect("the Overseerr cookie is set");
        assert!(
            overseerr_cookie
                .starts_with("connect.sid=s%3Aoverseerr-session-of-robin.sig%2Bnature;"),
            "{overseerr_cookie}"
        );
        // Each Tautulli install has a cookie with its id, at its own path.
        let one = cookie(&answer, "tautulli_token_made-up-uuid-one").expect("the first cookie");
        assert!(
            one.contains("=tautulli-session-of-robin;") && one.contains("path=/;"),
            "{one}"
        );
        let two = cookie(&answer, "tautulli_token_made-up-uuid-two").expect("the second cookie");
        assert!(two.contains("path=/tautulli;"), "{two}");

        // The applications got the Plex token, and not a password.
        assert_eq!(ombi.seen()[0].json()["plexToken"], ROBIN.token);
        assert_eq!(overseerr.seen()[0].json()["authToken"], ROBIN.token);
        assert_eq!(first.seen()[0].form()["token"], ROBIN.token);
        assert_eq!(second.seen()[0].path, "/tautulli/auth/signin");

        // A logout makes the browser drop them all.
        let session = cookie(
            &answer,
            "organizr_token_5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21",
        )
        .and_then(|cookie| cookie.split(';').next())
        .expect("the session cookie");
        let sent =
            format!("{session}; mpt=x; Auth=x; connect.sid=x; tautulli_token_made-up-uuid-one=x");
        let logout = call(&app, As::User(&sent), "GET", "/logout", None).await;
        for name in [
            "mpt",
            "Auth",
            "connect.sid",
            "oAuth",
            "tautulli_token_made-up-uuid-one",
        ] {
            let dropped = cookie(&logout, name).unwrap_or_else(|| panic!("{name} is dropped"));
            assert!(dropped.starts_with(&format!("{name}=;")), "{dropped}");
        }
    })
    .await;
}

#[tokio::test]
async fn login_with_a_stored_password_asks_the_applications_with_that_password() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (ombi, overseerr) = (services::ombi().await, services::overseerr().await);
        let tautulli = services::tautulli("made-up-uuid-one", "").await;
        let mut settings = sso_settings(&ombi, &overseerr, &[tautulli.url()]);
        settings.extend([
            ("loginAttempts", json!("1000000")),
            ("ombiFallbackUser", json!(LOCAL_ACCOUNT.0)),
            ("ombiFallbackPassword", json!(LOCAL_ACCOUNT.2)),
            // Only Admins and Co-Admins get a session of Tautulli.
            ("ssoTautulliAuth", json!("1")),
        ]);
        let app = app_with(database, &plex, &settings);
        let user = seed::logged_in(database, &app, "zed", 4).await;
        assert_ne!(user, "");

        let answer = log_in_with_password(&app, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        // No Plex token is involved, so there is no Plex cookie.
        assert!(cookie(&answer, "mpt").is_none());
        assert!(cookie(&answer, "oAuth").is_none());
        // Ombi does not know the Admin. The fallback account of the settings gets in.
        let asked = ombi.seen();
        let admin_request = asked
            .iter()
            .find(|request| request.json()["username"] == seed::ADMIN_NAME)
            .expect("Ombi was asked about the Admin");
        assert_eq!(admin_request.path, "/api/v1/Token");
        assert_eq!(admin_request.json()["password"], seed::ADMIN_PASSWORD);
        let ombi_cookie = cookie(&answer, "Auth").expect("the Ombi cookie is set");
        assert!(
            ombi_cookie.starts_with("Auth=ombi-token-of-fallback;"),
            "{ombi_cookie}"
        );
        // Overseerr has no fallback account here and does not know the Admin.
        assert!(cookie(&answer, "connect.sid").is_none());
        assert_eq!(
            overseerr
                .seen()
                .last()
                .map(|seen| seen.json()["email"].clone()),
            Some(json!(seed::ADMIN_EMAIL))
        );

        // Tautulli was asked for the Admin, and not for the user of the level 4.
        let asked: Vec<String> = tautulli
            .seen()
            .iter()
            .map(|request| request.form()["username"].clone())
            .collect();
        assert_eq!(asked, [seed::ADMIN_NAME]);
    })
    .await;
}

#[tokio::test]
async fn application_that_is_away_does_not_stop_the_login() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (ombi, overseerr) = (services::ombi().await, services::overseerr().await);
        let tautulli = services::tautulli("made-up-uuid-one", "").await;
        let settings = plex_settings(&sso_settings(&ombi, &overseerr, &[tautulli.url()]));
        let app = app_with(database, &plex, &settings);
        drop(ombi);
        drop(tautulli);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let answer = log_in_with_token(&app, ROBIN.token).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert!(cookie(&answer, "Auth").is_none());
        assert!(cookie(&answer, "tautulli_token_made-up-uuid-one").is_none());
        // The application that is there gave its session.
        assert!(cookie(&answer, "connect.sid").is_some());
    })
    .await;
}

#[tokio::test]
async fn plex_account_without_an_address_is_not_the_owner_of_an_install_that_names_none() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        // The `plexAdmin` setting is empty, and so is the address of this account.
        let app = app_with(database, &plex, &plex_settings(&[]));
        let nomail = &PLEX_USERS[4];

        let refused = log_in_with_password(&app, nomail.username, nomail.password).await;

        assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{}", refused.body);
        assert_eq!(user(database, nomail.username).await, None);
    })
    .await;
}

#[tokio::test]
async fn refused_plex_token_does_not_fall_back_to_a_password_of_the_same_request() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let settings = plex_settings(&[("authType", json!("both")), ("plexoAuth", json!(true))]);
        let app = app_with(database, &plex, &settings);

        // The name and the password are right. The token is the one of a stranger.
        let body = json!({
            "oAuth": STRANGER.token,
            "oAuthType": "plex",
            "username": seed::ADMIN_NAME,
            "password": seed::ADMIN_PASSWORD,
        });
        let refused = log_in(&app, body).await;

        assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{}", refused.body);
        assert!(
            cookie(
                &refused,
                "organizr_token_5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21"
            )
            .is_none()
        );
    })
    .await;
}

#[tokio::test]
async fn account_that_exists_takes_the_password_that_plex_checked() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let both = app_with(database, &plex, &plex_settings(&[("authType", json!("both"))]));
        // The account exists, with a password of its own.
        let created = call(
            &both,
            As::Admin,
            "POST",
            "/users",
            Some(json!({"username": "robin", "email": "robin@example.com", "password": "an older password"})),
        )
        .await;
        assert_eq!(created.status, StatusCode::OK, "{}", created.body);

        let passed = log_in_with_password(&both, "robin", ROBIN.password).await;
        assert_eq!(passed.status, StatusCode::OK, "{}", passed.body);
        assert_eq!(database.users().list().await.expect("it loads").len(), 2);

        // Without Plex, the account now has the password that Plex took.
        let internal =
            app_with(database, &plex, &plex_settings(&[("authType", json!("internal"))]));
        let passed = log_in_with_password(&internal, "robin", ROBIN.password).await;
        assert_eq!(passed.status, StatusCode::OK, "{}", passed.body);
        let refused = log_in_with_password(&internal, "robin", "an older password").await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn application_with_its_switch_off_is_not_asked() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (ombi, overseerr) = (services::ombi().await, services::overseerr().await);
        let tautulli = services::tautulli("made-up-uuid-one", "").await;
        // The addresses are set, and every switch is off.
        let settings = plex_settings(&[
            ("plexoAuth", json!(true)),
            ("ombiURL", json!(ombi.url())),
            ("overseerrURL", json!(overseerr.url())),
            ("tautulliURL", json!(tautulli.url())),
        ]);
        let app = app_with(database, &plex, &settings);

        let answer = log_in_with_token(&app, ROBIN.token).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        for name in [
            "mpt",
            "Auth",
            "connect.sid",
            "tautulli_token_made-up-uuid-one",
        ] {
            assert!(cookie(&answer, name).is_none(), "{name} is set");
        }
        assert_eq!(ombi.seen(), []);
        assert_eq!(overseerr.seen(), []);
        assert_eq!(tautulli.seen(), []);
    })
    .await;
}

#[tokio::test]
async fn fallback_user_without_a_password_is_no_fallback() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let ombi = services::ombi().await;
        let settings = [
            ("ssoOmbi", json!(true)),
            ("ombiURL", json!(ombi.url())),
            ("ombiFallbackUser", json!(LOCAL_ACCOUNT.0)),
        ];
        let app = app_with(database, &plex, &settings);

        let answer = log_in_with_password(&app, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert!(cookie(&answer, "Auth").is_none());
        // Ombi was asked once, for the user. The fallback user was not tried.
        assert_eq!(ombi.seen().len(), 1);
    })
    .await;
}

#[tokio::test]
async fn servers_of_the_stored_token_are_listed_for_admins() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let app = app_with(database, &plex, &plex_settings(&[("authType", json!("internal"))]));
        let path = "/plex/servers";

        let listed = call(&app, As::Admin, "GET", path, None).await;

        assert_eq!(listed.status, StatusCode::OK, "{}", listed.body);
        assert_eq!(
            listed.data(),
            json!([
                {"name": "Media", "address": "203.0.113.5", "machineIdentifier": SERVER_ID, "owned": 1},
                {"name": "Shared", "address": "198.51.100.9", "machineIdentifier": "made-up-server-nine", "owned": 0},
            ])
        );
        let owned = call(&app, As::Admin, "GET", "/plex/servers?owned", None).await;
        assert_eq!(owned.data().as_array().map(Vec::len), Some(1));

        // Only an Admin or a Co-Admin can ask.
        let user = seed::logged_in(database, &app, "zed", 4).await;
        for who in [As::Guest, As::User(&user)] {
            let refused = call(&app, who, "GET", path, None).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        }

        // No token is stored, and a token that Plex refuses.
        let empty = app_with(database, &plex, &plex_settings(&[("plexToken", json!(""))]));
        let refused = call(&empty, As::Admin, "GET", path, None).await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(refused.message(), json!("Plex Token cannot be empty"));
        let wrong = app_with(database, &plex, &plex_settings(&[("plexToken", json!("another-token"))]));
        let failed = call(&wrong, As::Admin, "GET", path, None).await;
        assert_eq!(failed.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(failed.message(), json!("Plex Error occurred"));
    })
    .await;
}

#[tokio::test]
async fn login_with_a_plex_token_gets_a_session_of_petio() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let petio = services::petio().await;
        let settings = plex_settings(&[
            ("plexoAuth", json!(true)),
            ("ssoPetio", json!(true)),
            ("petioURL", json!(petio.url())),
        ]);
        let app = app_with(database, &plex, &settings);

        let answer = log_in_with_token(&app, ROBIN.token).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let petio_cookie = cookie(&answer, "petio_jwt").expect("the Petio cookie is set");
        assert!(
            petio_cookie.starts_with("petio_jwt=petio-token-of-robin;"),
            "{petio_cookie}"
        );
        assert!(!petio_cookie.contains("HttpOnly"), "{petio_cookie}");
        assert_eq!(petio.seen()[0].json()["token"], ROBIN.token);
    })
    .await;
}

/// The requests to the user import of a stand-in for Ombi, after the background task
/// that sends them had time to run.
async fn ombi_imports(ombi: &StandIn, wanted: usize) -> Vec<services::Seen> {
    for _ in 0..100 {
        let imports: Vec<services::Seen> = ombi
            .seen()
            .into_iter()
            .filter(|seen| seen.path.starts_with("/api/v1/Job/"))
            .collect();
        if imports.len() >= wanted {
            return imports;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    ombi.seen()
}

#[tokio::test]
async fn first_login_with_plex_has_ombi_import_the_plex_users() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let ombi = services::ombi().await;
        let app = app_with(
            database,
            &plex,
            &plex_settings(&[
                ("ombiURL", json!(ombi.url())),
                ("ombiToken", json!("the key of ombi")),
            ]),
        );

        let answer = log_in_with_password(&app, "robin@example.com", ROBIN.password).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let imports = ombi_imports(&ombi, 1).await;
        assert_eq!(imports.len(), 1, "{imports:?}");
        let import = &imports[0];
        assert_eq!(import.method, "POST");
        assert_eq!(import.path, "/api/v1/Job/plexuserimporter");
        assert_eq!(import.header("apikey"), Some("the key of ombi"));

        // A login of an account that exists imports nothing.
        let again = log_in_with_password(&app, "robin", ROBIN.password).await;
        assert_eq!(again.status, StatusCode::OK, "{}", again.body);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert_eq!(ombi_imports(&ombi, 2).await.len(), 1);
    })
    .await;
}

/// Stores an enabled tab for the levels from Admin down to `least_access`.
async fn store_tab(database: &Database, name: &str, url: &str, least_access: u16) {
    let category = database
        .categories()
        .list()
        .await
        .expect("the categories can be read")
        .into_iter()
        .next()
        .expect("the seed has a category");
    let least_access = GroupLevel::new(least_access).expect("the level is valid");
    database
        .tabs()
        .insert(NewTab {
            position: 10,
            category: category.id,
            name: name.to_owned(),
            url: Some(url.to_owned()),
            url_local: None,
            is_default: false,
            enabled: true,
            access: LevelRange::up_to(least_access),
            add_to_admin: false,
            image: None,
            kind: TabKind::Iframe,
            splash: false,
            ping: false,
            ping_url: None,
            timeout: false,
            timeout_ms: None,
            preload: false,
        })
        .await
        .expect("the tab can be stored");
}

#[tokio::test]
async fn fallback_accounts_go_only_to_users_who_see_the_application() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let (ombi, overseerr) = (services::ombi().await, services::overseerr().await);
        let petio = services::petio().await;
        let mut settings = vec![
            ("loginAttempts", json!("1000000")),
            ("ssoOmbi", json!(true)),
            ("ombiURL", json!(ombi.url())),
            ("ombiFallbackUser", json!(LOCAL_ACCOUNT.0)),
            ("ombiFallbackPassword", json!(LOCAL_ACCOUNT.2)),
            ("ssoOverseerr", json!(true)),
            ("overseerrURL", json!(overseerr.url())),
            ("overseerrFallbackUser", json!(LOCAL_ACCOUNT.1)),
            ("overseerrFallbackPassword", json!(LOCAL_ACCOUNT.2)),
            ("ssoPetio", json!(true)),
            ("petioURL", json!(petio.url())),
            ("petioFallbackUser", json!(LOCAL_ACCOUNT.0)),
            ("petioFallbackPassword", json!(LOCAL_ACCOUNT.2)),
        ];
        let app = app_with(database, &plex, &settings);
        seed::logged_in(database, &app, "zed", 4).await;
        // A tab of Petio that only Co-Admins see.
        store_tab(database, "Petio", "https://requests.example.com", 1).await;
        let fallback_asked = || {
            ombi.seen()
                .iter()
                .chain(&overseerr.seen())
                .chain(&petio.seen())
                .filter(|seen| seen.body.contains(LOCAL_ACCOUNT.2))
                .count()
        };

        // The homepage items of Ombi and Overseerr are for Co-Admins, as by default, and
        // the user does not see the tab of Petio: no fallback account.
        let answer = log_in_with_password(&app, "zed", seed::USER_PASSWORD).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        for name in ["Auth", "connect.sid", "petio_jwt"] {
            assert!(cookie(&answer, name).is_none(), "{name} is set");
        }
        assert_eq!(fallback_asked(), 0);

        // The Admin still gets them.
        let answer = log_in_with_password(&app, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        assert!(cookie(&answer, "Auth").is_some_and(|c| c.contains("ombi-token-of-fallback")));
        assert!(cookie(&answer, "petio_jwt").is_some_and(|c| c.contains("of-fallback")));

        // With the homepage items for users, and a tab of Petio that users see, the user
        // gets the fallback accounts.
        settings.extend([
            ("homepageOmbiAuth", json!("4")),
            ("homepageOverseerrAuth", json!("4")),
        ]);
        store_tab(database, "Requests", "https://example.com/petio/", 4).await;
        let app = app_with(database, &plex, &settings);
        let answer = log_in_with_password(&app, "zed", seed::USER_PASSWORD).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert!(
            cookie(&answer, "Auth").is_some_and(|c| c.starts_with("Auth=ombi-token-of-fallback;"))
        );
        assert!(
            cookie(&answer, "connect.sid")
                .is_some_and(|c| c.contains("overseerr-session-of-fallback"))
        );
        assert!(
            cookie(&answer, "petio_jwt")
                .is_some_and(|c| c.starts_with("petio_jwt=petio-token-of-fallback;"))
        );
    })
    .await;
}
