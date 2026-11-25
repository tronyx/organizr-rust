# 🧹 Settings that could be removed

[← All documentation](README.md)

> [!NOTE]
> **Internal notes for contributors.** You don't need this page to run Organizr. It lists the settings that came over from PHP Organizr 2.1.5000 and that this version stores but never reads. They're kept for now. Each one is a candidate to remove, or, in the last group, to build.

Checked against the code on 2026-11-25: 82 of the 916 settings in `crates/settings/src/registry.rs` have no effect: nothing in the server or the web interface reads them, or, for the User Watch Stats item, only a generic check of its switch does. Settings whose names are built while the server runs, such as `customHTML01` to `customHTML08` or the `netdata1Title` family, are read and aren't listed.

## 🙈 Not shown on any page (46)

Nobody can change these in the interface. `GET /config` lists them, and an import keeps their values.

| Settings | What they were in PHP |
|---|---|
| `themeInstalled`, `themeVersion`, `INVITES-dbVersion` | Records PHP kept for its own updates |
| `HEALTHCHECKS-option2-include`, `SPEEDTEST-option2-include` | Empty placeholders of two plugin forms |
| `CHAT-authKey-include`, `CHAT-appID-include`, `CHAT-cluster-include`, `CHAT-secret`, `CHAT-userRefreshTimeout`, `CHAT-newMessageSoundz-include`, `CHAT-useSSL` | The Pusher service of PHP's chat. The chat of this version doesn't use Pusher ([Chat plugin](chat-plugin.md)) |
| `homepageOrderUserWatchStats` and the 16 `homepageUserWatchStats*` settings | The User Watch Stats homepage item, which this version doesn't have |
| `uTorrentCookie`, `uTorrentToken`, `unifiCookie` | Session values that PHP cached between requests |
| `localIPFrom`, `localIPTo` | An older form of **Override Local IP or Subnet** |
| `selfSignedCert`, `lockScreen`, `authBackendDomain`, `homepageDownloadRefresh`, `homepagePlexRecentlyAddedMethod`, `rTorrentReverseSorting`, `performanceDisableIconDropdown`, `performanceDisableImageDropdown` | Options of PHP for parts that this version builds differently or doesn't have |
| `authProxyEnabled`, `authProxyHeaderName`, `authProxyHeaderNameEmail`, `authProxyWhitelist` | The auth proxy login. Hidden on purpose ([D-96](deviations.md)); this version logs nobody in from a header |

## 🗑️ Shown on a page, and nothing would be missed (24)

These show on a settings page, but changing them does nothing.

| Page | Settings |
|---|---|
| Customize › Appearance | `gaTrackingID` (Google Analytics), `easterEggs` |
| System Settings › General | `socksDebug`, `maxSocksDebugSize` |
| System Settings › Logs | `includeDatabaseQueriesInDebug`, `slackLogLevel` |
| Plugins › Settings | `externalPluginMarketplaceRepos`, `githubAccessToken`, `checkForPluginUpdate` (this version has its own marketplace setting, see [Extensions](extensions.md)) |
| Customize › Marketplace Settings | `externalThemeMarketplaceRepos`, `checkForThemeUpdate` |
| PHP Mailer plugin | `PHPMAILER-debugTesting`, and the custom templates 1 to 4: `PHPMAILER-emailTemplateCustom-include-One` to `-Four`, each with its `Name` and `Subject` (12 settings) |

## 🛠️ Shown on a page, and PHP features that aren't built yet (12)

These worked in PHP. Each one can be built, or removed with its field.

| Settings | What it did in PHP |
|---|---|
| `hideRegistration` | Hides the Register link on the login page |
| `disableRecoverPass`, `customForgotPassText` | Switches "Forgot password" off, or changes its text |
| `blacklisted`, `blacklistedMessage` | Refuses the listed IP addresses at login, with a message |
| `matchUserIP` | Ties a session to the IP address it started from |
| `defaultSettingsTab` | Which settings page opens first |
| `ldapType` | The kind of LDAP server (Active Directory or OpenLDAP) |
| `calendarLocale`, `calendarTimeFormat` | The language of the calendar, and 12- or 24-hour times |
| `homepageEmbyLiveTVTrackerDaysShown` | How many days the Emby Live TV item shows |
| `homepageJellyStatShowRecentActivity` | Whether the Jellystat item shows recent activity |

## 🔧 How to remove one

Two ways, from light to complete:

1. **Hide it**: take its field off the settings page, and list it in `HIDDEN_KEYS` of `web/src/settings/forms/coverage.test.ts`. The server keeps it. This is what was done for the auth proxy.
2. **Remove it**: take it out of `registry.rs`, so `GET /config` no longer lists it. An import then reports it under "Settings that this application does not need", and a value already stored is deleted at start-up, as was done for the Shuck Stop settings ([D-116](deviations.md)). The parity replay of `/config` changes, so it needs an entry in [the deviations](deviations.md).
