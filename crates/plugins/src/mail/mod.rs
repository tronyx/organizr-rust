//! The mail plugin: the e-mails of Organizr, and how they are sent.
//!
//! An e-mail is made in three steps, as in the PHP application. [`fill_in`] puts the
//! values of a recipient into the text that an Admin wrote, for example the name in
//! place of `{user}`. [`lay_out`] wraps the result in one of the layouts of Organizr.
//! A [`Mailer`] hands the message to the SMTP server of the settings.

mod send;

use std::collections::BTreeMap;
use std::sync::OnceLock;

use organizr_settings::Settings;
use serde::Deserialize;

pub use send::{MailError, Mailer, Message, Recipient, Security, SmtpServer};

/// The layouts of the PHP application, made by `parity/php-mail-templates/generate.php`
/// from its own files. Each layout has the HTML of each kind of e-mail, with a marker
/// in place of each value.
const LAYOUTS: &str = include_str!("layouts.json");

/// The layout that the PHP application uses when the setting names none.
const DEFAULT_LAYOUT: &str = "default";

#[derive(Deserialize)]
struct Layouts {
    layouts: BTreeMap<String, Layout>,
}

#[derive(Deserialize)]
struct Layout {
    invite: String,
    reset: String,
    plain: String,
}

fn layouts() -> &'static BTreeMap<String, Layout> {
    static PARSED: OnceLock<BTreeMap<String, Layout>> = OnceLock::new();
    PARSED.get_or_init(|| {
        serde_json::from_str::<Layouts>(LAYOUTS)
            .map_or_else(|_| BTreeMap::new(), |file| file.layouts)
    })
}

/// The names of the layouts, sorted.
pub fn layout_names() -> Vec<&'static str> {
    layouts().keys().map(String::as_str).collect()
}

/// The kind of an e-mail. Two kinds get a button below the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailKind {
    /// An invite, with a button that leads to the site with the invite code.
    Invite,
    /// A new password, with a button that leads to the site.
    Reset,
    /// Every other e-mail.
    Plain,
}

/// The values of one e-mail for the placeholders that an Admin can write in a text:
/// `{user}`, `{password}`, `{inviteCode}`, `{domain}`, `{fullDomain}` and `{title}`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MailValues<'a> {
    /// The name of the recipient.
    pub user: Option<&'a str>,
    /// A new password.
    pub password: Option<&'a str>,
    /// An invite code.
    pub invite_code: Option<&'a str>,
    /// The address of the site.
    pub site: &'a str,
    /// The title of the install.
    pub title: &'a str,
}

/// How the values go into a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextForm {
    /// The text is HTML: the body of an e-mail. A value from a person is written so
    /// that a mail program shows it as text, whatever characters it holds.
    Html,
    /// The text is plain: the subject of an e-mail.
    Plain,
}

/// Writes a value so that HTML shows it as text.
fn escaped(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#039;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Puts the values in place of the placeholders of a text. A value that an e-mail does
/// not have leaves nothing in its place. The text is read once, so a value that holds a
/// placeholder stays as it is.
pub fn fill_in(text: &str, values: MailValues<'_>, form: TextForm) -> String {
    let from_a_person = |value: Option<&str>| {
        let value = value.unwrap_or_default();
        match form {
            TextForm::Html => escaped(value),
            TextForm::Plain => value.to_owned(),
        }
    };
    let placeholders = [
        ("{user}", from_a_person(values.user)),
        ("{password}", from_a_person(values.password)),
        ("{inviteCode}", from_a_person(values.invite_code)),
        ("{domain}", values.site.to_owned()),
        ("{fullDomain}", values.site.to_owned()),
        ("{title}", values.title.to_owned()),
    ];

    replace_each(text, "{", &placeholders)
}

/// Puts a value in place of each name in a text. Every name starts with `start`. The
/// text is read once, so a name inside a value is not replaced again.
fn replace_each(text: &str, start: &str, values: &[(&str, String)]) -> String {
    let mut filled = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(start) {
        let (before, from_start) = rest.split_at(at);
        filled.push_str(before);
        let named = values.iter().find(|(name, _)| from_start.starts_with(name));
        let (written, taken) = named.map_or((start, start.len()), |(name, value)| {
            (value.as_str(), name.len())
        });
        filled.push_str(written);
        rest = from_start.get(taken..).unwrap_or_default();
    }
    filled.push_str(rest);
    filled
}

/// What a layout shows around the text of an e-mail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'a> {
    /// The name of the layout: the `PHPMAILER-template` setting.
    pub layout: &'a str,
    /// The kind of the e-mail.
    pub kind: MailKind,
    /// The address of the picture at the top: the `PHPMAILER-logo` setting.
    pub logo: &'a str,
    /// The address of the site, for the button.
    pub site: &'a str,
    /// The invite code, for the button of an invite.
    pub invite_code: Option<&'a str>,
}

