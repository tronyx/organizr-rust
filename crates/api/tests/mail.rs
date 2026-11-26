//! Password recovery and the e-mails of the application, through the real route table:
//! what the recorded PHP answers cannot show (D-58).

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
use organizr_domain::{NewUser, PasswordHash, Timestamp};
use organizr_plugins::mail::Message;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call, request, send};
use organizr_testkit::outbox::Outbox;
use organizr_testkit::{installed_app_with_outbox, installed_settings, seed, state_with};
use serde_json::{Value, json};

/// The settings of an install whose mail plugin is on.
fn mail_on(more: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut settings = vec![
        ("PHPMAILER-enabled", json!(true)),
        ("PHPMAILER-smtpHost", json!("smtp.example.com")),
        ("PHPMAILER-template", json!("dark")),
        ("loginAttempts", json!("1000000")),
    ];
    settings.extend(more.iter().cloned());
    settings
}

/// Stores a user in the group "User".
async fn add_user(database: &Database, name: &str, email: Option<&str>, password_hash: &str) {
    let group = database
        .groups()
        .find_default()
        .await
        .expect("the lookup works")
        .expect("the install has a group for new users");
    database
        .users()
        .insert(NewUser {
            username: name.to_owned(),
            email: email.map(str::to_owned),
            password_hash: PasswordHash::new(password_hash.to_owned()),
            group: group.id,
            image: None,
            created_at: Timestamp::from_unix_seconds(1_800_000_000),
        })
        .await
        .expect("the user can be stored");
}

fn hash(password: &str) -> String {
    organizr_auth::password::hash(password)
        .expect("the password can be hashed")
        .expose()
        .to_owned()
}

async fn recover(app: &Router, email: &str) -> Answer {
    let body = json!({ "email": email });
    call(app, As::Guest, "POST", "/users/recover", Some(body)).await
}

async fn log_in(app: &Router, name: &str, password: &str) -> Answer {
    let body = json!({"username": name, "password": password});
    call(app, As::Guest, "POST", "/login", Some(body)).await
}

/// The new password of an e-mail of the recovery.
fn password_in(mail: &Message) -> String {
    mail.html
        .split_once("Here is your new password: ")
        .and_then(|(_, rest)| rest.split_once('<'))
        .map(|(password, _)| password.to_owned())
        .expect("the e-mail holds a new password")
}

#[tokio::test]
async fn old_password_works_until_the_new_one_is_used() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_user(
            database,
            "robin",
            Some("Robin@Example.com"),
            &hash("the old password"),
        )
        .await;
        let (app, outbox) = installed_app_with_outbox(database, &mail_on(&[]));

        let answer = recover(&app, "robin@example.COM").await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let mails = outbox.wait_for(1).await;
        let to = mails[0].to.clone().expect("the e-mail has a recipient");
        assert_eq!(
            (to.address.as_str(), to.name.as_deref()),
            ("robin@example.COM", Some("robin"))
        );
        assert_eq!(mails[0].subject, "Password Reset");
        let new_password = password_in(&mails[0]);
        assert_eq!(new_password.len(), 12);

        // A stranger who asked for the e-mail did not lock the account.
        assert_eq!(
            log_in(&app, "robin", "the old password").await.status,
            StatusCode::OK
        );
        assert_eq!(
            log_in(&app, "robin", "a wrong one").await.status,
            StatusCode::UNAUTHORIZED
        );
        // The password from the e-mail works, and is the password of the account then.
        // The sessions from before end, as somebody else may have one (D-146).
        let before = log_in(&app, "robin", "the old password").await;
        let before = before
            .cookies
            .iter()
            .find(|cookie| cookie.starts_with("organizr_token_") && !cookie.contains("=;"))
            .and_then(|cookie| cookie.split(';').next())
            .expect("the login sets the session cookie")
            .to_owned();
        assert_eq!(
            log_in(&app, "robin", &new_password).await.status,
            StatusCode::OK
        );
        let me = call(&app, As::User(&before), "GET", "/token/me", None).await;
        assert_ne!(me.data()["loggedin"], json!(true), "{}", me.body);
        assert_eq!(
            log_in(&app, "robin", &new_password).await.status,
            StatusCode::OK
        );
        assert_eq!(
            log_in(&app, "robin", "the old password").await.status,
            StatusCode::UNAUTHORIZED
        );
        // The database never held the new password in plain text.
        let user = database
            .users()
            .find_by_username("robin")
            .await
            .expect("the lookup works")
            .expect("the user is there");
        assert!(user.password_hash.expose().starts_with("$argon2id$"));
        assert_eq!(
            database
                .users()
                .recovery_of(user.id)
                .await
                .expect("it loads"),
            None
        );
    })
    .await;
}

