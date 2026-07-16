# syntax=docker/dockerfile:1.7
# Multi-stage Dockerfile for the phenotype-tooling workspace.
#
# Build modes:
#   * `cargo build`   – default CI / dev image, includes all tooling
#   * `cargo test`    – CI test image, runs the full workspace test suite
#   * `cargo bench`   – benchmark image, runs the criterion harness
#   * `runtime`       – distroless image with just the binaries
#
# Usage:
#   docker build --target ci            -t phenotype-tooling:ci      .
#   docker build --target runtime       -t phenotype-tooling:runtime .
#
# The `ci` target is the one used in `.github/workflows/ci.yml`.

ARG RUST_VERSION=1.83
ARG DEBIAN_RELEASE=bookworm

# ──── Shared base ─────────────────────────────────────────────────────────────
FROM rust:${RUST_VERSION}-${DEBIAN_RELEASE} AS base
LABEL org.opencontainers.image.source=https://github.com/KooshaPari/phenotype-tooling \
      org.opencontainers.image.licenses=MIT

# Non-interactive apt; install only what cargo actually needs at build time.
ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_TERM_COLOR=always \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_RETRY=10

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        ca-certificates \
        git \
        pkg-config \
        libssl-dev \
        protobuf-compiler \
 && rm -rf /var/lib/apt/lists/*

# Pre-install sccache for build cache sharing and the cargo subcommands used
# by both CI and local dev.
RUN cargo install --locked --root /usr/local/cargo \
        sccache \
        cargo-deny \
        cargo-audit \
        cargo-machete \
        cargo-nextest

ENV SCCACHE_DIR=/sccache \
    RUSTC_WRAPPER=/usr/local/cargo/bin/sccache

# ──── CI target ───────────────────────────────────────────────────────────────
FROM base AS ci
WORKDIR /workspace

# Copy manifests first so this layer caches separately from source changes.
COPY Cargo.toml Cargo.lock ./
COPY crates/*/Cargo.toml crates/
COPY bin/*/Cargo.toml bin/
# Stub source dirs so cargo metadata works without the real sources yet.
RUN mkdir -p crates/*/src bin/*/src \
 && for d in crates/* bin/*; do echo "// stub" > "$d/src/lib.rs"; done

RUN cargo fetch --locked || cargo fetch

# Now bring in the real sources.
COPY crates crates
COPY bin bin

# Default `cargo build --workspace` plus the smoke CI step.
CMD ["bash", "-lc", "cargo build --workspace --locked && cargo test --workspace --no-run"]

# ──── Test target ─────────────────────────────────────────────────────────────
FROM ci AS test
WORKDIR /workspace
CMD ["bash", "-lc", "cargo nextest run --workspace --locked || cargo test --workspace --locked"]

# ──── Bench target ────────────────────────────────────────────────────────────
FROM ci AS bench
WORKDIR /workspace
CMD ["bash", "-lc", "cargo bench --workspace --locked --no-fail-fast -- --output-format bencher"]

# ──── Runtime: distroless image with just the workspace binaries ──────────────
FROM gcr.io/distroless/cc-debian12 AS runtime
ARG BINARIES="pt hook-entry"
WORKDIR /app
COPY --from=ci /workspace/target/release/pt /app/pt 2>/dev/null || true
COPY --from=ci /workspace/target/release/hook-entry /app/hook-entry 2>/dev/null || true
# Distroless has no shell, so each binary must be run directly via ENTRYPOINT.
ENTRYPOINT ["/app/pt"]