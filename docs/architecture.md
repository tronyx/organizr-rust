# Organizr → Rust: Phase 2 architecture and scaffolding plan

Date: 2026-11-18. Baseline: Organizr 2.1.5000 (upstream `v2-develop` at `2c1d4199`). Status: approved by the owner on 2026-11-18.

## 1. Intent, goals, limits

**Intent.** Replace the PHP Organizr with a Rust application that does everything the original does, runs as one multi-architecture Docker image, and is safer and easier to maintain.

**Goals**

- Every feature of Organizr 2.1.5000 has a working equivalent.
- The JSON API under `/api/v2` keeps its paths, methods, status codes, messages and JSON shapes, except for the approved deviations in section 7.
- A one-time importer moves an existing install (config file, SQLite database, uploaded files) into the new app.
- An entirely new front end.
- Security holes found in the audit are fixed.

**Non-goals**

- Running existing PHP plugins, PHP custom routes or PHP custom pages. A Rust binary cannot execute PHP.
- Accepting session cookies issued by the PHP app. Every user logs in once after the switch.
- MySQL or MariaDB as a database for the new app. Reading a legacy MySQL install in the importer can be added later if needed.
- Porting the MediaButler plugin (not upstream, switched off in the live config). Its settings are imported and kept.

**Anti-goals** (would make the project a failure even if it works)

- A literal translation of the PHP class into Rust (global response state, string flags, one giant object).
- Live data, tokens or password hashes in the repository.
- A feature silently missing after the switch.

## 2. Decisions decided by the owner

| Topic | Decision |
|---|---|
| Existing data | One-time importer |
| Front end | New, framework chosen by me |
| PHP-only features | My call, but no feature may be lost |
| Integrations | Port all of them |
| Bugs | Keep the wire contract, fix security holes, record each deviation for approval |
| Database | The person installing chooses SQLite or PostgreSQL. SQLite is the default and the documented recommendation |
| Data layer | SeaORM (one set of queries for both engines) |
| Deviations | The list in section 7 is approved |
| Pace | One slice per step: code, tests, commit, push, short report, then wait |
| Git | I may commit and push to `organizr-rust` only |

## 3. Stack (selected with four research agents; versions checked against crates.io and npm on 2026-11-18)

| Layer | Choice | Why | Main alternative and why not |
|---|---|---|---|
| Web framework | **Axum 0.8.9** on hyper 1.x, tower-http 0.7.1 | Best security record (no hyper advisory since 2022; actix-http had seven this month); shares its HTTP stack with the outbound client; in-process tests | Actix-web 4.20: routes closer to Slim's, but a second HTTP stack and the advisory record |
| Async runtime | Tokio 1.54 | Fixed by the brief | |
| Database | **SQLite (WAL) by default, PostgreSQL 18 as an option**, selected by `ORGANIZR_DATABASE_URL` | See 3.1 | MariaDB: left out; each extra engine doubles part of the test matrix |
| Data layer | **SeaORM 2.0.4** (on SQLx 0.9.1) | Each query and each migration is written once and runs on both engines, so the two cannot drift apart | SQLx with two SQL sets: compile-time checks, but every change is made twice. Diesel 2.4: steep query language, SQLite async is a wrapper |
| Sessions | Own table, opaque 256-bit token, SHA-256 at rest, in-process cache (moka) | Instant revocation; the per-device list is the table; a database leak exposes no live token | JWT: no revocation without a database check. tower-sessions: opaque blob, no per-device columns |
| Passwords | bcrypt verify for imported `$2y$` hashes, argon2id for new, re-hash on login | Existing users keep their passwords | |
| CSRF | Fetch-metadata check (`Sec-Fetch-Site` / `Origin`) plus `SameSite=Lax` | No token to leak or rotate | Static form key (today): checked on POST only |
| Secrets at rest | XChaCha20-Poly1305, master key from env or key file, sub-keys by HKDF | Authenticated encryption; ends the one-key-for-everything design | AES-256-CBC with key-derived IV (today) |
| Outbound HTTP | reqwest 0.13.5 with rustls (aws-lc-rs) | No OpenSSL in the image | |
| Scheduler | croner 4 + a Tokio loop | Replaces `cron.php` and system cron | tokio-cron-scheduler: stale, pulls chrono |
| Front end | **Svelte 5 + Vite 8 + TypeScript 6**, Tailwind 4 + daisyUI 5, Bits UI; embedded in the binary | Escapes output by default; plain static build allows a strict Content-Security-Policy with no inline script; small runtime; readable components | SvelteKit 3: one month old, breaks sub-folder installs. Leptos/Dioxus: breaking releases pending, widgets need JS anyway. htmx: a second HTML API. React/Vue: viable, larger |
| API types for the front end | ts-rs 12 (Rust types → TypeScript), OpenAPI by utoipa 6 | Front-end build fails when a Rust type changes | openapi-typescript: no TypeScript 6 support yet |
| Bundled plugins | Compiled-in modules behind a `Plugin` trait | Same trust as core, no runtime cost | |
| Third-party extensions | WebAssembly components on wasmtime (LTS line), behind a cargo feature, built after core parity | Sandboxed replacement for "download PHP and run it" | Scripting engines: weaker sandbox or C code in-process. Sidecar containers: no one-click install |
| Container | Multi-stage build, distroless glibc runtime, non-root, one volume, one process | Small attack surface; mirrors the Docker-Nagios pipeline | Alpine/musl: allocator and DNS differences for no gain |

