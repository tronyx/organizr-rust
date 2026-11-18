# Organizr → Rust: Phase 1 audit and specification

Source audited: a clone of `causefx/Organizr`, branch `v2-develop`, commit `c095c115` (2024-05-29), Organizr version `2.1.2490`.

## 0. Coverage of this audit

| Area | Depth |
|---|---|
| Slim bootstrap, response envelope, auth/permission model, CSRF key | Read in full |
| Login, tokens, 2FA, auth backends, SSO cookie flow, forward-auth (`/auth`) | Read in full |
| Users, groups, tabs, categories, config, wizard, DB schema, backups, images, SOCKS proxy | Read in full |
| All route files (core + 7 bundled plugins): method, path, permission level, handler | Read in full |
| 38 homepage integrations (`api/homepage/*.php`, 9.4k lines) | Route level + shared permission pattern; one file (Pi-hole) read in full. Per-integration upstream calls and response shapes are **not yet audited** |
| HTML page builders (`api/pages/*.php`, 3.5k lines), settings option builders (`option-functions.php`, 1.4k lines) | Inventory only |
| Plugin internals (`plugin.php` of bookmark, chat, invites, php-mailer, healthchecks, shuck-stop, speedtest) | Structure only |
| Logging, self-update, version upgrade migrations | Function inventory + entry points |
| Front-end JavaScript (`js/functions.js`, 12.5k lines) | Size and API-call counts only |

Anything marked "not yet audited" gets its own detailed pass when its slice comes up in Phase 3.

## 1. Application shape

- About 35,000 lines of first-party PHP under `api/`. Slim 4 front controller at `api/v2/index.php`.
- One class, `Organizr` (`api/classes/organizr.class.php`, 7,636 lines), built from 20 function traits and 38 homepage-item traits. Plugins are classes that **extend** `Organizr`.
- A new `Organizr` object is built for every request. Its constructor: loads config from a PHP file, checks disk space (dies with HTML below 100 MB free), sets a device-UUID cookie, connects to the DB, resolves the user from the cookie, validates the token, runs the version upgrade check, runs the auth-proxy pre-check, and applies the IP blacklist (dies with HTML).
- The response is global mutable state: `$GLOBALS['api']['response'] = {result:'success', message:null, data:null}` and `$GLOBALS['responseCode'] = 200`. Handlers and helpers change it as a side effect; the route closure serialises whatever is there at the end.
- Dynamic PHP loading at runtime: `data/routes/*.php`, `data/pages/*.php`, `api/plugins/**` and `data/plugins/**` (`plugin.php`, `api.php`, `config.php`, `cron.php`, `advancedCron.php`, `*page.php`). A plugin and theme marketplace downloads PHP/CSS from GitHub into `data/`.
- Pages are HTML strings built in PHP (with inline `<script>`), returned inside the JSON envelope by `GET /page/{page}` or as raw HTML by `GET /organizr/{page}`. `index.php` is the single-page shell; it injects config values, plugin CSS/JS tags, language packs and the form key.
- Front end: jQuery, about 24,000 lines of first-party JS. `functions.js` makes about 128 `organizrAPI2()` calls (62 GET, 29 POST, 26 PUT, 11 DELETE).
- `cron.php`: called every minute by system cron, local/server callers only. Schedules auto-update, auto-backup, plugin cron jobs (`$GLOBALS['cron']`) and `advancedCron.php` files, then writes a timestamp file.
- Composer dependencies and what they do: dibi (DB layer), lcobucci/jwt, composer/semver, phpmailer, rmccue/requests (all outbound HTTP), kryptonit3 sonarr/couchpotato/sickrage clients, pusher (chat plugin), pragmarx/google2fa (TOTP), adldap2 (LDAP), swagger-php, oauth2-trakt, php-html-parser (scrape), php-json-logger, line-reader (log paging), php-cron-scheduler, simshaun/recurr (iCal recurrence), stripe (donate item), ramsey/uuid.

### Response envelope (the wire contract)

```json
{
    "response": {
        "result": "success | error | warning",
        "message": "string or null",
        "data": "any or null"
    }
}
```

- Encoded with `JSON_HEX_QUOT | JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES`: 4-space pretty print, `"` inside strings written as `\u0022`, `/` not escaped, non-ASCII escaped as `\uXXXX`. Invalid UTF-8 is repaired by `safe_json_encode`.
- For admins and co-admins only, PHP warnings/notices raised during the request are appended as `response.exceptions.{errors|warnings|notice|other}[]` with `{error, message, file, line}` (max 11 per type).
- `Content-Type: application/json;charset=UTF-8` on most routes. These send plain `application/json`: `GET /ping`, `PUT /categories`, `PUT|DELETE /categories/{id}`, `PUT|DELETE /groups/{id}`, `PUT /tabs`, `PUT|DELETE /tabs/{id}`, `DELETE /users/{id}`, `GET /news`, `POST /news/{id}`, and the matching bookmark-plugin routes.
- Unknown route: `404`, `result:error`, `message:"Endpoint Not Found or Defined"`, `data:{endpoint, method}`.
- Slim error middleware is on with `displayErrorDetails = true`: an uncaught exception returns Slim's HTML/JSON error page with a stack trace.

## 2. Authentication and authorisation model

### Groups (lower number = more access)