#[tokio::test]
async fn answer_does_not_tell_whether_an_address_is_known() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_user(
            database,
            "robin",
            Some("robin@example.com"),
            &hash("a password"),
        )
        .await;
        add_user(database, "nomail", None, &hash("a password")).await;
        let (app, outbox) = installed_app_with_outbox(database, &mail_on(&[]));

        let known = recover(&app, "robin@example.com").await;
        let unknown = recover(&app, "nobody@example.com").await;
        // An account gets one e-mail in five minutes, and the answer stays the same.
        let again = recover(&app, "robin@example.com").await;

        assert_eq!(known.status, StatusCode::OK);
        assert_eq!(
            known.message(),
            json!("If the email exists in our system, a password reset has been sent to the user")
        );
        assert_eq!(unknown.body, known.body);
        assert_eq!(again.body, known.body);
        outbox.wait_for(1).await;
        assert_eq!(outbox.after_a_moment().await.len(), 1);
        // The name of a user is no address.
        recover(&app, "nomail").await;
        assert_eq!(outbox.after_a_moment().await.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn requests_for_a_new_password_count_for_the_login_limit() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_user(
            database,
            "robin",
            Some("robin@example.com"),
            &hash("a password"),
        )
        .await;
        let settings = mail_on(&[
            ("loginAttempts", json!("3")),
            ("loginLockout", json!("60000")),
        ]);
        let (app, outbox) = installed_app_with_outbox(database, &settings);

        for address in ["a@example.com", "b@example.com"] {
            assert_eq!(recover(&app, address).await.status, StatusCode::OK);
        }
        // The third request reaches the limit.
        for address in ["robin@example.com", "c@example.com"] {
            let locked = recover(&app, address).await;
            assert_eq!(locked.status, StatusCode::FORBIDDEN, "{}", locked.body);
            assert_eq!(locked.message(), json!("User is locked out"));
            assert!(locked.headers.contains_key("retry-after"));
        }
        assert_eq!(outbox.after_a_moment().await, []);
        // The client is locked out as after failed logins, so it cannot log in either.
        assert_eq!(
            log_in(&app, "robin", "a password").await.status,
            StatusCode::FORBIDDEN
        );
    })
    .await;
}

#[tokio::test]
async fn recovery_needs_the_mail_plugin_and_an_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_user(
            database,
            "robin",
            Some("robin@example.com"),
            &hash("a password"),
        )
        .await;
        let (off, outbox) = installed_app_with_outbox(database, &[]);

        let refused = recover(&off, "robin@example.com").await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            refused.message(),
            json!("Email functionality is not enabled")
        );
        for body in [
            json!({}),
            json!({"email": ""}),
            json!({"email": "0"}),
            json!({"email": null}),
        ] {
            let refused = call(&off, As::Guest, "POST", "/users/recover", Some(body)).await;
            assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(refused.message(), json!("Email was not supplied"));
        }
        assert_eq!(outbox.after_a_moment().await, []);

        // The other methods of the path are the routes of the user with that "id".
        let read = call(&off, As::Admin, "GET", "/users/recover", None).await;
        assert_eq!(read.status, StatusCode::OK);
        assert_eq!(read.data(), Value::Null);
    })
    .await;
}

#[tokio::test]
async fn account_without_a_usable_password_gets_one_through_the_recovery() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // An imported account whose password another service checked.
        add_user(database, "pat", Some("pat@example.com"), "").await;
        let (app, outbox) = installed_app_with_outbox(database, &mail_on(&[]));

        recover(&app, "pat@example.com").await;
        let new_password = password_in(&outbox.wait_for(1).await[0]);

        assert_eq!(
            log_in(&app, "pat", "").await.status,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            log_in(&app, "pat", &new_password).await.status,
            StatusCode::OK
        );
    })
    .await;
}

#[tokio::test]
async fn recovery_e_mail_never_links_to_the_host_that_the_request_names() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_user(
            database,
            "robin",
            Some("robin@example.com"),
            &hash("a password"),
        )
        .await;
        // No address of the site is stored, and the caller names a host of their own.
        let (app, outbox) = installed_app_with_outbox(database, &mail_on(&[]));
        let body = json!({"email": "robin@example.com"}).to_string();
        let headers = [
            ("host", "evil.example.com"),
            ("content-type", "application/json"),
        ];

        let answer = send(
            &app,
            request("POST", "/api/v2/users/recover", &headers, &body),
        )
        .await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let mail = &outbox.wait_for(1).await[0];
        // The e-mail still holds the new password, with no link to any site (D-138).
        assert_eq!(password_in(mail).len(), 12);
        assert!(!mail.html.contains("evil.example.com"), "{}", mail.html);
        assert!(!mail.html.contains("href=\"\""), "{}", mail.html);
        assert!(!mail.html.contains("href=\"?"), "{}", mail.html);
    })
    .await;
}

