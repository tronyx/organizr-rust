//! The `/config` routes: the settings of the install, for the people who manage it.
//!
//! Every route needs an Admin or a Co-Admin. A secret is never shown: the answer says
//! `***Secure***` for a secret that has a value. A change is checked against the settings
//! registry first, and all settings of a request are stored, or none is.
//! `docs/deviations.md` lists the differences from the PHP application as D-06, D-07 and
//! D-53.

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use organizr_auth::api_key::API_KEY_LENGTH;
use organizr_auth::registration::{LONGEST_PASSWORD, hash_password};
use organizr_domain::GroupLevel;
use organizr_settings::Settings;
use serde_json::{Map, Value};

use crate::allowed_hosts::AllowedHosts;
use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::client::TrustedProxies;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::fields::{Refusal, is_nothing_in_php};
use crate::handlers::homepage::html;
use crate::handlers::login_backends;
use crate::state::AppState;

/// The level a caller needs for the configuration routes.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// What the answer shows for a secret that has a value.
pub(crate) const SECURE: &str = "***Secure***";

/// The setting that holds the API key. Only an Admin sees it and changes it.
const API_KEY: &str = "organizrAPI";

/// The setting that holds the registration password. A request sends the password, and
/// the install stores its hash.
const REGISTRATION_PASSWORD: &str = "registrationPassword";

/// The id of the install. It is made once, by the set-up, and is part of the name of the
/// session cookie.
const INSTALL_ID: &str = "uuid";

/// The settings that choose and address the service that vouches for a login, the Plex
/// owner, and the logins that need no second factor (D-132).
const LOGIN_SETTINGS: [&str; 15] = [
    "authType",
    "authBackend",
    "authBackendHost",
    "authBackendHostPrefix",
    "authBackendHostSuffix",
    "authBackendDomain",
    "authBaseDN",
    "ldapBindUsername",
    "ldapBindPassword",
    "ldapType",
    "ldapSSL",
    "ldapTLS",
    "plexAdmin",
    "ignoreTFALocal",
    "ignoreTFAIfPlexOAuth",
];

/// The settings of the hosts of Organizr and of the trusted proxies, which the variables
/// `ORGANIZR_ALLOWED_HOSTS` and `ORGANIZR_TRUSTED_PROXIES` win over (D-155).
const ALLOWED_HOSTS: &str = "allowedHosts";
const TRUSTED_PROXIES: &str = "trustedProxies";
/// The notice of an import for the Admin (D-158).
const IMPORT_NOTICE: &str = "importNotice";
/// The folder of the backups, whose *.zip files the backup routes list, send and delete
/// (D-183).
const BACKUP_LOCATION: &str = "backupLocation";

/// The services of the health checks plugin: the addresses that a run asks, and whether
/// each gets the API key.
const HEALTH_CHECK_SERVICES: &str = "HEALTHCHECKS-all-items";