### 3.1 Database: two engines, SQLite recommended

The person installing chooses the engine with one setting, `ORGANIZR_DATABASE_URL`. Without it the app creates a SQLite file in the data volume. A `postgres://` URL selects PostgreSQL. The first-run wizard does not offer this choice. It shows the engine in use and refuses a set-up that names another one (D-46). See `choosing-a-database.md`.

| | SQLite (WAL mode), the default | PostgreSQL 18 (separate container) |
|---|---|---|
| Deployment | One image, one process, one volume | Two containers and a compose file; database password to manage |
| Speed for this workload | Reads are in-process function calls; forward-auth needs no network hop | Every query crosses a socket. Faster only with many concurrent writers, which this app does not have |
| Robustness | Transactions, foreign keys, CHECK constraints. One writer at a time (irrelevant at a few writes per second) | Stricter types, flexible schema changes, concurrent writers, replication |
| Upgrades | Nothing to upgrade separately | Major versions need a dump and restore |
| Real weakness | **Unsafe on NFS or SMB shares.** The app detects this at start-up and refuses or falls back to the slower rollback journal | Larger memory use on small arm64 boards |

Why SQLite is recommended: for one app instance with a few writes per second it is as robust as PostgreSQL and faster, with nothing extra to run. Choose PostgreSQL when the data folder is on a network share, when a PostgreSQL server already exists and is backed up, or when more than one app instance is wanted. `docs/choosing-a-database.md` will explain this for users.

What two engines mean for the design:

- **One code path.** Queries use SeaORM's typed builder and migrations use `sea-orm-migration`, both written once. Raw SQL is allowed only in one small, tested module for engine-specific statements (journal mode, snapshot).
- **Caught by tests, not the compiler.** SeaORM does not check queries against the schema at compile time. Every database test and every parity test runs once per engine in CI.
- **Engine-neutral types.** Booleans, timestamps (Unix seconds) and JSON are stored in forms both engines share. Case-insensitive uniqueness (user names, group names) uses a normalised lower-case column, not engine collations.
- **Backups.** The backup feature writes an engine-neutral export (one JSON-lines file per table) zipped with the data folder. The same export moves an install from one engine to the other.
- **SQLite-only routes.** `GET /database/journal` and `PUT /database/journal/{option}` answer on PostgreSQL as the PHP app does on a non-SQLite database.
- **Docker.** Two compose examples: single container with SQLite, and with a PostgreSQL sidecar.

