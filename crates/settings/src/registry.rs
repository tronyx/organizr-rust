//! The settings of the application: every key, its type and its default.
//!
//! This is the one place a setting is defined. A key that is not listed here does not
//! exist: it cannot be read, and an attempt to store it is refused.
//!
//! The keys and the defaults are those of `api/config/default.php` of Organizr 2.1.5000.
//! Keys are added here when the feature that uses them is built.

use serde_json::Value;
use thiserror::Error;

use crate::choices::{AuthType, UnsortedTabs};
use crate::codec::{Anything, Choice, Codec, Flag, List, Number, Secret, Text, TextNumber};

/// Settings that the defaults of PHP name differently from the key that PHP reads and its
/// form writes: each default of PHP with the key that holds its value here (D-112). PHP's
/// defaults have `ProwlarrURL`, but its form and its code use `prowlarrURL`, so in PHP the
/// value that an Admin saves is in the second key.
pub const RENAMED_SETTINGS: [(&str, &str); 4] = [
    ("ProwlarrURL", "prowlarrURL"),
    ("ProwlarrToken", "prowlarrToken"),
    (
        "ProwlarrUseCustomCertificate",
        "prowlarrUseCustomCertificate",
    ),
    ("ProwlarrDisableCertCheck", "prowlarrDisableCertCheck"),
];

/// Beginnings of the keys of PHP's settings for parts that were removed, because the
/// service behind them shut down: Shuck Stop, a watcher of drive prices (D-116). Such a
/// setting is no setting here; the import leaves it out, and a stored value is deleted
/// at start-up.
pub const REMOVED_SETTING_PREFIXES: [&str; 1] = ["SHUCKSTOP-"];

/// Whether a key belongs to a removed part (see [`REMOVED_SETTING_PREFIXES`]).
pub fn is_removed_setting(key: &str) -> bool {
    REMOVED_SETTING_PREFIXES
        .iter()
        .any(|prefix| key.starts_with(prefix))
}

/// Why a setting cannot be read or changed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SettingsError {
    /// The key is not a setting.
    #[error("`{0}` is not a setting")]
    UnknownKey(String),

    /// The value does not fit the setting.
    #[error("the value of `{key}` is not valid: {reason}")]
    InvalidValue {
        /// The key of the setting.
        key: &'static str,
        /// What is wrong with the value.
        reason: String,
    },

    /// A stored secret cannot be opened with the key of the install.
    #[error("the stored secret `{key}` cannot be opened with the key of this install")]
    UnreadableSecret {
        /// The key the secret is stored under.
        key: String,
    },

    /// A stored value is not JSON.
    #[error("the stored value of `{key}` is not JSON: {reason}")]
    InvalidJson {
        /// The key the value is stored under.
        key: String,
        /// What the JSON parser reported.
        reason: String,
    },
}