/// The addresses of the services that the server sends a stored secret to, each with
/// those secrets. A Co-Admin sees a secret only as `***Secure***`, so a Co-Admin who
/// changes such an address enters each stored secret again in the same request (D-191).
/// Otherwise the server would send the secret to an address of the Co-Admin's choosing.
const SERVICE_SECRETS: [(&[&str], &[&str]); 34] = [
    (&["plexURL"], &["plexToken"]),
    (&["embyURL"], &["embyToken"]),
    (&["jellyfinURL"], &["jellyfinToken"]),
    (&["tautulliURL"], &["tautulliApikey"]),
    (&["ombiURL"], &["ombiToken", "ombiFallbackPassword"]),
    (
        &["overseerrURL"],
        &["overseerrToken", "overseerrFallbackPassword"],
    ),
    (&["petioURL"], &["petioToken", "petioFallbackPassword"]),
    (
        &["komgaURL"],
        &["komgaFallbackPassword", "komgaSSOMasterPassword"],
    ),
    (&["sonarrURL"], &["sonarrToken"]),
    (&["lidarrURL"], &["lidarrToken"]),
    (&["radarrURL"], &["radarrToken"]),
    (&["couchpotatoURL"], &["couchpotatoToken"]),
    (&["sickrageURL"], &["sickrageToken"]),
    (&["jdownloaderURL"], &["jdownloaderPassword"]),
    (&["sabnzbdURL"], &["sabnzbdToken"]),
    (&["nzbgetURL"], &["nzbgetPassword"]),
    (&["transmissionURL"], &["transmissionPassword"]),
    (&["delugeURL"], &["delugePassword"]),
    (&["qBittorrentURL"], &["qBittorrentPassword"]),
    (
        &["uTorrentURL"],
        &["uTorrentPassword", "uTorrentToken", "uTorrentCookie"],
    ),
    (
        &["rTorrentURL", "rTorrentURLOverride"],
        &["rTorrentPassword"],
    ),
    (&["jackettURL"], &["jackettToken"]),
    (&["prowlarrURL"], &["prowlarrToken"]),
    (
        &["homepageUserWatchStatsURL"],
        &["homepageUserWatchStatsToken"],
    ),
    (
        &["jellyStatURL", "jellyStatInternalURL"],
        &["jellyStatApikey"],
    ),
    (&["healthChecksURL"], &["healthChecksToken"]),
    (&["unifiURL"], &["unifiPassword", "unifiCookie"]),
    (&["piholeURL"], &["piholeToken"]),
    (&["adguardURL"], &["adGuardPassword"]),
    (&["octoprintURL"], &["octoprintToken"]),
    (&["uptimeKumaURL"], &["uptimeKumaToken"]),
    (
        &["INVITES-komga-uri"],
        &[
            "INVITES-komga-api-key",
            "INVITES-komga-default-user-password",
        ],
    ),
    (
        &["INVITES-nextcloud-url"],
        &["INVITES-nextcloud-admin-password"],
    ),
    (&["PHPMAILER-smtpHost"], &["PHPMAILER-smtpHostPassword"]),
];

/// The first address of a service that the changes move while a stored secret of that
/// service is not in the changes, with that secret (D-191). A secret that the form sent
/// back as `***Secure***` is not in the changes.
fn secret_left_behind(
    stored: &Settings,
    changes: &[(String, Value)],
) -> Option<(&'static str, &'static str)> {
    let sent = |key: &str| changes.iter().find(|(name, _)| name == key);
    SERVICE_SECRETS.iter().find_map(|(addresses, secrets)| {
        let moved = addresses.iter().find(|address| {
            sent(address).is_some_and(|(_, value)| stored.get(address).as_ref() != Some(value))
        })?;
        let secret = secrets.iter().find(|secret| {
            let has_value = stored
                .get(secret)
                .is_some_and(|value| value.as_str().is_some_and(|text| !text.is_empty()));
            has_value && sent(secret).is_none()
        })?;
        Some((*moved, *secret))
    })
}

/// Whether only an Admin can change a setting: the API key, the settings that let a
/// proxy say who the caller is, the scripts that run on every page, the default look
/// (D-125), the style sheets of every page (D-189), the trust of a custom HTML item with
/// the HTML of a trusted item (D-131), and the settings that a login trusts (D-132),
/// among them every setting of OpenID Connect, which picks the providers and the group
/// that a login gives (D-133), and the services of the health checks plugin, which get
/// the API key (D-134). Single sign-on sends the password of every login onward, so its
/// switches are for Admins only, and so are its addresses while it is on (D-135). The
/// folder of the backups is for Admins only too (D-183), and so is the way e-mails go
/// out: the mail switch, the SMTP server and the recovery e-mail, since a recovery e-mail
/// carries a new password of any account, the Admin's among them (D-197).
///
/// `settings` decides the settings that are for Admins only some of the time. Such a rule
/// reads only settings that are always for Admins only, so for a Co-Admin the copy that
/// takes the values of a request still holds what is stored.
fn is_for_admins_only(key: &str, settings: &Settings) -> bool {
    key == API_KEY
        || key.starts_with("authProxy")
        || LOGIN_SETTINGS.contains(&key)
        || key.starts_with("oidc")
        || key == HEALTH_CHECK_SERVICES
        || key.starts_with("sso")
        || key == ALLOWED_HOSTS
        || key == TRUSTED_PROXIES
        || key == IMPORT_NOTICE
        || key == BACKUP_LOCATION
        || is_mail_delivery(key)
        || is_trusted_by_a_login(key, settings)
        || key == "customJava"
        || key == "customThemeJava"
        || key == "customCss"
        || key == "customThemeCss"
        || key == "layout"
        || key == "modernTheme"
        || key == "modernColorSet"
        || key
            .strip_prefix("homepageCustomHTML")
            .and_then(|rest| rest.strip_suffix("Trusted"))
            .is_some_and(|number| html::NUMBERS.contains(&number))
        || key
            .strip_prefix("customHTML")
            .is_some_and(|number| html::is_trusted(settings, number))
}

