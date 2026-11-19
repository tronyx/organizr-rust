# Deviations from Organizr 2.1.5000

The Rust application keeps the PHP application's API contract: paths, methods, status codes, messages and JSON shapes. This file lists every place where it differs on purpose. Each entry has an id. A parity test may differ from the recorded PHP response only when it names one of these ids.

Status: the owner approved the entries below. An entry without a date was approved on 2026-11-18. An entry with a date was approved on that date.

On 2026-11-19 the owner gave a standing approval for every difference that comes from the fact that this application is not PHP. Such an entry ends with "Decided under the standing approval of 2026-11-19". The implementer decided it, and the owner reviews these entries in batches. Every other new deviation needs the owner's approval before it is added here.

**Security fixes**

| Id | Change |
|---|---|
| D-01 | Client IP comes from forwarding headers only when the direct peer is in a configured trusted-proxy list. Affects "local" detection, forward-auth IP lists, the blacklist, auth-proxy login and local login bypass |
| D-02 | Sessions are opaque random tokens, not JWTs. Cookie name unchanged. PHP-issued cookies stop working at the switch. `tokenList` no longer returns other sessions' token values |
| D-03 | The static form key is replaced by a same-origin check on every state-changing request. `formKey` is accepted and ignored. State-changing GET routes keep their path and method but need a same-origin request or the API key |
| D-04 | Login attempts are limited on the server by IP and by user name. The `loginAttempts` body field is ignored; the `403 "User is locked out"` answer is unchanged |
| D-05 | `GET /users` and `GET /users/{id}` return `password` and `plex_token` as `null` |
| D-06 | `GET /config` masks every secret as `***Secure***`. The API key is shown to Admin (level 0) only |
| D-07 | `PUT /config` rejects unknown keys with `422` and cannot write bootstrap keys. Auth-proxy, API key and custom-JavaScript keys need Admin |
| D-08 | `POST /ping` and `GET /ping/{host}` only probe hosts that are configured as a tab or ping URL the caller may see |
| D-09 | Password recovery e-mails a one-time, time-limited reset link, and is rate limited. The password changes only when the link is used |
| D-10 | Backup and image file names are validated strictly (no path parts, allow-listed extensions) |
| D-11 | TLS certificates are verified unless the integration's own "disable certificate check" setting is on. The importer switches that setting on where the old implicit rule applied (HTTPS URL with an IP address), so nothing breaks |
| D-12 | Hard-coded third-party tokens are removed; the ipinfo token becomes a setting |
| D-13 | No stack traces or PHP `exceptions` block in responses. Each response carries a request id that matches the log |
| D-14 | Secrets are re-encrypted with a new master key. `organizrHash` is used only during import |
| D-15 | Users can change only their own user name, e-mail, password and image through `PUT /users/{id}`. `locked`, `auth_service`, `plex_token` and `group_id` need level 1 |
| D-16 | URL fetches on behalf of non-admins (scrape, image cache) resolve the host once, check it, and connect to that address |
| D-17 | Marketplace downloads use TLS verification, a pinned commit and a hash, with path and size checks |
| D-18 | The session cookie is always `HttpOnly`, also over plain HTTP, so a script of the page cannot read it. For the same reason `GET /token/me` returns `null` for `token` and for every `token` in `tokenList`. Each entry of that list has a new field `current`, which is `true` for the session of the request. `POST /token/validate` still returns the token that the caller sent. Approved on 2026-11-18 |
| D-19 | The set-up wizard asks for a set-up code. The server makes a random code when it starts on an install that is not set up, and writes the code to its log. `POST /wizard` needs the code in the field `setupCode`. Without the right code the answer is `403`. In PHP the first visitor of a new install becomes the Admin. Approved on 2026-11-18 |

**Forced by the platform or by the new front end**

