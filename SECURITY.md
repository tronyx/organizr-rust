# 🔒 Security policy

Organizr guards the front door of your home server: logins, forward auth for your other apps, and the keys of your services. If you find a way through that door, thank you, and please tell us quietly first.

## 📦 Supported versions

This rewrite isn't released yet. Only the newest build of the `develop` branch gets security fixes, and that's the image `ghcr.io/tronyx/organizr-rust:develop`.

| Version | Supported |
|---|---|
| `develop` (latest image) | ✅ Yes |
| Older `develop` builds | ❌ No: update first, then check whether the problem is still there |
| PHP Organizr | ❌ Not here: report it to the PHP Organizr project |

## 📮 Report a vulnerability

> [!IMPORTANT]
> **Please don't open a public issue, discussion or pull request for a security problem.** That tells everyone about it before there's a fix.

Report it privately through GitHub instead:

1. Open the [**Security** tab of `tronyx/organizr-rust`](https://github.com/tronyx/organizr-rust/security).
2. Select **Report a vulnerability**. This opens a private security advisory that only you and the maintainers can see.
3. Fill in the form.

Helpful things to include:

- What an attacker can do, and what they need first (an account? which level? network access?).
- Steps to reproduce, or a small proof of concept.
- The version you tested: `docker compose exec organizr organizr --version`, and the image digest from `docker image inspect --format '{{index .RepoDigests 0}}' ghcr.io/tronyx/organizr-rust:develop`.
- Your setup, where it matters: database (SQLite or PostgreSQL), reverse proxy, and the relevant settings.

Please leave out real passwords, tokens and keys. Test values are fine.

## ⏳ What to expect

- **A first reply within 7 days**, saying whether we can reproduce it.
- **Updates in the advisory** while we work on a fix. We may ask you questions there.
- **A fix in `develop`**, and a new image, as soon as it's ready. Then the advisory is published, with credit to you if you'd like it.

This is a project run in spare time, so please be patient. Please give us a reasonable time to fix the problem before you talk about it in public.

## 🛡️ How the project watches for problems

| What | How |
|---|---|
| Rust libraries | `cargo-deny` checks them against the RustSec advisory database in each CI run |
| Libraries, base images and workflow tools | Renovate opens a pull request for each new version, a week after it's out, once its GitHub app is installed ([Releases](docs/releases.md#-updates-with-renovate)) |
| Known vulnerabilities in the libraries | GitHub's Dependabot alerts watch `Cargo.lock` and `web/package-lock.json`, once they're switched on in the repository settings |
| The image | Trivy scans each build, and the published images every Monday. A build with a fixable HIGH or CRITICAL vulnerability isn't published, and the weekly findings go to the Security tab ([Releases](docs/releases.md#-the-weekly-scan)) |

## ✅ Hardening checklist

A quick list for anyone running Organizr. Each item links to the page with the details.

- [ ] 🌐 **HTTPS through a reverse proxy.** Port 8080 speaks plain HTTP. Don't forward it from your router; put a TLS proxy in front ([Reverse proxy](docs/reverse-proxy.md)), and remove the published port or bind it to `127.0.0.1`.
- [ ] 🎯 **Narrow trusted proxies.** Set `ORGANIZR_TRUSTED_PROXIES` to the address of your proxy only, not a whole range such as `172.16.0.0/12`. Allow lists, local-network rules and lockouts depend on it ([Reverse proxy](docs/reverse-proxy.md)).
- [ ] 🚪 **Apps behind forward auth are reachable only through the proxy.** Remove their published ports ([Forward auth](docs/forward-auth.md)).
- [ ] 🔑 **Keep `secret.key` safe and backed up**, in a different place from your backups. Without it, stored secrets are gone and accounts with a second factor can't log in ([Secrets](docs/secrets.md)).
- [ ] 📱 **A second factor for every Admin** ([Second factor](docs/second-factor.md)).
- [ ] 📜 **Leave `ORGANIZR_CUSTOM_JS` off** unless you really need custom scripts. When it's on, the scripts run for every user, Admins included ([Appearance](docs/appearance.md)).
- [ ] 🧱 **Trust custom HTML only when you wrote it yourself.** Trusted HTML runs with the full rights of whoever views it ([Homepage](docs/homepage.md)).
- [ ] 🧩 **Review extension permissions** before you approve an extension, and install only extensions whose author you trust ([Extensions](docs/extensions.md)).
- [ ] 🍪 **Share your domain only with apps you trust.** The Organizr login cookie goes to every site under it ([Single sign-on](docs/single-sign-on.md)).
- [ ] 🔄 **Keep the image updated**: `docker compose pull && docker compose up -d` ([Installation](docs/installation.md)).
