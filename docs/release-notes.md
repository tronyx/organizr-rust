# Organizr (Rust) — first release

This is a rewrite of Organizr 2.1 in Rust. It keeps the API, the settings and the look of the PHP version. It runs as one container image for `amd64` and `arm64`: `ghcr.io/tronyx/organizr-rust`.

## Moving from PHP Organizr

- The importer reads the data folder of a PHP install: the settings, the users and groups, the tabs and categories, the homepage, the pictures and the custom certificate. See [Importing an install of PHP Organizr](importing.md).
- The database is SQLite or PostgreSQL. See [Choosing a database](choosing-a-database.md).
- The API keeps the routes, the envelope and the answers of PHP, so scripts and apps that call Organizr keep working.

## What works as in PHP

- Tabs, categories, groups and users, the sidebar, the top bar, the lock screen and the login page.
- The homepage items: Plex, Emby and Jellyfin, Tautulli, the calendar, the request services, the download clients, health checks, Uptime Kuma, Pi-hole, AdGuard Home, Netdata, UniFi and the others.
- The settings pages, drawn from the forms of PHP, with their groups, fields and test buttons.
- Single sign-on, the login services (Plex, LDAP, FTP, Emby, Jellyfin), OpenID Connect and the second factor.
- The plugins of PHP: invites, e-mail, speed test, health checks and bookmarks.

## What is different

Each difference is in [the list of deviations](deviations.md), with its reason. The ones you are most likely to notice:

- **Themes and custom CSS or JavaScript** written for the page structure of PHP do not apply as a whole (D-42). The pages carry PHP's ids and classes where custom CSS usually aims, such as the health checks, the calendar, the panels and the login page, so most custom CSS still works (D-110). The themes of the marketplace are rebuilt (D-92).
- **Custom HTML items** run in a closed frame, unless you switch the item to trusted (D-61, D-76). Their links cannot open the pop-ups of the page.
- **Plugins of other authors** are WebAssembly extensions, not PHP files (D-75).
- **Get Plex Token** uses the login page of Plex instead of a form for the Plex password (D-107).
- **The search for new requests** asks Overseerr or Ombi on the server, not the TMDB from the browser, and has no lists of suggestions (D-108).
- **Languages**: the 32 language files of PHP translate the interface. Texts that only this version has stay English (D-109).
- **Secrets** are sealed in the database, and the API shows them masked.
- The page has a strict security policy: it loads nothing from other sites. So the About page shows no news of Organizr.

## Not in this release

- Moving homepage items by dragging: Homepage Order moves them with buttons.
- Links in custom HTML items that open the pop-ups of the page, such as `#speedtest-area` or `#new-request`.

## Checks

Each image passes the checks in [Releases](releases.md) before it is published.