/// Wraps the subject and the body of an e-mail in a layout and returns the HTML of the
/// whole e-mail. A name that is no layout gives the default layout.
pub fn lay_out(frame: Frame<'_>, subject: &str, body: &str) -> String {
    let layouts = layouts();
    let Some(layout) = layouts
        .get(frame.layout)
        .or_else(|| layouts.get(DEFAULT_LAYOUT))
    else {
        return body.to_owned();
    };
    let html = match frame.kind {
        MailKind::Invite => &layout.invite,
        MailKind::Reset => &layout.reset,
        MailKind::Plain => &layout.plain,
    };
    let markers = [
        ("[[subject]]", subject.to_owned()),
        ("[[body]]", body.to_owned()),
        ("[[site]]", frame.site.to_owned()),
        (
            "[[invite_code]]",
            escaped(frame.invite_code.unwrap_or_default()),
        ),
        ("[[setting:PHPMAILER-logo]]", frame.logo.to_owned()),
    ];

    replace_each(html, "[[", &markers)
}

/// The SMTP server of the settings, or `None` when the settings name no host.
pub fn server_of(settings: &Settings) -> Option<SmtpServer> {
    let host = settings.mailer_smtp_host.trim();
    if host.is_empty() {
        return None;
    }
    let security = match settings.mailer_smtp_host_type.as_str() {
        "ssl" => Security::Tls,
        "tls" => Security::StartTls,
        _ => Security::Opportunistic,
    };
    // The ports that mail programs use for each kind of connection.
    let default_port = match security {
        Security::Tls => 465,
        Security::StartTls => 587,
        Security::Opportunistic => 25,
    };
    let credentials = settings.mailer_smtp_host_auth.then(|| {
        (
            settings.mailer_smtp_host_username.clone(),
            settings.mailer_smtp_host_password.clone(),
        )
    });
    Some(SmtpServer {
        host: host.to_owned(),
        port: settings
            .mailer_smtp_host_port
            .trim()
            .parse()
            .unwrap_or(default_port),
        security,
        credentials,
        verify_certificate: settings.mailer_verify_cert,
        sender_name: settings.mailer_smtp_host_sender_name.clone(),
        sender_address: settings.mailer_smtp_host_sender_email.clone(),
    })
}

#[cfg(test)]
mod tests {
    use organizr_settings::Settings;
    use serde_json::json;

    use super::{
        Frame, MailKind, MailValues, Security, TextForm, fill_in, lay_out, layout_names, server_of,
    };

