# Checking your services

The `organizr check` command asks the services of your settings the questions that Organizr asks them: plex.tv, Ombi, Overseerr and Tautulli for a login, and the applications of the homepage for their items. It prints for each question whether the answer was usable.

Use the command after you set up [the login through Plex](plex-login.md), [single sign-on](single-sign-on.md) or an item of [the homepage](homepage.md), or when one of them stops working.

## What the command does

The command reads the settings of the install: the Plex token of the owner, the id of the Plex server, and the addresses of the applications. Then it logs the owner of the Plex token in to each application, in the same way as a login through Plex does.

The command changes no setting and stores nothing. Each application sees one more login of the owner.

Then the command asks each application of the homepage that has an address in the settings:

- The media servers: Plex, Emby, Jellyfin, Tautulli and JellyStat. The live television of Emby, when its item is on.
- The requests: Overseerr and Ombi.
- The calendars: Sonarr, Radarr, Lidarr, SickRage, CouchPotato and Trakt.
- The download clients: SABnzbd, qBittorrent, Deluge, rTorrent, uTorrent and JDownloader.
- The indexers: Jackett and Prowlarr.
- The monitors: Healthchecks, Uptime Kuma, AdGuard Home, Netdata, Monitorr, Speedtest Tracker, OctoPrint and UniFi.
- The weather service.

Each question is the one that the homepage item asks. The command only reads. An application without an address has no line in the report. Plex, Emby, Jellyfin, Overseerr and Ombi also need their key, because their address also serves a login.

Trakt needs the login at Trakt in the settings. When the access token of Trakt is over, the command skips Trakt: the homepage asks Trakt for new tokens, and the command stores nothing.

The report holds no token, no password, no address and no name of an account.

## Run the check

Use the same data folder, database and secret key options that you use for `organizr serve`.

```sh
organizr check --data-dir /config
```

With the container image, run the command in the container of the server. The container already knows the data folder. Replace `organizr` with the name of your container.

```sh
docker exec organizr /usr/local/bin/organizr check
```

The server can stay in operation during the check.

## The report

Each line starts with the result of one question.

| Result | Meaning |
|---|---|
| `ok` | The service gave an answer that Organizr can use. |
| `FAILED` | The service gave no usable answer. The line says what went wrong. |
| `skipped` | The question was not asked. The line says why. |

```text
ok       plex.tv knows the stored Plex token
ok       plex.tv lists the Plex servers, and the stored id is one of them
ok       plex.tv gives the list of the accounts that the server is shared with
ok       Ombi gives a session for the Plex token
FAILED   Overseerr gives a session for the Plex token: no session
skipped  Tautulli gives a session for the Plex token: the settings hold no address
```

If a question fails, the command ends with an error. A script can thus use the command.

When plex.tv does not know the token, the command stops there. The other questions need the account of the token.

The questions about a fallback user are asked only when the settings hold a fallback user and its password.

The lines for the homepage come after the lines for the login:

```text
ok       The Plex server lists what plays now
ok       Tautulli gives its statistics
ok       Sonarr install 1 of 1 gives its calendar
FAILED   SABnzbd gives its queue: the answer has no queue; look at the API key
ok       qBittorrent takes the login and gives its torrents
```

## When a question fails

The command writes the reason to the log, above the report. The log names the address of the application that did not answer. For more detail, set the environment variable `ORGANIZR_LOG` to `debug`.

| Line | What to look at |
|---|---|
| plex.tv knows the stored Plex token | The setting `plexToken`. A Plex token stops working when the owner signs out of all devices. |
| The stored id is one of them | The setting `plexID`. The tab **Login** of the settings lists the servers with their ids. |
| An application gives a session for the Plex token | The address of the application, and whether the application lets Plex accounts log in. The owner must be able to log in to the application with Plex. |
| An application gives a session for the fallback user | The name and the password of the fallback user. For Overseerr, the name is an e-mail address. |
| An application of the homepage gives its data | The address and the key of the application in the settings of [the homepage](homepage.md). For qBittorrent, the user name, the password and the version of the API. For Deluge, rTorrent, uTorrent, JDownloader and UniFi, the user name and the password. |
| AdGuard Home gives its statistics | The homepage asks AdGuard Home without a login. AdGuard Home must answer without one. |
| Trakt gives the calendar of the account | The login at Trakt. Log in again in the settings of [the homepage](homepage.md). |

An address must start with `http://` or `https://`. The command skips an address without that start, because only a browser knows the host of such an address.
