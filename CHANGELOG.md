# 📜 Changelog

The notable changes to Organizr Rust, newest first.

There is no release yet. Each release will get an entry here, dated, with its version and its pull request into `main`. Changes waiting on `develop`, which the image `ghcr.io/tronyx/organizr-rust:develop` holds, are listed under **Unreleased**.

## Unreleased

Organizr Rust is a rewrite of PHP Organizr 2.1.5000 in Rust and Svelte. It keeps the API, the settings and the look of PHP Organizr, and an import moves a PHP install over. [Release notes](docs/release-notes.md) say what works as in PHP and what you'll notice is different, and [the deviations](docs/deviations.md) list every difference with its reason.

### Added

- Everything of PHP Organizr 2.1.5000: tabs and categories, users and groups, the homepage items, the settings pages, single sign-on, the login services (Plex, LDAP, FTP, Emby, Jellyfin), OpenID Connect, the second factor, and the plugins (invites, e-mail, speed test, health checks, bookmarks and chat).
- `organizr import` moves a PHP install: the settings, users, groups, tabs, categories, homepage, pictures and custom certificate. See [Moving from PHP Organizr](docs/moving-from-php.md).
- The import report lists the breaking changes that apply to the imported install, and [Moving from PHP Organizr](docs/moving-from-php.md) lists every one in a caution box.
- After an import, an Admin sees the breaking changes that the import found at the top of the page, until an Admin hides them (D-158).
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

### Fixed

- The jobs of a schedule start once per minute. The server could wake a few milliseconds before the minute and start the jobs of the minute before a second time; a job that still ran then logged "the job still runs; this start is left out".
- The settings of a plugin open in a wide window, as in PHP Organizr: ten of the twelve columns of a wide screen. The table of the HealthChecks services fits in it without scrolling to the side.
- Album covers in the poster rows of the homepage show as in PHP Organizr: a square cover with a darkened copy of it above and below, instead of a cover stretched to the shape of a poster.

### Security