| group_id | Name | Notes |
|---|---|---|
| 0 | Admin | Cannot be deleted or made default |
| 1 | Co-Admin | |
| 2 | Super User | |
| 3 | Power User | |
| 4 | User | Default group for new users |
| 5… | custom | `addGroup` assigns max(group_id below 999)+1 |
| 999 | Guest | Not logged in. Cannot be deleted or made default |

`qualifyRequest(N)` passes when `currentLevel <= N`. The three values used on routes: **1** (admin or co-admin), **998** (any logged-in user), **999** (anyone). Plugins and homepage items read their level from config (`*-Auth-include`, `homepage*Auth`, `*SocksAuth`).

### How the current level is resolved

1. Header `Token: <key>` or query `?apikey=<key>`: if the value is exactly 20 characters and equals config `organizrAPI`, level = 0. The request then runs as a synthetic user `{username:"Organizr API", group:"Admin", groupID:0}`.
2. Otherwise the cookie `organizr_token_<uuid>` (or `organizr_token_temp` before setup) holds a JWT. Level = the user's `groupID`.
3. Otherwise 999 (guest; the guest row is read from `groups`).

### Session token

- JWT, HS256, key = config `organizrHash` (plain text). Claims: `iss:"Organizr"`, `aud:"Organizr"`, `jti:"4f1g23a12aa"` (constant), `iat`, `nbf`, `exp`, plus `name`, `group`, `groupID`, `email`, `image`, `userID`.
- The token must also exist in table `tokens` for that user with `expires > now`. User fields are re-read from `users` on every request (JWT claims are only a fallback).
- Optional checks: `matchUserAgents` (token's stored User-Agent must equal the request's), `matchUserIP` (stored IP must equal the request's unless the caller is local).
- A failed check deletes the cookie. Cookies are set twice (once for the parsed parent domain, once for the full host), `SameSite=None; Secure; HttpOnly` only when the request is HTTPS (by `X-Forwarded-Proto` or `HTTPS`).
- Lifetime: 1 day, or `rememberMeDays` (default 7) when the login body contains any `remember` key. Capped at 365.

### Form key (CSRF-like check)

- `formKey` = `password_hash(substr(organizrHash, 2, 10), BCRYPT)`, injected into the page by `index.php`.
- `checkRoute()` requires it on **POST only** (body field `formKey` or header `formkey`), verified with `password_verify`. A valid API key skips it. PUT and DELETE are never checked.
- Bypass list: `/api/v2/upgrade`, `/update`, `/force`, `/auth`, `/wizard`, `/login`, `/wizard/path`, `/login/api`, `/plex/register`.
- Only some routes call `checkRoute()` at all (column **FK** in section 3).

### Login flow (`POST /login`), in order

1. Username is lower-cased unless the auth backend name contains `emby`.
2. Lockout: if body `loginAttempts` (client-supplied counter) is above config `loginAttempts` (default `'5'`), or a `lockout` cookie exists → set `lockout` cookie for `loginLockout` ms (default 60000), `403 "User is locked out"`.
3. Auth proxy: if enabled and the configured username or email header is present, and the caller IP is inside `authProxyWhitelist`, log in as the header value with no password and skip 2FA.
4. `authType`: `internal` (bcrypt check), `external` (backend only), `both` (backend, then internal). Backends: `plex`, `emby_local`, `emby_connect`, `emby_all`, `jellyfin`, `ldap`, `ftp`.
5. Plex OAuth (`oAuth` + `oAuthType:"plex"` in the body): validate the token with plex.tv; allow when the user is `plexAdmin` or on the friends list (optionally restricted to the configured server).
6. Success and user exists: if the stored password does not match, overwrite it with the submitted one; update `plex_token` if changed; 2FA when `users.auth_service` is `type::secret` (skippable for local IPs, Plex OAuth, auth proxy) → `422` with `result:"warning"`, `"2FA Code Needed"` or `422 "Wrong 2FA"`; create token; run SSO (sets cookies for Plex, Ombi, Tautulli, Jellyfin, Overseerr, Petio, Komga).
7. Success and user does not exist: auto-register (default group), optional welcome e-mail, create token.
8. Failure: `401 "User credentials incorrect"`, or `403 "User is locked out"` when the client counter has reached the limit.
9. New-device e-mail when this User-Agent has at most one token for the user and the mailer plugin is enabled.

### Forward auth (`/auth`, `/auth-{group}`, `/organizr-auth`), any method

Used by nginx `auth_request` and Traefik forward-auth. Inputs: `group` (number, or a tab name that is looked up to its `group_id`), `type` (`allow`/`deny` and aliases), `ips` (comma list; host names are resolved). A query string `?group=X` rewrites the request URI to `auth-X` before routing.

- IP in allow list → `200 "User is whitelisted"`.
- IP in deny list → `401 "User is blacklisted"`.
- Level check passes and user not locked → `200 "User is authorized"` with headers `X-Organizr-User`, `X-Organizr-Email`, `X-Organizr-Group`.
- Otherwise `401 "User is not Authorized or User is locked"`; when the caller is Traefik (`X-Forwarded-Server: traefik`) or `traefikAuthEnable` is on, `401` with a `Location` header to the Organizr login plus `?return=<original URL>`.
- `data` always carries `{user, group, email, user_ip, requested_group, uuid}`.

### Secrets at rest

- User passwords: bcrypt (`password_hash`), compatible with any standard bcrypt verifier (`$2y$`).
- Config secrets (any key containing `password`, DB password, registration password): `openssl_encrypt` AES-256-CBC, key = `organizrHash`, IV = the key truncated or padded to 16 bytes with a fixed filler string, base64 output. OpenSSL pads a short key with zero bytes to 32.
- `GET /config` masks only `organizrHash`. The API key, tokens and encrypted passwords are returned to admins and co-admins.

## 3. Endpoint and route inventory

All paths are under `/api/v2`. **Lvl** = `qualifyRequest` argument (`—` = none at route level). **FK** = route calls `checkRoute()` (form key on POST). Unless noted, the response is the standard envelope and handlers put their result in `data`.

### 3.1 Root, status, session

| Method | Path | Lvl | FK | Request | Response and notes |
|---|---|---|---|---|---|
| GET | `` and `/` | — | | | `302` to `/api/v2/status` (absolute path; breaks under a sub-folder install) |
| GET | `/status[/]` | — | ✓ | | `data:{writable, minVersion, os, php, userConfigPathExists, action}`; `action` ∈ `php, dependencies, permission, wizard, launch` |
| ANY | `/auth-[{group}[/]]` | — | | path `group` | Forward auth (section 2) |
| ANY | `/auth[/[{group}[/{type}[/{ips}]]]]` | — | | defaults `group=0`, `type=deny`, `ips=192.0.0.0` | Forward auth |
| ANY | `/organizr-auth[/…]` | — | | same | Forward auth |
| GET | `/launch[/]` | — | | | `data:{categories, tabs, user, branch, theme, style, version, settings, plugins, appearance, status, sso, warnings}`. The main bootstrap payload for the UI |
| POST | `/login` | — | | `{username, password, remember?, tfaCode?, oAuth?, oAuthType?, loginAttempts?, output?}` | `200` sets cookie; with `output` returns `{name, token}`. Errors 401/403/422/500 |
| GET | `/logout` | — | | | Deletes session, SSO cookies, DB token; new device UUID |
| GET | `/token/me` | — | ✓ | | `data` = current user object `{token, tokenDate, tokenExpire, username, uid, group, groupID, email, image, userID, loggedin, locked, tokenList, authService}` |
| POST | `/token/validate` | 999 | | `Token` in query/body/cookie (`$_REQUEST`) | `200 "Token is valid"` + user object, or `403` |
| DELETE | `/token/{id}` | 998 | | | `204 "Token revoked"`; only the caller's own tokens |
| POST | `/2fa/{type}` | 998 | | `type=google` | `{type, secret, url}` (QR via Google Charts URL) or `404` |
| POST | `/2fa` | 998 | | `{secret, code, type}` | `data:true/false`; invalid code is `401` with `result:"success"` |
| PUT | `/2fa` | 998 | | `{secret, type}` | Stores `type::secret` in `users.auth_service` |
| DELETE | `/2fa` | 998 | | | `204`, resets `auth_service` to `internal` |
| GET | `/ping` | — | | | `data:"pong"` |
| POST | `/ping` | — | | `{list: host or [hosts]}` | Map of host → latency ms or false. **No auth** |
| GET | `/ping/{ping}` | — | | | Same for one host. **No auth** |
| GET | `/rox` | — | | | Easter egg; `GET /roxinsocks` → `301` |

### 3.2 Users, groups, tabs, categories

| Method | Path | Lvl | FK | Request | Response and notes |
|---|---|---|---|---|---|
| GET | `/users` | 1 | | | All user rows, **including `password` hash and `plex_token`** |
| POST | `/users` | 1 | | `{username, password, email}` | `200 "User created"`; 409 on any validation failure |
| GET | `/users/{id}` | 1 | | | Raw user row (includes password hash); `false` when missing |
| PUT | `/users/{id}` | 998 | | any `users` columns | Self-edit allowed; others need level 1. Rules in section 5 |
| DELETE | `/users/{id}` | 1 | ✓ | | `204`; 409 for self; 404 unknown. Revokes the user's tokens |
| POST | `/users/lock` / `/users/unlock` | 998 | | unlock: `{password}` | Lock screen for the current user |
| POST | `/users/lock/{id}` / `/users/unlock/{id}` | 1 | | | |
| POST | `/users/import/{type}` | 1 | | `plex`, `jellyfin`, `emby` | Creates users from the media server |
| POST | `/users/register` | — | | `{username, password, email, registrationPassword}` | 422 per missing field; `401` wrong registration password; logs the new user in |
| POST | `/users/recover` | — | | `{email}` | Resets the password to 10 random characters; e-mails it if the mailer is on. `404 "User not found"` |
| GET | `/groups` | 1 | | | All groups |
| GET | `/groups/{id}` | 1 | | | One row by row id |
| POST | `/groups` | 1 | ✓ | `{group, image, default?}` | `"Group added"`; 409 taken; 422 missing |
| PUT | `/groups/{id}` | 1 | ✓* | group columns | 409 on `group_id` change, empty name/image, default on Admin/Guest |
| DELETE | `/groups/{id}` | 1 | ✓* | | `204`; 409 for Admin/Guest or group with users |
| GET | `/tabs` | 1 | | | `{tabs, categories, groups}` (all, unfiltered) |
| GET | `/tabs/{id}` | 1 | | | One tab; 404 `"id not found"` |
| POST | `/tabs` | 1 | ✓ | tab columns; needs `name`, `image`, and `url` or `url_local` | `"Tab added"`; 409 name taken; 422 missing |
| PUT | `/tabs` | 1 | ✓* | `[{id, order}, …]` | `"Tab Order updated"`; 422 malformed |
| PUT | `/tabs/{id}` | 1 | ✓* | tab columns | `"Tab info updated"`; 404; 409 name or group range |
| DELETE | `/tabs/{id}` | 1 | ✓* | | `204 "Tab deleted"`; 404 |
| GET | `/categories` | 1 | | | Same payload as `GET /tabs` |
| POST | `/categories` | 1 | ✓ | `{category, image, default?, order?, category_id?}` | |
| PUT | `/categories` | 1 | ✓* | `[{id, order}, …]` | |
| PUT | `/categories/{id}` | 1 | ✓* | category columns | |
| DELETE | `/categories/{id}` | 1 | ✓* | | `204` |

`✓*` = `checkRoute()` is called but it only checks POST, so it has no effect on PUT/DELETE.

### 3.3 Configuration, settings, system

| Method | Path | Lvl | FK | Notes |
|---|---|---|---|---|
| GET | `/config[/{item}[/{term}]]` | 1 | | All config (hash masked), one key, or `/config/search/{term}`. 404 when unset |
| PUT | `/config` | 1 | | `{key: value, …}`. `"true"`/`"false"` strings become booleans; keys containing `password` are encrypted; `logLocation`/`dbLocation` normalised. **Any key can be written, including unknown ones** |
| GET | `/settings/{appearance, plugin, theme, main, sso, homepage}` | 1 | | UI form definitions: nested arrays of option descriptors `{type, name, label, value, help, options…}` |
| GET | `/settings/homepage/{item}` and `/{item}/debug` | 1 | | One integration's form definition; debug dumps its config values |
| GET | `/page` | — | ✓ | List of page names |
| GET | `/page/{page}` | — | ✓ | `data` = HTML string; 409 `"Page not setup"`. Each page checks its own level. 26 pages: `login, lockscreen, homepage, tabs, error, dependencies, settings, settings_wizard, settings_template, settings_customize_{appearance,settings}, settings_image_manager, settings_plugins[_{enabled,disabled,settings}], settings_settings_{main,backup,logs,sso}, settings_tab_editor_{tabs,categories,homepage,homepage_order}, settings_user_manage_{users,groups}` |
| GET | `/organizr/{page}[/{var1}[/{var2}]]` | — | ✓ | Same pages as raw `text/html` |
| POST | `/wizard` | — | ✓(bypass) | First-run setup: `{driver, dbName, dbPath, license, hashKey, api, registrationPassword, username, password, email, dbHost?, dbUsername?, dbPassword?}` → writes config, creates tables and seed rows, logs in. 422 `"[field] cannot be empty"` |
| POST | `/test/path` | — | | `{path}`; refuses with 401 once configured. Creates the directory |
| POST | `/test/database` | 1 or not yet configured | | `{driver, dbHost, dbUsername, dbPassword, dbPath}`; Postgres always `409 "not available"` |
| GET | `/database/journal`; PUT `/database/journal/{WAL|DELETE}` | 1 | | SQLite `PRAGMA journal_mode` |
| GET/POST | `/backup` | 1 | | List (`{total_size, total_files, files:[{name,size,date}]}`) / create zip of DB + `data/` |
| GET/DELETE | `/backup/{filename}` | 1 | | Download zip (raw stream, process exits) / delete (zip only) |
| GET | `/image`, `/image/select` | 1 | | Image manager listings |
| POST | `/image` | 1 | | multipart field `file`; 10 MB; image MIME + extension allow-list |
| DELETE | `/image/{image}` | 1 | | Only inside `data/userTabs/` |
| GET | `/icon` | 1 | | `?search&page&limit` over `js/icons.json`; select2-style `{results, pagination:{more}, total, limit, page, term}` |
| GET/DELETE | `/log[/{number}[/{trace_id}]]` | 1 | ✓ | Paged JSON log read (`?pageSize=1000&offset=0&filter=NONE`) / purge |
| POST | `/certificate/custom` | 1 | | Upload a CA bundle |
| GET | `/ip/{ip}` | 1 | ✓ | Proxies ipinfo.io with a hard-coded token |
| GET | `/news`; POST `/news/{id}` | 1 | ✓ | Ignored-news id list in config |
| GET | `/help/smtp` | 1 | ✓ | Fetches shared SMTP settings from organizr.app and saves them |
| GET | `/sponsors/{opencollective,github,all}` | — | | Public sponsor lists from external APIs |
| GET | `/oauth/trakt` | 1 | ✓ | Trakt OAuth callback |
| POST | `/plex/register`, `/emby/register` | — | ✓ (plex bypassed) | Invite-style join for media servers |
| GET | `/plex/servers` | 1 | ✓ | `?owned`; list from plex.tv |
| GET | `/plugins`, `/plugins/enabled`, `/plugins/disabled` | — | ✓ | Plugin metadata (no auth) |
| POST/DELETE | `/plugins/manage/{plugin}` | 1 | | Install/remove a marketplace plugin (downloads PHP) |
| GET | `/plugins/marketplace` | 1 | ✓ | |
| POST/DELETE | `/themes/manage/{theme}`; GET `/themes`, `/themes/marketplace` | 1 | ✓ on GET | Theme marketplace |
| GET | `/update`, `/update/{download,unzip,move,cleanup}/{branch}`, `/update/{docker,windows,linux}`, `/update/migrate/{version}`, `/update/reset/{feature}` | 1 | | Self-update by zip download or git scripts |
| POST | `/test/{ldap, ldap/login, iframe, plex, emby, jellyfin, sabnzbd, pihole, adguard, rtorrent, sonarr, radarr, lidarr, sickrage, ombi, overseerr, nzbget, utorrent, deluge, jdownloader, transmission, qbittorrent, unifi, unifi/site, tautulli, cron, folder, jackett, prowlarr, slack-logs}` | 1 | | 31 connection tests. `200` success, or 401/409/422/500 with the upstream error text |
| GET | `/test/cron` | 1 | | Whether the cron timestamp file is fresh |
| ANY | `/socks/{app}/{route:.*}`; `/multiple/socks/{app}/{instance}/{route:.*}` | per-app config | | Reverse proxy to sonarr, radarr, lidarr, sabnzbd, nzbget, tautulli, qbittorrent. 404 unsupported app; 409 disabled or wrong single/multiple form. Upstream body returned as-is with JSON content type |

### 3.4 Homepage integrations (71 routes)

No route-level check. Each handler runs `homepageItemPermissions()` against its own config keys and returns, in order: `409 "<key> module is not enabled"`, `401 "User not approved to view this homepage item"`, `422 "<key> was not supplied"`.

- Media: `GET /homepage/{plex,emby,jellyfin}/{streams,recent}`, `GET /homepage/plex/playlists`, `POST /homepage/{plex,emby,jellyfin}/metadata`, `GET /homepage/plex/search/{query}`, `GET /homepage/image`
- Calendars: `GET /homepage/calendar`, `/homepage/{sonarr,radarr,lidarr,couchpotato,sickrage,ical,trakt}/calendar`
- Queues: `GET /homepage/{sonarr,radarr,rtorrent,deluge,transmission,qbittorrent,utorrent,jdownloader,nzbget,sabnzbd}/queue`; `POST /homepage/sabnzbd/queue/{pause,resume}` (level 1)
- Monitors: `GET /homepage/{pihole,adguard}/stats`, `/homepage/{unifi,tautulli,netdata,monitorr,kuma,speedtest,octoprint,weather}/data`, `/homepage/tautulli/names`, `/homepage/healthchecks[/{tags}]`, `POST /homepage/weather/coordinates`
- Requests: Overseerr (10 routes) and Ombi (7 routes): list, add, approve, deny, available, unavailable, pending, delete, metadata
- Search and misc: `GET /homepage/{jackett,prowlarr}/{query}`, `POST /homepage/{jackett,prowlarr}/download/`, `GET /homepage/youtube/{query}`, `POST /homepage/scrape`
- Donate (Stripe): `GET /homepage/donate`, `/homepage/donate/[{amount}]`, `/homepage/donate/{success,cancel,error}` (HTML), `POST /homepage/donate`

### 3.5 Bundled plugins (42 routes)

| Plugin | Routes | Notes |
|---|---|---|
| bookmark | 18 under `/plugins/bookmark/…` | Own tables; tabs and categories CRUD mirroring core; two HTML settings pages |
| chat | `GET /plugins/chat/settings`, `GET|POST /plugins/chat/message` | Table `chatroom`; pushes through Pusher |
| invites | `GET|POST /plugins/invites`, `GET|POST|DELETE /plugins/invites/{code}`, settings | Table `invites`; verify/use are level 999; can invite to Plex/Emby |
| php-mailer | settings, `GET email/test`, `POST email/send`, `GET email/list` | SMTP; templates used by core (login, recover, register) |
| healthChecks | settings, `GET run` | Pings tab URLs and reports to healthchecks.io; has cron |
| shuck-stop | settings, `GET run` | Drive price watcher; has cron |
| speedTest | settings | LibreSpeed back end in `misc/*.php` (called directly, outside the API) |

## 4. Data model and state

### 4.1 Tables (SQLite DDL is the source; rewritten by string replacement for MySQL/Postgres)

| Table | Columns |
|---|---|
| `users` | `id` PK autoinc, `username` TEXT UNIQUE, `password` (bcrypt), `email`, `plex_token`, `group` TEXT, `group_id` INT, `locked` INT, `image`, `register_date` DATETIME, `auth_service` TEXT default `internal` |
| `tokens` | `id` PK, `token` TEXT UNIQUE (full JWT), `user_id`, `browser` (User-Agent), `ip`, `created`, `expires` |
| `groups` | `id` PK, `group` TEXT UNIQUE, `group_id` INT, `image`, `default` INT |
| `categories` | `id` PK, `order`, `category` TEXT UNIQUE, `category_id`, `image`, `default` |
| `tabs` | `id` PK, `order`, `category_id`, `name`, `url`, `url_local`, `default`, `enabled`, `group_id` (minimum level), `group_id_max` default 0, `add_to_admin` default 0, `image`, `type` (0 internal, 1 iframe, 2 new window), `splash`, `ping`, `ping_url`, `timeout`, `timeout_ms`, `preload` |
| `options` | `id`, `name` UNIQUE, `value` (unused by core code read so far) |
| `chatroom` | `id`, `username`, `gravatar`, `uid`, `date`, `ip`, `message` |
| `invites` | `id`, `code` UNIQUE, `date`, `email`, `username`, `dateused`, `usedby`, `ip`, `valid`, `type`, `invitedby` |
| `BOOKMARK-categories` | `id`, `order`, `category` UNIQUE, `category_id`, `default` |
| `BOOKMARK-tabs` | `id`, `order`, `category_id`, `name`, `url`, `enabled`, `group_id`, `image`, `background_color`, `text_color` |

Seed data at setup: six groups (0–4, 999); tabs `Settings` (`api/v2/page/settings`, level 1, enabled) and `Homepage` (`api/v2/page/homepage`, level 4, disabled); category `Unsorted` (`category_id` 0, default).

### 4.2 Relationships (all implicit; there are no foreign keys)

- `users.group_id` → `groups.group_id` (not `groups.id`). `users.group` is a denormalised copy of the name; renaming a group does not update users.
- `tabs.category_id` → `categories.category_id` (not `categories.id`). Deleting a category leaves its tabs pointing at nothing.
- `tabs.group_id`/`group_id_max` are a level range, not a reference: a user sees a tab when `tab.group_id >= user.groupID AND tab.group_id_max <= user.groupID`.
- `tokens.user_id` → `users.id`. Deleting a user deletes the tokens in code.
- Exactly one row should have `default = 1` in each of `groups`, `categories`, `tabs`; kept by "clear all, then set" in code, with no constraint.

### 4.3 State outside the database

- **Config**: `data/config/config.php`, a PHP file returning an array, **rewritten in full on every change** with the previous copy kept as `config.bak.php`. 710 default keys in `api/config/default.php`, plus each plugin's `config.php`. Missing keys are filled from defaults on load. Numbers are mostly stored as strings (`'7'`, `'5'`, `'60000'`).
- **Files**: `data/userTabs/` (uploaded images), `data/cache/` (cached media images), `data/plugins/`, `data/routes/`, `data/pages/`, backups in `<dbLocation>/backups/`, JSON-lines log files (rotated by count), cron timestamp file, upgrade lock files `DBLOCK.txt`/`completed.txt`.
- **Marker files** in the web root switch behaviour: `Docker.txt`, `Dev.txt`, `Demo.txt`, `Github.txt` (commit hash used for cache-busting).
- **Cookies**: `organizr_token_<uuid>`, `organizr_user_uuid` (7 days), `lockout`, `organizrOAuth`, `oAuth`, and SSO cookies `mpt`, `Auth`, `connect.sid`, `petio_jwt`, `komga_token`, `tautulli_token_<uuid>`, Jellyfin `user-*` and `jellyfin_credentials`.

### 4.4 Implicit state machines

- **Install**: no config → `wizard` → configured. `status.action` and `launch.status` report it. Config version is compared on each request and upgrade steps run in sequence (2.0.0-beta-200 … 2.1.2200), holding a lock file.
- **User session**: guest → (login; maybe `2FA needed`) → logged in → locked (lock screen; `users.locked = 1`, fails forward-auth) → unlocked → logged out. Parallel client-side lockout by cookie.
- **Two-factor**: `auth_service` = `internal` → create secret (client holds it) → verify code → save `google::<secret>` → remove → `internal`.
- **Invite**: created (`valid = 'Yes'`) → used (`valid = 'No'`, `usedby`, `dateused`) or deleted.
- **Requests (Ombi/Overseerr)**: pending → approved/denied → available/unavailable; state lives in the upstream service.
- **Self-update**: download → unzip → move → cleanup, each a separate GET.

## 5. Hidden quirks, edge cases and probable bugs

### 5.1 Security-relevant

1. **Client IP is taken from request headers without any trusted-proxy check** (`Client-IP`, then `X-Forwarded-For`, …; first comma item). Everything IP-based can be spoofed by a direct client: the "local" bypasses, forward-auth allow lists, the auth-proxy whitelist (password-less login), the IP blacklist, cron access.
2. **`POST /users/recover` resets the password of any account by e-mail**, with no token, no confirmation and no rate limit. When the mailer is off the new password is thrown away and the user is locked out.
3. **`GET /users` and `GET /users/{id}` return bcrypt hashes and Plex tokens**; `GET /config` returns the API key and all service tokens to co-admins.
4. **Login lockout is client-side** (a counter in the request body and a cookie). It does not limit a scripted attacker.
5. **The form key is static** (derived from the install hash, never rotated), sent to every visitor, checked on POST only, and not checked at all on many state-changing routes. Cookies are `SameSite=None`.
6. **State-changing GETs**: `/logout`, `/update/*`, `/help/smtp`, plugin `run` routes, donate session creation.
7. **`POST /ping` and `GET /ping/{host}` need no login** and make the server probe arbitrary hosts and ports. `POST /homepage/scrape` and the image cache fetch arbitrary URLs.
8. **`PUT /config` accepts any key**, including `organizrHash`, `organizrAPI`, `dbLocation` and `authProxy*`. A co-admin can take over the instance.
9. **Path handling**: `GET /backup/{filename}` joins the name to the backup folder with no extension or traversal check. Image upload stores under the client file name after only `htmlspecialchars`.
10. **TLS verification is switched off** for any `https` URL containing an IPv4 address, for ipinfo, and LDAP accepts any certificate.
11. **Hard-coded third-party credentials** (ipinfo token, shared SMTP fetch) and **`displayErrorDetails = true`** (stack traces to clients).
12. **JWT**: constant `jti`; signing key is the same `organizrHash` used for config encryption and the form key. The AES IV is derived from the key (same plaintext → same ciphertext).
13. **Backend login overwrites the local password** with whatever the external backend accepted.
14. **Plugin and theme marketplace, `data/routes`, `data/pages`**: remote and local PHP is executed with full rights.

### 5.2 PHP semantics that shape the API

15. **`setAPIResponse` ignores falsy arguments.** A handler cannot clear `message` or `data`, cannot set `data` to `false`, `0`, `[]` or `""`, and `setAPIResponse(null, 'Tab added')` keeps whatever result and status were there. Several "success" paths return a body from an earlier step.
16. **`204` responses carry a JSON body** (`"Tab deleted"`, `"Token revoked"`, …). Most HTTP clients and servers drop it; some proxies reject it.
17. **`401` with `result:"success"`** for a wrong 2FA code in `POST /2fa`; **`422` with `result:"warning"`** for "2FA Code Needed". The UI keys on these.
18. **Token expiry uses a string comparison of two date formats**: `tokens.expires` is stored as `Y-m-d H:i:s`, "now" is `Y-m-d\TH:i:s\Z`. Because a space sorts before `T`, a token stops validating at 00:00 UTC on its expiry date, up to 24 hours early. (Read from the code; to be confirmed against a running instance.)
19. **Loose comparisons**: config numbers are strings (`$loginAttempts > '5'`); `if ($id && $order)` treats order `0` as missing in the reorder endpoints; `$v == null` in the wizard rejects `"0"` and `""` alike; `in_array` without strict mode in IP lists.
20. **`DELETE /groups/{id}` with an unknown id returns `409 "Cannot delete group:  as it is not allowed"`**, not 404: `false['group_id']` is `null`, and `null == 0` is true.
21. **Reorder endpoints are not atomic**: they update row by row, stop at the first malformed item, and leave earlier rows changed. The success message of an earlier item can be returned with a later error code.
22. **`checkKeys` silently drops unknown fields** on create/update; then `INSERT`/`UPDATE` takes every remaining client field (mass assignment within the table: `id`, `register_date` only on update is refused, `locked`, `auth_service`, `plex_token` are writable through `PUT /users/{id}` by the user on their own row).
23. **`PUT /users/{id}` ownership test is `(int)$id !== $this->user['userID']`** (strict, mixed types): when the DB driver returns the id as a string, a user editing their own row is treated as editing someone else.
24. **User text is HTML-escaped before storage** (`htmlspecialchars(trim())` on names, images, usernames). Values in the DB are already entity-encoded; the UI prints them raw. The Rust side must store and return the same encoded form to stay compatible.
25. **Case rules**: usernames are lower-cased at login only when the backend is not Emby; lookups use `COLLATE NOCASE` on SQLite, which is removed for MySQL/Postgres (behaviour then depends on DB collation). Login matches the typed name against username **or** e-mail.
26. **`register()` returns nothing when token creation fails** (falls off the end) → `200 success` with a "User created" message and no session. `login()` with an unknown `oAuthType` returns a bare string that is discarded → `200 success` with no login.
27. **`addUser` has unreachable branches** (`!$username` before `$username == ''`), so some messages in the code can never be returned; validation errors are all `409` here but `422` in `register`.
28. **`testCronSchedule` reads the schedule from the first key of the posted object**, replacing `_` with spaces (PHP mangles spaces in form keys).
29. **`getPlexServers`**: `isset($_GET['owned']) ?? false` is true for `?owned=0` as well.
30. **Dates and time zones**: PHP runs in UTC (`date_default_timezone_set`). DB dates are naive UTC strings. API dates vary by endpoint (ISO with `Z`, `Y-m-d H:i:s`, Unix seconds in the user object). The log rotation name uses `date('Y-m-d')`.
31. **Trailing slashes and optional segments**: Slim patterns such as `/status[/]`, `/auth[/[{group}[/{type}[/{ips}]]]]`, `/homepage/donate/[{amount}]` and the two `download/` routes that exist only with a trailing slash. The Rust router must reproduce each form exactly.
32. **Base path detection**: the app can live in a sub-folder; the base path is derived from `SCRIPT_NAME`. The root redirect ignores it.
33. **Body parsing**: JSON is decoded only when `Content-Type` contains `application/json`; otherwise POST uses form fields and **PUT/DELETE bodies are ignored** (`apiData` returns null → `"no data was sent"`). Several handlers read `$_GET`/`$_REQUEST`/`$_FILES` directly.
34. **Process-terminating paths** (`die`/`exit`): blacklist, low disk, upgrade lock, backup download, Traefik redirect. These bypass the envelope and return HTML or a bare status.
35. **Multi-database support is partial**: Postgres connection test is disabled, `PRAGMA` endpoints are SQLite-only, DDL is adapted by string replacement, backups only include the SQLite file.
36. **Error handler recursion**: PHP warnings are added to the response only if the caller is level ≤ 1, which itself reads headers and config during error handling.
37. **SOCKS proxy**: forwards only `Content-Type` and one auth header; unknown methods become GET; query string rebuilt with `http_build_query` (changes encoding and drops duplicates); upstream status code is not passed through (always the envelope's code, normally 200).

## 6. What cannot be carried over as-is

- **PHP as the extension mechanism**: plugins extending the `Organizr` class, marketplace downloads of PHP, custom routes and pages. A compiled Rust binary cannot load these.
- **`config.php`** as a PHP source file.
- **Self-update** by overwriting the web root or running git scripts; `status`/`launch` fields that report PHP version and PHP extensions.
- **Per-request construction** of the whole application; in Rust this becomes shared state built once, with per-request extractors.
- **`cron.php` driven by system cron**; becomes an in-process scheduler.

## 6a. Addendum (2026-11-18): baseline moves to 2.1.5000

The owner's live install runs Organizr **2.1.5000** (upstream `v2-develop` at `2c1d4199`, 2026-05-19), 212 commits after the clone audited above. The rewrite baseline is 2.1.5000. Differences from sections 1–5:

**New upstream features**

- **OIDC login** (`league/oauth2-client`): `GET /oidc/providers` (public, on the form-key bypass list), `GET /oidc/{provider}/authorize`, `GET /oidc/{provider}/callback` (writes HTML and returns outside the envelope), `GET /oidc/{provider}/test` (level 1). Providers: Authentik, Keycloak, PocketID, Zitadel. Discovery URL, PKCE, state held in a PHP session cookie (10 minutes, `SameSite=Lax`), group-claim mapping to Organizr groups (first / highest / lowest privilege), auto-create and link-by-email, auto-redirect from the login page (`#noredirect` to skip), custom logout URL. About 45 new config keys.
- **Local login bypass**: `bypassLoginForLocal` + `localLoginUserId` log a "local" caller in as a fixed user with no password. `POST /login` skips it when the body has `bypass`, `username` or `oAuth`. Depends on the spoofable client IP (5.1 item 1).
- **Homepage items**: Emby Live TV Tracker (`GET /homepage/embyLiveTVTracker/{stats,activity}`, `POST /test/embyLiveTVTracker`), JellyStat (`GET /homepage/jellystat`, `POST /homepage/jellystat/metadata`, `POST /test/jellystat`), PromPage (`GET /homepage/prompage/data`), plus config for a "User Watch Stats" item. Homepage integrations: 41 or 42, not 38.
- **Invites plugin**: Komga and Nextcloud account creation, Plex Home, Plex library labels (11 new functions).
- **Pi-hole** item reworked (v6 API). Komga SSO moved to `/api/v2/users/me`.

**Upstream fixes that change the audit**

- `POST /users/recover` now needs the mailer enabled (`422 "Email functionality is not enabled"`) and always answers `200 "If the email exists in our system, a password reset has been sent to the user"`. Quirk 2 in 5.1 is reduced to "no rate limit, no confirmation step".
- `POST /wizard` refuses with `401` once config and DB exist.
- `POST /homepage/scrape` blocks local and private addresses (`403`); the check resolves the host once, so DNS rebinding still gets through.
- Backend login no longer overwrites the stored password when no password was sent.
- 808 default config keys (was 710).

**What a real, long-lived install taught us (structure only; no live data is kept in this repository)**

- Expired session rows are never purged; a long-lived install holds over a thousand.
- **Mixed date formats in the same column**: older rows are `2018-05-08T01:28:03Z`, newer rows `2027-02-13 00:08:49`. Columns can be declared `DATE`/`TIMESTAMP` on an old install but `DATETIME` in current DDL. `users.locked` is often `NULL`. Column order differs from the DDL (`add_to_admin`, `group_id_max` appended by upgrades). The importer must read by column name and accept both date formats and NULLs.
- A real config file holds more keys than `default.php` (plugin keys are merged from each plugin's own `config.php`), and can hold keys of plugins that are not upstream (for example `MB-*` from MediaButler).
- Password hashes are `$2y$` bcrypt. Two-factor secrets are stored as `google::<secret>`.
- Installs carry local additions outside `data/`: extra tab and category images, extra CSS and JS files, static help pages. The importer report lists them.
- Typical deployment: behind nginx, linuxserver-style `/config` layout, SQLite in rollback-journal mode.

## 7. Notes for Phase 2 and 3

- The toolchain is pinned to Rust 1.100. The parity oracle (real Organizr) runs in a container.
- `rust-anything` supports PHP only in preview, at whole-app level (`app record` / `app replay` of a `php -S` server). Its useful idea for this project is the method: record the PHP app's real HTTP responses and replay them against the Rust build.
- `docker-nagios-develop` build pattern to mirror: per-architecture builds (amd64, arm64) joined with `buildx imagetools create`, pushed to Docker Hub and GHCR, Trivy gate on CRITICAL, SBOM and provenance, hadolint, weekly rebuild.
