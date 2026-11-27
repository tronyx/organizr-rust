# syntax=docker/dockerfile:1@sha256:572cebbc2b15e6f84c077742952674889cb63cd5e9b80abdffcbe587b6f19183
ARG BUILDKIT_SBOM_SCAN_CONTEXT=true
ARG BUILDKIT_SBOM_SCAN_STAGE=true

# Multi-arch index digests, so the same value works for amd64 and arm64.
# The Rust version must match rust-toolchain.toml; the build workflow checks it.
ARG RUST_IMAGE=rust:1.100.0-slim-trixie@sha256:6f14b2b4b278ec0d50aa1d69541c304ce0f95e0f6268487aa9924af1c2e9aebe
# Builds the web interface. Its files end up inside the binary, so this image is no
# part of the final image.
ARG NODE_IMAGE=node:26.13.1-trixie-slim@sha256:e6bedef217677b276c591795e169840edd9096dbf13f29df8ef8d1856b8820bc
# glibc and libgcc only, no shell and no package manager.
# Debian 13, like the builder, so the binary finds the glibc it was linked against.
ARG RUNTIME_IMAGE=gcr.io/distroless/cc-debian13:nonroot@sha256:29ee70762214fe8190cea04ef1c0a84a8365879e047ce8f911ae5a5ab2355fa2

############################
# Web stage: build the web interface
############################
FROM ${NODE_IMAGE} AS web

WORKDIR /web

# The list of packages first, so this layer is reused until the list changes.
COPY web/package.json web/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm,sharing=locked     npm ci

COPY web ./
RUN npm run build

############################
# Builder stage: compile the organizr binary
############################
FROM ${RUST_IMAGE} AS builder

WORKDIR /build

# The build context holds only these and the folder `web` (see .dockerignore).
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# The built interface, where the server crate looks for it.
COPY --from=web /web/dist ./web/dist

# The whole workspace is built, not only the binary, so that a crate that stops compiling
# on one architecture fails the image build at once.
# The cache mounts keep the downloaded crates and the build output between local builds.
# The binary is copied out because a cache mount is not part of the image layer.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/build/target,sharing=locked \
    cargo build --release --locked --workspace --features organizr/embedded-web,organizr/wasm-plugins \
    && cp target/release/organizr /organizr \
    && mkdir /config

############################
# Final image: the binary and an empty data folder
############################
FROM ${RUNTIME_IMAGE}

COPY --from=builder /organizr /usr/local/bin/organizr

# The data folder: the SQLite database and, later, uploads and backups. It belongs to the
# user the application runs as. A folder mounted here must be writable for that user.
COPY --from=builder --chown=65532:65532 /config /config
ENV ORGANIZR_DATA_DIR=/config
# The list of the marketplace for extensions. An empty value switches the marketplace off.
ENV ORGANIZR_EXTENSION_MARKETPLACE=https://raw.githubusercontent.com/tronyx/organizr-rust-marketplace/main/extensions.json
VOLUME /config

# The "nonroot" user of the base image, by number, so that a platform that checks for a
# non-root user id can see it.
USER 65532:65532

EXPOSE 8080

# The image has no shell and no curl, so the binary checks itself. During the start period
# the check runs every two seconds, so a container is reported healthy soon after it starts.
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --start-interval=2s --retries=3 \
    CMD ["/usr/local/bin/organizr", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/organizr"]
CMD ["serve"]