## 4. Workspace and directory layout

```
organizr-rust/
├── Cargo.toml                 workspace manifest, shared dependency versions and lints
├── Cargo.lock
├── rust-toolchain.toml        pins Rust 1.100
├── deny.toml                  cargo-deny: advisories, licences, banned crates (openssl, native-tls)
├── rustfmt.toml  clippy.toml
├── crates/
│   ├── domain/                organizr-domain: newtypes (UserId, GroupLevel, TabId …), enums (TabKind,
│   │                          AuthService, InstallState …), state machines, domain errors. No I/O.
│   ├── settings/              organizr-settings: typed registry of every config key (type, default, group,
│   │                          minimum level, secret flag, form descriptor), validated snapshot
│   ├── db/                    organizr-db: connection set-up for both engines, entities, migrations,
│   │                          repositories, encrypted secrets store, engine-neutral export
│   ├── auth/                  organizr-auth: passwords, sessions, API key, TOTP, rate limits, lock screen,
│   │                          backends (Plex, Emby, Jellyfin, LDAP, FTP), Plex OAuth, OIDC, auth proxy,
│   │                          SSO hand-off, forward-auth decision
│   ├── integrations/          organizr-integrations: outbound client, SSRF guard, one module per service
│   ├── plugins/               organizr-plugins: Plugin trait, bundled plugins, (later) WebAssembly host
│   ├── api/                   organizr-api: router, envelope and PHP-compatible JSON writer, extractors,
│   │                          unified ApiError → HTTP response, all handlers, OpenAPI, exported TS types
│   ├── import/                organizr-import: PHP config parser, legacy SQLite reader, legacy AES
│   │                          decryption, dry-run report
│   ├── server/                organizr (binary): bootstrap, scheduler, embedded web assets;
│   │                          subcommands serve, import, healthcheck, reset-password
│   └── testkit/               organizr-testkit (dev only): app on a temp database, upstream mocks,
│                              replay of recorded PHP cases
├── web/                       Svelte front end (src/shell, auth, homepage/widgets, settings, plugins, lib)
├── parity/
│   ├── oracle/                compose file that runs real Organizr 2.1.5000 with synthetic seed data
│   ├── recorder/              drives the oracle and writes cases
│   └── cases/                 recorded request/response pairs (synthetic data only)
├── docker/                    Dockerfile, compose.sqlite.yml, compose.postgres.yml, .hadolint.yaml
├── docs/                      audit.md, architecture.md, deviations.md, stack-research.md,
│                              choosing-a-database.md, importing.md
└── .github/workflows/         ci.yml, build.yml, scan.yml, lint.yml
```

Dependency direction: `domain` ← `settings` ← `db` ← `auth`, `integrations`, `plugins` ← `api` ← `server`. `import` uses `domain`, `settings`, `db`. Nothing depends on `api` except `server` and `testkit`.

How the four architectural rules are enforced:

1. **No naive translation.** Shared state is one immutable `AppState` (pools, settings snapshot, clients). Each request builds a `Principal` (API key, user session or guest) through an extractor. Handlers return `Result<ApiResponse<T>, ApiError>`; there is no mutable global response.
2. **Type safety.** Ids and levels are newtypes; tab type, install state, auth service, result kind are enums; settings are read through typed accessors generated from the registry.
3. **No panics.** Workspace lints deny `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` and `todo` outside tests. One `ApiError` type maps every domain error to the envelope.
4. **Exact business logic.** Recorded PHP responses are replayed against the Rust build; a difference must match a named, approved deviation or the test fails.

Error handling: `thiserror` enums in `domain`, `db`, `auth`, `integrations`, `plugins`, `api`; `anyhow` only in `server` (start-up) and the importer command.

## 5. Exact dependencies

Resolved together in a throwaway manifest on 2026-11-18: 635 packages on Rust 1.100, every feature name valid, no OpenSSL and no native-tls in the tree. Not yet compiled; first compile is milestone 0.

