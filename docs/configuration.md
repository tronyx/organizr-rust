# 🔧 Configuration

[← All documentation](README.md)

This is the one reference page for everything you set outside the web interface: the environment variables, the commands of the `organizr` program, the ports, the volume, the health check and the image tags. Other pages link here instead of repeating these tables.

Most settings live in the web interface, under **Settings**. The ones on this page are needed before Organizr can open its database, so they can't live in it.

## 🌍 Environment variables

Each variable is also an option of `organizr serve`. The option wins when you give both. Run `docker compose run --rm organizr serve --help` to see them all.

| Variable | Option | Default | What it does |
| --- | --- | --- | --- |
| `ORGANIZR_DATA_DIR` | `--data-dir` | `/config` in the image, `data` outside it | The data folder. See [The data folder](#-the-data-folder) |
| `ORGANIZR_DATABASE_URL` | `--database-url` | The SQLite file `organizr.db` in the data folder | The database: `sqlite:<file>` or `postgres://<user>:<password>@<host>:<port>/<database>` (`postgresql://` works too). See [Choosing a database](choosing-a-database.md) |
| `ORGANIZR_LISTEN` | `--listen` | `0.0.0.0:8080` | The address and port the server listens on. The health check reads it too |
| `ORGANIZR_TRUSTED_PROXIES` | `--trusted-proxies` | Empty: no proxy is trusted | Your reverse proxies, separated by commas. See [Reverse proxy](reverse-proxy.md) |
| `ORGANIZR_ALLOWED_HOSTS` | `--allowed-hosts` | Empty: no host | The host names you reach Organizr at, separated by commas, with a port where it matters, for example `organizr.example.com,192.168.1.10:8080`. A service address that is only a path, such as `/sonarr`, then works on those hosts. See [Homepage](homepage.md#-use-an-address-organizr-can-reach) |
| `ORGANIZR_EXTENSION_MARKETPLACE` | `--extension-marketplace` | The project's list in the image, empty outside it | The address of a marketplace list for extensions (a JSON file). An empty value switches the marketplace off; you can still install extensions by hand. See [Extensions](extensions.md) |
| `ORGANIZR_SECRET_KEY` | `--secret-key` | Not set | The key for the stored secrets, as 64 hex digits. When set, Organizr doesn't read or write a key file. See [Secrets](secrets.md) |
| `ORGANIZR_SECRET_KEY_FILE` | `--secret-key-file` | `secret.key` in the data folder | The file that holds the key. Organizr makes it on the first start when it's missing |
| `ORGANIZR_CUSTOM_JS` | `--custom-js` | Off | Runs the Admin's **Custom Javascript** and **Theme Javascript** on every page. Takes `1`, `true`, `yes` or `on` (and `0`, `false`, `no`, `off`). See [Appearance](appearance.md) |
| `ORGANIZR_LOG` | None | `info` | The level of the log on the standard output (`docker compose logs`), for example `debug` or `organizr_api=debug,info`. The log file follows the **Logs** settings instead. See [The log](log.md) |
| `TZ` | None | UTC | The time zone for the schedules, for example `America/New_York`. See [Scheduled jobs](scheduled-jobs.md) |

A wrong value stops the server at start with a clear message, for example `the list of trusted proxies is not valid` or `ORGANIZR_SECRET_KEY is not a key`.

### 🔒 Security notes

Some of these variables are sensitive. Keep these in mind:

| Variable | Watch out for |
| --- | --- |
| `ORGANIZR_TRUSTED_PROXIES` | Every address in the list can claim to be any client. Trust only your proxy, as narrowly as you can. Never a whole Docker range (`172.16.0.0/12`) or your home network while port 8080 is published |
| `ORGANIZR_ALLOWED_HOSTS` | Organizr sends the keys of your services to a path-only address on the host of the request when that host is listed. List only names that lead to your own Organizr, never a wildcard you don't control |
| `ORGANIZR_SECRET_KEY` | Anyone who can run `docker inspect` can read it, and it ends up in compose files and shell history. Prefer `ORGANIZR_SECRET_KEY_FILE` with a Docker secret (see [Secrets](secrets.md)) |
| `ORGANIZR_DATABASE_URL` | Holds the database password. Keep the compose file (or the `.env` file that holds it) readable only by you: `chmod 600 .env` |
| `ORGANIZR_EXTENSION_MARKETPLACE` | Decides whose list of extensions your Admins are offered. Point it only at a list you trust |
| `ORGANIZR_CUSTOM_JS` | The script runs in the session of every person who opens Organizr, guests on the login page too. A script that the Admin stores can read what every user sees |

### 🧪 For contributors

These two are read only by tests. You never need them to run Organizr.

| Variable | What it does |
| --- | --- |
| `ORGANIZR_TEST_POSTGRES_URL` | The URL of a PostgreSQL server the tests may use. Without it, the PostgreSQL tests are skipped |
| `ORGANIZR_REAL_SETTINGS` | Settings as a JSON object, for a test that asks real applications. Without it, that test does nothing |

## 💻 Commands

The image runs `organizr serve` by default. The other commands run in a one-off container from the same image, or in the running container. Run these in the folder that holds your `compose.yaml`:

| Command | What it does | How to run it |
| --- | --- | --- |
| `serve` | Runs the server | `docker compose up -d` |
| `healthcheck` | Asks the running server whether it answers. Exits with an error when it doesn't | `docker compose exec organizr organizr healthcheck` |
| `import` | Moves an install of PHP Organizr into a new, empty install. See [Importing](importing.md) | `docker compose run --rm -v /path/to/old:/old:ro organizr import --from /old --dry-run` |
| `move-to-postgres` | Copies the SQLite database into an empty PostgreSQL database. Stop the server first. See [Choosing a database](choosing-a-database.md#-move-from-sqlite-to-postgresql) | `docker compose run --rm organizr move-to-postgres --dry-run` |
| `check` | Asks plex.tv, Ombi, Overseerr, Tautulli and the homepage services what Organizr asks them, with your stored settings. See [Checking your services](checking-your-services.md) | `docker compose exec organizr organizr check` |
| `restore` | Puts the files and the database of a backup back. Stop the server first. See [Backups](backups.md) | `docker compose run --rm organizr restore --from "/config/backups/<file>.zip" --overwrite` |
| `reset-second-factor` | Removes the second factor of a user who lost the authenticator app. See [Second factor](second-factor.md#-lost-your-phone) | `docker compose exec organizr organizr reset-second-factor --user "<user name>"` |

`organizr --version` prints the version, and `organizr <command> --help` lists the options of a command.

> [!TIP]
> `docker compose run --rm organizr ...` starts a new container with the same image, volumes and environment as your server, and removes it when it's done. `docker compose exec organizr ...` runs the command inside the server that's already running. Did you start Organizr with `docker run --name organizr` instead? Then use `docker exec organizr organizr <command>` and `docker run --rm -v organizr-data:/config ghcr.io/tronyx/organizr-rust:develop <command>`.

### 📑 Options of each command

`serve`, `import`, `move-to-postgres`, `check`, `restore` and `reset-second-factor` share the storage options: `--data-dir`, `--database-url`, `--secret-key` and `--secret-key-file`. They read the same environment variables as `serve`, so inside the container you rarely need them.

| Command | Option | What it does |
| --- | --- | --- |
| `serve` | `--listen`, `--trusted-proxies`, `--extension-marketplace`, `--custom-js` | The same as the variables above |
| `healthcheck` | `--listen` | Where the server listens. Reads `ORGANIZR_LISTEN`, so it needs nothing of its own |
| `import` | `--from <folder>` | The folder of the PHP install, or the `/config` volume of the container that held it. Only read |
| `import` | `--database <file>` | The SQLite file of the PHP install, when the import doesn't find it by itself |
| `import` | `--dry-run` | Reads the old install, prints the report, and writes nothing |
| `move-to-postgres` | `--from <file>` | The SQLite file to copy. Default: `organizr.db` in the data folder |
| `move-to-postgres` | `--dry-run` | Prints what would be copied, and writes nothing |
| `restore` | `--from <file>` | The backup zip file |
| `restore` | `--overwrite` | Replaces the database when one exists. Without it, the command stops and changes nothing |
| `reset-second-factor` | `--user <name>` | The user whose second factor is removed, in any letter case |

## 🚪 Ports

| Port | What it is |
| --- | --- |
| `8080` | The web interface and the API, in plain HTTP. Change it with `ORGANIZR_LISTEN`, or map another host port in Compose (`"9090:8080"`) |

> [!WARNING]
> Port 8080 speaks plain HTTP: passwords, session cookies and the API key travel unencrypted. Never forward it from your router. To reach Organizr from outside, put a [reverse proxy](reverse-proxy.md) with HTTPS in front, then remove the `ports:` lines (or publish only to the host: `"127.0.0.1:8080:8080"`). Docker opens published ports past firewalls such as ufw and firewalld.

## 📁 The data folder

The image keeps everything in the volume `/config`. Compose names the volume `<project>_organizr-data`, where `<project>` is the name of the folder that holds your `compose.yaml`.

| Path in `/config` | What it holds |
| --- | --- |
| `organizr.db` | The SQLite database (not there with PostgreSQL) |
| `secret.key` | The key for the stored secrets. Back it up somewhere other than your backups. See [Secrets](secrets.md) |
| `userTabs/` | The pictures you upload |
| `favicon/` | Your custom favicon files |
| `cache/` | Pictures of other applications that Organizr fetched. Safe to lose |
| `backups/` | The backups, unless you set another **Backup Save Path** |
| `logs/` | The log files, one per day. See [The log](log.md) |

The container runs as the user and group `65532`, not as root. A named volume needs nothing. A folder of the host must belong to that user: `sudo chown -R 65532:65532 /path/on/the/host`. On Kubernetes, set `runAsUser: 65532` and `fsGroup: 65532`.

> [!CAUTION]
> `docker compose down -v` deletes the volume, and with it your install. Plain `docker compose down` keeps it.

## 🩺 The health check

The image checks itself: it has no shell and no `curl`, so the `organizr healthcheck` command asks the server for `GET /api/v2/ping` and expects status `200`.

| Setting | Value |
| --- | --- |
| Interval | Every 30 seconds |
| Time limit | 5 seconds |
| Start period | 30 seconds, with a check every 2 seconds, so the container turns `healthy` soon after it starts |
| Retries | 3, then the container is `unhealthy` |

For Kubernetes or another orchestrator:

- **Liveness**: `GET /api/v2/ping` answers with status `200` without touching the database.
- **Readiness**: `GET /api/v2/status` reads the database, and its answer names the engine in the field `database` (`sqlite` or `postgresql`).
- **Stop**: the server stops cleanly on `SIGTERM` (what `docker stop` sends).

## 🔖 Image tags and architectures

The images are at `ghcr.io/tronyx/organizr-rust`. Each tag holds `linux/amd64` and `linux/arm64`. There is no 32-bit ARM (armv7) image.

| Tag | What it is |
| --- | --- |
| `develop` | The newest state of the `develop` branch. The project has no release yet, so this is the one to use. It can change at any time |
| `latest` | The newest release, once there is one |
| A version, for example `1.0.0` | One release. The tag never moves |

To stay on one exact build, pin its digest: `ghcr.io/tronyx/organizr-rust@sha256:<digest>`. [Installation](installation.md#-update-and-roll-back) shows how to find it and how to roll back. [Releases](releases.md) tells you how the images are built and checked.

## 📏 Resources

- **One replica only.** The scheduler runs inside the server, so two copies would run every job twice, and two servers must never share one SQLite file.
- **Memory and CPU**: Organizr is one process with no PHP, web server or cron beside it. The project hasn't published measured figures yet. Extensions (WebAssembly) and a large picture cache add to the memory and disk use.
- **Disk**: the data folder grows with the uploaded pictures, the picture cache, the backups and the log files. Set how many backups and log files to keep in **Settings › System Settings › Backup** and **Settings › System Settings › Logs**.
- **SQLite on a network share** (NFS, SMB) is a common source of `database is locked` errors. Keep the data folder on a local disk, or use PostgreSQL.
