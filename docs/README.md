# 📖 Documentation

The [README](../README.md) tells you what this rewrite of Organizr is and how to start it. These pages have everything else. They describe the image with the tag `develop`.

New here? Read [Installation](installation.md), then [Reverse proxy](reverse-proxy.md). Coming from PHP Organizr? Start with [Moving from PHP Organizr](moving-from-php.md).

## 🚀 Getting started

| Page | Contents |
| --- | --- |
| [📦 Installation](installation.md) | Start the container, fill in the set-up wizard, add your first tab, update and roll back |
| [🧳 Moving from PHP Organizr](moving-from-php.md) | The checklist for the move: import, proxy, appearance, homepage, and what works differently |
| [🚚 Importing](importing.md) | What the import takes from a PHP install, what it repairs, and what its report means |
| [🗄️ Choosing a database](choosing-a-database.md) | SQLite or PostgreSQL, how to set each one up, and how to move to PostgreSQL later |
| [📰 Release notes](release-notes.md) | What this version is, what works, and what's different from PHP Organizr |
| [🩺 Troubleshooting](troubleshooting.md) | Common problems and what to do, plus recovery: lost password, lost key, a bad update |

## 🧭 Using Organizr

| Page | Contents |
| --- | --- |
| [🏠 Homepage](homepage.md) | The homepage items (Plex, Sonarr, downloaders, monitors and more), their settings, and how to test them |
| [📡 Checking your services](checking-your-services.md) | `organizr check`: ask all your services what Organizr asks them, at once |
| [🧪 Tests of the settings](settings-tests.md) | The test buttons in the settings, and what their answers mean |
| [🎨 Appearance](appearance.md) | Layouts, themes, colors, the look each person picks, custom JavaScript and the page icons |
| [🖌️ Custom CSS](custom-css.md) | The CSS variables and classes to use, and how to change CSS written for PHP |
| [✉️ E-mail](email.md) | The SMTP server (the PHP Mailer plugin), the e-mails Organizr sends, and password recovery |
| [💾 Backups](backups.md) | Make, download and restore a backup, also on a schedule |

## 🔐 Logging in

| Page | Contents |
| --- | --- |
| [🎬 Plex login](plex-login.md) | Let the people of your Plex server log in with their Plex account |
| [🗝️ LDAP, FTP, Emby and Jellyfin login](login-services.md) | Check passwords against a server of yours |
| [🪪 OpenID Connect login](openid-connect.md) | Log in with Authentik, Keycloak, PocketID or Zitadel |
| [📱 Second factor](second-factor.md) | A code from an authenticator app at the login |
| [🔗 Single sign-on](single-sign-on.md) | Log in to Plex Web, Ombi, Overseerr, Tautulli, Petio, Komga and Jellyfin with your Organizr login |
| [🔒 Lock screen](lock-screen.md) | Lock your account on every device without logging out |

## 🌐 Proxies and the API

| Page | Contents |
| --- | --- |
| [🌐 Reverse proxy](reverse-proxy.md) | Put Organizr behind nginx, Nginx Proxy Manager, SWAG, Traefik or Caddy, trust it safely, and make tabs work |
| [🚪 Forward auth](forward-auth.md) | Protect your other applications with Organizr's login |
| [🧦 API SOCKS](socks.md) | Reach an application's API through Organizr |

## 🧩 Plugins and extensions

| Page | Contents |
| --- | --- |
| [🔖 Bookmarks](bookmark-plugin.md) | A page of links as colored tiles, in categories |
| [💬 Chat](chat-plugin.md) | A chat room for everyone on your Organizr |
| [💓 Health checks](health-checks-plugin.md) | Ask your services whether they answer, and tell Healthchecks the result |
| [🎟️ Invites](invites-plugin.md) | Give someone access to your Plex server with a code |
| [📚 Plex Libraries](plex-libraries-plugin.md) | See and switch the Plex libraries that each shared account sees |
| [⚡ Speed test](speed-test-plugin.md) | Measure the connection between a browser and Organizr |
| [✉️ E-mail](email.md) | The PHP Mailer plugin, for password recovery and other e-mails |
| [🧩 Extensions](extensions.md) | Add-ons by other authors, which run in a sandbox |

## 🔩 Running the server

| Page | Contents |
| --- | --- |
| [🔑 Secrets](secrets.md) | How Organizr encrypts the API keys and passwords it stores, and the key you must back up |
| [⏰ Scheduled jobs](scheduled-jobs.md) | The jobs Organizr runs by itself, without a cron table |
| [📜 The log](log.md) | Where Organizr writes its log, and how to read it |
| [🚢 Releases](releases.md) | The images the project publishes, and the checks each one passes |
| [🛡️ Security policy](../SECURITY.md) | Supported versions, and how to report a vulnerability privately |

## 📚 Reference

| Page | Contents |
| --- | --- |
| [🔧 Configuration](configuration.md) | Every environment variable and command, ports, the data folder, the health check, image tags and resources |
| [↔️ Deviations](deviations.md) | Every place where this version answers differently from PHP Organizr, and why |
| [🖼️ Third-party assets](third-party-assets.md) | The pictures and icons of other projects in the image, and their licenses |

## 👷 Development

> [!NOTE]
> These pages are for contributors and the curious. You don't need them to run Organizr.

| Page | Contents |
| --- | --- |
| [🤝 Contributing](../CONTRIBUTING.md) | Build, test and check a change, and send it in |
| [🏗️ Architecture](architecture.md) | The design of the rewrite, and the order in which it was built |
| [🔍 Audit of PHP Organizr](audit.md) | What PHP Organizr does, route by route, as the rewrite found it |
| [📚 Stack research](stack-research.md) | How the libraries of the rewrite were chosen |