```toml
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.package]
edition = "2024"
rust-version = "1.100"
license = "GPL-3.0-only"          # Organizr is GPL-3.0; assumed for a port of it, owner to confirm
publish = false

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
indexing_slicing = "deny"

[workspace.dependencies]
# runtime and basics
tokio = { version = "1.54", features = ["rt-multi-thread", "macros", "net", "signal", "fs", "time", "sync", "io-util"] }
tokio-util = { version = "0.7.20", features = ["io", "rt"] }
futures-util = "0.3.34"
serde = { version = "1.0.229", features = ["derive"] }
serde_json = { version = "1.0.154", features = ["preserve_order"] }
thiserror = "2.0.21"
anyhow = "1.0.104"
clap = { version = "4.6.8", features = ["derive", "env"] }
arc-swap = "1.9.5"
bytes = "1.12"
http = "1.5"
http-body-util = "0.1.5"
url = { version = "2.5.8", features = ["serde"] }
percent-encoding = "2.3.2"
base64 = "0.22.1"
hex = "0.4.3"
uuid = { version = "1.29", features = ["v4", "serde"] }
semver = "1.0.28"
jiff = { version = "1.0.1", features = ["serde"] }

# web
axum = { version = "0.8.9", features = ["macros", "multipart", "ws"] }
axum-extra = { version = "0.12.6", features = ["cookie", "typed-header"] }
tower = { version = "0.5.3", features = ["util", "limit", "timeout"] }
tower-http = { version = "0.7.1", features = ["trace", "request-id", "sensitive-headers", "set-header", "compression-gzip", "compression-br", "limit", "timeout", "catch-panic", "csrf"] }
hyper-util = { version = "0.1.21", features = ["server-auto", "server-graceful", "tokio", "http1", "http2"] }
utoipa = { version = "6.0.0", features = ["axum_extras"] }
utoipa-axum = "0.3.0"
utoipa-scalar = { version = "0.4.0", features = ["axum"] }
rust-embed = "8.14.0"
mime_guess = "2.0.5"
ts-rs = "12.0.1"

# logging
tracing = "0.1.44"
tracing-subscriber = { version = "0.3.23", features = ["json", "env-filter"] }
tracing-appender = "0.2.5"

# data
sea-orm = { version = "2.0.4", default-features = false, features = ["sqlx-sqlite", "sqlx-postgres", "runtime-tokio-rustls", "macros", "with-json"] }
sea-orm-migration = { version = "2.0.4", default-features = false, features = ["sqlx-sqlite", "sqlx-postgres", "runtime-tokio-rustls"] }
moka = { version = "0.12.16", features = ["future"] }

# security
chacha20poly1305 = { version = "0.11.0", features = ["zeroize"] }
hkdf = "0.13.0"
sha2 = "0.11.1"
secrecy = "0.10.3"
zeroize = "1.9.2"
subtle = "2.6.1"
rand = "0.10.4"
getrandom = "0.4.3"
bcrypt = "0.19.3"
argon2 = "0.6.0"
totp-rs = { version = "6.0.0", features = ["otpauth", "qr", "gen_secret", "zeroize"] }
governor = "0.10.4"
ipnet = { version = "2.12.2", features = ["serde"] }

# outbound and integrations
reqwest = { version = "0.13.5", features = ["cookies", "json", "gzip", "socks", "stream", "multipart"] }
diqwest = "3.2.0"                 # HTTP digest auth (Komga)
ldap3 = { version = "0.12.1", default-features = false, features = ["tls-rustls-aws-lc-rs"] }
suppaftp = { version = "13.0.0", features = ["tokio-rustls-aws-lc-rs"] }
lettre = { version = "0.11.24", default-features = false, features = ["smtp-transport", "builder", "pool", "hostname", "tokio1-rustls", "aws-lc-rs", "rustls-platform-verifier"] }
minijinja = "3.1.0"               # mail and custom-page templates
croner = { version = "4.0.3", features = ["jiff"] }
icalendar = { version = "0.17.17", features = ["recurrence"] }
quick-xml = { version = "0.42.0", features = ["serialize"] }
scraper = "0.29.0"
dxr = "0.9.0"                     # XML-RPC types (rTorrent)
zip = { version = "9.7.0", default-features = false, features = ["deflate"] }
infer = "0.22.0"
image = { version = "0.25.10", default-features = false, features = ["jpeg", "png", "gif", "webp"] }
surge-ping = "0.9.1"

# importer only
aes = "0.9.4"
cbc = "0.2.1"
php-literal-parser = "0.7.1"

# tests
axum-test = "21.1.0"
wiremock = "0.6.5"
insta = { version = "1.49.0", features = ["json", "redactions"] }
tempfile = "3.27"
```

