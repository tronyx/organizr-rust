//! The e-mails of the application: `POST /users/recover` and the routes of the mail
//! plugin, `GET /plugins/php-mailer/email/test`, `POST /plugins/php-mailer/email/send`
//! and `GET /plugins/php-mailer/email/list`.

use std::time::{Duration, Instant, SystemTime};

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use organizr_auth::login_limit::{Attempt, LimitPolicy};
use organizr_auth::recovery;
use organizr_domain::GroupLevel;
use organizr_plugins::mail::{
    self, Frame, MailError, MailKind, MailValues, Message, Recipient, SmtpServer, TextForm,
};
use organizr_settings::Settings;
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use crate::api_data::ApiData;
use crate::caller::{Caller, now_stamp};
use crate::client::Client;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// What counts the requests for a new password of all clients together. No user can
/// have this name, because a name cannot hold a zero byte.
const RECOVERY_COUNTER: &str = "\0password recovery";

/// The answer of `POST /users/recover`, whether the address is known or not.
const RECOVERY_ANSWER: &str =
    "If the email exists in our system, a password reset has been sent to the user";

/// The address of the site for the links and the texts of an e-mail: the
/// `PHPMAILER-domain` setting, or else the address that the browser of this request
/// used.
pub fn site_address(settings: &Settings, headers: &HeaderMap, client: &Client) -> String {
    if settings.mailer_domain.is_empty() {
        request_site(headers, client)
    } else {
        settings.mailer_domain.clone()
    }
}

/// The address that the browser of a request used for this site, for example
/// `https://organizr.example.com/`. Empty when the request names no host.
pub fn request_site(headers: &HeaderMap, client: &Client) -> String {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    // Only what the name of a host can hold. Anything else could put markup into an
    // e-mail, or another address into a redirect.
    let is_host = !host.is_empty()
        && host
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-:[]".contains(character));
    if !is_host {
        return String::new();
    }
    let (scheme, usual_port) = if client.https {
        ("https", ":443")
    } else {
        ("http", ":80")
    };
    let host = host.strip_suffix(usual_port).unwrap_or(host);
    format!("{scheme}://{host}/")
}

/// One e-mail before the values are in it.
#[derive(Debug, Clone, Copy)]
pub struct Letter<'a> {
    /// The kind of the e-mail.
    pub kind: MailKind,
    /// The subject, with placeholders.
    pub subject: &'a str,
    /// The text as HTML, with placeholders.
    pub body: &'a str,
    /// The name of the recipient, for `{user}`.
    pub user: Option<&'a str>,
    /// A new password, for `{password}`.
    pub password: Option<&'a str>,
    /// The code of an invite, for `{inviteCode}` and the button of an invite.
    pub invite_code: Option<&'a str>,
}

/// Writes an e-mail: the subject and the HTML of the whole message.
pub fn compose(settings: &Settings, site: &str, letter: Letter<'_>) -> (String, String) {
    let values = MailValues {
        user: letter.user,
        password: letter.password,
        invite_code: letter.invite_code,
        site,
        title: &settings.title,
    };
    let subject = mail::fill_in(letter.subject, values, TextForm::Plain);
    let body = mail::fill_in(letter.body, values, TextForm::Html);
    let frame = Frame {
        layout: &settings.mailer_template,
        kind: letter.kind,
        logo: &settings.mailer_logo,
        site,
        invite_code: letter.invite_code,
    };
    let html = mail::lay_out(frame, &subject, &body);
    (subject, html)
}

/// Sends an e-mail and does not make the request wait for the SMTP server. A failure
/// goes to the log. `what` names the e-mail there.
pub fn send_in_background(state: &AppState, message: Message, what: &'static str) {
    let Some(server) = mail::server_of(&state.settings()) else {
        tracing::warn!(
            what,
            "an e-mail was not sent, because no SMTP server is set"
        );
        return;
    };
    let mailer = state.mailer().clone();
    tokio::spawn(async move {
        match mailer.send(server, message).await {
            Ok(()) => tracing::info!(what, "an e-mail was sent"),
            Err(error) => tracing::warn!(what, %error, "an e-mail was not sent"),
        }
    });
}

/// Sends the e-mail that welcomes a new account, when the settings ask for one and the
/// account has an address.
pub fn welcome(state: &AppState, site: &str, username: &str, address: Option<&str>) {
    let settings = state.settings();
    let wanted = settings.mailer_enabled && settings.mailer_email_template_register_user_enabled;
    let Some(address) = address.filter(|_| wanted) else {
        return;
    };
    let letter = Letter {
        kind: MailKind::Plain,
        subject: &settings.mailer_email_template_register_user_subject,
        body: &settings.mailer_email_template_register_user,
        user: Some(username),
        password: None,
        invite_code: None,
    };
    let (subject, html) = compose(&settings, site, letter);
    let message = Message {
        to: Some(Recipient {
            address: address.to_owned(),
            name: Some(username.to_owned()),
        }),
        bcc: Vec::new(),
        subject,
        html,
    };
    send_in_background(state, message, "welcome");
}

