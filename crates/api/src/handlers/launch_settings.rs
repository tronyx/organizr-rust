//! The `settings`, `plugins` and `sso` parts of the launch answer.
//!
//! These parts tell the front end how the install is configured, as far as a page needs
//! to know it, and which cookies of other services the browser holds. The names and the
//! shapes are those of `organizrSpecialSettings()`, `pluginGlobalList()` and
//! `ssoCookies()` of the PHP application.

use axum::http::{HeaderMap, header};
use organizr_domain::GroupLevel;
use organizr_settings::Settings;
use serde_json::{Map, Value, json};

use crate::client::Client;
use crate::cookies::request_cookies;

/// The cookies of a request as PHP reads them: with the names and the values of
/// `$_COOKIE`.
///
/// PHP writes `_` for a dot or a space in a name, decodes `%XX` in a value, and keeps the
/// first of two cookies with the same name.
fn php_cookies(headers: &HeaderMap) -> Vec<(String, String)> {
    let mut cookies: Vec<(String, String)> = Vec::new();
    for (name, value) in request_cookies(headers) {
        let name = name.replace(['.', ' '], "_");
        if cookies.iter().all(|(known, _)| *known != name) {
            cookies.push((name, percent_decoded(value)));
        }
    }
    cookies
}

/// Decodes each `%XX` of a text. A `+` stays a `+`, and a `%` that no two hex digits
/// follow stays as it is.
fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while let Some((&first, after)) = rest.split_first() {
        if let (b'%', Some(&[high, low])) = (first, after.first_chunk::<2>())
            && let (Some(high), Some(low)) = (hex_digit(high), hex_digit(low))
        {
            decoded.push(high * 16 + low);
            rest = after.get(2..).unwrap_or_default();
        } else {
            decoded.push(first);
            rest = after;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_digit(byte: u8) -> Option<u8> {
    char::from(byte)
        .to_digit(16)
        .and_then(|digit| u8::try_from(digit).ok())
}

/// Whether PHP takes a text for true: every text but the empty one and `0`.
fn is_truthy(text: &str) -> bool {
    !text.is_empty() && text != "0"
}

/// The settings whose key holds the given text, each in its stored form. PHP picks
/// settings by a part of their key in several places.
///
/// The settings come sorted by key. PHP gives them in the order of its configuration
/// file, which is this order from the first change of a setting on (D-50).
fn settings_named(settings: &Settings, part: &str) -> Vec<(&'static str, Value)> {
    let mut named: Vec<(&'static str, Value)> = Settings::KEYS
        .iter()
        .filter(|key| key.contains(part))
        .filter_map(|key| settings.get(key).map(|value| (*key, value)))
        .collect();
    named.sort_by_key(|(key, _)| *key);
    named
}

fn whole_number(value: &Value) -> Option<u64> {
    value.as_str().and_then(|text| text.parse().ok())
}

/// The caller of the launch route, and what the answer says about that caller.
#[derive(Debug)]
pub struct Visitor<'a> {
    /// The settings of the install.
    pub settings: &'a Settings,
    /// The level of the caller.
    pub level: GroupLevel,
    /// The sender of the request.
    pub client: &'a Client,
    /// Whether the sender is on the local network.
    pub is_local: bool,
    /// The headers of the request.
    pub headers: &'a HeaderMap,
}

impl Visitor<'_> {
    /// Whether the caller has the given level or more access.
    fn may(&self, least_access: u32) -> bool {
        u32::from(self.level.get()) <= least_access
    }

    /// A setting in its stored form.
    fn stored(&self, key: &str) -> Value {
        self.settings.get(key).unwrap_or(Value::Null)
    }

    /// A text setting, or `false` when it is empty.
    fn stored_or_false(&self, key: &str) -> Value {
        match self.stored(key) {
            Value::String(text) if text.is_empty() => Value::Bool(false),
            value => value,
        }
    }

    /// A value that only Admins and Co-Admins get. Everybody else gets an empty text.
    fn for_admins(&self, value: Value) -> Value {
        if self.may(1) { value } else { json!("") }
    }

    fn header(&self, name: header::HeaderName) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }

    /// The `settings` part.
    pub fn settings(&self) -> Value {
        let cookies = php_cookies(self.headers);
        let has_cookie = |name: &str| cookies.iter().any(|(cookie, _)| cookie == name);
        let settings = self.settings;
        json!({
            "homepage": self.homepage(&has_cookie),
            "sso": self.single_sign_on(&has_cookie),
            "ping": {
                "onlineSound": self.stored("pingOnlineSound"),
                "offlineSound": self.stored("pingOfflineSound"),
                "statusSounds": self.stored("statusSounds"),
                "auth": self.stored("pingAuth"),
                "authMessage": self.stored("pingAuthMessage"),
                "authMs": self.stored("pingAuthMs"),
                "ms": self.stored("pingMs"),
                "adminRefresh": self.stored("adminPingRefresh"),
                "everyoneRefresh": self.stored("otherPingRefresh"),
            },
            "notifications": {
                "backbone": self.stored("notificationBackbone"),
                "position": self.stored("notificationPosition"),
            },
            "lockout": {
                "enabled": self.stored("lockoutSystem"),
                "timer": self.stored("lockoutTimeout"),
                "minGroup": self.stored("lockoutMinAuth"),
                "maxGroup": self.stored("lockoutMaxAuth"),
            },
            "user": {
                "agent": self.header(header::USER_AGENT),
                "oAuthLogin": has_cookie("oAuth"),
                "local": self.is_local,
                "ip": self.client.address.to_string(),
                "bypass": settings.bypass_login_for_local
                    && settings.local_login_user_id != 0
                    && self.is_local,
            },
            "login": {
                "rememberMe": self.stored("rememberMe"),
                "rememberMeDays": self.stored("rememberMeDays"),
                "wanDomain": self.stored("wanDomain"),
                "localAddress": self.stored("localAddress"),
                "enableLocalAddressForward": self.stored("enableLocalAddressForward"),
            },
            "misc": self.miscellaneous(),
            "menuLink": {
                "githubMenuLink": self.stored("githubMenuLink"),
                "organizrSupportMenuLink": self.stored("organizrSupportMenuLink"),
                "organizrDocsMenuLink": self.stored("organizrDocsMenuLink"),
                "organizrSignoutMenuLink": self.stored("organizrSignoutMenuLink"),
                "organizrFeatureRequestLink": self.stored("organizrFeatureRequestLink"),
            },
        })
    }

    fn homepage(&self, has_cookie: &dyn Fn(&str) -> bool) -> Value {
        let settings = self.settings;

        // Every setting with "Refresh" in its key, which are numbers. A secret is never
        // one of them: PHP sends a refresh token here when it is a number.
        let refresh: Map<String, Value> = settings_named(settings, "Refresh")
            .into_iter()
            .filter(|(key, _)| !Settings::is_secret(key))
            .map(|(key, value)| (key.to_owned(), value))
            .collect();
        let mut order = settings_named(settings, "homepageOrder");
        // The sort keeps the order of the keys for two items on one place.
        order.sort_by_key(|(_, place)| whole_number(place).unwrap_or(0));
        let order: Map<String, Value> = order
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect();

        let ombi_view = self.may(settings.homepage_ombi_auth);
        let ombi_request = self.may(settings.homepage_ombi_request_auth);
        let overseerr_view = self.may(settings.homepage_overseerr_auth);
        let overseerr_request = self.may(settings.homepage_overseerr_request_auth);
        json!({
            "refresh": refresh,
            "order": order,
            "search": {
                "enabled": self.may(settings.media_search_auth)
                    && settings.media_search
                    && is_truthy(&settings.plex_token),
                "type": self.stored("mediaSearchType"),
            },
            "requests": {
                "service": self.stored("defaultRequestService"),
            },
            "ombi": {
                "enabled": ombi_view
                    && ombi_request
                    && settings.homepage_ombi_enabled
                    && settings.sso_ombi
                    && has_cookie("Auth"),
                "authView": ombi_view,
                "authRequest": ombi_request,
                "sso": settings.sso_ombi,
                "cookie": has_cookie("Auth"),
                "alias": settings.ombi_alias,
                "ombiDefaultFilterAvailable": settings.ombi_default_filter_available,
                "ombiDefaultFilterUnavailable": settings.ombi_default_filter_unavailable,
                "ombiDefaultFilterApproved": settings.ombi_default_filter_approved,
                "ombiDefaultFilterUnapproved": settings.ombi_default_filter_unapproved,
                "ombiDefaultFilterDenied": settings.ombi_default_filter_denied,
            },
            "overseerr": {
                "enabled": overseerr_view
                    && overseerr_request
                    && settings.homepage_overseerr_enabled
                    && settings.sso_overseerr
                    && has_cookie("connect_sid"),
                "authView": overseerr_view,
                "authRequest": overseerr_request,
                "sso": settings.sso_overseerr,
                "cookie": has_cookie("connect_sid"),
                // PHP compares the level with the text "user" here, which is true for
                // every level but 0.
                "userSelectTv": settings.homepage_overseerr_request_auth != 0,
                "overseerrDefaultFilterAvailable": settings.overseerr_default_filter_available,
                "overseerrDefaultFilterUnavailable": settings.overseerr_default_filter_unavailable,
                "overseerrDefaultFilterApproved": settings.overseerr_default_filter_approved,
                "overseerrDefaultFilterUnapproved": settings.overseerr_default_filter_unapproved,
                "overseerrDefaultFilterDenied": settings.overseerr_default_filter_denied,
            },
            "jackett": {
                "homepageJackettBackholeDownload": settings.homepage_jackett_backhole_download,
            },
            "prowlarr": {
                "homepageProwlarrBackholeDownload": settings.homepage_prowlarr_backhole_download,
            },
            "options": {
                "alternateHomepageHeaders": self.stored("alternateHomepageHeaders"),
                "healthChecksTags": self.stored("healthChecksTags"),
                "titles": {
                    "tautulli": self.stored("tautulliHeader"),
                },
            },
            "media": {
                "jellyfin": self.stored("homepageJellyfinInstead"),
            },
        })
    }

    fn single_sign_on(&self, has_cookie: &dyn Fn(&str) -> bool) -> Value {
        let settings = self.settings;
        let plex_admin = if settings.plex_admin.is_empty() {
            json!(false)
        } else if settings.plex_admin.contains('@') {
            json!("email")
        } else {
            json!("username")
        };
        json!({
            "misc": {
                "oAuthLogin": has_cookie("oAuth"),
                "rememberMe": self.stored("rememberMe"),
                "rememberMeDays": self.stored("rememberMeDays"),
            },
            "plex": {
                "enabled": settings.sso_plex,
                "cookie": has_cookie("mpt"),
                "machineID": settings.plex_id.len() == 40,
                "token": !settings.plex_token.is_empty(),
                "plexAdmin": plex_admin,
                "strict": settings.plex_strict_friends,
                "oAuthEnabled": settings.plex_oauth,
                "backend": settings.auth_backend == "plex",
            },
            "tautulli": {
                "enabled": settings.sso_tautulli,
                // PHP looks for a setting with "tautulli_token" in its key. There is none.
                "cookie": false,
                "url": self.stored_or_false("tautulliURL"),
            },
            "overseerr": {
                "enabled": settings.sso_overseerr,
                // PHP looks for the cookie under a name that PHP never gives a cookie.
                "cookie": false,
                "url": self.stored_or_false("overseerrURL"),
                "api": !settings.overseerr_token.is_empty(),
            },
            "petio": {
                "enabled": settings.sso_petio,
                "cookie": has_cookie("petio_jwt"),
                "url": self.stored_or_false("petioURL"),
                "api": !settings.petio_token.is_empty(),
            },
            "ombi": {
                "enabled": settings.sso_ombi,
                "cookie": has_cookie("Auth"),
                "url": self.stored_or_false("ombiURL"),
                "api": !settings.ombi_token.is_empty(),
            },
            "jellyfin": {
                "enabled": settings.sso_jellyfin,
                "url": self.stored_or_false("jellyfinURL"),
                "ssoUrl": self.stored_or_false("jellyfinSSOURL"),
            },
            "komga": {
                "enabled": settings.sso_komga,
                "cookie": has_cookie("komga_token"),
                "url": self.stored_or_false("komgaURL"),
            },
        })
    }

    fn miscellaneous(&self) -> Value {
        let settings = self.settings;
        let install_id = if settings.install_id.is_empty() {
            Value::Null
        } else {
            json!(settings.install_id)
        };
        let menu_folds = settings.allow_collapsable_side_menu;
        json!({
            "installedPlugins": self.for_admins(self.stored("installedPlugins")),
            "installedThemes": self.for_admins(self.stored("installedThemes")),
            "return": self.header(header::REFERER).map_or(json!(false), Value::from),
            "authDebug": self.stored("authDebug"),
            "minimalLoginScreen": self.stored("minimalLoginScreen"),
            "unsortedTabs": self.stored("unsortedTabs"),
            "authType": self.stored("authType"),
            "authBackend": self.stored("authBackend"),
            "newMessageSound": self.stored("CHAT-newMessageSound-include"),
            "uuid": install_id,
            // The three values below exist for the page of the PHP application: whether
            // it runs in the official container, its git commit and its tables. They are
            // fixed here (D-50).
            "docker": self.for_admins(json!(false)),
            "githubCommit": self.for_admins(Value::Null),
            "schema": self.for_admins(json!([])),
            "debugArea": self.may(settings.debug_area_auth),
            "debugErrors": self.stored("debugErrors"),
            "sandbox": self.stored("sandbox"),
            "iframeAllow": self.stored("iframeAllow"),
            "expandCategoriesByDefault": self.stored("expandCategoriesByDefault"),
            "autoCollapseCategories": self.stored("autoCollapseCategories"),
            "autoExpandNavBar": self.stored("autoExpandNavBar"),
            "sideMenuCollapsed": menu_folds && settings.side_menu_collapsed,
            "collapseSideMenuOnClick": menu_folds && settings.collapse_side_menu_on_click,
            "authProxyOverrideLogout": self.stored("authProxyOverrideLogout"),
            "authProxyLogoutURL": self.stored("authProxyLogoutURL"),
            "disableHomepageModals": self.stored("disableHomepageModals"),
            "checkForUpdate": self.stored("checkForUpdate"),
        })
    }

    /// The `plugins` part: the switch of each plugin, and under `includes` the plugin
    /// settings that every page gets.
    pub fn plugins(&self) -> Value {
        let mut plugins: Map<String, Value> = settings_named(self.settings, "-enabled")
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect();
        let includes: Map<String, Value> = settings_named(self.settings, "-include")
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect();
        plugins.insert("includes".to_owned(), Value::Object(includes));
        Value::Object(plugins)
    }

    /// The `sso` part: the cookies of other services that the browser sent, or `false`
    /// for each one that it did not send.
    pub fn sso_cookies(&self) -> Value {
        let cookies = php_cookies(self.headers);
        let value_of = |name: &str| {
            cookies
                .iter()
                .find(|(cookie, _)| cookie == name)
                .map_or(json!(false), |(_, value)| json!(value))
        };
        let mut answer = Map::new();
        answer.insert("myPlexAccessToken".to_owned(), value_of("mpt"));
        answer.insert("id_token".to_owned(), value_of("Auth"));
        answer.insert(
            "jellyfin_credentials".to_owned(),
            value_of("jellyfin_credentials"),
        );
        answer.insert("komga_token".to_owned(), value_of("komga_token"));
        // Jellyfin keeps its login in cookies with "user-" in their names.
        for (name, value) in &cookies {
            if name.contains("user-") {
                answer.insert(name.clone(), json!(value));
            }
        }
        Value::Object(answer)
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header};

    use super::{percent_decoded, php_cookies};

    #[test]
    fn value_of_a_cookie_is_decoded_as_php_decodes_it() {
        assert_eq!(percent_decoded("plex%20token+one"), "plex token+one");
        assert_eq!(percent_decoded("%7B%22a%22%3a1%7d"), "{\"a\":1}");
        assert_eq!(percent_decoded("caf%C3%A9"), "café");
        // What is no code stays as it is.
        assert_eq!(percent_decoded("100%"), "100%");
        assert_eq!(percent_decoded("%4"), "%4");
        assert_eq!(percent_decoded("%zz%41"), "%zzA");
        assert_eq!(percent_decoded(""), "");
    }

    #[test]
    fn cookies_have_the_names_that_php_gives_them() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("connect.sid=one; a b=two; connect_sid=three; plain=four"),
        );

        assert_eq!(
            php_cookies(&headers),
            [
                ("connect_sid".to_owned(), "one".to_owned()),
                ("a_b".to_owned(), "two".to_owned()),
                ("plain".to_owned(), "four".to_owned()),
            ]
        );
    }
}