Added later, with its milestone: `wasmtime` (48.x LTS line) behind the `wasm-plugins` feature. Not used on purpose: `chrono` in our own code (flagged unmaintained on 2026-11-17; it arrives only through `icalendar`, which is kept inside one module), `jsonwebtoken` (no JWT sessions), `openssl`.

Tools: cargo-nextest 0.9.152, cargo-deny 0.21.0, cargo-machete 0.10.1.

Front end (`web/package.json`): svelte 5.58.0, vite 8.3.8, @sveltejs/vite-plugin-svelte 7.3.1, typescript 6.0.3, svelte-check 4.7.8, tailwindcss and @tailwindcss/vite 4.3.5, daisyui 5.8.1, bits-ui 2.21.0, @tanstack/svelte-query 6.4.3, svelte-dnd-action 0.9.80, @event-calendar/core 5.19.0, embla-carousel-svelte 8.6.0, codemirror 6.0.2 with CSS, JavaScript and HTML language packs, vitest 5.1.1, @testing-library/svelte 5.4.2, jsdom 31.0.1, @playwright/test 1.65.1, eslint 10.15.0, eslint-plugin-svelte 3.24.1, typescript-eslint 8.73.0, prettier 3.10.1, prettier-plugin-svelte 4.2.0. Build stage on Node 26 LTS.

## 6. How PHP-only features are kept

| Original feature | Replacement | What changes for the user |
|---|---|---|
| 7 bundled plugins | Compiled in. Same routes, settings and cron jobs | Nothing |
| Plugin marketplace | Same endpoints and list format. Entries carry a WebAssembly component, a SHA-256 and a permission list; the admin approves permissions at install | Existing PHP entries show as "not compatible". The one real marketplace plugin (Plex Libraries) is ported as a bundled plugin, licence permitting |
| Custom PHP routes and pages | Local WebAssembly plugins in `data/plugins/`, and plain HTML pages in `data/pages/` rendered by the app | PHP files must be rewritten. The importer lists any it finds and never runs them. The live install has none |
| Theme marketplace, custom CSS | Same endpoints and install flow, with safe downloads. Themes are CSS files against a documented set of custom properties | Old theme CSS targets the old page structure and does not apply to the new front end. The 11 marketplace themes are rebuilt as native themes |
| Custom JavaScript and custom HTML blocks | Kept. Custom JS is served as a file, admin-only, with a safe-mode switch. Custom HTML blocks run in a sandboxed frame | Existing snippets that use jQuery or old element ids need rewriting |
| Self-update | Scheduled version check with a notice in the UI and optional e-mail. Old `/update/*` routes stay and answer `409` with the current and latest version | Updating means pulling the new image |
| `cron.php` + system cron | In-process scheduler with overlap guard and timeouts | No cron entry needed |
| Chat over Pusher | Built-in server-sent events | No Pusher account needed |
| Config as a PHP file | Settings and secrets tables with a typed registry | Bootstrap values (data path, port, database URL, trusted proxies, master key) come from environment variables |

The WebAssembly extension system is the largest single item that exists only to keep a rarely used feature. It is scheduled last so it cannot delay the rest, and it stays behind a build feature because the runtime has a high advisory rate.

