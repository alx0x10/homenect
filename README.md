# Homenect

This repository is the foundation for a modular system to support **private data backup** and later **private VPN routing**.  
It follows a strict architecture:  

- **Reusable crates** hold all shared domain and technical capabilities.  
- **Applications** are thin deployables (server nodes, CLI clients, mobile clients).  
- **Dependency arrows are one-way:** `applications/*` → `crates/*`.

## Repository Layout

```text
your-repo/
├─ Cargo.toml
├─ rust-toolchain.toml
├─ crates/                       # reusable, versionable building blocks
│  ├─ context-backup-core/       # domain + use-cases for backups (no IO)
│  ├─ context-vpn-core/          # domain + use-cases for VPN (no IO)
│  ├─ transfer-over-iroh/        # QUIC endpoints and streams via iroh
│  ├─ client-transfer-sdk/       # high-level send/receive API for any client
│  ├─ repository-storage-rustic/ # adapter around rustic (CLI now, lib later)
│  ├─ filesystem-atomic-writes/  # durable atomic file writes
│  ├─ crypto-keystore/           # key generation, persistence, rotation
│  └─ telemetry-observability/   # tracing, metrics, structured logs
├─ applications/                 # atomic deployables; no business logic inside
│  ├─ backup-ingestion-node/     # server daemon for SBC or x86
│  ├─ backup-laptop-cli/         # laptop CLI client
│  ├─ backup-mobile-client/      # mobile app client (later)
│  ├─ vpn-gateway-node/          # home gateway exposing private routing
│  └─ vpn-access-client/         # client that routes traffic through home
├─ platforms/
│  └─ yocto/                     # images/manifests; no code coupling
```

## Principles

- Shared capabilities live in `crates/*`.
- Deployables live in `applications/*`.
- Traits are declared in context crates, and implemented by adapter crates.
- Explicit naming: no abbreviations, one result + one error type per function.
- No reverse dependencies (`crates/*` never depend on `applications/*`).

## Quickstart

```bash
cargo fmt --all
cargo clippy --all -- -D warnings
cargo test --workspace --no-run
cargo check --workspace