/// Whether a setting decides where e-mails go and what the recovery e-mail says: the mail
/// switch, every setting of the SMTP server (its address, port, account, encryption and
/// sender), the check of its certificate, and the text and subject of the recovery
/// e-mail, which holds the new password (D-197).
fn is_mail_delivery(key: &str) -> bool {
    key == "PHPMAILER-enabled"
        || key.starts_with("PHPMAILER-smtpHost")
        || key == "PHPMAILER-verifyCert"
        || key.starts_with("PHPMAILER-emailTemplateReset")
}

/// Whether a login trusts a setting that is open to a Co-Admin otherwise: the address of
/// the Emby or Jellyfin server that may vouch for an Admin, and the local network that
/// needs no second factor (D-132), and the address that single sign-on sends the password
/// of a login to (D-135). A login through Emby Connect opens no Admin account (D-127), so
/// it does not count.
fn is_trusted_by_a_login(key: &str, settings: &Settings) -> bool {
    let backend = settings.auth_backend.as_str();
    match key {
        "embyURL" => [login_backends::EMBY_LOCAL, login_backends::EMBY_ALL].contains(&backend),
        "jellyfinURL" => backend == login_backends::JELLYFIN || settings.sso_jellyfin,
        "jellyfinSSOURL" => settings.sso_jellyfin,
        "localIPList" => settings.ignore_tfa_local,
        "ombiURL" => settings.sso_ombi,
        "overseerrURL" => settings.sso_overseerr,
        "tautulliURL" => settings.sso_tautulli,
        "petioURL" => settings.sso_petio,
        "komgaURL" => settings.sso_komga,
        _ => false,
    }
}

/// A setting as the API shows it to a caller of the given level.
pub(crate) fn shown(key: &str, value: Value, level: GroupLevel) -> Value {
    let is_empty = value.as_str().is_some_and(str::is_empty);
    let may_see = key == API_KEY && level == GroupLevel::ADMIN;
    if Settings::is_secret(key) && !is_empty && !may_see {
        Value::String(SECURE.to_owned())
    } else {
        value
    }
}

/// `GET /config`: every setting, sorted by key.
pub async fn all(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Map<String, Value>>, Refusal> {
    caller.require(MANAGER)?;
    Ok(Envelope::success(matching(
        &state.settings(),
        caller.level(),
        "",
    )))
}

/// The settings whose key holds the text, in any letter case, sorted by key.
fn matching(settings: &Settings, level: GroupLevel, part: &str) -> Map<String, Value> {
    let part = part.to_ascii_lowercase();
    let mut keys: Vec<&str> = Settings::KEYS
        .iter()
        .copied()
        .filter(|key| key.to_ascii_lowercase().contains(&part))
        .collect();
    keys.sort_unstable();
    keys.into_iter()
        .filter_map(|key| {
            settings
                .get(key)
                .map(|value| (key.to_owned(), shown(key, value, level)))
        })
        .collect()
}

/// `GET /config/{item}`: one setting, by its key.
pub async fn item(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(item): Path<String>,
) -> Result<Response, Refusal> {
    read(&state, &caller, &item, "")
}

/// `GET /config/{item}/{term}`: with `search` as the item, the settings whose key holds
/// the term. With another item, that setting: the term is ignored.
pub async fn item_with_term(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path((item, term)): Path<(String, String)>,
) -> Result<Response, Refusal> {
    read(&state, &caller, &item, &term)
}

fn read(state: &AppState, caller: &Caller, item: &str, term: &str) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    let settings = state.settings();
    if item.eq_ignore_ascii_case("search") {
        let found = matching(&settings, caller.level(), term);
        // PHP sends `null` for an empty result.
        let data = (!found.is_empty()).then_some(found);
        return Ok(Envelope::success(data)
            .with_message(format!("Search results for term: {term}"))
            .into_response());
    }
    let Some(value) = settings.get(item) else {
        return Err(Refusal::not_found(&format!(
            "{item} is not defined or is blank"
        )));
    };
    let value = shown(item, value, caller.level());
    let data = (!is_nothing_in_php(&value)).then_some(value);
    Ok(Envelope::success(data)
        .with_message(format!("The value for {item}"))
        .into_response())
}