## 7. Deviations from the PHP behaviour

See `deviations.md`. It lists every approved difference from Organizr 2.1.5000, each with an id that the parity tests refer to.

## 8. Importer

`organizr import --from <old /config or web root> [--dry-run]`, run once against an empty new database.

1. Parse `data/config/config.php` (PHP array literal) without running PHP.
2. Open the legacy SQLite file read-only; read by column name; accept both date formats and NULLs. The target can be either engine.
3. Decrypt legacy AES-256-CBC values with `organizrHash`; re-encrypt with the new master key.
4. Map config keys through the settings registry. Unknown keys (for example `MB-*`) are stored in a `legacy_settings` table and listed in the report, never dropped.
5. Users keep their bcrypt hashes and 2FA secrets. Sessions are not imported.
6. Copy `data/userTabs`, favicons, custom images, installed themes; skip the image cache.
7. Hash or encrypt the API key so the existing key keeps working.
8. Write everything in one transaction and print a report: counts per table, repaired rows (orphan tabs, duplicate names by case), skipped items, PHP files found.

Dry-run mode prints the report and writes nothing. Testing uses synthetic legacy fixtures in the repository, plus a local run against a copy of a live install, which never enters the repository.

## 9. Parity verification

- **Oracle.** Real Organizr 2.1.5000 in a container (`parity/oracle`), seeded with synthetic users, tabs and config, with upstream services replaced by canned responses.
- **Recording.** A recorder sends each case (method, path, headers, body, as guest / user / co-admin / admin / API key) and stores status, selected headers and body.
- **Replay.** `#[tokio::test]` integration tests start the Rust app in-process on a temp database with the same seed and compare byte for byte. CI runs them once on SQLite and once on PostgreSQL. A difference passes only when it is listed under a deviation id.
- **Integrations.** Each service module is tested against recorded upstream responses served by wiremock; where a live instance is available, recordings come from it and are scrubbed first.
- **Legacy crypto.** Test vectors for AES-CBC, `$2y$` bcrypt and the TOTP window are generated in the oracle container.
- **Front end.** Component tests (Vitest) and browser tests (Playwright) against the real binary with the Content-Security-Policy enforced.
- **Every slice ends with** `cargo fmt --check`, `cargo check`, `cargo clippy -- -D warnings`, `cargo nextest run`, `cargo deny check`, and for front-end slices `svelte-check`, ESLint, Vitest and a build, all with zero warnings.

## 10. Migration order

Each numbered item is one or more vertical slices. A slice is finished when its parity tests and the checks above pass.