| Id | Change |
|---|---|
| D-20 | `204` responses have no body |
| D-21 | `status` and `launch` keep their PHP-specific keys (`php`, `php_user`, `minVersion`, `dependenciesActive`, `dependenciesInactive`) with fixed values |
| D-22 | `GET /page/{page}` and `GET /organizr/{page}` serve custom pages only. Built-in page names answer with the envelope and the front-end route in `data`. `/settings/*` form descriptors drop the raw `html` and `attr` fields in favour of typed kinds |
| D-23 | `/update/*` behaves as in section 6. New route `GET /update/check` |
| D-24 | Text is stored and returned as plain text, not HTML-entity-encoded. The importer decodes existing values once |
| D-25 | Response header names are lower-case (HTTP treats names as case-insensitive) |
| D-26 | Backup archives hold an engine-neutral export of the database instead of the raw SQLite file. The backup routes and their responses are unchanged |
| D-27 | No `PHPSESSID` cookie. The PHP application starts a PHP session on every request and needs it only during an OIDC login. The OIDC login keeps its state in a short-lived cookie of its own, set only during that login. The three no-cache headers that came with the PHP session are still sent on every answer. Approved on 2026-11-18 |
| D-28 | The switches `splash`, `ping`, `timeout` and `preload` of a tab are always `0` or `1`. PHP sends `null` for a switch that was never set, for example on the tabs of a new install. A request that sets such a switch to `null` sets it to `0`. Approved on 2026-11-18 |
| D-29 | `GET /launch` has no `status.userConfigPath`. PHP sends the path of its configuration file there, to every visitor. This application has no such file (D-45). Approved on 2026-11-18 |

**Replacements for PHP-only features** (described in `architecture.md`, section 6)

| Id | Change |
|---|---|
| D-40 | Marketplace plugins are WebAssembly components with declared permissions. PHP plugins cannot run and show as "not compatible" |
| D-41 | Custom PHP routes and pages are replaced by local WebAssembly plugins and plain HTML pages. The importer reports PHP files and never runs them |
| D-42 | Themes and custom CSS/JS written for the old page structure do not apply to the new front end. The 11 marketplace themes are rebuilt natively |
| D-43 | Scheduled jobs run in-process. `cron.php` and the system cron entry are not needed |
| D-44 | Chat uses built-in server-sent events. Pusher settings are imported and ignored |
| D-45 | Config lives in the database, not in `config.php`. Bootstrap values come from environment variables |
| D-46 | The set-up wizard does not choose or create the database. The database is the one the server was started with: SQLite, or the database that `ORGANIZR_DATABASE_URL` names. `GET /status` has a new field `database` (`sqlite` or `postgresql`), so the wizard can show the engine. `POST /wizard` does not read `dbName`, `dbPath`, `dbHost`, `dbUsername`, `dbPassword` and `hashKey`. A `driver` that names another engine than the one in use gets `422`. The registration password is stored as a hash. See `choosing-a-database.md`. Approved on 2026-11-18 |

**Bug fixes that change an observable result**