/// Defines the settings: one line per setting gives the field, the codec, the key of the
/// PHP application and the default.
macro_rules! settings {
    ($( $(#[$doc:meta])* $field:ident: $codec:ty = $key:literal, $default:expr; )*) => {
        /// All settings with their current values, typed.
        ///
        /// A field is read directly, for example `settings.remember_me_days`. [`Settings::get`]
        /// and [`Settings::set`] work with the key and the stored form instead, for the
        /// configuration routes of the API.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Settings {
            $( $(#[$doc])* pub $field: <$codec as Codec>::Value, )*
        }

        impl Settings {
            /// The key of every setting, in the order of definition.
            pub const KEYS: &'static [&'static str] = &[ $( $key, )* ];

            /// Every setting at its default.
            pub fn defaults() -> Self {
                Self { $( $field: $default, )* }
            }

            /// The value of a setting in its stored form, or `None` for an unknown key.
            pub fn get(&self, key: &str) -> Option<Value> {
                match key {
                    $( $key => Some(<$codec as Codec>::encode(&self.$field)), )*
                    _ => None,
                }
            }

            /// Whether a setting is a secret. An unknown key is none.
            pub fn is_secret(key: &str) -> bool {
                match key {
                    $( $key => <$codec as Codec>::SECRET, )*
                    _ => false,
                }
            }

            /// Changes a setting from a value in its stored form.
            ///
            /// An unknown key or a value that does not fit is refused, and nothing changes.
            pub fn set(&mut self, key: &str, stored: &Value) -> Result<(), SettingsError> {
                match key {
                    $(
                        $key => {
                            self.$field = <$codec as Codec>::decode(stored)
                                .map_err(|reason| SettingsError::InvalidValue { key: $key, reason })?;
                            Ok(())
                        }
                    )*
                    _ => Err(SettingsError::UnknownKey(key.to_owned())),
                }
            }
        }
    };
}

settings! {
    /// The name of the install, shown as the page title.
    title: Text = "title", "Organizr V2".to_owned();
    /// The description of the install, for search engines and link previews.
    description: Text = "description", "Organizr - Accept no others".to_owned();
    /// The theme of the user interface.
    theme: Text = "theme", "Organizr".to_owned();
    /// The color style of the user interface.
    style: Text = "style", "dark".to_owned();
    /// The layout that a person sees until they choose one: `og` or `modern` (D-125).
    layout: Text = "layout", "og".to_owned();
    /// The theme of the modern layout until a person chooses one (D-125).
    modern_theme: Text = "modernTheme", "organizr".to_owned();
    /// The color set of the modern layout until a person chooses one (D-125).
    modern_color_set: Text = "modernColorSet", "dark".to_owned();
    /// Where tabs of the default category stand in the menu.
    unsorted_tabs: Choice<UnsortedTabs> = "unsortedTabs", UnsortedTabs::Top;
    /// The release line of the PHP application. Kept for clients that read it.
    branch: Text = "branch", "v2-master".to_owned();

    /// The picture at the top of the menu, when `useLogo` is on.
    logo: Text = "logo", "plugins/images/organizr/organizr-logo-h.png".to_owned();
    /// The picture on the login page, when `useLogoLogin` is on.
    login_logo: Text = "loginLogo", "plugins/images/organizr/organizr-logo-h.png".to_owned();
    /// The background picture of the login page.
    login_wallpaper: Text = "loginWallpaper", String::new();
    /// Whether the menu shows the logo and not the title.
    use_logo: Flag = "useLogo", false;
    /// Whether the login page shows the logo and not the title.
    use_logo_login: Flag = "useLogoLogin", false;
    /// The colors of the page. An empty value means the color of the theme.
    header_color: Text = "headerColor", String::new();
    header_text_color: Text = "headerTextColor", String::new();
    sidebar_color: Text = "sidebarColor", String::new();
    sidebar_text_color: Text = "sidebarTextColor", String::new();
    accent_color: Text = "accentColor", String::new();
    accent_text_color: Text = "accentTextColor", String::new();
    button_color: Text = "buttonColor", String::new();
    button_text_color: Text = "buttonTextColor", String::new();
    button_hover_color: Text = "buttonHoverColor", String::new();
    button_text_hover_color: Text = "buttonTextHoverColor", String::new();
    /// Style sheets and scripts that the admin adds to every page.
    custom_css: Text = "customCss", String::new();
    custom_theme_css: Text = "customThemeCss", String::new();
    custom_java: Text = "customJava", String::new();
    custom_theme_java: Text = "customThemeJava", String::new();

    /// Which password check a login uses.
    auth_type: Choice<AuthType> = "authType", AuthType::Internal;
    /// The external service that checks passwords, or nothing.
    auth_backend: Text = "authBackend", String::new();
    /// How many days a login with "remember me" lasts.
    remember_me_days: TextNumber<u32> = "rememberMeDays", 7;
    /// How many failed logins lock a caller out.
    login_attempts: TextNumber<u32> = "loginAttempts", 5;
    /// How long a lock-out lasts, in milliseconds.
    login_lockout: TextNumber<u64> = "loginLockout", 60_000;
    /// Whether a session is only valid from the address it was created at.
    match_user_ip: Flag = "matchUserIP", false;
    /// Whether a session is only valid from the browser it was created in.
    match_user_agents: Flag = "matchUserAgents", false;
    /// Whether users can lock their session behind the lock screen.
    lock_screen: Flag = "lockScreen", false;
    /// Whether the login page hides the link to register.
    hide_registration: Flag = "hideRegistration", false;
    /// Addresses and ranges, separated by commas, that count as the local network.
    local_ip_list: Text = "localIPList", String::new();
    /// The host names of Organizr that a service address which is only a path is
    /// completed with, when `ORGANIZR_ALLOWED_HOSTS` names none (D-155).
    allowed_hosts: Text = "allowedHosts", String::new();
    /// The reverse proxies whose forwarding headers are believed, when
    /// `ORGANIZR_TRUSTED_PROXIES` names none (D-155).
    trusted_proxies: Text = "trustedProxies", String::new();

    /// The random id of the install. It is part of the name of the session cookie. It is
    /// empty until the install is set up.
    install_id: Text = "uuid", String::new();
    /// The API key of the install: 20 characters that give a script the access of an
    /// Admin. It is empty until the install is set up.
    api_key: Secret = "organizrAPI", String::new();
    /// The license the install is used under: `personal` or `business`. It is empty until
    /// the install is set up.
    license: Text = "license", String::new();
    /// The hash of the password that a new user must know to register an account. Empty
    /// means that nobody can register.
    registration_password_hash: Secret = "registrationPassword", String::new();

    // The settings below have the type that their default shows: a switch, a number or
    // text. A setting gets its description, and a narrower type where it has one, when
    // the feature that reads it is built.
    auth_backend_host: Text = "authBackendHost", String::new();
    auth_backend_host_prefix: Text = "authBackendHostPrefix", String::new();
    auth_backend_host_suffix: Text = "authBackendHostSuffix", String::new();
    ldap_bind_username: Text = "ldapBindUsername", String::new();
    ldap_bind_password: Secret = "ldapBindPassword", String::new();
    ldap_ssl: Flag = "ldapSSL", false;
    ldap_tls: Flag = "ldapTLS", false;
    auth_base_dn: Text = "authBaseDN", String::new();
    auth_backend_domain: Text = "authBackendDomain", String::new();
    ldap_type: TextNumber<u32> = "ldapType", 1;
    alternate_homepage_headers: Flag = "alternateHomepageHeaders", false;
    plex_url: Text = "plexURL", String::new();
    plex_tab_url: Text = "plexTabURL", String::new();
    plex_token: Secret = "plexToken", String::new();
    plex_tab_name: Text = "plexTabName", String::new();
    plex_admin: Text = "plexAdmin", String::new();
    plex_disable_cert_check: Flag = "plexDisableCertCheck", false;
    plex_use_custom_certificate: Flag = "plexUseCustomCertificate", false;
    emby_tab_url: Text = "embyTabURL", String::new();
    emby_tab_name: Text = "embyTabName", String::new();
    emby_url: Text = "embyURL", String::new();
    emby_token: Secret = "embyToken", String::new();
    emby_disable_cert_check: Flag = "embyDisableCertCheck", false;
    emby_use_custom_certificate: Flag = "embyUseCustomCertificate", false;
    jellyfin_tab_name: Text = "jellyfinTabName", String::new();
    jellyfin_url: Text = "jellyfinURL", String::new();
    jellyfin_token: Secret = "jellyfinToken", String::new();
    jellyfin_ssourl: Text = "jellyfinSSOURL", String::new();
    jellyfin_use_custom_certificate: Flag = "jellyfinUseCustomCertificate", false;
    jellyfin_disable_cert_check: Flag = "jellyfinDisableCertCheck", false;
    plex_id: Text = "plexID", String::new();
    tautulli_url: Text = "tautulliURL", String::new();
    ombi_url: Text = "ombiURL", String::new();
    ombi_token: Secret = "ombiToken", String::new();
    ombi_alias: Flag = "ombiAlias", false;
    ombi_fallback_user: Text = "ombiFallbackUser", String::new();
    ombi_fallback_password: Secret = "ombiFallbackPassword", String::new();
    ombi_disable_cert_check: Flag = "ombiDisableCertCheck", false;
    ombi_use_custom_certificate: Flag = "ombiUseCustomCertificate", false;
    overseerr_url: Text = "overseerrURL", String::new();
    overseerr_token: Secret = "overseerrToken", String::new();
    overseerr_fallback_user: Text = "overseerrFallbackUser", String::new();
    overseerr_fallback_password: Secret = "overseerrFallbackPassword", String::new();
    overseerr_disable_cert_check: Flag = "overseerrDisableCertCheck", false;
    overseerr_use_custom_certificate: Flag = "overseerrUseCustomCertificate", false;
    petio_url: Text = "petioURL", String::new();
    petio_token: Secret = "petioToken", String::new();
    petio_fallback_user: Text = "petioFallbackUser", String::new();
    petio_fallback_password: Secret = "petioFallbackPassword", String::new();
    sso_plex: Flag = "ssoPlex", false;
    sso_ombi: Flag = "ssoOmbi", false;
    sso_tautulli: Flag = "ssoTautulli", false;
    sso_tautulli_auth: TextNumber<u32> = "ssoTautulliAuth", 4;
    sso_jellyfin: Flag = "ssoJellyfin", false;
    sso_overseerr: Flag = "ssoOverseerr", false;
    sso_petio: Flag = "ssoPetio", false;
    sso_komga: Flag = "ssoKomga", false;
    sso_komga_auth: TextNumber<u32> = "ssoKomgaAuth", 4;
    komga_url: Text = "komgaURL", String::new();
    komga_fallback_user: Text = "komgaFallbackUser", String::new();
    komga_fallback_password: Secret = "komgaFallbackPassword", String::new();
    komga_sso_master_password: Secret = "komgaSSOMasterPassword", String::new();
    sonarr_url: Text = "sonarrURL", String::new();
    sonarr_unmonitored: Flag = "sonarrUnmonitored", false;
    sonarr_icon: Flag = "sonarrIcon", true;
    sonarr_calendar_link: Text = "sonarrCalendarLink", String::new();
    sonarr_frame_target: Text = "sonarrFrameTarget", String::new();
    sonarr_token: Secret = "sonarrToken", String::new();
    sonarr_socks_enabled: Flag = "sonarrSocksEnabled", false;
    sonarr_socks_auth: TextNumber<u32> = "sonarrSocksAuth", 999;
    sonarr_disable_cert_check: Flag = "sonarrDisableCertCheck", false;
    sonarr_use_custom_certificate: Flag = "sonarrUseCustomCertificate", false;
    lidarr_url: Text = "lidarrURL", String::new();
    lidarr_token: Secret = "lidarrToken", String::new();
    lidarr_socks_enabled: Flag = "lidarrSocksEnabled", false;
    lidarr_socks_auth: TextNumber<u32> = "lidarrSocksAuth", 999;
    lidarr_disable_cert_check: Flag = "lidarrDisableCertCheck", false;
    lidarr_use_custom_certificate: Flag = "lidarrUseCustomCertificate", false;
    lidarr_icon: Flag = "lidarrIcon", true;
    lidarr_calendar_link: Text = "lidarrCalendarLink", String::new();
    lidarr_frame_target: Text = "lidarrFrameTarget", String::new();
    radarr_url: Text = "radarrURL", String::new();
    radarr_unmonitored: Flag = "radarrUnmonitored", false;
    radarr_icon: Flag = "radarrIcon", true;
    radarr_calendar_link: Text = "radarrCalendarLink", String::new();
    radarr_frame_target: Text = "radarrFrameTarget", String::new();
    radarr_physical_release: Flag = "radarrPhysicalRelease", true;
    radarr_digital_release: Flag = "radarrDigitalRelease", false;
    radarr_cinema_release: Flag = "radarrCinemaRelease", false;
    radarr_token: Secret = "radarrToken", String::new();
    radarr_socks_enabled: Flag = "radarrSocksEnabled", false;
    radarr_socks_auth: TextNumber<u32> = "radarrSocksAuth", 999;
    radarr_disable_cert_check: Flag = "radarrDisableCertCheck", false;
    radarr_use_custom_certificate: Flag = "radarrUseCustomCertificate", false;
    couchpotato_url: Text = "couchpotatoURL", String::new();
    couchpotato_token: Secret = "couchpotatoToken", String::new();
    couchpotato_disable_cert_check: Flag = "couchpotatoDisableCertCheck", false;
    couchpotato_use_custom_certificate: Flag = "couchpotatoUseCustomCertificate", false;
    sickrage_url: Text = "sickrageURL", String::new();
    sickrage_token: Secret = "sickrageToken", String::new();
    sickrage_disable_cert_check: Flag = "sickrageDisableCertCheck", false;
    sickrage_use_custom_certificate: Flag = "sickrageUseCustomCertificate", false;
    jdownloader_url: Text = "jdownloaderURL", String::new();
    jdownloader_username: Text = "jdownloaderUsername", String::new();
    jdownloader_password: Secret = "jdownloaderPassword", String::new();
    jdownloader_combine: Flag = "jdownloaderCombine", false;
    jdownloader_refresh: TextNumber<u64> = "jdownloaderRefresh", 60_000;
    jdownloader_use_custom_certificate: Flag = "jdownloaderUseCustomCertificate", false;
    jdownloader_disable_cert_check: Flag = "jdownloaderDisableCertCheck", false;
    sabnzbd_url: Text = "sabnzbdURL", String::new();
    sabnzbd_token: Secret = "sabnzbdToken", String::new();
    sabnzbd_socks_enabled: Flag = "sabnzbdSocksEnabled", false;
    sabnzbd_socks_auth: TextNumber<u32> = "sabnzbdSocksAuth", 999;
    sabnzbd_combine: Flag = "sabnzbdCombine", false;
    sabnzbd_refresh: TextNumber<u64> = "sabnzbdRefresh", 60_000;
    sabnzbd_disable_cert_check: Flag = "sabnzbdDisableCertCheck", false;
    sabnzbd_use_custom_certificate: Flag = "sabnzbdUseCustomCertificate", false;
    nzbget_url: Text = "nzbgetURL", String::new();
    nzbget_username: Text = "nzbgetUsername", String::new();
    nzbget_password: Secret = "nzbgetPassword", String::new();
    nzbget_socks_enabled: Flag = "nzbgetSocksEnabled", false;
    nzbget_socks_auth: TextNumber<u32> = "nzbgetSocksAuth", 999;
    nzbget_combine: Flag = "nzbgetCombine", false;
    nzbget_refresh: TextNumber<u64> = "nzbgetRefresh", 60_000;
    nzbget_disable_cert_check: Flag = "nzbgetDisableCertCheck", false;
    nzbget_use_custom_certificate: Flag = "nzbgetUseCustomCertificate", true;
    transmission_url: Text = "transmissionURL", String::new();
    transmission_username: Text = "transmissionUsername", String::new();
    transmission_password: Secret = "transmissionPassword", String::new();
    transmission_hide_seeding: Flag = "transmissionHideSeeding", false;
    transmission_hide_completed: Flag = "transmissionHideCompleted", false;
    transmission_combine: Flag = "transmissionCombine", false;
    transmission_refresh: TextNumber<u64> = "transmissionRefresh", 60_000;
    transmission_use_custom_certificate: Flag = "transmissionUseCustomCertificate", false;
    transmission_disable_cert_check: Flag = "transmissionDisableCertCheck", true;
    deluge_url: Text = "delugeURL", String::new();
    deluge_password: Secret = "delugePassword", String::new();
    deluge_hide_seeding: Flag = "delugeHideSeeding", false;
    deluge_hide_completed: Flag = "delugeHideCompleted", false;
    deluge_hide_status: Flag = "delugeHideStatus", true;
    deluge_combine: Flag = "delugeCombine", false;
    deluge_refresh: TextNumber<u64> = "delugeRefresh", 60_000;
    deluge_disable_cert_check: Flag = "delugeDisableCertCheck", false;
    deluge_use_custom_certificate: Flag = "delugeUseCustomCertificate", false;
    qbittorrent_url: Text = "qBittorrentURL", String::new();
    qbittorrent_username: Text = "qBittorrentUsername", String::new();
    qbittorrent_password: Secret = "qBittorrentPassword", String::new();
    qbittorrent_hide_seeding: Flag = "qBittorrentHideSeeding", false;
    qbittorrent_hide_completed: Flag = "qBittorrentHideCompleted", false;
    qbittorrent_sort_order: Text = "qBittorrentSortOrder", "eta".to_owned();
    qbittorrent_reverse_sorting: Flag = "qBittorrentReverseSorting", false;
    qbittorrent_combine: Flag = "qBittorrentCombine", false;
    qbittorrent_api_version: TextNumber<u32> = "qBittorrentApiVersion", 1;
    qbittorrent_disable_cert_check: Flag = "qBittorrentDisableCertCheck", false;
    qbittorrent_use_custom_certificate: Flag = "qBittorrentUseCustomCertificate", false;
    qbittorrent_refresh: TextNumber<u64> = "qBittorrentRefresh", 60_000;
    qbittorrent_socks_enabled: Flag = "qBittorrentSocksEnabled", false;
    qbittorrent_socks_auth: TextNumber<u32> = "qBittorrentSocksAuth", 999;
    rtorrent_url: Text = "rTorrentURL", String::new();
    utorrent_combine: Flag = "uTorrentCombine", false;
    utorrent_cookie: Secret = "uTorrentCookie", String::new();
    utorrent_disable_cert_check: Flag = "uTorrentDisableCertCheck", false;
    utorrent_hide_completed: Flag = "uTorrentHideCompleted", false;
    utorrent_hide_seeding: Flag = "uTorrentHideSeeding", false;
    utorrent_password: Secret = "uTorrentPassword", String::new();
    utorrent_refresh: TextNumber<u64> = "uTorrentRefresh", 60_000;
    utorrent_token: Secret = "uTorrentToken", String::new();
    utorrent_url: Text = "uTorrentURL", String::new();
    utorrent_use_custom_certificate: Flag = "uTorrentUseCustomCertificate", false;
    utorrent_username: Text = "uTorrentUsername", String::new();
    rtorrent_url_override: Text = "rTorrentURLOverride", String::new();
    rtorrent_username: Text = "rTorrentUsername", String::new();
    rtorrent_password: Secret = "rTorrentPassword", String::new();
    rtorrent_hide_seeding: Flag = "rTorrentHideSeeding", false;
    rtorrent_hide_completed: Flag = "rTorrentHideCompleted", false;
    rtorrent_sort_order: Text = "rTorrentSortOrder", "datea".to_owned();
    rtorrent_reverse_sorting: Flag = "rTorrentReverseSorting", false;
    rtorrent_combine: Flag = "rTorrentCombine", false;
    rtorrent_ignore_label: Text = "rTorrentIgnoreLabel", String::new();
    rtorrent_disable_cert_check: Flag = "rTorrentDisableCertCheck", false;
    rtorrent_use_custom_certificate: Flag = "rTorrentUseCustomCertificate", false;
    rtorrent_limit: TextNumber<u32> = "rTorrentLimit", 200;
    rtorrent_refresh: TextNumber<u64> = "rTorrentRefresh", 60_000;
    homepage_jackett_enabled: Flag = "homepageJackettEnabled", false;
    homepage_jackett_auth: TextNumber<u32> = "homepageJackettAuth", 1;
    jackett_url: Text = "jackettURL", String::new();
    jackett_token: Secret = "jackettToken", String::new();
    jackett_use_custom_certificate: Flag = "jackettUseCustomCertificate", false;
    jackett_disable_cert_check: Flag = "jackettDisableCertCheck", false;
    homepage_jackett_backhole_download: Flag = "homepageJackettBackholeDownload", false;
    homepage_prowlarr_backhole_download: Flag = "homepageProwlarrBackholeDownload", false;
    homepage_prowlarr_enabled: Flag = "homepageProwlarrEnabled", false;
    homepage_prowlarr_auth: TextNumber<u32> = "homepageProwlarrAuth", 1;
    prowlarr_url: Text = "prowlarrURL", String::new();
    prowlarr_token: Secret = "prowlarrToken", String::new();
    prowlarr_use_custom_certificate: Flag = "prowlarrUseCustomCertificate", false;
    prowlarr_disable_cert_check: Flag = "prowlarrDisableCertCheck", false;
    homepage_calendar_enabled: Flag = "homepageCalendarEnabled", false;
    homepage_calendar_auth: TextNumber<u32> = "homepageCalendarAuth", 4;
    calendar_ical: Text = "calendariCal", String::new();
    homepage_custom_html01_enabled: Flag = "homepageCustomHTML01Enabled", false;
    homepage_custom_html01_auth: TextNumber<u32> = "homepageCustomHTML01Auth", 1;
    custom_html01: Text = "customHTML01", String::new();
    homepage_custom_html01_trusted: Flag = "homepageCustomHTML01Trusted", false;
    homepage_custom_html02_enabled: Flag = "homepageCustomHTML02Enabled", false;
    homepage_custom_html02_auth: TextNumber<u32> = "homepageCustomHTML02Auth", 1;
    custom_html02: Text = "customHTML02", String::new();
    homepage_custom_html02_trusted: Flag = "homepageCustomHTML02Trusted", false;
    homepage_custom_html03_enabled: Flag = "homepageCustomHTML03Enabled", false;
    homepage_custom_html03_auth: TextNumber<u32> = "homepageCustomHTML03Auth", 1;
    custom_html03: Text = "customHTML03", String::new();
    homepage_custom_html03_trusted: Flag = "homepageCustomHTML03Trusted", false;
    homepage_custom_html04_enabled: Flag = "homepageCustomHTML04Enabled", false;
    homepage_custom_html04_auth: TextNumber<u32> = "homepageCustomHTML04Auth", 1;
    custom_html04: Text = "customHTML04", String::new();
    homepage_custom_html04_trusted: Flag = "homepageCustomHTML04Trusted", false;
    homepage_custom_html05_enabled: Flag = "homepageCustomHTML05Enabled", false;
    homepage_custom_html05_auth: TextNumber<u32> = "homepageCustomHTML05Auth", 1;
    custom_html05: Text = "customHTML05", String::new();
    homepage_custom_html05_trusted: Flag = "homepageCustomHTML05Trusted", false;
    homepage_custom_html06_enabled: Flag = "homepageCustomHTML06Enabled", false;
    homepage_custom_html06_auth: TextNumber<u32> = "homepageCustomHTML06Auth", 1;
    custom_html06: Text = "customHTML06", String::new();
    homepage_custom_html06_trusted: Flag = "homepageCustomHTML06Trusted", false;
    homepage_custom_html07_enabled: Flag = "homepageCustomHTML07Enabled", false;
    homepage_custom_html07_auth: TextNumber<u32> = "homepageCustomHTML07Auth", 1;
    custom_html07: Text = "customHTML07", String::new();
    homepage_custom_html07_trusted: Flag = "homepageCustomHTML07Trusted", false;
    homepage_custom_html08_enabled: Flag = "homepageCustomHTML08Enabled", false;
    homepage_custom_html08_auth: TextNumber<u32> = "homepageCustomHTML08Auth", 1;
    custom_html08: Text = "customHTML08", String::new();
    homepage_custom_html08_trusted: Flag = "homepageCustomHTML08Trusted", false;
    homepage_deluge_enabled: Flag = "homepageDelugeEnabled", false;
    homepage_deluge_auth: TextNumber<u32> = "homepageDelugeAuth", 1;
    homepage_jdownloader_enabled: Flag = "homepageJdownloaderEnabled", false;
    homepage_jdownloader_auth: TextNumber<u32> = "homepageJdownloaderAuth", 1;
    homepage_sabnzbd_enabled: Flag = "homepageSabnzbdEnabled", false;
    homepage_sabnzbd_auth: TextNumber<u32> = "homepageSabnzbdAuth", 1;
    homepage_sonarr_enabled: Flag = "homepageSonarrEnabled", false;
    homepage_sonarr_auth: TextNumber<u32> = "homepageSonarrAuth", 1;
    homepage_sonarr_queue_enabled: Flag = "homepageSonarrQueueEnabled", false;
    homepage_sonarr_queue_auth: TextNumber<u32> = "homepageSonarrQueueAuth", 1;
    homepage_sonarr_queue_combine: Flag = "homepageSonarrQueueCombine", false;
    homepage_sonarr_queue_refresh: TextNumber<u64> = "homepageSonarrQueueRefresh", 60_000;
    homepage_lidarr_enabled: Flag = "homepageLidarrEnabled", false;
    homepage_lidarr_auth: TextNumber<u32> = "homepageLidarrAuth", 1;
    homepage_couchpotato_enabled: Flag = "homepageCouchpotatoEnabled", false;
    homepage_couchpotat_oauth: TextNumber<u32> = "homepageCouchpotatoAuth", 1;
    homepage_sickrage_enabled: Flag = "homepageSickrageEnabled", false;
    homepage_sickrage_auth: TextNumber<u32> = "homepageSickrageAuth", 1;
    homepage_radarr_enabled: Flag = "homepageRadarrEnabled", false;
    homepage_radarr_auth: TextNumber<u32> = "homepageRadarrAuth", 1;
    homepage_radarr_queue_enabled: Flag = "homepageRadarrQueueEnabled", false;
    homepage_radarr_queue_auth: TextNumber<u32> = "homepageRadarrQueueAuth", 1;
    homepage_radarr_queue_combine: Flag = "homepageRadarrQueueCombine", false;
    homepage_radarr_queue_refresh: TextNumber<u64> = "homepageRadarrQueueRefresh", 60_000;
    homepage_transmission_enabled: Flag = "homepageTransmissionEnabled", false;
    homepage_transmission_auth: TextNumber<u32> = "homepageTransmissionAuth", 1;
    homepage_qbittorrent_enabled: Flag = "homepageqBittorrentEnabled", false;
    homepage_qbittorrent_auth: TextNumber<u32> = "homepageqBittorrentAuth", 1;
    homepage_rtorrent_enabled: Flag = "homepagerTorrentEnabled", false;
    homepage_rtorrent_auth: TextNumber<u32> = "homepagerTorrentAuth", 1;
    homepage_utorrent_auth: TextNumber<u32> = "homepageuTorrentAuth", 1;
    homepage_utorrent_enabled: Flag = "homepageuTorrentEnabled", false;
    homepage_nzbget_enabled: Flag = "homepageNzbgetEnabled", false;
    homepage_nzbget_auth: TextNumber<u32> = "homepageNzbgetAuth", 1;
    homepage_plex_enabled: Flag = "homepagePlexEnabled", false;
    homepage_plex_auth: TextNumber<u32> = "homepagePlexAuth", 1;
    homepage_emby_enabled: Flag = "homepageEmbyEnabled", false;
    homepage_emby_auth: TextNumber<u32> = "homepageEmbyAuth", 1;
    homepage_jellyfin_enabled: Flag = "homepageJellyfinEnabled", false;
    homepage_jellyfin_auth: TextNumber<u32> = "homepageJellyfinAuth", 1;
    homepage_ombi_enabled: Flag = "homepageOmbiEnabled", false;
    homepage_ombi_auth: TextNumber<u32> = "homepageOmbiAuth", 1;
    homepage_ombi_request_auth: TextNumber<u32> = "homepageOmbiRequestAuth", 1;
    homepage_overseerr_enabled: Flag = "homepageOverseerrEnabled", false;
    homepage_overseerr_auth: TextNumber<u32> = "homepageOverseerrAuth", 1;
    homepage_overseerr_request_auth: TextNumber<u32> = "homepageOverseerrRequestAuth", 1;
    homepage_jellyfin_instead: Flag = "homepageJellyfinInstead", false;
    ombi_limit_user: Flag = "ombiLimitUser", false;
    ombi_refresh: TextNumber<u64> = "ombiRefresh", 600_000;
    ombi_tv_default: Text = "ombiTvDefault", "all".to_owned();
    overseerr_limit_user: Flag = "overseerrLimitUser", false;
    overseerr_refresh: TextNumber<u64> = "overseerrRefresh", 600_000;
    overseerr_tv_default: Text = "overseerrTvDefault", "all".to_owned();
    overseerr_prefer4_k: Flag = "overseerrPrefer4K", false;
    homepage_weather_and_air_enabled: Flag = "homepageWeatherAndAirEnabled", false;
    homepage_weather_and_air_auth: TextNumber<u32> = "homepageWeatherAndAirAuth", 1;
    homepage_weather_and_air_refresh: TextNumber<u64> = "homepageWeatherAndAirRefresh", 3_600_000;
    homepage_weather_and_air_latitude: Text = "homepageWeatherAndAirLatitude", String::new();
    homepage_weather_and_air_longitude: Text = "homepageWeatherAndAirLongitude", String::new();
    homepage_weather_and_air_units: Text = "homepageWeatherAndAirUnits", "imperial".to_owned();
    homepage_weather_and_air_pollen_enabled: Flag = "homepageWeatherAndAirPollenEnabled", false;
    homepage_weather_and_air_air_quality_enabled: Flag = "homepageWeatherAndAirAirQualityEnabled", false;
    homepage_weather_and_air_weather_enabled: Flag = "homepageWeatherAndAirWeatherEnabled", false;
    homepage_weather_and_air_weather_header: Text = "homepageWeatherAndAirWeatherHeader", String::new();
    homepage_weather_and_air_weather_header_toggle: Flag = "homepageWeatherAndAirWeatherHeaderToggle", false;
    homepage_health_checks_enabled: Flag = "homepageHealthChecksEnabled", false;
    health_checks_token: Secret = "healthChecksToken", String::new();
    health_checks_tags: Text = "healthChecksTags", String::new();
    homepage_health_checks_auth: TextNumber<u32> = "homepageHealthChecksAuth", 1;
    homepage_health_checks_show_desc: Flag = "homepageHealthChecksShowDesc", false;
    homepage_health_checks_show_tags: Flag = "homepageHealthChecksShowTags", false;
    health_checks_disable_cert_check: Flag = "healthChecksDisableCertCheck", false;
    health_checks_use_custom_certificate: Flag = "healthChecksUseCustomCertificate", false;
    homepage_donate_enabled: Flag = "homepageDonateEnabled", false;
    homepage_donate_auth: TextNumber<u32> = "homepageDonateAuth", 999;
    homepage_donate_public_token: Text = "homepageDonatePublicToken", String::new();
    homepage_donate_secret_token: Secret = "homepageDonateSecretToken", String::new();
    homepage_donate_product_id: Text = "homepageDonateProductID", String::new();
    homepage_donate_customize_heading: Text = "homepageDonateCustomizeHeading", "Donate Please".to_owned();
    homepage_donate_customize_description: Text = "homepageDonateCustomizeDescription", "Hi!  Help me out with a donation...".to_owned();
    homepage_donate_minimum: TextNumber<u32> = "homepageDonateMinimum", 100;
    homepage_donate_show_user_history: Flag = "homepageDonateShowUserHistory", false;
    homepage_ordercustomhtml01: TextNumber<u32> = "homepageOrdercustomhtml01", 1;
    homepage_ordercustomhtml02: TextNumber<u32> = "homepageOrdercustomhtml02", 2;
    homepage_ordertransmission: TextNumber<u32> = "homepageOrdertransmission", 3;
    homepage_order_qbittorrent: TextNumber<u32> = "homepageOrderqBittorrent", 4;
    homepage_orderdeluge: TextNumber<u32> = "homepageOrderdeluge", 5;
    homepage_ordernzbget: TextNumber<u32> = "homepageOrdernzbget", 6;
    homepage_ordersabnzbd: TextNumber<u32> = "homepageOrdersabnzbd", 7;
    homepage_orderplexnowplaying: TextNumber<u32> = "homepageOrderplexnowplaying", 8;
    homepage_orderplexrecent: TextNumber<u32> = "homepageOrderplexrecent", 9;
    homepage_orderplexplaylist: TextNumber<u32> = "homepageOrderplexplaylist", 10;
    homepage_orderembynowplaying: TextNumber<u32> = "homepageOrderembynowplaying", 11;
    homepage_orderembyrecent: TextNumber<u32> = "homepageOrderembyrecent", 12;
    homepage_orderombi: TextNumber<u32> = "homepageOrderombi", 13;
    homepage_ordercalendar: TextNumber<u32> = "homepageOrdercalendar", 14;
    homepage_order_rtorrent: TextNumber<u32> = "homepageOrderrTorrent", 15;
    homepage_orderdownloader: TextNumber<u32> = "homepageOrderdownloader", 16;
    homepage_orderhealthchecks: TextNumber<u32> = "homepageOrderhealthchecks", 17;
    homepage_orderjdownloader: TextNumber<u32> = "homepageOrderjdownloader", 18;
    homepage_orderunifi: TextNumber<u32> = "homepageOrderunifi", 19;
    homepage_order_pihole: TextNumber<u32> = "homepageOrderPihole", 20;
    homepage_ordertautulli: TextNumber<u32> = "homepageOrdertautulli", 21;
    homepage_order_monitorr: TextNumber<u32> = "homepageOrderMonitorr", 22;
    homepage_order_weather_and_air: TextNumber<u32> = "homepageOrderWeatherAndAir", 23;
    homepage_order_speedtest: TextNumber<u32> = "homepageOrderSpeedtest", 24;
    homepage_order_netdata: TextNumber<u32> = "homepageOrderNetdata", 25;
    homepage_order_sonarr_queue: TextNumber<u32> = "homepageOrderSonarrQueue", 26;
    homepage_order_radarr_queue: TextNumber<u32> = "homepageOrderRadarrQueue", 27;
    homepage_order_octoprint: TextNumber<u32> = "homepageOrderOctoprint", 28;
    homepage_orderjellyfinnowplaying: TextNumber<u32> = "homepageOrderjellyfinnowplaying", 29;
    homepage_orderjellyfinrecent: TextNumber<u32> = "homepageOrderjellyfinrecent", 30;
    homepage_order_jackett: TextNumber<u32> = "homepageOrderJackett", 31;
    homepage_ordercustomhtml03: TextNumber<u32> = "homepageOrdercustomhtml03", 32;
    homepage_ordercustomhtml04: TextNumber<u32> = "homepageOrdercustomhtml04", 33;
    homepage_ordercustomhtml05: TextNumber<u32> = "homepageOrdercustomhtml05", 34;
    homepage_ordercustomhtml06: TextNumber<u32> = "homepageOrdercustomhtml06", 35;
    homepage_ordercustomhtml07: TextNumber<u32> = "homepageOrdercustomhtml07", 36;
    homepage_ordercustomhtml08: TextNumber<u32> = "homepageOrdercustomhtml08", 37;
    homepage_order_utorrent: TextNumber<u32> = "homepageOrderuTorrent", 38;
    homepage_orderoverseerr: TextNumber<u32> = "homepageOrderoverseerr", 39;
    homepage_order_bookmarks: TextNumber<u32> = "homepageOrderBookmarks", 40;
    homepage_order_donate: TextNumber<u32> = "homepageOrderDonate", 41;
    homepage_order_adguard: TextNumber<u32> = "homepageOrderAdguard", 42;
    homepage_order_prowlarr: TextNumber<u32> = "homepageOrderProwlarr", 43;
    homepage_order_uptime_kuma: TextNumber<u32> = "homepageOrderUptimeKuma", 44;
    homepage_order_prom_page: TextNumber<u32> = "homepageOrderPromPage", 45;
    homepage_order_emby_live_tv_tracker: TextNumber<u32> = "homepageOrderEmbyLiveTVTracker", 46;
    homepage_order_user_watch_stats: TextNumber<u32> = "homepageOrderUserWatchStats", 47;
    homepage_order_jelly_stat: TextNumber<u32> = "homepageOrderJellyStat", 48;
    homepage_order_netronome: TextNumber<u32> = "homepageOrderNetronome", 49;
    homepage_emby_live_tv_tracker_enabled: Flag = "homepageEmbyLiveTVTrackerEnabled", false;
    homepage_emby_live_tv_tracker_auth: TextNumber<u32> = "homepageEmbyLiveTVTrackerAuth", 1;
    homepage_emby_live_tv_tracker_refresh: TextNumber<u64> = "homepageEmbyLiveTVTrackerRefresh", 5;
    homepage_emby_live_tv_tracker_days_shown: TextNumber<u32> = "homepageEmbyLiveTVTrackerDaysShown", 7;
    homepage_emby_live_tv_tracker_compact_view: Flag = "homepageEmbyLiveTVTrackerCompactView", false;
    homepage_emby_live_tv_tracker_show_duration: Flag = "homepageEmbyLiveTVTrackerShowDuration", true;
    homepage_emby_live_tv_tracker_show_series_info: Flag = "homepageEmbyLiveTVTrackerShowSeriesInfo", true;
    homepage_emby_live_tv_tracker_show_user_info: Flag = "homepageEmbyLiveTVTrackerShowUserInfo", false;
    homepage_emby_live_tv_tracker_max_items: TextNumber<u32> = "homepageEmbyLiveTVTrackerMaxItems", 10;
    homepage_emby_live_tv_tracker_show_completed: Flag = "homepageEmbyLiveTVTrackerShowCompleted", true;
    homepage_emby_live_tv_tracker_max_completed_items: TextNumber<u32> = "homepageEmbyLiveTVTrackerMaxCompletedItems", 5;
    homepage_user_watch_stats_enabled: Flag = "homepageUserWatchStatsEnabled", false;
    homepage_user_watch_stats_auth: TextNumber<u32> = "homepageUserWatchStatsAuth", 1;
    homepage_user_watch_stats_refresh: TextNumber<u64> = "homepageUserWatchStatsRefresh", 30;
    homepage_user_watch_stats_service: Text = "homepageUserWatchStatsService", "plex".to_owned();
    homepage_user_watch_stats_url: Text = "homepageUserWatchStatsURL", String::new();
    homepage_user_watch_stats_token: Secret = "homepageUserWatchStatsToken", String::new();
    homepage_user_watch_stats_disable_cert_check: Flag = "homepageUserWatchStatsDisableCertCheck", false;
    homepage_user_watch_stats_use_custom_certificate: Flag = "homepageUserWatchStatsUseCustomCertificate", false;
    homepage_user_watch_stats_days: TextNumber<u32> = "homepageUserWatchStatsDays", 30;
    homepage_user_watch_stats_compact_view: Flag = "homepageUserWatchStatsCompactView", false;
    homepage_user_watch_stats_show_top_users: Flag = "homepageUserWatchStatsShowTopUsers", true;
    homepage_user_watch_stats_show_most_watched: Flag = "homepageUserWatchStatsShowMostWatched", true;
    homepage_user_watch_stats_show_recent_activity: Flag = "homepageUserWatchStatsShowRecentActivity", true;
    homepage_user_watch_stats_max_items: TextNumber<u32> = "homepageUserWatchStatsMaxItems", 10;
    homepage_user_watch_stats_header: Text = "homepageUserWatchStatsHeader", "User Watch Statistics".to_owned();
    homepage_user_watch_stats_header_toggle: Flag = "homepageUserWatchStatsHeaderToggle", true;
    homepage_jelly_stat_enabled: Flag = "homepageJellyStatEnabled", false;
    homepage_jelly_stat_auth: TextNumber<u32> = "homepageJellyStatAuth", 1;
    homepage_jelly_stat_display_mode: Text = "homepageJellyStatDisplayMode", "native".to_owned();
    jelly_stat_url: Text = "jellyStatURL", String::new();
    jelly_stat_internal_url: Text = "jellyStatInternalURL", String::new();
    jelly_stat_apikey: Secret = "jellyStatApikey", String::new();
    jelly_stat_disable_cert_check: Flag = "jellyStatDisableCertCheck", false;
    jelly_stat_use_custom_certificate: Flag = "jellyStatUseCustomCertificate", false;
    homepage_jelly_stat_refresh: TextNumber<u64> = "homepageJellyStatRefresh", 5;
    homepage_jelly_stat_days: TextNumber<u32> = "homepageJellyStatDays", 30;
    homepage_jelly_stat_show_libraries: Flag = "homepageJellyStatShowLibraries", true;
    homepage_jelly_stat_show_users: Flag = "homepageJellyStatShowUsers", true;
    homepage_jelly_stat_show_most_watched: Flag = "homepageJellyStatShowMostWatched", true;
    homepage_jelly_stat_show_recent_activity: Flag = "homepageJellyStatShowRecentActivity", true;
    homepage_jelly_stat_max_items: TextNumber<u32> = "homepageJellyStatMaxItems", 10;
    homepage_jelly_stat_show_most_watched_movies: Flag = "homepageJellyStatShowMostWatchedMovies", true;
    homepage_jelly_stat_show_most_watched_shows: Flag = "homepageJellyStatShowMostWatchedShows", true;
    homepage_jelly_stat_show_most_listened_music: Flag = "homepageJellyStatShowMostListenedMusic", true;
    homepage_jelly_stat_most_watched_count: TextNumber<u32> = "homepageJellyStatMostWatchedCount", 10;
    homepage_jelly_stat_iframe_height: TextNumber<u32> = "homepageJellyStatIframeHeight", 800;
    homepage_jelly_stat_iframe_scrolling: Flag = "homepageJellyStatIframeScrolling", true;
    homepage_show_stream_names: Flag = "homepageShowStreamNames", false;
    homepage_show_stream_names_auth: TextNumber<u32> = "homepageShowStreamNamesAuth", 1;
    homepage_show_stream_names_without_ip: Flag = "homepageShowStreamNamesWithoutIp", false;
    homepage_show_stream_names_without_ip_auth: TextNumber<u32> = "homepageShowStreamNamesWithoutIpAuth", 1;
    homepage_use_custom_stream_names: Flag = "homepageUseCustomStreamNames", false;
    homepage_custom_stream_names: Text = "homepageCustomStreamNames", String::new();
    homepage_stream_refresh: TextNumber<u64> = "homepageStreamRefresh", 60_000;
    homepage_recent_refresh: TextNumber<u64> = "homepageRecentRefresh", 60_000;
    homepage_download_refresh: TextNumber<u64> = "homepageDownloadRefresh", 60_000;
    homepage_health_checks_refresh: TextNumber<u64> = "homepageHealthChecksRefresh", 60_000;
    homepage_plex_streams: Flag = "homepagePlexStreams", false;
    homepage_plex_streams_auth: TextNumber<u32> = "homepagePlexStreamsAuth", 1;
    homepage_plex_streams_exclude: Text = "homepagePlexStreamsExclude", String::new();
    homepage_plex_recent: Flag = "homepagePlexRecent", false;
    homepage_plex_recent_exclude: Text = "homepagePlexRecentExclude", String::new();
    homepage_recent_limit: TextNumber<u32> = "homepageRecentLimit", 50;
    homepage_plex_recent_auth: TextNumber<u32> = "homepagePlexRecentAuth", 1;
    homepage_plex_playlist: Flag = "homepagePlexPlaylist", false;
    homepage_plex_playlist_auth: TextNumber<u32> = "homepagePlexPlaylistAuth", 1;
    homepage_plex_search_exclude: Text = "homepagePlexSearchExclude", String::new();
    homepage_emby_streams: Flag = "homepageEmbyStreams", false;
    homepage_emby_streams_auth: TextNumber<u32> = "homepageEmbyStreamsAuth", 1;
    homepage_emby_recent: Flag = "homepageEmbyRecent", false;
    homepage_emby_recent_auth: TextNumber<u32> = "homepageEmbyRecentAuth", 1;
    homepage_emby_link: Text = "homepageEmbyLink", " http://app.emby.media/#!/item/item.html?id={id}&serverId={serverId}".to_owned();
    homepage_jellyfin_streams: Flag = "homepageJellyfinStreams", false;
    homepage_jelly_streams_auth: TextNumber<u32> = "homepageJellyStreamsAuth", 1;
    homepage_jellyfin_recent: Flag = "homepageJellyfinRecent", false;
    homepage_jellyfin_recent_auth: TextNumber<u32> = "homepageJellyfinRecentAuth", 1;
    homepage_jellyfin_link: Text = "homepageJellyfinLink", "http://hostname:port/jellyfin/web/index.html#!/details?id={id}&serverId={serverId}".to_owned();
    homepage_bookmarks_auth: TextNumber<u32> = "homepageBookmarksAuth", 1;
    homepage_bookmarks_enabled: Flag = "homepageBookmarksEnabled", false;
    homepage_bookmarks_refresh: TextNumber<u64> = "homepageBookmarksRefresh", 3_600_000;
    calendar_default: Text = "calendarDefault", "month".to_owned();
    calendar_first_day: TextNumber<u32> = "calendarFirstDay", 1;
    calendar_start: TextNumber<u32> = "calendarStart", 14;
    calendar_end: TextNumber<u32> = "calendarEnd", 14;
    calendar_refresh: TextNumber<u64> = "calendarRefresh", 60_000;
    calendar_time_format: Text = "calendarTimeFormat", "h(:mm)t".to_owned();
    calendar_limit: TextNumber<u32> = "calendarLimit", 1000;
    calendar_locale: Text = "calendarLocale", "en".to_owned();
    media_search: Flag = "mediaSearch", false;
    media_search_type: Text = "mediaSearchType", String::new();
    media_search_auth: TextNumber<u32> = "mediaSearchAuth", 1;
    fav_icon: Text = "favIcon", String::new();
    ping_auth: TextNumber<u32> = "pingAuth", 1;
    ping_auth_message: TextNumber<u32> = "pingAuthMessage", 1;
    admin_ping_refresh: TextNumber<u64> = "adminPingRefresh", 60_000;
    other_ping_refresh: TextNumber<u64> = "otherPingRefresh", 600_000;
    ping_offline_sound: Text = "pingOfflineSound", "plugins/sounds/default/open-ended.mp3".to_owned();
    ping_online_sound: Text = "pingOnlineSound", "plugins/sounds/default/awareness.mp3".to_owned();
    ping_ms: Flag = "pingMs", false;
    ping_auth_ms: TextNumber<u32> = "pingAuthMs", 1;
    notification_backbone: Text = "notificationBackbone", "izi".to_owned();
    notification_position: Text = "notificationPosition", "br".to_owned();
    lockout_system: Flag = "lockoutSystem", false;
    lockout_timeout: TextNumber<u64> = "lockoutTimeout", 60;
    lockout_max_auth: TextNumber<u32> = "lockoutMaxAuth", 0;
    lockout_min_auth: TextNumber<u32> = "lockoutMinAuth", 1;
    theme_installed: Text = "themeInstalled", String::new();
    theme_version: Text = "themeVersion", String::new();
    installed_plugins: List = "installedPlugins", Vec::new();
    /// The installed themes of the marketplace: an object from the name of each theme to
    /// its details, as PHP stores it, or the empty text while none is installed.
    installed_themes: Anything = "installedThemes", Value::String(String::new());
    auth_debug: Flag = "authDebug", false;
    minimal_login_screen: Flag = "minimalLoginScreen", false;
    cache_image_size: TextNumber<u32> = "cacheImageSize", 2;
    plex_oauth: Flag = "plexoAuth", false;
    status_sounds: Flag = "statusSounds", false;
    remember_me: Flag = "rememberMe", true;
    plex_strict_friends: Flag = "plexStrictFriends", true;
    ignore_tfa_if_plex_o_auth: Flag = "ignoreTFAIfPlexOAuth", false;
    debug_area_auth: TextNumber<u32> = "debugAreaAuth", 1;
    commit: Text = "commit", "n/a".to_owned();
    ombi_limit: TextNumber<u32> = "ombiLimit", 50;
    overseerr_limit: TextNumber<u32> = "overseerrLimit", 50;
    local_ip_from: Text = "localIPFrom", String::new();
    local_ip_to: Text = "localIPTo", String::new();
    sandbox: Text = "sandbox", "allow-presentation,allow-forms,allow-same-origin,allow-pointer-lock,allow-scripts,allow-popups,allow-modals,allow-top-navigation,allow-downloads,allow-orientation-lock,allow-popups-to-escape-sandbox,allow-top-navigation-by-user-activation".to_owned();
    iframe_allow: Text = "iframeAllow", "fullscreen,autoplay,clipboard-read,clipboard-write,camera,microphone,speaker-selection,display-capture,web-share,encrypted-media,picture-in-picture".to_owned();
    debug_errors: Flag = "debugErrors", false;
    health_checks_url: Text = "healthChecksURL", "https://healthchecks.io/api/v1/checks/".to_owned();
    ga_tracking_id: Text = "gaTrackingID", String::new();
    ombi_default_filter_available: Flag = "ombiDefaultFilterAvailable", true;
    ombi_default_filter_unavailable: Flag = "ombiDefaultFilterUnavailable", true;
    ombi_default_filter_approved: Flag = "ombiDefaultFilterApproved", true;
    ombi_default_filter_unapproved: Flag = "ombiDefaultFilterUnapproved", true;
    ombi_default_filter_denied: Flag = "ombiDefaultFilterDenied", true;
    overseerr_default_filter_available: Flag = "overseerrDefaultFilterAvailable", true;
    overseerr_default_filter_unavailable: Flag = "overseerrDefaultFilterUnavailable", true;
    overseerr_default_filter_approved: Flag = "overseerrDefaultFilterApproved", true;
    overseerr_default_filter_unapproved: Flag = "overseerrDefaultFilterUnapproved", true;
    overseerr_default_filter_denied: Flag = "overseerrDefaultFilterDenied", true;
    self_signed_cert: Text = "selfSignedCert", String::new();
    homepage_plex_recently_added_method: Text = "homepagePlexRecentlyAddedMethod", "legacy".to_owned();
    auth_proxy_enabled: Flag = "authProxyEnabled", false;
    auth_proxy_header_name: Text = "authProxyHeaderName", String::new();
    auth_proxy_header_name_email: Text = "authProxyHeaderNameEmail", String::new();
    auth_proxy_whitelist: Text = "authProxyWhitelist", String::new();
    auth_proxy_override_logout: Flag = "authProxyOverrideLogout", false;
    auth_proxy_logout_url: Text = "authProxyLogoutURL", String::new();
    ignore_tfa_local: Flag = "ignoreTFALocal", false;
    unifi_url: Text = "unifiURL", String::new();
    unifi_username: Text = "unifiUsername", String::new();
    unifi_password: Secret = "unifiPassword", String::new();
    unifi_site_name: Text = "unifiSiteName", String::new();
    unifi_cookie: Secret = "unifiCookie", String::new();
    unifi_use_custom_certificate: Flag = "unifiUseCustomCertificate", false;
    unifi_disable_cert_check: Flag = "unifiDisableCertCheck", true;
    homepage_unifi_enabled: Flag = "homepageUnifiEnabled", false;
    homepage_unifi_auth: TextNumber<u32> = "homepageUnifiAuth", 1;
    homepage_unifi_refresh: TextNumber<u64> = "homepageUnifiRefresh", 600_000;
    youtube_api: Secret = "youtubeAPI", String::new();
    wan_domain: Text = "wanDomain", String::new();
    local_address: Text = "localAddress", String::new();
    enable_local_address_forward: Flag = "enableLocalAddressForward", false;
    performance_disable_icon_dropdown: Flag = "performanceDisableIconDropdown", false;
    performance_disable_image_dropdown: Flag = "performanceDisableImageDropdown", false;
    traefik_auth_enable: Flag = "traefikAuthEnable", false;
    traefik_domain_override: Text = "traefikDomainOverride", String::new();
    homepage_tautulli_enabled: Flag = "homepageTautulliEnabled", false;
    homepage_tautulli_auth: TextNumber<u32> = "homepageTautulliAuth", 1;
    homepage_tautulli_library_auth: TextNumber<u32> = "homepageTautulliLibraryAuth", 1;
    homepage_tautulli_library_stats_exclude: Text = "homepageTautulliLibraryStatsExclude", String::new();
    homepage_tautulli_views_auth: TextNumber<u32> = "homepageTautulliViewsAuth", 1;
    homepage_tautulli_misc_auth: TextNumber<u32> = "homepageTautulliMiscAuth", 1;
    homepage_tautulli_viewing_stats_exclude: Text = "homepageTautulliViewingStatsExclude", String::new();
    homepage_tautulli_refresh: TextNumber<u64> = "homepageTautulliRefresh", 60_000;
    tautulli_apikey: Secret = "tautulliApikey", String::new();
    tautulli_libraries: Flag = "tautulliLibraries", true;
    tautulli_top_movies: Flag = "tautulliTopMovies", true;
    tautulli_top_tv: Flag = "tautulliTopTV", true;
    tautulli_top_users: Flag = "tautulliTopUsers", true;
    tautulli_top_platforms: Flag = "tautulliTopPlatforms", true;
    tautulli_popular_movies: Flag = "tautulliPopularMovies", true;
    tautulli_popular_tv: Flag = "tautulliPopularTV", true;
    tautulli_header: Text = "tautulliHeader", "Tautulli".to_owned();
    tautulli_header_toggle: Flag = "tautulliHeaderToggle", true;
    tautulli_friendly_name: Flag = "tautulliFriendlyName", true;
    tautulli_socks_enabled: Flag = "tautulliSocksEnabled", false;
    tautulli_socks_auth: TextNumber<u32> = "tautulliSocksAuth", 999;
    tautulli_use_custom_certificate: Flag = "tautulliUseCustomCertificate", false;
    tautulli_disable_cert_check: Flag = "tautulliDisableCertCheck", false;
    homepage_pihole_enabled: Flag = "homepagePiholeEnabled", false;
    homepage_pihole_auth: TextNumber<u32> = "homepagePiholeAuth", 1;
    homepage_pihole_refresh: TextNumber<u64> = "homepagePiholeRefresh", 10_000;
    homepage_pihole_combine: Flag = "homepagePiholeCombine", false;
    pihole_header_toggle: Flag = "piholeHeaderToggle", true;
    pihole_url: Text = "piholeURL", String::new();
    pihole_token: Secret = "piholeToken", String::new();
    homepage_ad_guard_enabled: Flag = "homepageAdGuardEnabled", false;
    homepage_ad_guard_auth: TextNumber<u32> = "homepageAdGuardAuth", 1;
    homepage_ad_guard_refresh: TextNumber<u64> = "homepageAdGuardRefresh", 10_000;
    homepage_ad_guard_combine: Flag = "homepageAdGuardCombine", false;
    adguard_queries_toggle: Flag = "adguardQueriesToggle", true;
    adguard_queries_blocked_toggle: Flag = "adguardQueriesBlockedToggle", true;
    adguard_percent_toggle: Flag = "adguardPercentToggle", true;
    adguard_processing_toggle: Flag = "adguardProcessingToggle", true;
    adguard_domain_list_toggle: Flag = "adguardDomainListToggle", false;
    adguard_header_toggle: Flag = "adguardHeaderToggle", true;
    adguard_url: Text = "adguardURL", String::new();
    adguard_username: Text = "adGuardUsername", String::new();
    adguard_password: Secret = "adGuardPassword", String::new();
    homepage_monitorr_enabled: Flag = "homepageMonitorrEnabled", false;
    homepage_monitorr_auth: TextNumber<u32> = "homepageMonitorrAuth", 1;
    homepage_monitorr_refresh: TextNumber<u64> = "homepageMonitorrRefresh", 60_000;
    monitorr_url: Text = "monitorrURL", String::new();
    monitorr_header_toggle: Flag = "monitorrHeaderToggle", true;
    monitorr_header: Text = "monitorrHeader", "Monitorr".to_owned();
    monitorr_compact: Flag = "monitorrCompact", false;
    monitorr_disable_cert_check: Flag = "monitorrDisableCertCheck", false;
    monitorr_use_custom_certificate: Flag = "monitorrUseCustomCertificate", false;
    homepage_speedtest_enabled: Flag = "homepageSpeedtestEnabled", false;
    homepage_speedtest_auth: TextNumber<u32> = "homepageSpeedtestAuth", 1;
    homepage_speedtest_refresh: TextNumber<u64> = "homepageSpeedtestRefresh", 1_800_000;
    speedtest_url: Text = "speedtestURL", String::new();
    speedtest_header_toggle: Flag = "speedtestHeaderToggle", true;
    speedtest_header: Text = "speedtestHeader", "Speedtest".to_owned();
    speedtest_disable_cert_check: Flag = "speedtestDisableCertCheck", false;
    speedtest_use_custom_certificate: Flag = "speedtestUseCustomCertificate", false;
    homepage_netronome_enabled: Flag = "homepageNetronomeEnabled", false;
    homepage_netronome_auth: TextNumber<u32> = "homepageNetronomeAuth", 1;
    homepage_netronome_refresh: TextNumber<u64> = "homepageNetronomeRefresh", 300_000;
    netronome_url: Text = "netronomeURL", String::new();
    netronome_disable_cert_check: Flag = "netronomeDisableCertCheck", false;
    netronome_use_custom_certificate: Flag = "netronomeUseCustomCertificate", false;
    homepage_netronome_header_toggle: Flag = "homepageNetronomeHeaderToggle", true;
    homepage_netronome_header: Text = "homepageNetronomeHeader", "Netronome".to_owned();
    homepage_netronome_time_range: Text = "homepageNetronomeTimeRange", "week".to_owned();
    homepage_netdata_enabled: Flag = "homepageNetdataEnabled", false;
    homepage_netdata_refresh: TextNumber<u64> = "homepageNetdataRefresh", 10_000;
    homepage_netdata_auth: TextNumber<u32> = "homepageNetdataAuth", 1;
    netdata_disable_cert_check: Flag = "netdataDisableCertCheck", false;
    netdata_use_custom_certificate: Flag = "netdataUseCustomCertificate", false;
    netdata_url: Text = "netdataURL", String::new();
    netdata1_title: Text = "netdata1Title", String::new();
    netdata1_chart: Text = "netdata1Chart", String::new();
    netdata1_data: Text = "netdata1Data", String::new();
    netdata1_colour: Text = "netdata1Colour", String::new();
    netdata1_size: Text = "netdata1Size", String::new();
    netdata1sm: Flag = "netdata1sm", true;
    netdata1md: Flag = "netdata1md", true;
    netdata1lg: Flag = "netdata1lg", true;
    netdata2_title: Text = "netdata2Title", String::new();
    netdata2_chart: Text = "netdata2Chart", String::new();
    netdata2_data: Text = "netdata2Data", String::new();
    netdata2_colour: Text = "netdata2Colour", String::new();
    netdata2_size: Text = "netdata2Size", String::new();
    netdata2sm: Flag = "netdata2sm", true;
    netdata2md: Flag = "netdata2md", true;
    netdata2lg: Flag = "netdata2lg", true;
    netdata3_title: Text = "netdata3Title", String::new();
    netdata3_chart: Text = "netdata3Chart", String::new();
    netdata3_data: Text = "netdata3Data", String::new();
    netdata3_colour: Text = "netdata3Colour", String::new();
    netdata3_size: Text = "netdata3Size", String::new();
    netdata3sm: Flag = "netdata3sm", true;
    netdata3md: Flag = "netdata3md", true;
    netdata3lg: Flag = "netdata3lg", true;
    netdata4_title: Text = "netdata4Title", String::new();
    netdata4_chart: Text = "netdata4Chart", String::new();
    netdata4_data: Text = "netdata4Data", String::new();
    netdata4_colour: Text = "netdata4Colour", String::new();
    netdata4_size: Text = "netdata4Size", String::new();
    netdata4sm: Flag = "netdata4sm", true;
    netdata4md: Flag = "netdata4md", true;
    netdata4lg: Flag = "netdata4lg", true;
    netdata5_title: Text = "netdata5Title", String::new();
    netdata5_chart: Text = "netdata5Chart", String::new();
    netdata5_data: Text = "netdata5Data", String::new();
    netdata5_colour: Text = "netdata5Colour", String::new();
    netdata5_size: Text = "netdata5Size", String::new();
    netdata5sm: Flag = "netdata5sm", true;
    netdata5md: Flag = "netdata5md", true;
    netdata5lg: Flag = "netdata5lg", true;
    netdata6_title: Text = "netdata6Title", String::new();
    netdata6_chart: Text = "netdata6Chart", String::new();
    netdata6_data: Text = "netdata6Data", String::new();
    netdata6_colour: Text = "netdata6Colour", String::new();
    netdata6_size: Text = "netdata6Size", String::new();
    netdata6sm: Flag = "netdata6sm", true;
    netdata6md: Flag = "netdata6md", true;
    netdata6lg: Flag = "netdata6lg", true;
    netdata7_title: Text = "netdata7Title", String::new();
    netdata7_chart: Text = "netdata7Chart", String::new();
    netdata7_data: Text = "netdata7Data", String::new();
    netdata7_colour: Text = "netdata7Colour", String::new();
    netdata7_size: Text = "netdata7Size", String::new();
    netdata7sm: Flag = "netdata7sm", true;
    netdata7md: Flag = "netdata7md", true;
    netdata7lg: Flag = "netdata7lg", true;
    netdata1_enabled: Flag = "netdata1Enabled", false;
    netdata2_enabled: Flag = "netdata2Enabled", false;
    netdata3_enabled: Flag = "netdata3Enabled", false;
    netdata4_enabled: Flag = "netdata4Enabled", false;
    netdata5_enabled: Flag = "netdata5Enabled", false;
    netdata6_enabled: Flag = "netdata6Enabled", false;
    netdata7_enabled: Flag = "netdata7Enabled", false;
    netdata_custom: Text = "netdataCustom", "{}".to_owned();
    homepage_octoprint_enabled: Flag = "homepageOctoprintEnabled", false;
    homepage_octoprint_auth: TextNumber<u32> = "homepageOctoprintAuth", 1;
    homepage_octoprint_refresh: TextNumber<u64> = "homepageOctoprintRefresh", 10_000;
    octoprint_url: Text = "octoprintURL", String::new();
    octoprint_token: Secret = "octoprintToken", String::new();
    octoprint_header_toggle: Flag = "octoprintHeaderToggle", true;
    octoprint_header: Text = "octoprintHeader", "Octoprint".to_owned();
    octoprint_disable_cert_check: Flag = "octoprintDisableCertCheck", false;
    octoprint_use_custom_certificate: Flag = "octoprintUseCustomCertificate", false;
    github_menu_link: Flag = "githubMenuLink", true;
    organizr_support_menu_link: Flag = "organizrSupportMenuLink", true;
    organizr_docs_menu_link: Flag = "organizrDocsMenuLink", true;
    organizr_signout_menu_link: Flag = "organizrSignoutMenuLink", true;
    organizr_feature_request_link: Flag = "organizrFeatureRequestLink", true;
    breezometer_token: Secret = "breezometerToken", String::new();
    custom_forgot_pass_text: Text = "customForgotPassText", String::new();
    disable_recover_pass: Flag = "disableRecoverPass", false;
    expand_categories_by_default: Flag = "expandCategoriesByDefault", false;
    ignored_news_ids: List = "ignoredNewsIds", Vec::new();
    homepage_trakt_enabled: Flag = "homepageTraktEnabled", false;
    homepage_trakt_auth: TextNumber<u32> = "homepageTraktAuth", 1;
    calendar_start_trakt: TextNumber<u32> = "calendarStartTrakt", 14;
    calendar_end_trakt: TextNumber<u32> = "calendarEndTrakt", 14;
    trakt_client_id: Text = "traktClientId", String::new();
    trakt_client_secret: Secret = "traktClientSecret", String::new();
    trakt_access_token: Secret = "traktAccessToken", String::new();
    trakt_access_token_expires: Text = "traktAccessTokenExpires", String::new();
    trakt_refresh_token: Secret = "traktRefreshToken", String::new();
    auto_collapse_categories: Flag = "autoCollapseCategories", false;
    auto_expand_nav_bar: Flag = "autoExpandNavBar", true;
    default_settings_tab: TextNumber<u32> = "defaultSettingsTab", 5;
    blacklisted: Text = "blacklisted", String::new();
    blacklisted_message: Text = "blacklistedMessage", "You have been blacklisted from this site.".to_owned();
    default_request_service: Text = "defaultRequestService", "ombi".to_owned();
    easter_eggs: Flag = "easterEggs", true;
    allow_collapsable_side_menu: Flag = "allowCollapsableSideMenu", false;
    side_menu_collapsed: Flag = "sideMenuCollapsed", false;
    collapse_side_menu_on_click: Flag = "collapseSideMenuOnClick", false;
    log_level: Text = "logLevel", "INFO".to_owned();
    max_log_files: TextNumber<u32> = "maxLogFiles", 7;
    log_live_update_refresh: TextNumber<u64> = "logLiveUpdateRefresh", 5000;
    log_page_size: TextNumber<u32> = "logPageSize", 50;
    include_database_queries_in_debug: Flag = "includeDatabaseQueriesInDebug", false;
    external_plugin_marketplace_repos: Text = "externalPluginMarketplaceRepos", String::new();
    external_theme_marketplace_repos: Text = "externalThemeMarketplaceRepos", String::new();
    github_access_token: Secret = "githubAccessToken", String::new();
    check_for_plugin_update: Flag = "checkForPluginUpdate", true;
    check_for_theme_update: Flag = "checkForThemeUpdate", true;
    auto_update_cron_enabled: Flag = "autoUpdateCronEnabled", false;
    auto_update_cron_schedule: Text = "autoUpdateCronSchedule", "@weekly".to_owned();
    auto_backup_cron_enabled: Flag = "autoBackupCronEnabled", false;
    auto_backup_cron_schedule: Text = "autoBackupCronSchedule", "@weekly".to_owned();
    keep_backups_count_cron: TextNumber<u32> = "keepBackupsCountCron", 20;
    backup_location: Text = "backupLocation", String::new();
    use_random_media_image: Flag = "useRandomMediaImage", false;
    send_logs_to_slack: Flag = "sendLogsToSlack", false;
    slack_log_level: Text = "slackLogLevel", "WARNING".to_owned();
    slack_log_webhook: Secret = "slackLogWebhook", String::new();
    slack_log_web_hook_channel: Text = "slackLogWebHookChannel", String::new();
    disable_homepage_modals: Flag = "disableHomepageModals", false;
    homepage_uptime_kuma_enabled: Flag = "homepageUptimeKumaEnabled", false;
    homepage_uptime_kuma_auth: TextNumber<u32> = "homepageUptimeKumaAuth", 1;
    uptime_kuma_url: Text = "uptimeKumaURL", String::new();
    uptime_kuma_token: Secret = "uptimeKumaToken", String::new();
    homepage_uptime_kuma_refresh: TextNumber<u64> = "homepageUptimeKumaRefresh", 60_000;
    homepage_uptime_kuma_header: Text = "homepageUptimeKumaHeader", "Uptime Kuma".to_owned();
    homepage_uptime_kuma_header_toggle: Flag = "homepageUptimeKumaHeaderToggle", true;
    homepage_uptime_kuma_compact: Flag = "homepageUptimeKumaCompact", true;
    homepage_uptime_kuma_show_latency: Flag = "homepageUptimeKumaShowLatency", true;
    homepage_prom_page_enabled: Flag = "homepagePromPageEnabled", false;
    prom_page_url: Text = "promPageURL", String::new();
    homepage_prom_page_refresh: TextNumber<u64> = "homepagePromPageRefresh", 60_000;
    homepage_prom_page_header: Text = "homepagePromPageHeader", "Status Page".to_owned();
    homepage_prom_page_header_toggle: Flag = "homepagePromPageHeaderToggle", true;
    homepage_prom_page_compact: Flag = "homepagePromPageCompact", true;
    homepage_prom_page_show_uptime: Flag = "homepagePromPageShowUptime", true;
    check_for_update: Flag = "checkForUpdate", true;
    socks_debug: Flag = "socksDebug", false;
    max_socks_debug_size: Number<u32> = "maxSocksDebugSize", 100;
    bypass_login_for_local: Flag = "bypassLoginForLocal", false;
    local_login_user_id: TextNumber<u32> = "localLoginUserId", 1;
    oidc_enabled: Flag = "oidcEnabled", false;
    oidc_auto_redirect: Flag = "oidcAutoRedirect", false;
    oidc_auto_redirect_provider: Text = "oidcAutoRedirectProvider", String::new();
    oidc_auto_redirect_logout_url: Text = "oidcAutoRedirectLogoutUrl", String::new();
    oidc_auto_create_users: Flag = "oidcAutoCreateUsers", true;
    oidc_link_existing_users: Flag = "oidcLinkExistingUsers", true;
    oidc_default_group_id: TextNumber<u32> = "oidcDefaultGroupId", 4;
    oidc_group_claim_name: Text = "oidcGroupClaimName", "groups".to_owned();
    oidc_group_mappings: Text = "oidcGroupMappings", "{}".to_owned();
    oidc_group_mapping_mode: Text = "oidcGroupMappingMode", "first".to_owned();
    oidc_update_groups_on_login: Flag = "oidcUpdateGroupsOnLogin", false;
    oidc_authentik_enabled: Flag = "oidcAuthentikEnabled", false;
    oidc_authentik_name: Text = "oidcAuthentikName", "Authentik".to_owned();
    oidc_authentik_client_id: Text = "oidcAuthentikClientId", String::new();
    oidc_authentik_client_secret: Secret = "oidcAuthentikClientSecret", String::new();
    oidc_authentik_discovery_url: Text = "oidcAuthentikDiscoveryUrl", String::new();
    oidc_authentik_scopes: Text = "oidcAuthentikScopes", "openid,profile,email,groups".to_owned();
    oidc_authentik_group_claim: Text = "oidcAuthentikGroupClaim", String::new();
    oidc_keycloak_enabled: Flag = "oidcKeycloakEnabled", false;
    oidc_keycloak_name: Text = "oidcKeycloakName", "Keycloak".to_owned();
    oidc_keycloak_client_id: Text = "oidcKeycloakClientId", String::new();
    oidc_keycloak_client_secret: Secret = "oidcKeycloakClientSecret", String::new();
    oidc_keycloak_discovery_url: Text = "oidcKeycloakDiscoveryUrl", String::new();
    oidc_keycloak_scopes: Text = "oidcKeycloakScopes", "openid,profile,email".to_owned();
    oidc_keycloak_group_claim: Text = "oidcKeycloakGroupClaim", String::new();
    oidc_pocketid_enabled: Flag = "oidcPocketidEnabled", false;
    oidc_pocketid_name: Text = "oidcPocketidName", "PocketID".to_owned();
    oidc_pocketid_client_id: Text = "oidcPocketidClientId", String::new();
    oidc_pocketid_client_secret: Secret = "oidcPocketidClientSecret", String::new();
    oidc_pocketid_discovery_url: Text = "oidcPocketidDiscoveryUrl", String::new();
    oidc_pocketid_scopes: Text = "oidcPocketidScopes", "openid,profile,email".to_owned();
    oidc_pocketid_group_claim: Text = "oidcPocketidGroupClaim", String::new();
    oidc_zitadel_enabled: Flag = "oidcZitadelEnabled", false;
    oidc_zitadel_name: Text = "oidcZitadelName", "Zitadel".to_owned();
    oidc_zitadel_client_id: Text = "oidcZitadelClientId", String::new();
    oidc_zitadel_client_secret: Secret = "oidcZitadelClientSecret", String::new();
    oidc_zitadel_discovery_url: Text = "oidcZitadelDiscoveryUrl", String::new();
    oidc_zitadel_scopes: Text = "oidcZitadelScopes", "openid,profile,email,urn:zitadel:iam:org:project:roles".to_owned();
    oidc_zitadel_group_claim: Text = "oidcZitadelGroupClaim", "urn:zitadel:iam:org:project:roles".to_owned();
    bookmark_enabled: Flag = "BOOKMARK-enabled", false;
    chat_enabled: Flag = "CHAT-enabled", false;
    chat_auth_include: TextNumber<u32> = "CHAT-Auth-include", 1;
    chat_auth_key_include: Text = "CHAT-authKey-include", String::new();
    chat_app_id_include: Text = "CHAT-appID-include", String::new();
    chat_cluster_include: Text = "CHAT-cluster-include", String::new();
    chat_secret: Secret = "CHAT-secret", String::new();
    chat_message_load_limit: TextNumber<u32> = "CHAT-messageLoadLimit", 200;
    chat_user_refresh_timeout: TextNumber<u64> = "CHAT-userRefreshTimeout", 60_000;
    chat_new_message_sound_include: Text = "CHAT-newMessageSound-include", "plugins/sounds/default/newmessage.mp3".to_owned();
    chat_new_message_soundz_include: Text = "CHAT-newMessageSoundz-include", "plugins/sounds/default/newmessage.mp3".to_owned();
    chat_use_ssl: Flag = "CHAT-useSSL", false;
    health_checks_enabled: Flag = "HEALTHCHECKS-enabled", false;
    health_checks_401_enabled: Flag = "HEALTHCHECKS-401-enabled", false;
    health_checks_403_enabled: Flag = "HEALTHCHECKS-403-enabled", false;
    health_checks_cron_run_enabled: Flag = "HEALTHCHECKS-cron-run-enabled", false;
    health_checks_cron_run_schedule: Text = "HEALTHCHECKS-cron-run-schedule", "*/5 * * * *".to_owned();
    health_checks_auth_include: TextNumber<u32> = "HEALTHCHECKS-Auth-include", 1;
    health_checks_option2_include: Text = "HEALTHCHECKS-option2-include", String::new();
    // The plugin stores its checks here, as a map from a name to a check.
    health_checks_all_items: Anything = "HEALTHCHECKS-all-items", Value::String(String::new());
    health_checks_ping_url: Text = "HEALTHCHECKS-PingURL", "https://hc-ping.com/".to_owned();
    invites_enabled: Flag = "INVITES-enabled", false;
    invites_auth_include: TextNumber<u32> = "INVITES-Auth-include", 1;
    invites_db_version: Text = "INVITES-dbVersion", "1.0.0".to_owned();
    invites_type_include: Text = "INVITES-type-include", "plex".to_owned();
    invites_plex_libraries: Text = "INVITES-plexLibraries", String::new();
    invites_add_plex_home: Flag = "INVITES-add-plex-home", false;
    invites_emby_template: Text = "INVITES-EmbyTemplate", String::new();
    invites_plex_tv_labels: Text = "INVITES-plex-tv-labels", String::new();
    invites_plex_music_labels: Text = "INVITES-plex-music-labels", String::new();
    invites_plex_movies_labels: Text = "INVITES-plex-movies-labels", String::new();
    invites_allow_delete_include: Flag = "INVITES-allow-delete-include", false;
    invites_maximum_invites: TextNumber<u32> = "INVITES-maximum-invites", 0;
    invites_komga_enabled: Flag = "INVITES-komga-enabled", false;
    invites_komga_uri: Text = "INVITES-komga-uri", String::new();
    invites_komga_api_key: Secret = "INVITES-komga-api-key", String::new();
    invites_komga_default_user_password: Secret = "INVITES-komga-default-user-password", String::new();
    invites_komga_roles: Text = "INVITES-komga-roles", String::new();
    invites_komga_library_ids: Text = "INVITES-komga-libraryIds", String::new();
    invites_nextcloud_enabled: Flag = "INVITES-nextcloud-enabled", false;
    invites_nextcloud_plex_sso: Flag = "INVITES-nextcloud-plex-sso", false;
    invites_nextcloud_url: Text = "INVITES-nextcloud-url", String::new();
    invites_nextcloud_admin_user: Text = "INVITES-nextcloud-admin-user", String::new();
    invites_nextcloud_admin_password: Secret = "INVITES-nextcloud-admin-password", String::new();
    invites_nextcloud_groups_member: Text = "INVITES-nextcloud-groups-member", String::new();
    invites_nextcloud_quota: Text = "INVITES-nextcloud-quota", String::new();
    // The Plex Libraries plugin of this application (D-121).
    plex_libraries_enabled: Flag = "PLEXLIBRARIES-enabled", false;
    plex_libraries_auth: TextNumber<u32> = "PLEXLIBRARIES-Auth-include", 4;
    plex_libraries_include: Text = "PLEXLIBRARIES-librariesToInclude", String::new();
    mailer_enabled: Flag = "PHPMAILER-enabled", false;
    mailer_debug_testing: Flag = "PHPMAILER-debugTesting", false;
    mailer_smtp_host: Text = "PHPMAILER-smtpHost", String::new();
    mailer_smtp_host_port: Text = "PHPMAILER-smtpHostPort", String::new();
    mailer_smtp_host_auth: Flag = "PHPMAILER-smtpHostAuth", true;
    mailer_smtp_host_username: Text = "PHPMAILER-smtpHostUsername", String::new();
    mailer_smtp_host_password: Secret = "PHPMAILER-smtpHostPassword", String::new();
    mailer_smtp_host_sender_name: Text = "PHPMAILER-smtpHostSenderName", "Organizr".to_owned();
    mailer_smtp_host_sender_email: Text = "PHPMAILER-smtpHostSenderEmail", "no-reply@Organizr.tld".to_owned();
    mailer_smtp_host_type: Text = "PHPMAILER-smtpHostType", "tls".to_owned();
    mailer_domain: Text = "PHPMAILER-domain", String::new();
    mailer_template: Text = "PHPMAILER-template", "default".to_owned();
    mailer_logo: Text = "PHPMAILER-logo", "https://raw.githubusercontent.com/causefx/Organizr/v2-develop/plugins/images/organizr/logo-wide.png".to_owned();
    mailer_email_template_reset: Text = "PHPMAILER-emailTemplateReset", "\n\t<h2>Hey there {user}!</h2><br />\n\tLooks like you forgot your password.  Well, I got you...  Here is your new password: {password}<br />\n\tIf you want to change it once you log in, you can.  Head over to my website: {domain}<br />\n\t".to_owned();
    mailer_email_template_reset_subject: Text = "PHPMAILER-emailTemplateResetSubject", "Password Reset".to_owned();
    mailer_email_template_invite_user: Text = "PHPMAILER-emailTemplateInviteUser", "\n\t<h2>Hey there {user}!</h2><br />\n\tHere is the invite code to join my cool media server: {inviteCode}<br/>\n\tHead over to my website and enter the code to join: {domain}<br />\n\t".to_owned();
    mailer_email_template_invite_user_subject: Text = "PHPMAILER-emailTemplateInviteUserSubject", "You have been invited to join my server".to_owned();
    mailer_email_template_register_user: Text = "PHPMAILER-emailTemplateRegisterUser", "\n\t<h2>Hey there {user}!</h2><br />\n\tWelcome to my site.<br/>\n\tIf you need anything, please let me know.<br />\n\t".to_owned();
    mailer_email_template_register_user_subject: Text = "PHPMAILER-emailTemplateRegisterUserSubject", "Thank you For Registering".to_owned();
    mailer_email_template_register_user_enabled: Flag = "PHPMAILER-emailTemplateRegisterUserEnabled", true;
    mailer_email_template_custom_include_one: Text = "PHPMAILER-emailTemplateCustom-include-One", String::new();
    mailer_email_template_custom_include_one_name: Text = "PHPMAILER-emailTemplateCustom-include-OneName", "Template #1".to_owned();
    mailer_email_template_custom_include_one_subject: Text = "PHPMAILER-emailTemplateCustom-include-OneSubject", String::new();
    mailer_email_template_custom_include_two: Text = "PHPMAILER-emailTemplateCustom-include-Two", String::new();
    mailer_email_template_custom_include_two_name: Text = "PHPMAILER-emailTemplateCustom-include-TwoName", "Template #2".to_owned();
    mailer_email_template_custom_include_two_subject: Text = "PHPMAILER-emailTemplateCustom-include-TwoSubject", String::new();
    mailer_email_template_custom_include_three: Text = "PHPMAILER-emailTemplateCustom-include-Three", String::new();
    mailer_email_template_custom_include_three_name: Text = "PHPMAILER-emailTemplateCustom-include-ThreeName", "Template #3".to_owned();
    mailer_email_template_custom_include_three_subject: Text = "PHPMAILER-emailTemplateCustom-include-ThreeSubject", String::new();
    mailer_email_template_custom_include_four: Text = "PHPMAILER-emailTemplateCustom-include-Four", String::new();
    mailer_email_template_custom_include_four_name: Text = "PHPMAILER-emailTemplateCustom-include-FourName", "Template #4".to_owned();
    mailer_email_template_custom_include_four_subject: Text = "PHPMAILER-emailTemplateCustom-include-FourSubject", String::new();
    mailer_verify_cert: Flag = "PHPMAILER-verifyCert", true;
    speed_test_enabled: Flag = "SPEEDTEST-enabled", false;
    speed_test_auth_include: TextNumber<u32> = "SPEEDTEST-Auth-include", 1;
    speed_test_option2_include: Text = "SPEEDTEST-option2-include", String::new();
}

impl Settings {
    /// Builds the settings from what the database holds: the defaults, with each stored
    /// value on top.
    ///
    /// `stored` gives each key with its value as JSON text. A stored entry that cannot be
    /// used (an unknown key, a value that does not fit, text that is not JSON) is left out
    /// and reported, and its setting keeps the default. The application can then still
    /// start and show the problem.
    pub fn from_stored<'a, I>(stored: I) -> (Self, Vec<SettingsError>)
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut settings = Self::defaults();
        let mut problems = Vec::new();
        for (key, text) in stored {
            let applied = serde_json::from_str::<Value>(text)
                .map_err(|error| SettingsError::InvalidJson {
                    key: key.to_owned(),
                    reason: error.to_string(),
                })
                .and_then(|value| settings.set(key, &value));
            if let Err(problem) = applied {
                problems.push(problem);
            }
        }
        (settings, problems)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{Settings, SettingsError};
    use crate::choices::{AuthType, UnsortedTabs};

    /// The default of every setting of Organizr 2.1.5000, written by the PHP application
    /// itself. `parity/php-settings/generate.php` makes the file.
    fn php_defaults() -> Vec<(String, Value)> {
        let fixture: Value =
            serde_json::from_str(include_str!("../tests/fixtures/php_defaults.json"))
                .expect("the fixture is JSON");
        fixture["defaults"]
            .as_array()
            .expect("the fixture has a list of defaults")
            .iter()
            .map(|entry| {
                let key = entry["key"].as_str().expect("a key is text").to_owned();
                (key, entry["value"].clone())
            })
            .collect()
    }

    /// Keys of the PHP defaults that are no settings here: the database is chosen with
    /// `ORGANIZR_DATABASE_URL` (D-45, D-46).
    const NOT_SETTINGS: [&str; 4] = ["driver", "dbHost", "dbUsername", "dbPassword"];

    /// Settings that the PHP defaults do not have: the set-up wizard writes them. Before
    /// that, PHP reads them as empty.
    const WRITTEN_BY_THE_SET_UP: [&str; 3] = ["uuid", "organizrAPI", "license"];

    /// Settings that the forms and the code of PHP read, but that the PHP defaults do not
    /// have. Before an Admin saves them, PHP reads them as empty (D-102).
    const READ_WITHOUT_A_PHP_DEFAULT: [&str; 2] = ["adGuardUsername", "adGuardPassword"];

    #[test]
    fn prowlarr_is_stored_under_the_keys_that_php_reads() {
        let defaults = Settings::defaults();
        for (php_default, read) in crate::RENAMED_SETTINGS {
            assert_eq!(defaults.get(php_default), None, "`{php_default}`");
            assert!(defaults.get(read).is_some(), "`{read}`");
        }
        assert_eq!(defaults.get("prowlarrURL"), Some(json!("")));
        assert!(Settings::is_secret("prowlarrToken"));
    }

    /// Settings whose default differs from PHP, each with the default here.
    fn changed_defaults() -> Vec<(&'static str, Value)> {
        // PHP ships a token of a third party as the default (D-12).
        vec![("breezometerToken", json!(""))]
    }

    #[test]
    fn every_default_is_the_default_of_the_php_application() {
        let defaults = Settings::defaults();
        let changed = changed_defaults();
        let php = php_defaults();
        assert!(php.len() > 900, "the fixture holds every default");

        for (key, value) in &php {
            if NOT_SETTINGS.contains(&key.as_str()) || crate::is_removed_setting(key) {
                assert_eq!(defaults.get(key), None, "`{key}` is no setting");
                continue;
            }
            // A default of PHP under another name than the key that PHP reads has its
            // default under that key here.
            let key = crate::RENAMED_SETTINGS
                .iter()
                .find(|(php_default, _)| php_default == key)
                .map_or(key.as_str(), |(_, read)| read);
            let expected = changed
                .iter()
                .find(|(changed_key, _)| *changed_key == key)
                .map_or(value, |(_, own)| own);
            assert_eq!(
                defaults.get(key).as_ref(),
                Some(expected),
                "the default of `{key}`"
            );
        }
        for (key, own) in &changed {
            let (_, of_php) = php
                .iter()
                .find(|(php_key, _)| php_key == key)
                .expect("a changed default is a default of PHP");
            assert_ne!(of_php, own, "`{key}` is listed as changed");
        }
    }

    /// The settings that the PHP application does not have. Each one is a recorded
    /// difference: see `docs/deviations.md`.
    const OWN_SETTINGS: [&str; 8] = [
        "homepageCustomHTML01Trusted",
        "homepageCustomHTML02Trusted",
        "homepageCustomHTML03Trusted",
        "homepageCustomHTML04Trusted",
        "homepageCustomHTML05Trusted",
        "homepageCustomHTML06Trusted",
        "homepageCustomHTML07Trusted",
        "homepageCustomHTML08Trusted",
    ];

    /// The settings of this application that PHP Organizr does not have, each with its
    /// default: those of the Plex Libraries plugin (D-121), the look settings of the
    /// Admin (D-125), those of the Netronome item (D-128), and the allowed hosts and the
    /// trusted proxies (D-155).
    fn own_plugin_settings() -> Vec<(&'static str, Value)> {
        vec![
            ("PLEXLIBRARIES-enabled", json!(false)),
            ("PLEXLIBRARIES-Auth-include", json!("4")),
            ("PLEXLIBRARIES-librariesToInclude", json!("")),
            ("layout", json!("og")),
            ("modernTheme", json!("organizr")),
            ("modernColorSet", json!("dark")),
            ("homepageOrderNetronome", json!("49")),
            ("homepageNetronomeEnabled", json!(false)),
            ("homepageNetronomeAuth", json!("1")),
            ("homepageNetronomeRefresh", json!("300000")),
            ("netronomeURL", json!("")),
            ("netronomeDisableCertCheck", json!(false)),
            ("netronomeUseCustomCertificate", json!(false)),
            ("homepageNetronomeHeaderToggle", json!(true)),
            ("homepageNetronomeHeader", json!("Netronome")),
            ("homepageNetronomeTimeRange", json!("week")),
            ("allowedHosts", json!("")),
            ("trustedProxies", json!("")),
        ]
    }

    #[test]
    fn every_setting_is_a_setting_of_the_php_application() {
        let php = php_defaults();

        for key in Settings::KEYS {
            let known = php.iter().any(|(php_key, _)| php_key == key);
            if let Some((_, default)) = own_plugin_settings()
                .into_iter()
                .find(|(own, _)| own == key)
            {
                assert!(
                    !known,
                    "`{key}` is listed as a setting of this application only"
                );
                assert_eq!(Settings::defaults().get(key), Some(default), "`{key}`");
                continue;
            }
            if OWN_SETTINGS.contains(key) {
                assert!(
                    !known,
                    "`{key}` is listed as a setting of this application only"
                );
                // A setting of this application only is off until the Admin changes it.
                assert_eq!(Settings::defaults().get(key), Some(json!(false)), "`{key}`");
                continue;
            }
            if crate::RENAMED_SETTINGS.iter().any(|(_, read)| read == key) {
                assert!(
                    !known,
                    "`{key}` is listed as a PHP default under another name"
                );
                continue;
            }
            let without_default =
                WRITTEN_BY_THE_SET_UP.contains(key) || READ_WITHOUT_A_PHP_DEFAULT.contains(key);
            assert_ne!(
                known, without_default,
                "`{key}` is a PHP default or is read without one, and not both"
            );
        }
        for key in WRITTEN_BY_THE_SET_UP
            .iter()
            .chain(&READ_WITHOUT_A_PHP_DEFAULT)
        {
            assert_eq!(Settings::defaults().get(key), Some(json!("")), "`{key}`");
        }
    }

    /// Keys that sound like a secret and are none: a time, and two keys that every page
    /// gets and that the service in question publishes itself.
    const NOT_SECRET: [&str; 3] = [
        "traktAccessTokenExpires",
        "homepageDonatePublicToken",
        "CHAT-authKey-include",
    ];

    /// Secrets whose key does not say so: an address that lets its holder post messages,
    /// and two keys of services.
    const SECRET_BY_USE: [&str; 3] = ["slackLogWebhook", "organizrAPI", "youtubeAPI"];

    #[test]
    fn password_token_or_key_of_a_service_is_a_secret() {
        let sounds_secret = |key: &str| {
            let key = key.to_ascii_lowercase();
            [
                "password", "token", "secret", "apikey", "api-key", "cookie", "authkey",
            ]
            .iter()
            .any(|word| key.contains(word))
        };

        for key in Settings::KEYS {
            let expected =
                SECRET_BY_USE.contains(key) || (sounds_secret(key) && !NOT_SECRET.contains(key));
            assert_eq!(Settings::is_secret(key), expected, "`{key}`");
        }
        for key in NOT_SECRET.iter().chain(&SECRET_BY_USE) {
            assert!(Settings::KEYS.contains(key), "`{key}` is a setting");
        }
        assert!(Settings::is_secret("plexToken"));
        assert!(Settings::is_secret("registrationPassword"));
        assert!(!Settings::is_secret("title"));
        assert!(!Settings::is_secret("noSuchSetting"));
    }

    #[test]
    fn refusal_of_a_secret_does_not_repeat_the_value() {
        let mut settings = Settings::defaults();

        let error = settings
            .set("plexToken", &json!(["a-token"]))
            .expect_err("a list is no text");

        assert!(!error.to_string().contains("a-token"), "{error}");
    }

    #[test]
    fn no_key_is_listed_twice() {
        let mut keys = Settings::KEYS.to_vec();
        keys.sort_unstable();
        let count = keys.len();
        keys.dedup();

        assert_eq!(keys.len(), count);
    }

    #[test]
    fn number_and_list_keep_their_stored_form() {
        let mut settings = Settings::defaults();

        assert_eq!(settings.get("maxSocksDebugSize"), Some(json!(100)));
        assert_eq!(settings.get("ignoredNewsIds"), Some(json!([])));
        settings
            .set("maxSocksDebugSize", &json!(250))
            .expect("a number is valid");
        settings
            .set("ignoredNewsIds", &json!([4, "7"]))
            .expect("a list is valid");

        assert_eq!(settings.max_socks_debug_size, 250);
        assert_eq!(settings.get("ignoredNewsIds"), Some(json!([4, "7"])));
        for (key, value) in [
            ("maxSocksDebugSize", json!("250")),
            ("maxSocksDebugSize", json!(-1)),
            ("maxSocksDebugSize", json!(4_294_967_296_u64)),
            ("ignoredNewsIds", json!("4")),
        ] {
            assert!(
                matches!(
                    settings.set(key, &value),
                    Err(SettingsError::InvalidValue { .. })
                ),
                "`{key}` = {value}"
            );
        }
    }

    #[test]
    fn fields_are_typed() {
        let defaults = Settings::defaults();

        assert_eq!(defaults.remember_me_days, 7_u32);
        assert_eq!(defaults.login_lockout, 60_000_u64);
        assert_eq!(defaults.auth_type, AuthType::Internal);
        assert_eq!(defaults.unsorted_tabs, UnsortedTabs::Top);
        assert!(!defaults.match_user_ip);
        assert_eq!(defaults.title, "Organizr V2");
    }

    #[test]
    fn valid_value_changes_the_setting() {
        let mut settings = Settings::defaults();

        settings
            .set("rememberMeDays", &json!("30"))
            .expect("thirty days is valid");
        settings
            .set("authType", &json!("both"))
            .expect("both is a choice");
        settings
            .set("matchUserIP", &json!(true))
            .expect("true is valid");
        settings
            .set("title", &json!("Home"))
            .expect("text is valid");

        assert_eq!(settings.remember_me_days, 30);
        assert_eq!(settings.auth_type, AuthType::Both);
        assert!(settings.match_user_ip);
        assert_eq!(settings.title, "Home");
        assert_eq!(settings.get("rememberMeDays"), Some(json!("30")));
        assert_eq!(settings.get("authType"), Some(json!("both")));
    }

    #[test]
    fn number_that_is_stored_as_a_number_is_read_and_written_back_as_text() {
        let mut settings = Settings::defaults();

        settings
            .set("loginAttempts", &json!(3))
            .expect("a JSON number is read");

        assert_eq!(settings.login_attempts, 3);
        assert_eq!(settings.get("loginAttempts"), Some(json!("3")));
    }

    #[test]
    fn value_that_does_not_fit_is_refused_and_nothing_changes() {
        let mut settings = Settings::defaults();

        let cases = [
            ("rememberMeDays", json!("soon")),
            ("rememberMeDays", json!("-1")),
            ("rememberMeDays", json!(true)),
            ("authType", json!("sometimes")),
            ("authType", json!(1)),
            ("matchUserIP", json!("true")),
            ("title", json!(42)),
        ];
        for (key, value) in cases {
            let result = settings.set(key, &value);
            assert!(
                matches!(result, Err(SettingsError::InvalidValue { key: refused, .. }) if refused == key),
                "`{key}` = {value} gave {result:?}"
            );
        }

        assert_eq!(settings, Settings::defaults());
    }

    #[test]
    fn refusal_of_a_choice_names_the_choices() {
        let mut settings = Settings::defaults();

        let error = settings
            .set("authType", &json!("sometimes"))
            .expect_err("sometimes is not a choice");

        assert_eq!(
            error.to_string(),
            "the value of `authType` is not valid: expected one of internal, external, both, found \"sometimes\""
        );
    }

    #[test]
    fn unknown_key_is_refused() {
        let mut settings = Settings::defaults();

        assert_eq!(
            settings.set("organizrHash", &json!("x")),
            Err(SettingsError::UnknownKey("organizrHash".to_owned()))
        );
        assert_eq!(settings.get("organizrHash"), None);
        assert_eq!(settings, Settings::defaults());
    }

    #[test]
    fn stored_values_go_on_top_of_the_defaults() {
        let stored = [("rememberMeDays", "\"14\""), ("lockScreen", "true")];

        let (settings, problems) = Settings::from_stored(stored);

        assert_eq!(problems, []);
        assert_eq!(settings.remember_me_days, 14);
        assert!(settings.lock_screen);
        assert_eq!(
            settings.login_attempts, 5,
            "a setting that is not stored keeps its default"
        );
    }

    #[test]
    fn stored_entry_that_cannot_be_used_is_reported_and_left_out() {
        let stored = [
            ("rememberMeDays", "\"14\""),
            ("noSuchSetting", "true"),
            ("loginAttempts", "\"many\""),
            ("title", "not json"),
        ];

        let (settings, problems) = Settings::from_stored(stored);

        assert_eq!(settings.remember_me_days, 14, "the usable entry is applied");
        assert_eq!(settings.login_attempts, 5);
        assert_eq!(settings.title, "Organizr V2");
        assert_eq!(problems.len(), 3);
        assert!(matches!(&problems[0], SettingsError::UnknownKey(key) if key == "noSuchSetting"));
        assert!(matches!(
            &problems[1],
            SettingsError::InvalidValue {
                key: "loginAttempts",
                ..
            }
        ));
        assert!(matches!(&problems[2], SettingsError::InvalidJson { key, .. } if key == "title"));
    }
}
