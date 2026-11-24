# Organizr in Rust

This project is a new version of [Organizr](https://github.com/causefx/Organizr), written in Rust. Organizr is a start page for the web applications of a home server. It shows each application as a tab, and it controls who can open which tab.

The project keeps the API of Organizr 2.1.5000, so that scripts and reverse proxies continue to work. [Deviations](docs/deviations.md) lists each place where an answer is different, and the reason.

## State of the project

The project is not released yet. The image with the tag `develop` holds the newest state.

These parts work:

- Users, groups, tabs, categories, settings and the set-up wizard
- Login with a password, with Plex, with a second factor, and password recovery by e-mail
- Forward auth for a reverse proxy, and single sign-on to Plex, Jellyfin, Ombi, Overseerr, Petio, Tautulli and Komga
- The import of an install of PHP Organizr
- The homepage with Plex, Tautulli, Overseerr, Sonarr, Radarr, Lidarr, SABnzbd, qBittorrent, NZBGet, Transmission, Pi-hole, Healthchecks, Uptime Kuma, weather and custom HTML
- The plugins for bookmarks, health checks, invites, e-mail, Plex Libraries and the speed test
- Backup and restore, also on a schedule
- Extensions of other authors, which run in a sandbox and reach only the sites that you approve

Where this application works differently from PHP Organizr, and the few parts of PHP that it leaves out on purpose, are listed in [Deviations](docs/deviations.md).

## Start it

You need Docker. The image is for the architectures `amd64` and `arm64`.

1. Copy the file [docker/examples/compose.sqlite.yaml](docker/examples/compose.sqlite.yaml) to your server.
2. Run this command in the folder of the file:

   ```sh
   docker compose -f compose.sqlite.yaml up -d
   ```

3. Open `http://<your server>:8080` in a browser.
4. Read the set-up code from the log of the container. The wizard asks for it:

   ```sh
   docker compose -f compose.sqlite.yaml logs organizr
   ```

5. Complete the wizard.

Organizr stores its data in the volume `/config`. The container runs as the user with the UID 65532, and not as root. [Installation](docs/installation.md) tells you more: `docker run`, the data folder, updates, and the settings of the container.

To use PostgreSQL in place of SQLite, start from [docker/examples/compose.postgres.yaml](docker/examples/compose.postgres.yaml). [Choosing a database](docs/choosing-a-database.md) tells you which one fits your install.

## Move from PHP Organizr

[Moving from PHP Organizr](docs/moving-from-php.md) is a checklist for the move. [Importing](docs/importing.md) tells you how to bring the users, tabs, settings and pictures of your PHP install into a new install. The import does not change the old install.

Custom CSS written for PHP Organizr needs changes, because the pages use other ids and classes. [Custom CSS](docs/custom-css.md) lists the names to use, with a table that turns the names of PHP into them.

## Settings of the container

Most settings are in the web interface. A few are environment variables, because Organizr needs them before it can read its database: the data folder, the database, the port, the trusted reverse proxies, the marketplace for extensions, the key for the stored secrets, and custom JavaScript. [Installation](docs/installation.md#settings-of-the-container) lists them.

Behind a reverse proxy, set `ORGANIZR_TRUSTED_PROXIES`. [Reverse proxy](docs/reverse-proxy.md) tells you why, with examples for nginx, Traefik and Caddy.

## Documentation

| Subject | Page |
| --- | --- |
| Start and update | [Installation](docs/installation.md), [Troubleshooting](docs/troubleshooting.md) |
| Database | [Choosing a database](docs/choosing-a-database.md) |
| Move from PHP Organizr | [Moving from PHP Organizr](docs/moving-from-php.md), [Importing](docs/importing.md) |
| Stored secrets and their key | [Secrets](docs/secrets.md) |
| Backup and restore | [Backups](docs/backups.md) |
| Login | [Plex login](docs/plex-login.md), [LDAP, FTP, Emby and Jellyfin login](docs/login-services.md), [OpenID Connect login](docs/openid-connect.md), [Second factor](docs/second-factor.md), [Single sign-on](docs/single-sign-on.md), [Lock screen](docs/lock-screen.md) |
| Reverse proxy | [Reverse proxy](docs/reverse-proxy.md), [Forward auth](docs/forward-auth.md), [API SOCKS](docs/socks.md) |
| E-mail | [E-mail](docs/email.md) |
| Look of the pages | [Appearance](docs/appearance.md), [Custom CSS](docs/custom-css.md) |
| Homepage | [Homepage](docs/homepage.md), [Checking your services](docs/checking-your-services.md), [Tests of the settings](docs/settings-tests.md) |
| Plugins | [Bookmarks](docs/bookmark-plugin.md), [Health checks](docs/health-checks-plugin.md), [Invites](docs/invites-plugin.md), [Speed test](docs/speed-test-plugin.md) |
| Extensions of other authors | [Extensions](docs/extensions.md) |
| Scheduled work | [Scheduled jobs](docs/scheduled-jobs.md) |
| The log of the server | [The log](docs/log.md) |
| Images and their checks | [Releases](docs/releases.md) |
| Design | [Architecture](docs/architecture.md), [Audit of PHP Organizr](docs/audit.md), [Deviations](docs/deviations.md) |

## License

The project has the license GPL-3.0, as Organizr has. [Third-party assets](docs/third-party-assets.md) lists the pictures and icons of other projects that the image holds.
