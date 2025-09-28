# Backup Ingestion Node

## Purpose

The **Backup Ingestion Node** is a lightweight server process that:

- Listens for incoming backup data over **QUIC** using [iroh].
- Writes received data to a mounted SSD in a safe, atomic way.
- Forms the core of the **private backup system**, running on low-power boards like Raspberry Pi 4 (DietPi) or embedded Linux (Yocto later).

This component is deployed as a **Docker container**.  
All business logic lives in reusable crates under `crates/`; this project is only the thin deployment layer.

---

## Running on DietPi (Raspberry Pi 4)

### Prerequisites

- Docker installed on DietPi
- SSD mounted at `/mnt/ssd/backups`

### Build and Run the server container

Build from repo root using the app-local Dockerfile:

```bash
docker build -t homenect-server:latest -f applications/server/Dockerfile .
````

Optional features:

```bash
docker build -t homenect-server:featX \
  -f applications/server/Dockerfile \
  --build-arg FEATURES="feat-x,feat-y"
```

Run (host networking recommended for iroh):

```bash
sudo mkdir -p /srv/homenect/store && sudo chown 10001:10001 /srv/homenect/store

docker run -d --name homenect \
  --network host \
  -e RUST_LOG="homenect=debug,iroh=info" \
  -e HOMENECT_STORE_PATH="/data/store" \
  -e HOMENECT_ALLOW_NODE_IDS="n0id1...,n0id2..." \
  -v /srv/homenect/store:/data/store:rw \
  --restart unless-stopped \
  homenect-server:latest
```

### Environment variables

- `HOMENECT_STORE_PATH`: absolute path for blobs inside container, default `/data/store`.
- `HOMENECT_ALLOW_NODE_IDS`: comma-separated allow-list of peer NodeIds, e.g. `n0abc...,n0def....`
- `RUST_LOG`: tracing filter, default `homenect=debug,iroh=info`.

---

## Running on Yocto (future)

When moving to Yocto, the Docker container can be replaced by:

- a native package built via a Yocto recipe,
- or a systemd service wrapping the compiled binary.

The same environment variables are used for configuration.

---

## Other environments

The node can be compiled and run as a plain binary on any Linux distribution with Rust installed.
Adapt mounting and env vars accordingly.

## Ports

The node exposes:

- `TCP/UDP 4444` (configurable via `SERVICE_LISTEN_ADDR`)
