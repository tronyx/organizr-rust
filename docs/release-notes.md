# 📰 Release notes

[← All documentation](README.md)

What this version of Organizr is, what works as in PHP Organizr, and what you will notice is different. Read it if you run PHP Organizr today and wonder whether to try this one.

> [!NOTE]
> There is no release yet. These notes describe the `develop` image, `ghcr.io/tronyx/organizr-rust:develop`, and will become the notes of the first release. [Releases and images](releases.md) explains the tags.

## 🦀 What this is

This is a rewrite of Organizr 2.1 in Rust. It aims to keep the API, the settings and the look of the PHP version (2.1.5000), and lists every place where it differs on purpose in [the deviations](deviations.md). It runs as one container image for `amd64` and `arm64`: `ghcr.io/tronyx/organizr-rust`.

## 🤔 Should I switch now?

- **Trying it out?** Go ahead. [Installation](installation.md) gets a fresh install running in a few minutes.
- **Moving a PHP install?** Import a copy and keep the PHP install until you're happy. The importer only reads the old folder, so your PHP install stays as it is. Follow [Moving from PHP Organizr](moving-from-php.md).
- **Want something that never changes under you?** Wait for the first release, or pin one `develop` build by its digest ([Pin one exact build](releases.md#-pin-one-exact-build)).

## 🚚 Moving from PHP Organizr

- The importer reads the data folder of a PHP install: the settings, the users and groups, the tabs and categories, the homepage, the pictures and the custom certificate. See [Importing an install of PHP Organizr](importing.md).
- The database is SQLite or PostgreSQL. See [Choosing a database](choosing-a-database.md).
- The API keeps the routes, the envelope and the answers of PHP, so scripts and apps that call Organizr keep working, apart from the differences in [the deviations](deviations.md).
- Everybody logs in once after the move: the sessions of PHP don't carry over.

## ✅ What works as in PHP

- Tabs, categories, groups and users, the sidebar, the top bar, the lock screen and the login page.
- The homepage items: Plex, Emby and Jellyfin, Tautulli, the calendar, the request services, the download clients, health checks, Uptime Kuma, Pi-hole, AdGuard Home, Netdata, UniFi and the others.
- The settings pages, drawn from the forms of PHP, with their groups, fields and test buttons.
- Single sign-on, the login services (Plex, LDAP, FTP, Emby, Jellyfin), OpenID Connect and the second factor.
- The plugins of PHP: invites, e-mail, speed test, health checks, bookmarks and chat.
- **New:** the Plex Libraries plugin, which shares and unshares the libraries of your Plex server (details: D-121).
- The 32 language files of PHP translate the interface.

## ✨ What is different

Every difference is in [the deviations](deviations.md), with its reason. These are the ones you are most likely to notice:

- **Layouts and themes.** A new install starts on the modern layout; an imported install stays on the OG layout. Each person can pick their own layout, theme and color set. See [Appearance](appearance.md) (details: D-125).
- **Themes and custom CSS.** PHP themes and custom CSS written for the page structure of PHP don't apply. Change your custom CSS once to the names in [Custom CSS](custom-css.md), which has a table from the PHP names. The 11 themes of the marketplace are rebuilt (details: D-42, D-92, D-110).
- **Custom JavaScript** runs only when the server starts with `ORGANIZR_CUSTOM_JS=1` (details: D-93).
- **Custom HTML items** run in a closed frame, unless you switch the item to trusted. Their links to the PHP pop-ups, such as `#new-request` or `#speedtest-area`, open the matching search or page (details: D-61, D-76, D-111).
- **Plugins of other authors** are WebAssembly extensions, not PHP files. See [Extensions](extensions.md) (details: D-75).
- **Get Plex Token** opens the login page of Plex instead of asking for your Plex password (details: D-107).
- **The search for new requests** asks Overseerr or Ombi through the server, not TMDB from the browser, and has no suggestion lists (details: D-108).
- **Languages.** Texts that only this version has stay in English (details: D-109).
- **Safer logins.** A login through Plex or Emby Connect never opens an Admin or Co-Admin account, except the Plex owner's, and no outside service replaces an Admin or Co-Admin password (details: D-127).
- **Secrets** (passwords, tokens and API keys of your services) are stored encrypted in the database, and the API shows them masked.
- **Updates** mean pulling a new image. The update page checks for a new version and installs nothing (details: D-23).
- **No news on the About page.** The page has a strict security policy and loads nothing from other sites, so it can't fetch the Organizr news.

## 🧪 Checks

Each image passes the checks in [Releases and images](releases.md#-the-checks-before-an-image-is-published) before it is published.
