# 🩺 Checking your services

[← All documentation](README.md)

The `organizr check` command asks your services the same questions Organizr asks them, and prints for each one whether the answer was usable. Run it after you set up [the login through Plex](plex-login.md), [single sign-on](single-sign-on.md) or a [homepage](homepage.md) item, or when one of them stops working.

## 🚀 Run the check

With Docker Compose, run it in the server's container (it already knows the data folder and the database):

```bash
docker compose exec organizr /usr/local/bin/organizr check
```

If your Compose file isn't called `compose.yaml`, add `-f <your-file>` after `docker compose`. With `docker run`, use `docker exec <container> /usr/local/bin/organizr check`, where `<container>` is the name you gave it (for example `organizr`).

Without Docker, pass the same data folder, database and secret key options that you use for `organizr serve`:

```bash
organizr check --data-dir /config
```

The server can keep running during the check.

## 🔍 What it checks

The command reads your settings: the Plex token of the owner, the id of the Plex server, and the addresses of your applications.

**1. The login.** It asks plex.tv about the token, then logs the owner of the Plex token in to Ombi, Overseerr and Tautulli, the same way a login through Plex does.

**2. The homepage.** It asks each homepage application that has an address in the settings:

| Group | Applications |
|---|---|
| Media servers | Plex, Emby, Jellyfin, Tautulli and JellyStat; Emby live TV, when its item is on |
| Requests | Overseerr and Ombi |
| Calendars | Sonarr, Radarr, Lidarr, SickRage, CouchPotato and Trakt |
| Download clients | SABnzbd, qBittorrent, Deluge, rTorrent, uTorrent and JDownloader |
| Indexers | Jackett and Prowlarr |
| Monitors | Healthchecks, Uptime Kuma, AdGuard Home, Netdata, Monitorr, Speedtest Tracker, Netronome, OctoPrint and UniFi |
| Other | The weather service |

NZBGet, Transmission, Pi-hole and PromPage aren't checked yet. Use their **Test Connection** button instead.

Good to know:

- Each question is the one the homepage item asks. The command only reads.
- It changes no setting and stores nothing. Each application just sees one more login of the owner.
- An application without an address gets no line in the report.
- Plex, Emby, Jellyfin, Overseerr and Ombi also need their key, because their address also serves a login.
- Trakt needs the Trakt login in the settings. When the Trakt access token has expired, the command skips Trakt: the homepage asks Trakt for new tokens, and the command stores nothing.
- The report holds no token, no password, no address and no account name.

## 📋 The report

Each line starts with the result of one question:

| Result | Meaning |
|---|---|
| `ok` | The service gave an answer Organizr can use |
| `FAILED` | The service gave no usable answer. The line says what went wrong |
| `skipped` | The question wasn't asked. The line says why |

The login lines come first:

```text
ok       plex.tv knows the stored Plex token
ok       plex.tv lists the Plex servers, and the stored id is one of them
ok       plex.tv gives the list of the accounts that the server is shared with
ok       Ombi gives a session for the Plex token
FAILED   Overseerr gives a session for the Plex token: no session
skipped  Tautulli gives a session for the Plex token: the settings hold no address
```

Then the homepage lines:

```text
ok       The Plex server lists what plays now
ok       Tautulli gives its statistics
ok       Sonarr install 1 of 1 gives its calendar
FAILED   SABnzbd gives its queue: the answer has no queue; look at the API key
ok       qBittorrent takes the login and gives its torrents
```

- If any question fails, the command ends with an error, so a script can rely on it.
- When plex.tv doesn't know the token, the command stops there: the other questions need the token's account.
- The questions about a fallback user are asked only when the settings hold a fallback user and its password.

## 🔧 When a question fails

The command writes the reason to the log, above the report. The log names the address of the application that didn't answer. For more detail, run the command with the environment variable `ORGANIZR_LOG=debug`:

```bash
docker compose exec -e ORGANIZR_LOG=debug organizr /usr/local/bin/organizr check
```

| Line | What to look at |
|---|---|
| plex.tv knows the stored Plex token | The setting `plexToken`. A Plex token stops working when the owner signs out of all devices |
| The stored id is one of them | The setting `plexID`. The button **Get Plex Machine**, on **Settings › System Settings › Sign-in** or **SSO**, lists the servers with their ids |
| An application gives a session for the Plex token | The address of the application, and whether it lets Plex accounts log in. The owner must be able to log in to the application with Plex |
| An application gives a session for the fallback user | The name and the password of the fallback user. For Overseerr, the name is an e-mail address |
| An application of the homepage gives its data | The address and the key of the application in the settings of [the homepage](homepage.md). For qBittorrent, the user name, the password and the API version. For Deluge, rTorrent, uTorrent, JDownloader and UniFi, the user name and the password |
| AdGuard Home gives its statistics | The homepage asks AdGuard Home without a login, so AdGuard Home must answer without one |
| Trakt gives the calendar of the account | The Trakt login. Log in again in the settings of [the homepage](homepage.md) |

> [!TIP]
> An address must start with `http://` or `https://`. The command skips an address without that start, because only a browser knows the host of such an address.