#[tokio::test]
async fn admin_login_stores_the_address_of_the_site_once() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_user(
            database,
            "robin",
            Some("robin@example.com"),
            &hash("a password"),
        )
        .await;
        let (app, _outbox) = installed_app_with_outbox(database, &mail_on(&[]));
        let log_in_at = async |host: &str, name: &str, password: &str| {
            let body = json!({"username": name, "password": password}).to_string();
            let headers = [("host", host), ("content-type", "application/json")];
            send(&app, request("POST", "/api/v2/login", &headers, &body)).await
        };
        let domain = async || {
            call(&app, As::Admin, "GET", "/config/PHPMAILER-domain", None)
                .await
                .data()
                .clone()
        };

        // A user who is no Admin names no address for the e-mails (D-138).
        let user = log_in_at("evil.example.com", "robin", "a password").await;
        assert_eq!(user.status, StatusCode::OK, "{}", user.body);
        assert_eq!(domain().await, Value::Null);

        let admin = log_in_at(
            "organizr.example.com",
            seed::ADMIN_NAME,
            seed::ADMIN_PASSWORD,
        )
        .await;
        assert_eq!(admin.status, StatusCode::OK, "{}", admin.body);
        assert_eq!(domain().await, json!("http://organizr.example.com/"));

        // The stored address stays, whatever host a later login names.
        log_in_at("other.example.com", seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        assert_eq!(domain().await, json!("http://organizr.example.com/"));
    })
    .await;
}

#[tokio::test]
async fn name_of_a_user_cannot_put_markup_into_an_e_mail() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let name = "<img src=x onerror=alert(1)>";
        add_user(
            database,
            name,
            Some("robin@example.com"),
            &hash("a password"),
        )
        .await;
        let settings = mail_on(&[
            ("PHPMAILER-domain", json!("https://my.example.org/")),
            (
                "PHPMAILER-emailTemplateResetSubject",
                json!("For {user} of {title}"),
            ),
        ]);
        let (app, outbox) = installed_app_with_outbox(database, &settings);

        recover(&app, "robin@example.com").await;

        let mail = &outbox.wait_for(1).await[0];
        assert!(!mail.html.contains("<img src=x"), "{}", mail.html);
        assert!(
            mail.html
                .contains("Hey there &lt;img src=x onerror=alert(1)&gt;!")
        );
        // The subject is plain text.
        assert_eq!(mail.subject, format!("For {name} of Organizr V2"));
        // The setting names the site, and the button of the e-mail leads there.
        assert!(
            mail.html
                .contains("Head over to my website: https://my.example.org/<br />")
        );
        assert!(mail.html.contains("<a href=\"https://my.example.org/\""));
        assert!(mail.html.contains(">Goto My Site</a>"));
    })
    .await;
}