- A request body may pause for at most a minute, and Organizr keeps at most 4,096 connections open at once (fewer when the limit of open files is lower), so slow or idle clients can't hold the server (D-200).
- Session cookies that another host of your cookie domain plants can no longer log a user out after every login: Organizr leaves the real cookie alone and also drops plants at `/api/v2/` (D-145).
- Names that differ only by spaces, letter case or wide letters share one count of failed logins, and a login name with hidden characters is refused, so look-alike names can't get around the lock-out at LDAP, FTP, Emby or Jellyfin (D-199).
- Host names in the lists of a forward auth path are looked up only when the check comes from a trusted proxy, so a visitor who reaches Organizr directly can't make it run DNS lookups (D-141).
- Requests to your services send no `Referer`, so a redirect can't hand an API key from the address to another host (D-165).
- A `#` in the address of a forward auth check no longer makes a visitor's allow list count as one the proxy wrote (D-157).
- The picture cache names an Emby or Jellyfin picture after the picture it fetched, so nobody can put one picture under the name of another, as for Plex (D-153).
- A new e-mail address, a change of the second factor, an unlinked OpenID Connect provider and a password stored from a login service now end the other sessions of the account, as a new password does (D-146).
- A Co-Admin can no longer rename or delete another Co-Admin; only an Admin does that (D-188).
- A password check keeps its place in the queue until it ends, also when the client hangs up, so the limit on password checks running at once holds (D-140).
- A login with the password from a recovery e-mail, or through a login service, changes the account only after its second factor passed (D-198).
- Only an Admin changes the mail server, the switch of the mail plugin and the password-reset e-mail, since that e-mail carries a new password of any account, the Admin's too (D-197).
- Secrets (passwords, tokens and API keys of your services) are stored encrypted with a key of the install, and the API shows them masked. See [Secrets](docs/secrets.md).
- A login through Plex or Emby Connect never opens an Admin or Co-Admin account, except the Plex owner's, and no outside service replaces an Admin or Co-Admin password (D-127).
- Proxy headers are believed only from the proxies in `ORGANIZR_TRUSTED_PROXIES`, and wrong passwords lock the client out for a while.
- Custom HTML items run in a closed frame unless you trust them, and the About page loads nothing from other sites.
- Only an Admin trusts a custom HTML item or changes the HTML of a trusted one (D-131).
- Only an Admin changes the sign-in settings, the Plex owner and the logins that skip the second factor. While a login uses the Emby or Jellyfin server, only an Admin changes its address (D-132).
- Only an Admin changes the OpenID Connect settings, which pick the providers and the group a login gives (D-133).
- Only an Admin changes the services of the health checks plugin, which can get the API key (D-134).
- Only an Admin switches single sign-on on, and only an Admin changes the address of an application while its single sign-on is on (D-135).
- With a group in the path, the query of a forward auth check can't lower the group or add an allow list. Caddy sends the visitor's query along, so before this anyone could pass with `?group=999` (D-136).
- Organizr sends no request to a service address that is only a path, such as `/sonarr`, since its host came from the caller. Enter the full address of each service (D-137).
- `ORGANIZR_ALLOWED_HOSTS` brings back service addresses that are only a path, such as `/sonarr`, for the host names you list (D-154).
- **Allowed Hosts** and **Trusted Proxies** are also settings on Login & Security; the variables win. The first Admin login after an import stores its host (D-155).
- E-mails name the site from **Domain Link Override** only, never from the request. The set-up and an Admin login fill it in (D-138).
- A login, registration password, invite code or lock-screen password counts toward the lock-out before it's checked, so many attempts at once get no more tries than the limit (D-139).
- Only a few password checks run at once, as many as the server has processors; the others wait their turn, and after 10 seconds get `503` (D-140).
- The lists of forward auth take host names only in the path, count their first 100 entries, and remember a lookup for a minute (D-141, D-157).
- An allow list in the query of a forward auth check counts again when a trusted proxy wrote that query itself (D-157).
- iCal calendars are read with limits on their repeats and events, away from the request threads, and kept for 5 minutes (D-142).
- One call of an extension has 64 MiB of memory for all its memories and tables together (D-143).
- The GET routes that set up the Bookmarks and Plex Libraries tabs, and the one that sends the test e-mail, refuse requests from other sites (D-144).
- A session cookie that another host of the cookie domain plants hides no session and is dropped (D-145).
- A new password ends the other sessions of the account; a login with a recovery password ends all from before (D-146).
- A backup leaves out the key file that the server uses, also when `ORGANIZR_SECRET_KEY_FILE` puts it elsewhere in the data folder (D-147).
- An imported Admin or Co-Admin account linked to an OpenID Connect provider is taken at the first login only by its own checked e-mail address, never by its name (D-148, D-156).
- **Make an Emby user** works only while the invites are set to Emby; a Plex invite can't make an Emby user (D-149).
- The Komga master password goes with the e-mail address of the Plex account, not the address users can edit (D-150).
- With **Limit to User** for Overseerr, the details of a title name nobody who asked for it, for users (D-151).
- A user's donation history shows the payments Organizr marked with the account at Stripe, not every payment with the account's e-mail (D-152).
- The cache names a Plex picture after the picture itself, and only pictures of the Plex library are fetched (D-153).
- A code of the second factor works for one login; the same code or an older one is refused (D-159).
- The recovery e-mail goes to the stored address of the account, and a login with the old password or a new password ends the recovery password (D-160).
- The name and the e-mail address of an account share one count of failed logins (D-161).
- `PUT /config` refuses an API key with characters outside ASCII, which never worked (D-162).
- A login ends the session that the browser had before (D-163).
- Nobody can make the Co-Admin group the default group, so new accounts never get Co-Admin access (D-164).
- A request that carries a key in a header, such as `X-Api-Key`, follows a redirect only on the same host, so the key goes to no other host (D-165).
- The address of an Uptime Kuma monitor shows no user name or password (D-166).
- The lists of sponsors are kept for 10 minutes, and the answers of Open Collective, GitHub, SickRage and CouchPotato have a largest size (D-167).
- A picture can't get the name of a Windows device, such as `con.png` (D-168).
- A backup leaves out the backup folder and the live database file however their paths are written, so it never holds earlier backups or the live database (D-169).
- Behind a proxy that is not trusted, a forward auth allow list lets nobody through by the proxy's address, which every visitor shares (D-170).
- The address to come back to after a forward auth login is encoded as one value of the query (D-171).
- A cron schedule whose step counts past the largest number, such as `5/4294967295`, is refused (D-172).
- Only Organizr itself may put an answer of the server in a frame: every answer sends `X-Frame-Options` and `frame-ancestors` (D-173).
- A tab address that runs a script, such as `javascript:`, is refused, and the interface never opens one in a frame or a link (D-174).
- The JellyStat details ask Jellyfin or Emby only for a caller who has the level of that server on the homepage (D-175).
- The Overseerr list asks Overseerr for at most 50 requests at a time, or for the **Item Limit** when that is larger (D-176).
- The Tautulli statistic of the top libraries needs the level of the libraries (D-177).
- Single sign-on cookies that are hidden from scripts stay hidden over plain HTTP (D-178).
- The title, the site address and the logo address are written as text in the HTML of an e-mail (D-179).
- An invite counts as "already has access" only when plex.tv says so; another refusal leaves the code for another try (D-180).
- An invite never makes a Komga account with the `ADMIN` role (D-181).
- Guests never make, list or delete invites, whatever level the invites plugin names (D-182).
- Only an Admin changes the backup folder, whose zip files the backup routes list, send and delete (D-183).
- An OpenID Connect login links an existing account by its user name only through `preferred_username`, never by the display name (D-184).
- A new password or a new e-mail address of the own account needs the current password (D-185).
- Turning the own second factor on needs the password; replacing it or turning it off needs the password or a code of the second factor (D-186).
- A link of the own account to an OpenID Connect provider needs the password of the account (D-187).
- Only an Admin changes the password, the e-mail address, the lock or the group of a Co-Admin, or removes its second factor; a Co-Admin keeps those for the own account (D-188).
- Only an Admin changes the custom style sheets, which go into every page (D-189).
- Only an Admin purges a log file (D-190).
- A Co-Admin who changes the address of a service enters its stored secrets again in the same save (D-191).
- `ORGANIZR_SESSION_COOKIE_HOST_ONLY=1` makes the session cookie one for the host of Organizr alone, for installs without forward auth on other hosts.
- An OpenID Connect login comes back to Organizr once: a copy of its login cookie is refused until the cookie expires.
- The approval of an extension also names the level of callers that the dialog showed, and a call of an extension ends after 30 seconds, also while it waits.
- The weekly scan of the published images files the vulnerabilities that have a fix in the Security tab, once the repository is public. See [Releases and images](docs/releases.md#-the-weekly-scan).
- OpenID Connect links an existing account by e-mail address only when the provider says it checked the address; a missing `email_verified` no longer counts (D-192).
- A member who is not an Admin can have only 10 invite e-mails sent in 24 hours (D-193).
- A health check that sends the API key no longer follows a redirect to another host; the service then counts as down (D-194).
- The shared fallback account of Ombi, Overseerr or Petio goes only to users who see the homepage item or a tab of that application (D-195).
- The Netdata and OctoPrint items no longer send the address of the service to viewers, except the OctoPrint address when the webcam stream needs it (D-196).
- An extension download from the marketplace checks every redirect as it checks the first address, so a redirect cannot lead to this computer or a private network.
- An extension download from the marketplace connects to the address it checked, so a name that changes its address between the check and the request cannot lead it to this computer or a private network.
- A wrong password for the unlink from an OpenID Connect provider counts toward the login limit.
- An extension gets the header `x-organizr-same-site` (`true` or `false`) with each call, so it can tell whether a page of Organizr sent a GET or HEAD request.

### Documentation

- The docs are rewritten to be easier to follow, with screenshots, a [security policy](SECURITY.md) and a [contributing guide](CONTRIBUTING.md).
- The reverse proxy and forward auth steps for Caddy, SWAG and Nginx Proxy Manager are tested. In NPM, the lines that pass the user to an application belong in a custom location for `/`, or a forged `X-Organizr-User` reaches the application. See [Forward auth](docs/forward-auth.md#-passing-the-user-to-the-application).
- [Settings that could be removed](docs/unused-settings.md) lists the 82 settings from PHP that this version stores but never reads.