/// The refusal for a value that does not fit its setting.
fn not_valid(key: &str) -> Refusal {
    Refusal::unprocessable(&format!("Config item: {key} is not valid"))
}

/// The value to store for one setting of a request, or `None` when the request leaves the
/// setting as it is.
///
/// `settings` is a copy of the settings that takes each value, so that a value that does
/// not fit is found before anything is stored.
async fn accepted(
    settings: &mut Settings,
    level: GroupLevel,
    key: &str,
    sent: &Value,
) -> Result<Option<Value>, Refusal> {
    if !Settings::KEYS.contains(&key) {
        return Err(Refusal::unprocessable(&format!(
            "Config item: {key} is not a setting"
        )));
    }
    if key == INSTALL_ID {
        return Err(Refusal::unprocessable(&format!(
            "Config item: {key} cannot be changed"
        )));
    }
    if level != GroupLevel::ADMIN && is_for_admins_only(key, settings) {
        return Err(Refusal::Request(
            StatusCode::UNAUTHORIZED,
            format!("Config item: {key} can only be changed by an admin"),
        ));
    }
    // A form sends the text that the answer showed for a secret back when nobody changed it.
    if Settings::is_secret(key) && sent.as_str() == Some(SECURE) {
        return Ok(None);
    }

    let value = match (key, sent) {
        // PHP stores the empty text for "nothing".
        (_, Value::Null) => Value::String(String::new()),
        // The lists are read as the variables are, so a value that would stop the server
        // at start is refused here too (D-155).
        (ALLOWED_HOSTS, Value::String(list)) if list.parse::<AllowedHosts>().is_err() => {
            return Err(not_valid(key));
        }
        (TRUSTED_PROXIES, Value::String(list)) if list.parse::<TrustedProxies>().is_err() => {
            return Err(not_valid(key));
        }
        (REGISTRATION_PASSWORD, Value::String(password)) if !password.is_empty() => {
            if password.len() > LONGEST_PASSWORD {
                return Err(Refusal::over_limit(LONGEST_PASSWORD));
            }
            let hash = hash_password(password.clone())
                .await
                .map_err(Refusal::from)?;
            Value::String(hash.expose().to_owned())
        }
        // The key is compared byte by byte, so it must have 20 ASCII characters.
        (API_KEY, Value::String(key)) if key.len() != API_KEY_LENGTH || !key.is_ascii() => {
            return Err(Refusal::unprocessable(&format!(
                "Config item: {API_KEY} must have {API_KEY_LENGTH} characters"
            )));
        }
        (_, value) => value.clone(),
    };
    if settings.set(key, &value).is_ok() {
        return Ok(Some(value));
    }
    // A form sends a switch as the text `true` or `false`.
    let switch = match value.as_str() {
        Some("true") => Value::Bool(true),
        Some("false") => Value::Bool(false),
        _ => return Err(not_valid(key)),
    };
    settings.set(key, &switch).map_err(|_| not_valid(key))?;
    Ok(Some(switch))
}

