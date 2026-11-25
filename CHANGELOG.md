# 📜 Changelog

The notable changes to Organizr Rust, newest first.

There is no release yet. Each release will get an entry here, dated, with its version and its pull request into `main`. Changes waiting on `develop`, which the image `ghcr.io/tronyx/organizr-rust:develop` holds, are listed under **Unreleased**.

## Unreleased

Organizr Rust is a rewrite of PHP Organizr 2.1.5000 in Rust and Svelte. It keeps the API, the settings and the look of PHP Organizr, and an import moves a PHP install over. [Release notes](docs/release-notes.md) say what works as in PHP and what you'll notice is different, and [the deviations](docs/deviations.md) list every difference with its reason.

### Added

- Everything of PHP Organizr 2.1.5000: tabs and categories, users and groups, the homepage items, the settings pages, single sign-on, the login services (Plex, LDAP, FTP, Emby, Jellyfin), OpenID Connect, the second factor, and the plugins (invites, e-mail, speed test, health checks, bookmarks and chat).
- `organizr import` moves a PHP install: the settings, users, groups, tabs, categories, homepage, pictures and custom certificate. See [Moving from PHP Organizr](docs/moving-from-php.md).
- SQLite or PostgreSQL, chosen with `ORGANIZR_DATABASE_URL`, and `organizr move-to-postgres` to move an install from SQLite to PostgreSQL. See [Choosing a database](docs/choosing-a-database.md).
- The modern layout, with brand themes (Plex, Jellyfin, Emby, Silo) and scene themes. Each person picks a layout, theme and color set. See [Appearance](docs/appearance.md).
- The Plex Libraries plugin shares and unshares the libraries of your Plex server (D-121).
- The Netronome homepage item shows the last speed test of a Netronome install (D-128).
- An Admin or a Co-Admin removes the second factor of a user who lost the authenticator app, in **Manage Users** or with `organizr reset-second-factor --user <name>` (D-129). See [Lost your phone?](docs/second-factor.md#-lost-your-phone).
- `organizr check` asks your services what a login and the homepage ask them, all at once. See [Checking your services](docs/checking-your-services.md).
- `organizr restore` puts a backup back. See [Backups](docs/backups.md).
- Plugins by other authors are WebAssembly extensions, run in a sandbox. See [Extensions](docs/extensions.md).

### Changed

- One container, with no PHP, web server or cron. Updates mean pulling a new image.
- The set-up wizard asks for a set-up code from the log, so nobody else can take over a new install.
- The pages are new, so custom CSS written for PHP needs the names in [Custom CSS](docs/custom-css.md). Custom JavaScript runs only when the server starts with `ORGANIZR_CUSTOM_JS=1`.
- The settings pages group the settings by topic. The Auth proxy section of PHP isn't shown, because this version logs nobody in from a header; [Forward auth](docs/forward-auth.md) and [OpenID Connect](docs/openid-connect.md) do that job (D-96).
- Dates show as PHP shows them, in the local time of the browser: "Nov 18, 2026 1:02 PM" in Manage Users, and "November 18, 2026 1:02 PM" for sessions, chat messages and donations.
- Each service of the health checks plugin has an **API key** switch. Only a service with the switch on gets Organizr's API key; services from before the switch keep getting it, and new or imported ones start with it off (D-130).

### Security

- Secrets (passwords, tokens and API keys of your services) are stored encrypted with a key of the install, and the API shows them masked. See [Secrets](docs/secrets.md).
- A login through Plex or Emby Connect never opens an Admin or Co-Admin account, except the Plex owner's, and no outside service replaces an Admin or Co-Admin password (D-127).
- Proxy headers are believed only from the proxies in `ORGANIZR_TRUSTED_PROXIES`, and wrong passwords lock the client out for a while.
- Custom HTML items run in a closed frame unless you trust them, and the About page loads nothing from other sites.
- Only an Admin trusts a custom HTML item or changes the HTML of a trusted one (D-131).
- Only an Admin changes the sign-in settings, the Plex owner and the logins that skip the second factor. While a login uses the Emby or Jellyfin server, only an Admin changes its address (D-132).
- Only an Admin changes the OpenID Connect settings, which pick the providers and the group a login gives (D-133).
- Only an Admin changes the services of the health checks plugin, which can get the API key (D-134).
- The weekly scan of the published images files the vulnerabilities that have a fix in the Security tab, once the repository is public. See [Releases and images](docs/releases.md#-the-weekly-scan).

### Documentation

- The docs are rewritten to be easier to follow, with screenshots, a [security policy](SECURITY.md) and a [contributing guide](CONTRIBUTING.md).
- The reverse proxy and forward auth steps for Caddy, SWAG and Nginx Proxy Manager are tested. In NPM, the lines that pass the user to an application belong in a custom location for `/`, or a forged `X-Organizr-User` reaches the application. See [Forward auth](docs/forward-auth.md#-passing-the-user-to-the-application).
- [Settings that could be removed](docs/unused-settings.md) lists the 82 settings from PHP that this version stores but never reads.