#[tokio::test]
async fn test_e_mail_goes_to_the_caller_and_reports_a_failure() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (app, outbox) = installed_app_with_outbox(database, &mail_on(&[]));
        let path = "/plugins/php-mailer/email/test";
        let admin = seed::logged_in(database, &app, "boss", 0).await;
        let helper = seed::logged_in(database, &app, "helper", 2).await;
        assert_eq!(
            outbox.after_a_moment().await,
            [],
            "these users have no address"
        );

        // Only an Admin or a Co-Admin can send it.
        for who in [As::Guest, As::User(&helper)] {
            let refused = call(&app, who, "GET", path, None).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{}", refused.body);
        }
        // An account without an address, and a script with the API key, get no e-mail.
        for who in [As::User(&admin), As::Admin] {
            let refused = call(&app, who, "GET", path, None).await;
            assert_eq!(refused.status, StatusCode::CONFLICT, "{}", refused.body);
            assert_eq!(
                refused.message(),
                json!("The account has no e-mail address")
            );
        }
        assert_eq!(outbox.mails(), []);

        // The Admin of the install has an address.
        let body = json!({"username": seed::ADMIN_NAME, "password": seed::ADMIN_PASSWORD});
        let login = call(&app, As::Guest, "POST", "/login", Some(body)).await;
        let cookie = login
            .cookies
            .iter()
            .find(|cookie| cookie.starts_with("organizr_token_") && !cookie.contains("=;"))
            .and_then(|cookie| cookie.split(';').next())
            .expect("the login sets the session cookie")
            .to_owned();
        let sent = call(&app, As::User(&cookie), "GET", path, None).await;
        assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
        assert_eq!(sent.message(), json!("Email sent"));
        let mail = outbox
            .mails()
            .into_iter()
            .find(|mail| mail.subject == "Test E-Mail")
            .expect("the test e-mail was sent");
        let to = mail.to.expect("the e-mail has a recipient");
        assert_eq!(
            (to.address.as_str(), to.name.as_deref()),
            (seed::ADMIN_EMAIL, Some(seed::ADMIN_NAME))
        );
        assert!(mail.html.contains("<p>This is just a test email.</p>"));

        // A server that takes no e-mail: the Admin learns why.
        let state =
            state_with(database, installed_settings(&mail_on(&[]))).with_mailer(Outbox::failing());
        let failing = router(state).expect("the route table is valid");
        let failed = call(&failing, As::User(&cookie), "GET", path, None).await;
        assert_eq!(failed.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(failed.message(), json!("the e-mail has no recipient"));
        // No server is set.
        let (unset, _) = installed_app_with_outbox(database, &[]);
        let failed = call(&unset, As::User(&cookie), "GET", path, None).await;
        assert_eq!(failed.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(failed.message(), json!("No SMTP server is set"));
    })
    .await;
}

#[tokio::test]
async fn e_mail_of_an_admin_goes_to_hidden_recipients() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The address of the site comes from the setting only (D-138).
        let settings = mail_on(&[("PHPMAILER-domain", json!("http://organizr.example.com/"))]);
        let (app, outbox) = installed_app_with_outbox(database, &settings);
        let path = "/plugins/php-mailer/email/send";
        let helper = seed::logged_in(database, &app, "helper", 2).await;
        let letter = json!({
            "bcc": " a@example.com , b@example.com,,",
            "subject": "Hello {user}, from {title}",
            "body": "<b>Bold</b> at {fullDomain}",
        });

        let refused = call(&app, As::User(&helper), "POST", path, Some(letter.clone())).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        let sent = call(&app, As::Admin, "POST", path, Some(letter.clone())).await;
        assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);

        let mails = outbox.mails();
        assert_eq!(mails.len(), 1);
        assert_eq!(mails[0].to, None);
        assert_eq!(mails[0].bcc, ["a@example.com", "b@example.com"]);
        assert_eq!(mails[0].subject, "Hello , from Organizr V2");
        // What an Admin writes is HTML, and stays HTML.
        assert!(
            mails[0]
                .html
                .contains("<p><b>Bold</b> at http://organizr.example.com/</p>")
        );

        // A server that takes no e-mail.
        let state =
            state_with(database, installed_settings(&mail_on(&[]))).with_mailer(Outbox::failing());
        let failing = router(state).expect("the route table is valid");
        let failed = call(&failing, As::Admin, "POST", path, Some(letter.clone())).await;
        assert_eq!(failed.status, StatusCode::CONFLICT);
        assert_eq!(failed.message(), json!("the e-mail has no recipient"));
        // The plugin is on, and no server is set.
        let (unset, _) = installed_app_with_outbox(database, &[("PHPMAILER-enabled", json!(true))]);
        let failed = call(&unset, As::Admin, "POST", path, Some(letter)).await;
        assert_eq!(failed.status, StatusCode::CONFLICT);
        assert_eq!(failed.message(), json!("No SMTP server is set"));
    })
    .await;
}

async fn register(app: &Router, name: &str) -> Answer {
    let body = json!({
        "username": name,
        "email": format!("{name}@example.com"),
        "password": "the password of a new user",
        "registrationPassword": "let me in",
    });
    call(app, As::Guest, "POST", "/users/register", Some(body)).await
}

