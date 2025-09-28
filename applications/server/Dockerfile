# syntax=docker/dockerfile:1.7

########################
# 1) Builder
########################
ARG RUST_VERSION=1.81
FROM rust:${RUST_VERSION}-bookworm AS builder
ARG FEATURES=""

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
    pkg-config build-essential ca-certificates git \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# --- Cache-friendly: copy workspace manifests only ---
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY crates/proto-control/Cargo.toml crates/proto-control/Cargo.toml
COPY applications/server/Cargo.toml applications/server/Cargo.toml
COPY applications/backup-laptop-cli/Cargo.toml applications/backup-laptop-cli/Cargo.toml

# --- Warm cache with stub sources for ALL workspace members ---
RUN mkdir -p \
      crates/proto-control/src \
      applications/server/src \
      applications/backup-laptop-cli/src \
 && printf "fn main() {}\n" > applications/server/src/main.rs \
 && printf "\n" > crates/proto-control/src/lib.rs \
 && printf "fn main() {}\n" > applications/backup-laptop-cli/src/main.rs

# Prebuild deps (conditional features)
RUN cargo build -p server --release ${FEATURES:+--features ${FEATURES}} || true

# --- Now copy the full source tree ---
COPY . .

# Real build
RUN cargo build -p server --release ${FEATURES:+--features ${FEATURES}}

########################
# 2) Runtime
########################
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates tini \
 && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 10001 -s /usr/sbin/nologin app
WORKDIR /app
RUN mkdir -p /data/store && chown -R app:app /data

COPY --from=builder /app/target/release/server /usr/local/bin/server

USER app
ENV RUST_LOG=homenect=debug,iroh=info \
    HOMENECT_STORE_PATH=/data/store

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/usr/local/bin/server"]
