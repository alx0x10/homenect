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

### Build the image on a dev machine (for arm64)

```bash
docker buildx build \
  --platform linux/arm64 \
  -t backup-ingestion-node:latest \
  -f applications/backup-ingestion-node/Dockerfile \
  .
```

### Run on DietPi

```bash
docker run -d \
  --name backup-ingestion-node \
  --restart unless-stopped \
  -p 4444:4444/tcp -p 4444:4444/udp \
  -v /mnt/ssd/backups:/mnt/ssd/backups \
  -e BACKUP_REPOSITORY_PATH=/mnt/ssd/backups \
  -e SERVICE_LISTEN_ADDR=0.0.0.0:4444 \
  backup-ingestion-node:latest
```

### Environment variables

- `BACKUP_REPOSITORY_PATH` → Absolute path inside the container where backups are written (mount host SSD).
- `SERVICE_LISTEN_ADDR` → IP:PORT to bind the QUIC endpoint.

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
