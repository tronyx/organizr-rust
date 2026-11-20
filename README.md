# Organizr in Rust

This project is a new version of [Organizr](https://github.com/causefx/Organizr), written in Rust. Organizr is a start page for the web applications of a home server. It shows each application as a tab, and it controls who can open which tab.

The project keeps the API of Organizr 2.1.5000, so that scripts and reverse proxies continue to work. [Deviations](docs/deviations.md) lists each place where an answer is different, and the reason.

## State of the project

The project is not released yet. The image with the tag `develop` holds the newest state.

These parts work:

- Users, groups, tabs, categories, settings and the set-up wizard
- Login with a password, with Plex, with a second factor, and password recovery by e-mail
- Forward auth for a reverse proxy, and single sign-on to Plex, Ombi, Overseerr and Tautulli
- The import of an install of PHP Organizr
- The homepage with Plex, Tautulli, Overseerr, Sonarr, Radarr, Lidarr, SABnzbd, qBittorrent, NZBGet, Transmission, Pi-hole, Healthchecks, Uptime Kuma, weather and custom HTML
- The plugins for bookmarks, health checks, invites, e-mail and the speed test

Parts of PHP Organizr that are not built yet are named in [Architecture](docs/architecture.md), section 10.

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

Organizr stores its data in the volume `/config`. The container runs as the user with the UID 65532, and not as root.

To use PostgreSQL in place of SQLite, start from [docker/examples/compose.postgres.yaml](docker/examples/compose.postgres.yaml). [Choosing a database](docs/choosing-a-database.md) tells you which one fits your install.

## Move from PHP Organizr

[Importing](docs/importing.md) tells you how to bring the users, tabs, settings and pictures of your PHP install into a new install. The import does not change the old install.

## Settings of the container

| Variable | Meaning | Default |
|---|---|---|
| `ORGANIZR_DATA_DIR` | The data folder | `/config` in the image |
| `ORGANIZR_DATABASE_URL` | The database. See [Choosing a database](docs/choosing-a-database.md) | SQLite in the data folder |
| `ORGANIZR_LISTEN` | The address and the port that the server listens on | Port 8080 on all addresses |
| `ORGANIZR_TRUSTED_PROXIES` | The addresses of your reverse proxies. See [Forward auth](docs/forward-auth.md) | None |
| `ORGANIZR_SECRET_KEY`, `ORGANIZR_SECRET_KEY_FILE` | The key that seals the stored secrets. See [Secrets](docs/secrets.md) | A key file in the data folder |

All other settings are in the web interface.

## Documentation

| Subject | Page |
|---|---|
| Database | [Choosing a database](docs/choosing-a-database.md) |
| Import from PHP Organizr | [Importing](docs/importing.md) |
| Stored secrets and their key | [Secrets](docs/secrets.md) |
| Login | [Plex login](docs/plex-login.md), [Second factor](docs/second-factor.md), [Single sign-on](docs/single-sign-on.md), [Lock screen](docs/lock-screen.md) |
| Reverse proxy | [Forward auth](docs/forward-auth.md) |
| E-mail | [E-mail](docs/email.md) |
| Homepage | [Homepage](docs/homepage.md), [Checking your services](docs/checking-your-services.md) |
| Plugins | [Bookmarks](docs/bookmark-plugin.md), [Health checks](docs/health-checks-plugin.md), [Invites](docs/invites-plugin.md), [Speed test](docs/speed-test-plugin.md) |
| Scheduled work | [Scheduled jobs](docs/scheduled-jobs.md) |
| Images and their checks | [Releases](docs/releases.md) |
| Design | [Architecture](docs/architecture.md), [Audit of PHP Organizr](docs/audit.md), [Deviations](docs/deviations.md) |

## Licence

The project has the licence GPL-3.0, as Organizr has. [Third-party assets](docs/third-party-assets.md) lists the pictures and icons of other projects that the image holds.