| Id | Change |
|---|---|
| D-30 | Session expiry compares real times, so a session lasts its full lifetime instead of ending at 00:00 UTC on its last day |
| D-31 | `DELETE /groups/{id}` with an unknown id returns `404 "id not found"` instead of `409` |
| D-32 | The reorder routes (`PUT /tabs`, `PUT /categories`, bookmark equivalents) apply all changes or none |
| D-33 | Expired sessions are purged by a scheduled job |
| D-34 | A request that makes the PHP application crash gets an error answer in the usual form. PHP answers such a request with status `500` and an HTML page. A list or an object as `username` or `password` of `POST /login` gets `401 "User credentials incorrect"`. `POST /token/validate` without the field `Token` gets `403 "Token was invalid"`. Approved on 2026-11-18 |
| D-35 | The API key must match character for character. PHP compares the key with `==`. For a key that looks like a number, PHP also accepts other spellings of the same number, for example `000000000000000001e1` for the key `00000000000000000010`. Approved on 2026-11-18 |
| D-36 | `POST /users/register` refuses a name that is only spaces with `409 "Username was set but empty"`. PHP creates an account without a name. Approved on 2026-11-18 |
| D-37 | Wrong registration passwords are counted like failed logins (D-04): by the address of the client, and for the install as a whole. When a count reaches the `loginAttempts` setting, `POST /users/register` answers `403 "User is locked out"` for `loginLockout` milliseconds. PHP has no limit. Approved on 2026-11-18 |
| D-38 | The registration password that the set-up wizard stores works at once. In PHP the wizard stores it as plain text and the registration reads it as encrypted text, so nobody can register until an admin saves the password again. Approved on 2026-11-18 |
| D-39 | `POST /tabs` and `PUT /tabs/{id}` check what they store. PHP stores any value in any field. (1) A value must be of the kind its field takes: a switch is `0`, `1`, `true` or `false`, `type` is `0` to `2`, `group_id` and `group_id_max` are `0` to `999`, `category_id` names a category, and `order` and `timeout_ms` are whole numbers. Another value gets `422 "Tab <field> is not valid"`. (2) A name that is empty gets `422 "Tab name was not supplied"`. PHP stores a tab without a name. (3) The field `id` is ignored. PHP lets a request choose or change the id of a tab. (4) A new tab whose `group_id` is a lower number than its `group_id_max` gets `409`, with the text that `PUT /tabs/{id}` has for it. PHP checks this only when a tab is changed. (5) A new tab that is the default takes that mark from every other tab. PHP does this only when a tab is changed. (6) A name is taken when another tab has the same name, whatever the letter case. PHP compares with `LIKE`, so `Pl_x` counts as taken when `Plex` exists. Approved on 2026-11-19 |
| D-47 | The placeholder `{username_lower}` in the address of a tab stands for the name of the user in lower case. In PHP it stands for the name as it is, the same as `{username}`. Approved on 2026-11-19 |
| D-48 | `POST /categories`, `PUT /categories/{id}` and `DELETE /categories/{id}` check what they do. (1) The rules of D-39 apply to categories: `default` is `0`, `1`, `true` or `false`, and `order` and `category_id` are whole numbers. Another value gets `422 "Category <field> is not valid"`. A name that is empty gets `422 "Category name was not supplied"`. The field `id` is ignored. A new category that is the default takes that mark from every other category. A name is taken when another category has the same name, whatever the letter case. (2) Two categories cannot share a number. A `category_id` that another category has gets `409 "Category category_id: <n> is already taken"`. PHP stores both, and a tab with that number then belongs to two categories. (3) A category that still has tabs is not deleted. The answer is `409 "Category name: <name> still has tabs"`. PHP deletes the category and leaves its tabs without one. Approved on 2026-11-19 |
| D-49 | The check for a caller on the local network reads addresses as the standards define them. (1) The address `::1`, the machine itself in IPv6, is local, and the `localIPList` setting takes addresses and networks of IPv6. PHP knows IPv4 only. (2) An entry of `localIPList` such as `192.168.1.130/25` stands for its network, `192.168.1.128` to `192.168.1.255`. PHP sets the last number to `0` and counts up from there, which gives `192.168.1.0` to `192.168.1.127`. An entry that is written as a network, such as `192.168.1.128/25`, gives the same in both. (3) An entry that is no address is left out. PHP reads it as `0.0.0.0`. Approved on 2026-11-19 |
| D-50 | Five details of the answer of `GET /launch`. (1) For an Admin or a Co-Admin, `settings.misc.schema` is an empty list. PHP sends the SQL text of each of its tables there. The tables of this application are not those of PHP. (2) For the same callers, `settings.misc.docker` is `false` and `settings.misc.githubCommit` is `null`. PHP reports there whether it runs in the official container, and its git commit. (3) The list `settings.homepage.refresh` never holds a secret. PHP puts every setting in that list whose key has `Refresh` in it and whose value is a number. A Trakt refresh token of digits only is such a setting, and PHP sends it to every visitor. (4) The default of the setting `breezometerToken` is empty (D-12). (5) The keys of the lists `settings.homepage.refresh`, `plugins` and `plugins.includes` are sorted, capital letters first. PHP gives them in the order of its configuration file. On a new install that is the order of the default settings and of a folder listing. From the first change of a setting on, it is the sorted order. Decided under the standing approval of 2026-11-19 |

Kept exactly as they are, because clients depend on them: the envelope and pretty-printed JSON, `401` with `result:"success"` for a wrong 2FA code, `422` with `result:"warning"` for "2FA Code Needed", per-route content types, optional path segments and trailing-slash forms, message texts, validation rules and their order, numbers that the API returns as strings.