/// `PUT /config`: changes the settings that the body names.
///
/// The body is an object: each key is a setting, each value its new value. A key that is
/// no setting, and a value that does not fit its setting, are refused, and then nothing
/// changes. The field `formKey` is ignored.
pub async fn change(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    // A body that is a list names no setting. PHP would store its entries under numbers.
    if data.fields().next().is_none() {
        return Err(Refusal::conflict("No data submitted"));
    }
    // A copy that takes each value, so that a value that does not fit is found before
    // anything is stored. The settings are large, so the copy does not stand in this future.
    let mut settings = Box::new((*state.settings()).clone());
    let mut changes = Vec::new();
    for (key, sent) in data.fields() {
        if key.eq_ignore_ascii_case("formkey") {
            continue;
        }
        // A variable of the container wins over its setting (D-155).
        let variable = match key {
            ALLOWED_HOSTS if !state.allowed_hosts_of_the_variable().is_empty() => {
                Some("ORGANIZR_ALLOWED_HOSTS")
            }
            TRUSTED_PROXIES if !state.trusted_proxies_of_the_variable().is_empty() => {
                Some("ORGANIZR_TRUSTED_PROXIES")
            }
            _ => None,
        };
        if let Some(variable) = variable {
            let message = format!(
                "Config item: {key} is set by {variable} in the environment of the container"
            );
            return Err(Refusal::conflict(&message));
        }
        if let Some(value) = accepted(&mut settings, caller.level(), key, sent).await? {
            changes.push((key.to_owned(), value));
        }
    }

    if caller.level() != GroupLevel::ADMIN
        && let Some((address, secret)) = secret_left_behind(&state.settings(), &changes)
    {
        return Err(Refusal::unprocessable(&format!(
            "Config item: {secret} must be entered again when {address} changes"
        )));
    }

    let keys: Vec<&str> = changes.iter().map(|(key, _)| key.as_str()).collect();
    tracing::info!(settings = ?keys, "settings were changed");
    state
        .change_settings(changes)
        .await
        .map_err(ApiError::from)?;
    Ok(Envelope::success(NoData)
        .with_message("Config items updated")
        .into_response())
}

#[cfg(test)]
mod tests {
    use organizr_settings::Settings;
    use serde_json::Value;

    use super::{SERVICE_SECRETS, is_for_admins_only};

    #[test]
    fn settings_for_admins_only() {
        let mut settings = Settings::defaults();
        for key in [
            "homepageCustomHTML01Trusted",
            "homepageCustomHTML08Trusted",
            "organizrAPI",
            "authProxyEnabled",
            "authProxyHeaderName",
            "authProxyWhitelist",
            "customJava",
            "customThemeJava",
            "customCss",
            "customThemeCss",
            "layout",
            "modernTheme",
            "modernColorSet",
            "authType",
            "authBackend",
            "ldapBindPassword",
            "plexAdmin",
            "ignoreTFALocal",
            "oidcEnabled",
            "oidcGroupMappings",
            "oidcDefaultGroupId",
            "HEALTHCHECKS-all-items",
            "ssoOmbi",
            "ssoTautulliAuth",
            "backupLocation",
        ] {
            assert!(is_for_admins_only(key, &settings), "{key}");
        }
        for key in [
            "title",
            "plexToken",
            "customHTML01",
            "embyURL",
            "jellyfinURL",
            "localIPList",
        ] {
            assert!(!is_for_admins_only(key, &settings), "{key}");
        }

        // The HTML of a trusted item, and only of that item.
        settings
            .set("homepageCustomHTML01Trusted", &Value::Bool(true))
            .expect("a switch takes true");
        assert!(is_for_admins_only("customHTML01", &settings));
        for key in ["customHTML02", "customHTML1", "customHTML01x"] {
            assert!(!is_for_admins_only(key, &settings), "{key}");
        }
    }

    #[test]
    fn each_address_of_a_service_is_a_setting_with_secrets() {
        for (addresses, secrets) in SERVICE_SECRETS {
            for address in addresses {
                assert!(Settings::KEYS.contains(address), "{address}");
                assert!(!Settings::is_secret(address), "{address}");
            }
            for secret in secrets {
                assert!(Settings::is_secret(secret), "{secret}");
            }
        }
    }
}
