# 📦 Releases and images

[← All documentation](README.md)

This page tells you which container images the project publishes, what each tag means, how to pin one exact build, and which checks an image passes before anybody can pull it. Read it before you choose a tag for your server.

> [!IMPORTANT]
> There is no release yet. Until the first one, the only image is `develop`. The tags `latest` and `<version>` appear with the first release.

## 🐳 The images

The images live in the GitHub container registry at `ghcr.io/tronyx/organizr-rust`. Each tag holds an image for `amd64` and one for `arm64`, and Docker picks the right one for your machine. There is no image for 32-bit ARM (`armv7`).

| Tag | What it holds | When it is built |
|---|---|---|
| `develop` | The newest build of the branch `develop` | On request only: a maintainer starts the `build` workflow (**Run workflow**). There is no daily or nightly run, to save GitHub Actions minutes, so `develop` can be behind the newest commit |
| `latest` | The newest release | On each push to the branch `main` |
| `<version>`, for example `1.0.0` | One release | Together with `latest` |

A few more things worth knowing:

- **No short version tags.** There are no tags such as `1` or `1.2` that follow the newest patch. Use the full version, or pin a digest (see below).
- **Per-architecture tags.** The build also pushes `develop-linux-amd64`, `develop-linux-arm64` (and the same with `latest-`). They are steps of the build, which joins them into the tag above. You don't need them.
- **`develop` moves.** Each new build of `develop` replaces the tag. Pin a digest if you want to stay on one build.
- **`latest` is for releases.** Each push to `main` builds it, and `main` only gets reviewed releases.

## 🔢 Versions

The version of a release is the version of the workspace in the root `Cargo.toml` (today `0.1.0`). The binary reports it:

```bash
docker compose exec organizr organizr --version
```

The answer is `organizr <version>`, for example `organizr 0.1.0`. The image also carries it in the label `org.opencontainers.image.version`, next to the commit in `org.opencontainers.image.revision`.

The update check of the server (**Settings › System Settings › Updates**) compares this version with the newest release on GitHub. It reads release tags written as `v1.2.0` or `1.2.0`, and skips drafts and pre-releases (D-23 in [the deviations](deviations.md)).

> [!NOTE]
> A push to `main` with an unchanged workspace version would move the `<version>` tag to the new build. So each release raises the version in `Cargo.toml` first.

## 📌 Pin one exact build

A digest is the fingerprint of one exact image. A tag can move; a digest never does.

1. Find the digest of the tag you run now:

   ```bash
   docker buildx imagetools inspect ghcr.io/tronyx/organizr-rust:develop
   ```

   The line `Digest:` near the top shows it, for example `sha256:4f3c…`. That is the digest of the multi-architecture list, which works on `amd64` and `arm64`.

2. Put it into your compose file in place of the tag:

   ```yaml
   services:
     organizr:
       image: ghcr.io/tronyx/organizr-rust@sha256:<digest>
   ```

   `<digest>` is the long hexadecimal value from step 1, without `sha256:`.

3. Start the container again:

   ```bash
   docker compose up -d
   ```

To update later, look up the digest of the new build and change the line. To follow the tag again, put `:develop` (or `:latest` after the first release) back.

> [!TIP]
> Your running container already knows its digest: `docker image inspect --format '{{index .RepoDigests 0}}' ghcr.io/tronyx/organizr-rust:develop` prints it.

## ✅ The checks before an image is published

The workflow `build` builds the image for each architecture on a machine of that architecture. An image is published only when all of these pass for both:

1. The program starts and reports the version of the workspace.
2. The image is for the right architecture, and does not run as root (it runs as user `65532:65532`).
3. The server starts, becomes healthy, answers `GET /api/v2/ping`, and hands out the web interface with its security policy (`Content-Security-Policy: default-src 'self'; script-src 'self'; …`).
4. The server stops cleanly when asked (exit code 0).
5. The scanner Trivy finds no vulnerability of the level HIGH or CRITICAL that has a fix.

A vulnerability is a known security fault in a software package of the image. One without a fix does not stop the build, because no build can remove it. The weekly scan lists it (see below).

The scan reads the system packages of the image. The Rust libraries are not in that list: the job **Advisories, licenses and banned crates** of the workflow `ci` checks them with `cargo-deny` on each push.

Only a run for the branch `develop` or `main` publishes. A run for a pull request builds and tests the image and publishes nothing.

## 🧾 What an image carries

Each published image carries two records:

- **The software list**, also called SBOM (software bill of materials). It names each software package in the image, with its version.
- **The build record**, also called provenance. It names the repository, the commit and the steps that made the image.

This command shows both:

```bash
docker buildx imagetools inspect ghcr.io/tronyx/organizr-rust:develop --format '{{ json . }}'
```

> [!NOTE]
> The images are not signed: there is no cosign signature and no GitHub artifact attestation, so `cosign verify` and `gh attestation verify` have nothing to check. The provenance is a record you can read, not a signature. To trust a build, compare its `org.opencontainers.image.revision` label with the commit on GitHub.

## 🔎 The weekly scan

New vulnerabilities become known after an image is published. The workflow `scan` scans the published `develop` and `latest` images every Monday (06:23 UTC), and on request. It lists each finding of the levels HIGH and CRITICAL, and fails when a finding has a fix. A new build with updated base images then removes it. Until the first release, it skips `latest`, which does not exist yet.

## 🧰 For maintainers: pinned versions and updates

You don't need this part to run Organizr. [CONTRIBUTING](../CONTRIBUTING.md) has the rest of the developer workflow.

### 📌 Pinned versions

`docker/Dockerfile` names each base image with its version and its digest. `Cargo.lock` and `web/package-lock.json` name the exact version of each library. So two builds of one commit use the same software.

### 🤖 Updates with Renovate

`renovate.json` configures Renovate, a service that looks for new versions of the pinned software and opens a pull request for each update. It runs only once the Renovate GitHub app is installed for the repository.

| Rule | Value |
|---|---|
| Target branch | `develop` |
| Schedule | Mondays before 6 in the morning |
| Waiting time | 7 days after a release before it offers that release |
| Open pull requests at once | 5 at most, with the label `dependencies` |
| Lock file upkeep | Before 6 in the morning on the first day of each month |
| Dependency dashboard | Off |

What it updates, and how it groups the pull requests:

- The base images of `docker/Dockerfile` with their digests, the Rust and web libraries, the GitHub actions and the tools that the workflows run.
- **Rust toolchain**: one pull request for `rust-toolchain.toml` and `docker/Dockerfile` together. The `build` workflow fails when the two versions differ.
- **Rust libraries**: minor and patch updates in one pull request.
- **Web libraries**: minor and patch updates in one pull request.
- **Workflow tools**: the actions and tools in `.github/workflows/` in one pull request.
- It leaves the folder `parity` alone. The PHP image there is the reference for the parity tests, and its version stays.

The workflows `ci` and `lint` run for each Renovate pull request, as for every other pull request.
