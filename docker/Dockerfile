# syntax=docker/dockerfile:1
ARG BUILDKIT_SBOM_SCAN_CONTEXT=true
ARG BUILDKIT_SBOM_SCAN_STAGE=true

# Multi-arch index digests, so the same value works for amd64 and arm64.
# The Rust version must match rust-toolchain.toml; the build workflow checks it.
ARG RUST_IMAGE=rust:1.100.0-slim-trixie@sha256:6f14b2b4b278ec0d50aa1d69541c304ce0f95e0f6268487aa9924af1c2e9aebe
# glibc and libgcc only, no shell and no package manager.
# Debian 13, like the builder, so the binary finds the glibc it was linked against.
ARG RUNTIME_IMAGE=gcr.io/distroless/cc-debian13:nonroot@sha256:cb87acc574abe719b6e5c547384defa2f7b1f24af9605d3f618cf852495d8be2

############################
# Builder stage: compile the organizr binary
############################
FROM ${RUST_IMAGE} AS builder

WORKDIR /build

# The build context holds only these (see .dockerignore).
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# The whole workspace is built, not only the binary, so that a crate that stops compiling
# on one architecture fails the image build at once.
# The cache mounts keep the downloaded crates and the build output between local builds.
# The binary is copied out because a cache mount is not part of the image layer.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/build/target,sharing=locked \
    cargo build --release --locked --workspace \
    && cp target/release/organizr /organizr

############################
# Final image: the binary and nothing else
############################
FROM ${RUNTIME_IMAGE}

COPY --from=builder /organizr /usr/local/bin/organizr

# The "nonroot" user of the base image, by number, so that a platform that checks for a
# non-root user id can see it.
USER 65532:65532

ENTRYPOINT ["/usr/local/bin/organizr"]
