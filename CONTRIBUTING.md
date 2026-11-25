# 🤝 Contributing

Thanks for wanting to help! This page shows you how to build Organizr from source, run its tests and checks, and get a change ready. If you only want to run Organizr, you don't need any of this: start with the [README](README.md).

## 📋 What you need

| Tool | Version | What for |
|---|---|---|
| Rust (through [rustup](https://rustup.rs/)) | Pinned in `rust-toolchain.toml`: 1.100.0, with `rustfmt`, `clippy` and the target `wasm32-wasip2` | The server and the tests. Run `rustup toolchain install` once in the repository to get the pinned version |
| Node.js and npm | 24 or newer (`web/package.json`); CI uses 26 | The web interface in `web/` |
| Python 3 | Any current one | The docs checks, the import scripts and the parity recorder |
| Docker | Any current one | Only for the PHP reference in `parity/`, the real-application tests, hadolint and actionlint |
| PostgreSQL 18 | Optional | The PostgreSQL run of the database tests |

## 🧭 Find your way around

| Folder | What lives there |
|---|---|
| `crates/domain` | `organizr-domain`: ids, levels, enums and domain errors. No I/O |
| `crates/settings` | `organizr-settings`: the typed registry of every setting |
| `crates/db` | `organizr-db`: both database engines, entities, migrations, the encrypted secrets store |
| `crates/auth` | `organizr-auth`: passwords, sessions, the API key, the second factor, login limits, OpenID Connect |
| `crates/integrations` | `organizr-integrations`: the outbound HTTP client and one module per service |
| `crates/plugins` | `organizr-plugins`: the built-in plugins and the WebAssembly host for extensions |
| `crates/api` | `organizr-api`: the router, the PHP-compatible envelope and every handler |
| `crates/import` | `organizr-import`: the importer of PHP installs |
| `crates/server` | `organizr`, the binary: `serve`, `import`, `restore`, `move-to-postgres`, `check`, `healthcheck` |
| `crates/testkit` | `organizr-testkit`: test helpers, the engine runner and the parity replay |
| `web/` | The Svelte 5 interface, its unit tests (`src/**/*.test.ts`) and browser tests (`e2e/`) |
| `parity/` | The PHP reference container, the recorder and the recorded cases |
| `examples/extension-template` | A small extension to copy when you write your own |
| `docker/` | The `Dockerfile` and the compose examples |
| `docs/` | The user and contributor docs, copied to the wiki |

[Architecture](docs/architecture.md) explains the design and the reasons behind it.

## 🚀 Build and run it from source

1. Build the web interface:

   ```bash
   cd web
   npm ci
   npm run build
   cd ..
   ```

   The build lands in `web/dist`.

2. Build and start the server with the interface built in:

   ```bash
   cargo run -p organizr --features embedded-web -- serve
   ```

   The server listens on `0.0.0.0:8080` and keeps its data in `./data` (the repository ignores that folder). The log prints the set-up code that the wizard asks for.

3. Open `http://localhost:8080/` and run the wizard.

To try extensions too, add their runtime: `--features embedded-web,wasm-plugins`. The Docker image is built with both features.

### ⚡ Work on the interface with live reload

1. Start the server as in step 2 above, on port 8080.
2. In a second terminal, start the Vite dev server:

   ```bash
   cd web
   npm run dev
   ```

3. Open the address that Vite prints. Vite sends every request below `/api` to the server on `127.0.0.1:8080`, and reloads the page when you save a file.

## 🧪 Run the tests

### 🦀 Rust

```bash
cargo test --workspace --locked
```

This includes the parity replay: it replays the recorded PHP answers in `parity/cases/` against the Rust router. It doesn't need the PHP container.

The extension tests need the `wasm-plugins` feature. CI builds them separately, because that runtime makes each test program large:

```bash
cargo test --locked -p organizr-plugins --features wasm-plugins --test extension
cargo test --locked -p organizr-api --features wasm-plugins --lib --test extensions
```

### 🐘 The PostgreSQL tests

The database tests always run on SQLite. They also run on PostgreSQL when `ORGANIZR_TEST_POSTGRES_URL` names a server where the tests may create databases. Each test gets a database of its own.

1. Start a throwaway server:

   ```bash
   docker run --rm -d --name organizr-test-postgres -e POSTGRES_PASSWORD=organizr-test -p 127.0.0.1:5432:5432 postgres:18-alpine
   ```

2. Run the tests with the URL:

   ```bash
   ORGANIZR_TEST_POSTGRES_URL=postgres://postgres:organizr-test@localhost:5432/postgres cargo test --workspace --locked
   ```

3. Stop the server when you're done:

   ```bash
   docker stop organizr-test-postgres
   ```

### 🌐 The web interface

Run these in `web/`:

| Command | What it does |
|---|---|
| `npm test` | The unit tests (Vitest) |
| `npm run check` | The types, with `svelte-check`; warnings fail |
| `npm run lint` | ESLint; warnings fail |
| `npm run format:check` | Prettier, without changing files |
| `npm run build` | The production build |

### 🎭 The browser tests

The browser tests (Playwright) run against the real server binary, which hands out the built interface with its security headers. Build both first:

```bash
cd web
npm run build
cd ..
cargo build -p organizr --features embedded-web
cd web
npx playwright install chromium
npm run e2e
```

> [!IMPORTANT]
> Run the suite as a whole. The tests share one server and one install, and each test builds on the one before it, so a single spec on its own fails.

The tests start `target/debug/organizr` on `127.0.0.1:8171` with an empty data folder. To test another binary, set `ORGANIZR_BINARY` to its path.

### 🐳 The real applications

The homepage tests talk to stand-ins. This script starts real Sonarr, Radarr, SABnzbd and qBittorrent in containers and asks them the same questions:

```bash
bash parity/services/run.sh
```

### 📝 Record new PHP answers

You only need this when you add or change a parity case. The recorder talks to the real PHP Organizr 2.1.5000 in a container:

```bash
docker compose -f parity/oracle/compose.yml up --build --detach
python parity/recorder/record.py
docker compose -f parity/oracle/compose.yml down --volumes
```

## 🧹 Format and lint

CI fails on any warning, so run these before you push:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --features organizr/wasm-plugins -- -D warnings
cargo clippy -p organizr --features embedded-web --all-targets --locked -- -D warnings
cargo test -p organizr --features embedded-web --locked
cargo deny check
```

`cargo fmt --all` fixes the formatting. `cargo deny` checks advisories, licenses and banned crates (such as OpenSSL); install it with `cargo install cargo-deny`. The second clippy run and the `embedded-web` test need a built `web/dist`.

In `web/`, `npm run format` fixes the formatting; the other checks are in the table above.

The Dockerfile and the workflows have linters too:

```bash
docker run --rm -i -v "$PWD/docker/.hadolint.yaml:/.config/hadolint.yaml:ro" hadolint/hadolint:v2.15.1 < docker/Dockerfile
docker run --rm -i -v "$PWD/docker/.hadolint.yaml:/.config/hadolint.yaml:ro" hadolint/hadolint:v2.15.1 < parity/oracle/Dockerfile
docker run --rm -v "$PWD:/repo:ro" -w /repo rhysd/actionlint:1.7.12 -color
```

## 🤖 What CI runs

| Workflow | When | What |
|---|---|---|
| `ci` | Each push and pull request to `develop` or `main` | Rust format, clippy and tests (with PostgreSQL), the extension tests, `cargo-deny` (through its GitHub action), the web checks and build, clippy and `cargo test -p organizr --features embedded-web --locked` with the interface built in, the browser tests, and the real applications |
| `lint` | Same | hadolint on `docker/Dockerfile` and `parity/oracle/Dockerfile`, actionlint, and the docs checks |
| `build` | On request, each push to `main`, and a pull request to `main` that changes the image | Builds, tests and publishes the image. See [Releases and images](docs/releases.md) |
| `scan` | Mondays, and on request | Scans the published images for vulnerabilities, and files the ones with a fix in the Security tab |
| `sync-wiki` | Each push to `develop` that changes `docs/` | Copies `docs/` to the wiki |

## 📝 Commits and branches

- Work on the branch `develop`. The branch `main` only gets reviewed releases, through pull requests.
- Keep commits small and focused: one change per commit, with its tests.
- Write the subject in plain American English, as a short command: "Show the poster of PHP for a missing poster", not "Fixed posters". Explain the why in the body when it isn't obvious.
- Use American spelling everywhere, also in code, setting keys and API names (`color`, `colorSet`, `canceled`).

## 🔀 Differences from PHP need an entry

Organizr keeps the API of PHP Organizr 2.1.5000. Every intended difference needs a row in [the deviations](docs/deviations.md) with a new id, written as `| D-nn |` at the start of the row. The parity replay checks this: a test may expect another answer than the recorded PHP one only when it names an id that exists in that file, and fails otherwise. [How entries are approved](docs/deviations.md#-how-entries-are-approved) tells you who decides.

## 📚 Docs

- `docs/` is the source of the docs. A push to `develop` that changes it copies every page to the GitHub wiki, which a footer marks as a copy. Edit `docs/`, never the wiki.
- Link between pages with relative links, such as `installation.md`. The docs check (`python3 -m unittest discover -s .github/scripts`) checks every link and `#anchor` in the README, `CONTRIBUTING.md` and `docs/`, and the copy for the wiki.
- GitHub makes an anchor from a heading by lower-casing it, dropping emoji and punctuation, and turning spaces into hyphens: `## 📦 Installation` becomes `#-installation`. If you rename a heading, fix the links to it.
- Two tests read docs: the parity replay needs every `| D-nn |` row of `docs/deviations.md`, and `web/src/customCss.test.ts` reads `docs/custom-css.md`.

## 🧩 Extensions

`examples/extension-template` is a small extension in Rust to copy. Build it in that folder with:

```bash
cargo build --release --target wasm32-wasip2
```

[Extensions](docs/extensions.md) explains the manifest, the permissions and how to install one.

## 🔒 Security problems

Found a security problem? Please don't describe it in a public issue. Report it privately: on the **Security** tab of the repository, select **Report a vulnerability**. [SECURITY.md](SECURITY.md) explains what to include and which versions get fixes.