#[tokio::test]
async fn new_account_gets_a_welcome_when_the_settings_ask_for_one() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let registration = json!(hash("let me in"));
        let on = mail_on(&[("registrationPassword", registration.clone())]);
        let (app, outbox) = installed_app_with_outbox(database, &on);

        let answer = register(&app, "newcomer").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

        let mails = outbox.wait_for(1).await;
        assert_eq!(mails[0].subject, "Thank you For Registering");
        let to = mails[0].to.clone().expect("the e-mail has a recipient");
        assert_eq!(
            (to.address.as_str(), to.name.as_deref()),
            ("newcomer@example.com", Some("newcomer"))
        );
        assert!(mails[0].html.contains("<h2>Hey there newcomer!</h2>"));
        assert!(!mails[0].html.contains("Goto My Site"));

        // With the switch of the welcome off, or the plugin off, nothing is sent.
        let quiet = mail_on(&[
            ("registrationPassword", registration.clone()),
            ("PHPMAILER-emailTemplateRegisterUserEnabled", json!(false)),
        ]);
        let (app, outbox) = installed_app_with_outbox(database, &quiet);
        assert_eq!(register(&app, "second").await.status, StatusCode::OK);
        assert_eq!(outbox.after_a_moment().await, []);
        // The server is still set. Only the switch of the plugin is off.
        let off = mail_on(&[
            ("registrationPassword", registration),
            ("PHPMAILER-enabled", json!(false)),
        ]);
        let (app, outbox) = installed_app_with_outbox(database, &off);
        assert_eq!(register(&app, "third").await.status, StatusCode::OK);
        assert_eq!(outbox.after_a_moment().await, []);
    })
    .await;
}

/// Logs in with the name that a browser gives itself.
async fn log_in_with(app: &Router, name: &str, browser: Option<&str>) -> Answer {
    let body = json!({"username": name, "password": "a password"}).to_string();
    let mut headers = vec![("content-type", "application/json")];
    if let Some(browser) = browser {
        headers.push(("user-agent", browser));
    }
    send(app, request("POST", "/api/v2/login", &headers, &body)).await
}

#[tokio::test]
async fn login_from_a_new_browser_tells_the_user() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_user(
            database,
            "robin",
            Some("robin@example.com"),
            &hash("a password"),
        )
        .await;
        add_user(database, "nomail", None, &hash("a password")).await;
        let (app, outbox) = installed_app_with_outbox(database, &mail_on(&[]));
        let browser = "Mozilla/5.0 <script>{password}</script>";

        assert_eq!(
            log_in_with(&app, "robin", Some(browser)).await.status,
            StatusCode::OK
        );

        let mails = outbox.wait_for(1).await;
        assert_eq!(
            mails[0].subject,
            "We noticed a login attempt to your account on a new device."
        );
        let to = mails[0].to.clone().expect("the e-mail has a recipient");
        assert_eq!(to.address, "robin@example.com");
        assert!(mails[0].html.contains("<h2>Hey there robin!</h2>"));
        assert!(
            mails[0].html.contains("IP: 0.0.0.0<br />"),
            "{}",
            mails[0].html
        );
        // The browser names itself. Its text is shown as text, with no value in it.
        assert!(
            mails[0].html.contains(
                "Browser: Mozilla/5.0 &lt;script&gt;&#123;password}&lt;/script&gt;<br />"
            ),
            "{}",
            mails[0].html
        );

        // The same browser again, a login without a browser name, a wrong password, and
        // an account without an address: no e-mail.
        log_in_with(&app, "robin", Some(browser)).await;
        log_in_with(&app, "robin", None).await;
        let body = json!({"username": "robin", "password": "wrong"});
        call(&app, As::Guest, "POST", "/login", Some(body)).await;
        log_in_with(&app, "nomail", Some(browser)).await;
        assert_eq!(outbox.after_a_moment().await.len(), 1);
        // Another browser: a second e-mail.
        log_in_with(&app, "robin", Some("Another/1.0")).await;
        assert_eq!(outbox.wait_for(2).await.len(), 2);

        // With the plugin off, a login sends nothing. The server is still set.
        let off = mail_on(&[("PHPMAILER-enabled", json!(false))]);
        let (off, outbox) = installed_app_with_outbox(database, &off);
        log_in_with(&off, "robin", Some("A third/1.0")).await;
        assert_eq!(outbox.after_a_moment().await, []);
    })
    .await;
}

#[tokio::test]
async fn list_names_each_user_with_the_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_user(database, "zed", Some("zed@example.com"), "").await;
        add_user(database, "amy", None, "").await;
        let (app, _) = installed_app_with_outbox(database, &[]);
        let path = "/plugins/php-mailer/email/list";

        let listed = call(&app, As::Admin, "GET", path, None).await;

        // In the order of the ids, and not sorted by name.
        assert!(
            listed.body.find("\"zed\"") < listed.body.find("\"amy\""),
            "{}",
            listed.body
        );
        assert_eq!(
            listed.data(),
            json!({"admin": "admin@example.com", "zed": "zed@example.com", "amy": null})
        );
        let helper = seed::logged_in(database, &app, "helper", 2).await;
        let refused = call(&app, As::User(&helper), "GET", path, None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}