| # | Milestone | Content |
|---|---|---|
| 0 | Scaffolding and proofs | Workspace, lints, CI, cargo-deny, Dockerfile skeleton building on amd64 and arm64. Prototypes for the risky pieces: Slim-style route expander, PHP-identical JSON writer, tower-http 0.7 with Axum 0.8, SeaORM with newtypes and enums on both engines |
| 1 | Parity harness | Oracle container, synthetic seed, recorder, replay test runner, first recorded cases (`/status`, `/ping`, unknown route) |
| 2 | Domain types | Ids, levels, enums, state machines, domain errors |
| 3 | Database | Engine selection, schema, migrations, repositories, secrets store, engine-neutral export, start-up checks (file system, SQLite version), CI on both engines |
| 4 | Settings registry | All keys of 2.1.5000 plus plugin keys, typed accessors, form descriptors, snapshot |
| 5 | Importer | Section 8, with dry-run; verified locally against the live copy |
| 6 | API foundation | Envelope, `ApiError`, body-parsing rules, trusted-proxy client IP, `Principal`, level guard, CSRF, request ids, logging, catch-all 404, `/status`, `/ping`, `/rox` |
| 7 | Core auth | Login with local passwords, logout, sessions, API key, rate limit, 2FA, lock screen, `/token/*`, `/users/register`, recovery, mailer plugin (needed for auth mail) |
| 8 | Forward auth | `/auth`, `/auth-{group}`, `/organizr-auth`, Traefik redirect, response headers, cache |
| 9 | Core data slices | Users → groups → categories → tabs → `/launch` → `/config` and `/settings/*` → images and icons → backups → logs → news, sponsors, ip, help → wizard and first run → `/test/*` basics |
| 10 | Front-end shell | Build pipeline, login and lock screen, tab shell with iframe manager, settings forms, tab/category/user/group editors, image manager, logs, backups |
| 11 | External auth | Plex (password and OAuth), Jellyfin, Emby, LDAP, FTP, auth proxy, local bypass, OIDC (four providers), SSO hand-off to Plex, Ombi, Overseerr, Tautulli, Petio, Jellyfin, Komga, user import |
| 12 | Homepage framework | Permissions pattern, ordering, refresh, image cache, custom HTML blocks, combined calendar, widget framework in the front end |
| 13 | Homepage integrations, live ones first | Plex, Sonarr, Radarr, Lidarr, SABnzbd, qBittorrent, Overseerr, Tautulli, Uptime Kuma, health checks, weather, Netdata. Then: Emby, Jellyfin, JellyStat, Emby Live TV Tracker, Ombi, NZBGet, Deluge, Transmission, rTorrent, uTorrent, JDownloader, Pi-hole, AdGuard, UniFi, Monitorr, PromPage, Speedtest, OctoPrint, Jackett, Prowlarr, Trakt, iCal, CouchPotato, SickRage, bookmarks, donate, YouTube and scrape. Each with its `/test/*` route and widget |
| 14 | SOCKS proxy | Both route forms, streaming, per-app auth |
| 15 | Remaining plugins | Invites (with Plex, Emby, Komga, Nextcloud), bookmarks, health checks, speed test, shuck-stop, chat, Plex Libraries |
| 16 | Scheduler and upkeep | Auto-backup, plugin jobs, session purge, version check, `/update/*` |
| 17 | Themes and customisation | Appearance settings, native themes, theme marketplace, custom CSS/JS, languages (32 packs reused) |
| 18 | Release pipeline | Per-architecture image builds, manifest, Trivy gate, SBOM and provenance, weekly rebuild, dependency updates, both compose examples, user docs including `choosing-a-database.md` |
| 19 | Extension system | WebAssembly host, plugin template, marketplace install, local plugins and custom routes |
| 20 | Cutover | Import the live install, run next to the PHP app behind nginx, full replay, switch |

## 11. Working method in Phase 3

- One slice per step: code, tests, checks, commit, push, short report, then wait for the go-ahead. This follows the standing one-change-per-step rule; commit and push are done by me for this repository only.
- Work happens on a `develop` branch; `main` receives reviewed releases through pull requests the owner opens.
- Images are pushed to GHCR only while the repository is private.
- Sub-agents are used for independent integration modules once the framework for them exists (milestone 13), each with its own review.
- Anything that turns out to need a deviation not listed in section 7 stops the slice and is raised first.

## 12. Risks

| Risk | Mitigation |
|---|---|
| Size: about 265 routes, 42 integrations, a full new UI | Strict milestone order; live-used features first; parity harness from the start |
| A route shape or JSON detail that Axum/serde cannot reproduce | Proven in milestone 0 before anything depends on it |
| SeaORM does not check queries at compile time | Every database and parity test runs on both engines in CI |
| SQLx 0.9.1 (under SeaORM) is one day old | Lockfile pins the tested version |
| Upstream services differ by version (Pi-hole 5 vs 6, Sonarr 3 vs 4) | Port what 2.1.5000 does; record against real instances where available |
| Old themes and custom snippets do not fit the new UI | Native themes; documented theme contract; safe mode for custom JS |
| Loss of the master key makes stored secrets unreadable | First-run and backup documentation; key file inside the data volume by default |
| Advisory churn (lettre, zip, wasmtime) | cargo-deny in CI, weekly rebuild, automated dependency updates |