/// What an e-mail about a login from a new device says about the device.
#[derive(Debug, Clone, Copy)]
pub struct NewDevice<'a> {
    /// The name of the account.
    pub username: &'a str,
    /// The e-mail address of the account.
    pub address: &'a str,
    /// The address that the login came from.
    pub ip: std::net::IpAddr,
    /// The browser, as it names itself.
    pub browser: &'a str,
}

/// Sends the e-mail that tells a user about a login from a browser that has no other
/// session of the account.
pub fn new_device(state: &AppState, site: &str, device: NewDevice<'_>) {
    let settings = state.settings();
    // The browser names itself, so its text is written as text.
    let body = format!(
        "<h2>Hey there {{user}}!</h2>\n\
         We noticed a login attempt to your account and want to make sure it's you.<br />\n\
         If this was you, please ignore this email.<br /><br />\n\
         If this wasn't you, please change your password and revoke all tokens.<br /><br />\n\
         <b>Details:</b><br />\n\
         IP: {}<br />\n\
         Browser: {}<br />\n",
        device.ip,
        mail::escaped(device.browser).replace('{', "&#123;"),
    );
    let letter = Letter {
        kind: MailKind::Plain,
        subject: "We noticed a login attempt to your account on a new device.",
        body: &body,
        user: Some(device.username),
        password: None,
        invite_code: None,
    };
    let (subject, html) = compose(&settings, site, letter);
    let message = Message {
        to: Some(Recipient {
            address: device.address.to_owned(),
            name: None,
        }),
        bcc: Vec::new(),
        subject,
        html,
    };
    send_in_background(state, message, "login from a new device");
}

/// A field that PHP counts as given: it has text, and the text is not `0`.
fn given(data: &ApiData, name: &str) -> Option<String> {
    data.text(name)
        .filter(|text| !text.is_empty() && text != "0")
}

/// `POST /users/recover`: sends a new password to the e-mail address of an account.
///
/// The answer is the same for a known and for an unknown address. The password of the
/// account stays until somebody logs in with the new one (D-58).
pub async fn recover(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    let settings = state.settings();
    let email =
        given(&data, "email").ok_or_else(|| Refusal::unprocessable("Email was not supplied"))?;
    if !settings.mailer_enabled {
        return Err(Refusal::unprocessable("Email functionality is not enabled"));
    }

    // Each request counts, for the client and for the install, as a failed login does.
    let limiter = state.login_limiter();
    let attempt = Attempt::new(client.address, RECOVERY_COUNTER);
    let policy = LimitPolicy {
        failures: settings.login_attempts,
        lockout: Duration::from_millis(settings.login_lockout),
    };
    let locked = limiter
        .locked_for(&attempt, Instant::now())
        .or_else(|| limiter.record_failure(&attempt, policy, Instant::now()));
    if let Some(retry_after) = locked {
        tracing::warn!(client = %client.address, "too many requests for a new password");
        return Err(ApiError::LockedOut { retry_after }.into());
    }

    let now = now_stamp(SystemTime::now());
    let started = recovery::start(state.database(), &email, now)
        .await
        .map_err(|error| {
            tracing::error!(%error, "a recovery password could not be made");
            Refusal::Request(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the request could not be completed".to_owned(),
            )
        })?;
    if let Some(started) = started {
        let site = site_address(&settings, &headers, &client);
        let letter = Letter {
            kind: MailKind::Reset,
            subject: &settings.mailer_email_template_reset_subject,
            body: &settings.mailer_email_template_reset,
            user: Some(&started.user.username),
            password: Some(&started.password),
            invite_code: None,
        };
        let (subject, html) = compose(&settings, &site, letter);
        let message = Message {
            // The address as the person wrote it, which is the address of the account
            // in some letter case.
            to: Some(Recipient {
                address: email,
                name: Some(started.user.username.clone()),
            }),
            bcc: Vec::new(),
            subject,
            html,
        };
        send_in_background(&state, message, "new password");
    }
    Ok(Envelope::success(NoData).with_message(RECOVERY_ANSWER))
}

/// The SMTP server of the settings, or the answer for an install that has none.
fn server(settings: &Settings, status: StatusCode) -> Result<SmtpServer, Refusal> {
    mail::server_of(settings)
        .ok_or_else(|| Refusal::Request(status, "No SMTP server is set".to_owned()))
}

/// `GET /plugins/php-mailer/email/test`: sends a test e-mail to the address of the
/// caller, and answers when the SMTP server has taken it.
pub async fn test(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let settings = state.settings();
    // A script with the API key has no account, and so no address.
    let user = caller.user().map(|caller| &caller.authenticated.user);
    let Some((user, address)) = user.and_then(|user| Some((user, user.email.clone()?))) else {
        return Err(Refusal::conflict("The account has no e-mail address"));
    };
    let server = server(&settings, StatusCode::INTERNAL_SERVER_ERROR)?;

    let site = site_address(&settings, &headers, &client);
    let letter = Letter {
        kind: MailKind::Plain,
        subject: "Test E-Mail",
        body: "This is just a test email.",
        user: None,
        password: None,
        invite_code: None,
    };
    let (subject, html) = compose(&settings, &site, letter);
    let message = Message {
        to: Some(Recipient {
            address,
            name: Some(user.username.clone()),
        }),
        bcc: Vec::new(),
        subject,
        html,
    };
    state
        .mailer()
        .send(server, message)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "the test e-mail was not sent");
            // Only an Admin or a Co-Admin gets here, and needs the reason to correct the
            // settings.
            Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
        })?;
    tracing::info!(user = %user.username, "a test e-mail was sent");
    Ok(Envelope::success(NoData).with_message("Email sent"))
}

