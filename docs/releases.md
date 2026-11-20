# Releases

This page tells you which images the project publishes, and which checks an image passes before it is published.

## The images

The images are in the GitHub container registry, under `ghcr.io/tronyx/organizr-rust`. Each image is for the architectures `amd64` and `arm64`.

| Tag | Content | Built |
|---|---|---|
| `develop` | The newest state of the branch `develop` | Once a day, when the branch has new commits |
| `latest` | The newest release | For each change of the branch `main` |
| A version, for example `1.0.0` | One release. The tag stays on that release | With `latest` |

The version of a release is the version of the workspace in the file `Cargo.toml`.

## The checks before an image is published

The workflow `build` builds the image for each architecture on a machine of that architecture. An image is published only when all these checks pass for both architectures:

1. The program starts and reports the version of the workspace.
2. The image is for the right architecture, and does not run as root.
3. The server starts, becomes healthy, answers a request, and hands out the web interface with its security policy.
4. The server stops cleanly on request.
5. The scanner Trivy finds no vulnerability of the level HIGH or CRITICAL that has a correction.

A vulnerability is a known security fault in a software package of the image. A vulnerability without a correction does not stop the build, because no build can remove it.

The scan reads the system packages of the image. The Rust libraries are not in that list. The job `Advisories, licences and banned crates` of the workflow `ci` looks for known faults in them on each push.

## What an image carries

Each published image carries two records:

- The software list, also named SBOM. It names each software package in the image, with its version.
- The build record, also named provenance. It names the repository, the commit and the steps that made the image.

This command shows both records:

```sh
docker buildx imagetools inspect ghcr.io/tronyx/organizr-rust:develop --format '{{ json . }}'
```

## The weekly scan

New vulnerabilities become known after an image is published. The workflow `scan` scans the published images each Monday. It lists each finding of the levels HIGH and CRITICAL. It fails when a finding has a correction. A new build then removes the finding, when the pinned base images are updated.

## Pinned versions

The file `docker/Dockerfile` names each base image with its version and its digest. A digest is the fingerprint of one exact image. The file `Cargo.lock` and the file `web/package-lock.json` name the exact version of each library. So two builds of one commit use the same software.

To get a correction of a base image, change its version and its digest in `docker/Dockerfile`. The workflow checks that the Rust version there is the same as in `rust-toolchain.toml`.
