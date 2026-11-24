//! The domains that cookies are set for.
//!
//! The PHP application sets each cookie twice: once for a wider domain that it works out
//! from the Host header, so that other sub-domains receive the cookie, and once for the
//! host itself. Forward-auth depends on the wider one: a browser sends the session cookie
//! to `sonarr.example.com` only when the cookie was set for `.example.com`.
//!
//! This is a port of the PHP method `parseDomain`, kept exactly, including the cases where
//! its result is odd. `parity/php-cookie-domain/generate.php` records what PHP returns,
//! and a test compares every recorded host.

/// Providers of dynamic DNS that give each customer a sub-domain. For a host under one of
/// them the cookie must not be set for the provider's own domain, which other customers
/// share.
const DYNAMIC_DNS_DOMAINS: [&str; 31] = [
    "ddns.net",
    "ddnsking.com",
    "3utilities.com",
    "bounceme.net",
    "freedynamicdns.net",
    "freedynamicdns.org",
    "gotdns.ch",
    "hopto.org",
    "myddns.me",
    "myds.me",
    "myftp.biz",
    "myftp.org",
    "myvnc.com",
    "noip.com",
    "onthewifi.com",
    "redirectme.net",
    "serveblog.net",
    "servecounterstrike.com",
    "serveftp.com",
    "servegame.com",
    "servehalflife.com",
    "servehttp.com",
    "serveirc.com",
    "serveminecraft.net",
    "servemp3.com",
    "servepics.com",
    "servequake.com",
    "sytes.net",
    "viewdns.net",
    "webhop.me",
    "zapto.org",
];

/// The two domains of a cookie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookieDomains {
    /// The wider domain, with a leading dot, for example `.example.com`. Empty when the
    /// cookie gets no domain at all: for an IP address or a host without a dot.
    pub wide: String,
    /// The host without its port.
    pub host: String,
}

/// Works out the cookie domains for the value of a Host header.
pub fn cookie_domains(host_header: &str) -> CookieDomains {
    // Everything from the first colon on is the port.
    let host = host_header.split(':').next().unwrap_or_default();
    let labels: Vec<&str> = host.split('.').collect();

    let wide = match labels.as_slice() {
        [_, _] => format!(".{host}"),
        [first, second, third] => {
            // The order of the checks is PHP's: `www` is looked at before `co`, so
            // `www.co.uk` loses its first label and `example.co.uk` keeps it.
            let keeps_first_label =
                is_dynamic_dns(second, third) || (*first != "www" && *second == "co");
            if keeps_first_label {
                format!(".{first}.{second}.{third}")
            } else {
                format!(".{second}.{third}")
            }
        }
        [first, second, third, fourth, ..] => {
            if host.starts_with(|character: char| character.is_ascii_digit()) {
                String::new()
            } else if is_dynamic_dns(third, fourth) {
                format!(".{first}.{second}.{third}.{fourth}")
            } else {
                format!(".{second}.{third}.{fourth}")
            }
        }
        _ => String::new(),
    };

    CookieDomains {
        wide,
        host: host.to_owned(),
    }
}

/// Whether two neighboring labels are the domain of a dynamic DNS provider.
fn is_dynamic_dns(label: &str, next_label: &str) -> bool {
    let domain = format!("{label}.{next_label}").to_ascii_lowercase();
    DYNAMIC_DNS_DOMAINS.contains(&domain.as_str())
}