    const VALUES: MailValues<'static> = MailValues {
        user: Some("robin"),
        password: Some("s3cr3t"),
        invite_code: Some("ABC123"),
        site: "https://organizr.example.com/",
        title: "My Organizr",
    };

    #[test]
    fn placeholders_get_the_values_of_the_mail() {
        let text = "Hey {user}! Password: {password}, code: {inviteCode}. \
                    Visit {domain} or {fullDomain}, the site of {title}.";

        assert_eq!(
            fill_in(text, VALUES, TextForm::Html),
            "Hey robin! Password: s3cr3t, code: ABC123. Visit https://organizr.example.com/ \
             or https://organizr.example.com/, the site of My Organizr."
        );
    }

    #[test]
    fn value_that_a_mail_does_not_have_leaves_nothing() {
        let values = MailValues {
            user: None,
            password: None,
            invite_code: None,
            ..VALUES
        };

        assert_eq!(
            fill_in("[{user}|{password}|{inviteCode}]", values, TextForm::Plain),
            "[||]"
        );
    }

    #[test]
    fn other_braces_stay_and_a_value_is_not_read_again() {
        let values = MailValues {
            user: Some("{password}"),
            ..VALUES
        };

        assert_eq!(
            fill_in("{user} {unknown} {user {{user}} {", values, TextForm::Plain),
            "{password} {unknown} {user {{password}} {"
        );
        assert_eq!(fill_in("", values, TextForm::Plain), "");
    }

    #[test]
    fn name_of_a_person_cannot_put_markup_into_the_body() {
        let values = MailValues {
            user: Some("<a href=\"https://evil.example\">Tom & 'Jerry'</a>"),
            ..VALUES
        };

        assert_eq!(
            fill_in("<h2>Hey {user}!</h2>", values, TextForm::Html),
            "<h2>Hey &lt;a href=&quot;https://evil.example&quot;&gt;Tom &amp; &#039;Jerry&#039;&lt;/a&gt;!</h2>"
        );
        // The subject is no HTML, so it keeps the name as it is.
        assert_eq!(
            fill_in("For {user}", values, TextForm::Plain),
            "For <a href=\"https://evil.example\">Tom & 'Jerry'</a>"
        );
    }

    fn frame(layout: &'static str, kind: MailKind) -> Frame<'static> {
        Frame {
            layout,
            kind,
            logo: "https://example.com/logo.png",
            site: "https://organizr.example.com/",
            invite_code: Some("ABC123"),
        }
    }

    #[test]
    fn organizr_has_five_layouts() {
        assert_eq!(
            layout_names(),
            ["dark", "default", "gray", "light", "plehex"]
        );
    }

    #[test]
    fn layout_wraps_the_text_and_leaves_no_marker() {
        for layout in layout_names() {
            for kind in [MailKind::Invite, MailKind::Reset, MailKind::Plain] {
                let html = lay_out(frame(layout, kind), "The subject", "<p>The body</p>");

                assert!(html.contains("<p>The body</p>"), "{layout} {kind:?}");
                assert!(html.contains("</html>"), "{layout} {kind:?}");
                assert!(!html.contains("[["), "{layout} {kind:?}");
            }
        }
    }

    #[test]
    fn dark_layout_is_the_one_of_php() {
        let html = lay_out(frame("dark", MailKind::Invite), "Welcome", "Hello");

        assert!(html.contains(
            "<td style=\"background: #1b1a1a;padding:20px;color:#fff;text-align:center;\">Welcome</td>"
        ));
        assert!(html.contains("<p>Hello</p>"));
        assert!(html.contains("src=\"https://example.com/logo.png\""));
        assert!(html.contains("<a href=\"https://organizr.example.com/?invite=ABC123\""));
        assert!(html.contains(">Use Invite Code</a>"));

        let reset = lay_out(frame("dark", MailKind::Reset), "Welcome", "Hello");
        assert!(reset.contains("<a href=\"https://organizr.example.com/\""));
        assert!(reset.contains(">Goto My Site</a>"));
        assert!(!reset.contains("invite"));

        let plain = lay_out(frame("dark", MailKind::Plain), "Welcome", "Hello");
        assert!(!plain.contains("Goto My Site") && !plain.contains("Use Invite Code"));
    }

    #[test]
    fn unknown_layout_is_the_default_one_and_a_marker_in_the_text_stays() {
        let unknown = lay_out(
            frame("no-such-layout", MailKind::Plain),
            "S",
            "[[site]] [[x",
        );
        let default = lay_out(frame("default", MailKind::Plain), "S", "[[site]] [[x");

        assert_eq!(unknown, default);
        assert!(default.contains("[[site]] [[x"));
    }

    fn settings(changed: &[(&str, serde_json::Value)]) -> Settings {
        let mut settings = Settings::defaults();
        for (key, value) in changed {
            settings.set(key, value).expect("the setting can be set");
        }
        settings
    }

    #[test]
    fn server_is_read_from_the_settings() {
        assert_eq!(server_of(&Settings::defaults()), None);
        assert_eq!(
            server_of(&settings(&[("PHPMAILER-smtpHost", json!("  "))])),
            None
        );

        let server = server_of(&settings(&[
            ("PHPMAILER-smtpHost", json!(" smtp.example.com ")),
            ("PHPMAILER-smtpHostPort", json!("2525")),
            ("PHPMAILER-smtpHostUsername", json!("mailer")),
            ("PHPMAILER-smtpHostPassword", json!("the password")),
            ("PHPMAILER-smtpHostSenderName", json!("My Organizr")),
            (
                "PHPMAILER-smtpHostSenderEmail",
                json!("organizr@example.com"),
            ),
        ]))
        .expect("the settings name a server");
        assert_eq!(server.host, "smtp.example.com");
        assert_eq!(server.port, 2525);
        assert_eq!(server.security, Security::StartTls);
        assert_eq!(
            server.credentials,
            Some(("mailer".to_owned(), "the password".to_owned()))
        );
        assert!(server.verify_certificate);
        assert_eq!(server.sender_name, "My Organizr");
        assert_eq!(server.sender_address, "organizr@example.com");
    }

    #[test]
    fn kind_of_connection_decides_the_port_when_none_is_set() {
        for (kind, security, port) in [
            ("tls", Security::StartTls, 587),
            ("ssl", Security::Tls, 465),
            ("n/a", Security::Opportunistic, 25),
            ("", Security::Opportunistic, 25),
        ] {
            let server = server_of(&settings(&[
                ("PHPMAILER-smtpHost", json!("smtp.example.com")),
                ("PHPMAILER-smtpHostType", json!(kind)),
                ("PHPMAILER-smtpHostPort", json!("not a port")),
                ("PHPMAILER-smtpHostAuth", json!(false)),
                ("PHPMAILER-verifyCert", json!(false)),
            ]))
            .expect("the settings name a server");

            assert_eq!((server.security, server.port), (security, port), "{kind}");
            assert_eq!(server.credentials, None);
            assert!(!server.verify_certificate);
        }
    }
}