/// `POST /plugins/php-mailer/email/send`: sends an e-mail of an Admin to the addresses
/// in `bcc`, which are separated by commas. No recipient sees the others.
pub async fn send(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let settings = state.settings();
    if !settings.mailer_enabled {
        return Err(Refusal::conflict("PHP-Mailer is not enabled"));
    }
    let bcc: Vec<String> = data
        .text("bcc")
        .unwrap_or_default()
        .split(',')
        .map(|address| address.trim().to_owned())
        .filter(|address| !address.is_empty())
        .collect();
    let server = server(&settings, StatusCode::CONFLICT)?;

    let site = site_address(&settings, &headers, &client);
    let letter = Letter {
        kind: MailKind::Plain,
        subject: &data.text("subject").unwrap_or_default(),
        body: &data.text("body").unwrap_or_default(),
        user: None,
        password: None,
        invite_code: None,
    };
    let (subject, html) = compose(&settings, &site, letter);
    let recipients = bcc.len();
    if recipients == 0 {
        return Err(Refusal::conflict(MailError::NoRecipient.to_string()));
    }
    let message = Message {
        to: None,
        bcc,
        subject,
        html,
    };
    state
        .mailer()
        .send(server, message)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "an e-mail of an Admin was not sent");
            Refusal::conflict(error.to_string())
        })?;
    tracing::info!(recipients, "an e-mail of an Admin was sent");
    Ok(Envelope::success(NoData).with_message("Email sent"))
}

/// The users with their e-mail addresses: an object from each name to the address, in
/// the order of the ids of the users.
#[derive(Debug)]
pub struct Addresses(Vec<(String, Option<String>)>);

impl Serialize for Addresses {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, address) in &self.0 {
            map.serialize_entry(name, address)?;
        }
        map.end()
    }
}

/// `GET /plugins/php-mailer/email/list`: the name and the e-mail address of each user,
/// for the page on which an Admin chooses recipients.
pub async fn list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Addresses>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let users = state.database().users().list().await?;
    Ok(Envelope::success(Addresses(
        users
            .into_iter()
            .map(|user| (user.username, user.email))
            .collect(),
    )))
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use axum::http::{HeaderMap, HeaderValue, header};
    use organizr_settings::Settings;
    use serde_json::json;

    use super::site_address;
    use crate::client::{AddressSource, Client};

    fn client(https: bool) -> Client {
        Client {
            address: IpAddr::from([203, 0, 113, 9]),
            https,
            source: AddressSource::Connection,
            via_trusted_proxy: false,
        }
    }

    fn host(host: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static(host));
        headers
    }

    #[test]
    fn site_is_the_address_that_the_browser_used() {
        let settings = Settings::defaults();

        for (name, https, site) in [
            (
                "organizr.example.com",
                true,
                "https://organizr.example.com/",
            ),
            (
                "organizr.example.com",
                false,
                "http://organizr.example.com/",
            ),
            (
                "organizr.example.com:8443",
                true,
                "https://organizr.example.com:8443/",
            ),
            // The usual port of the scheme is left out.
            (
                "organizr.example.com:443",
                true,
                "https://organizr.example.com/",
            ),
            (
                "organizr.example.com:80",
                false,
                "http://organizr.example.com/",
            ),
            (
                "organizr.example.com:80",
                true,
                "https://organizr.example.com:80/",
            ),
            ("[2001:db8::1]:8080", false, "http://[2001:db8::1]:8080/"),
        ] {
            assert_eq!(site_address(&settings, &host(name), &client(https)), site);
        }
    }

    #[test]
    fn host_that_is_no_host_name_gives_no_address() {
        let settings = Settings::defaults();

        for name in ["evil.example/\"><script>", "a b", "host?x=1", "user@host"] {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::HOST,
                HeaderValue::from_str(name).expect("the value can be a header"),
            );
            assert_eq!(
                site_address(&settings, &headers, &client(true)),
                "",
                "{name}"
            );
        }
        assert_eq!(
            site_address(&settings, &HeaderMap::new(), &client(true)),
            ""
        );
    }

    #[test]
    fn setting_names_the_site_whatever_the_request_says() {
        let mut settings = Settings::defaults();
        settings
            .set(
                "PHPMAILER-domain",
                &json!("https://my.example.org/organizr"),
            )
            .expect("the setting can be set");

        assert_eq!(
            site_address(&settings, &host("evil.example.com"), &client(false)),
            "https://my.example.org/organizr"
        );
    }
}
