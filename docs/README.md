# 📖 Documentation

The [README](../README.md) tells you what this rewrite of Organizr is and how to start it. These pages have all the other information. The pages describe the image with the tag `develop`.

## Using Organizr

| Page | Contents |
| --- | --- |
| [📦 Release notes](release-notes.md) | What this version is, what works, and what is different from PHP Organizr |
| [🚀 Installation](installation.md) | How to start the container, the set-up wizard, the data folder, the settings of the container, and updates |
| [🔀 Reverse proxy](reverse-proxy.md) | nginx, Traefik and Caddy in front of Organizr, `ORGANIZR_TRUSTED_PROXIES`, and tabs through the proxy |
| [🧳 Moving from PHP Organizr](moving-from-php.md) | A checklist for the move: import, proxy, appearance, homepage, and what works differently |
| [🗄️ Choosing a database](choosing-a-database.md) | SQLite or PostgreSQL, how to choose, and how to set each one up |
| [🚚 Importing](importing.md) | How to move the users, tabs, settings and pictures of a PHP install into a new install |
| [🔐 Secrets](secrets.md) | How Organizr seals the API keys and passwords that it stores, and the key that it uses |
| [💾 Backups](backups.md) | How to make, download and restore a backup, also on a schedule |
| [🎨 Appearance](appearance.md) | Themes, colours, custom CSS and JavaScript, and the icons of the page |
| [🖌️ Custom CSS](custom-css.md) | The CSS variables and classes to use, and how to change CSS written for PHP |
| [🏠 Homepage](homepage.md) | The items of the homepage, their settings, and how to test the connections |
| [📡 Checking your services](checking-your-services.md) | The `organizr check` command, which asks your services what Organizr asks them |
| [🧪 Tests of the settings](settings-tests.md) | The test buttons of the settings, and what their answers mean |
| [✉️ E-mail](email.md) | The SMTP server, the e-mails that Organizr sends, and password recovery |
| [🩺 Troubleshooting](troubleshooting.md) | Common problems, what causes them, and what to do |

## Logging in

| Page | Contents |
| --- | --- |
| [🎬 Plex login](plex-login.md) | Let the people of your Plex server log in with their Plex account |
| [🗝️ LDAP, FTP, Emby and Jellyfin login](login-services.md) | Check passwords with a server of yours |
| [🪪 OpenID Connect login](openid-connect.md) | Log in with Authentik, Keycloak, PocketID or Zitadel |
| [📱 Second factor](second-factor.md) | A code from an authenticator app at the login |
| [🔗 Single sign-on](single-sign-on.md) | Log in to Plex Web, Ombi, Overseerr and Tautulli with the login of Organizr |
| [🔒 Lock screen](lock-screen.md) | Lock the account on every device without a logout |

## Reverse proxy and API

| Page | Contents |
| --- | --- |
| [🚪 Forward auth](forward-auth.md) | Protect your other applications with the login of Organizr, behind a reverse proxy |
| [🧦 API SOCKS](socks.md) | Reach the API of an application through Organizr |

## Plugins and extensions

| Page | Contents |
| --- | --- |
| [🔖 Bookmarks](bookmark-plugin.md) | A page of links as coloured tiles, in categories |
| [💓 Health checks](health-checks-plugin.md) | Ask your services whether they answer, and tell Healthchecks the result |
| [🎟️ Invites](invites-plugin.md) | Give a person access to your Plex server with a code |
| [📚 Plex Libraries](plex-libraries-plugin.md) | See and switch the libraries of the Plex server that each shared account sees |
| [⚡ Speed test](speed-test-plugin.md) | Measure the connection between a browser and Organizr |
| [🧩 Extensions](extensions.md) | Programs of other authors, which run in a sandbox |

## Running the server

| Page | Contents |
| --- | --- |
| [⏰ Scheduled jobs](scheduled-jobs.md) | The jobs that Organizr runs itself, without a cron table |
| [📜 The log](log.md) | Where Organizr writes its log, and how to read it |
| [🚢 Releases](releases.md) | The images that the project publishes, and the checks that each one passes |

## How it is built

| Page | Contents |
| --- | --- |
| [🏗️ Architecture](architecture.md) | The design of the rewrite, and the order in which it was built |
| [🔍 Audit of PHP Organizr](audit.md) | What PHP Organizr does, route by route, as the rewrite found it |
| [↔️ Deviations](deviations.md) | Every place where this version answers differently from PHP Organizr, and why |
| [📚 Stack research](stack-research.md) | How the libraries of the rewrite were chosen |
| [🖼️ Third-party assets](third-party-assets.md) | The pictures and icons of other projects that the image holds, and their licences |
