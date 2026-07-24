# Soroban Registry

A contract registry and package manager for the Soroban smart‑contract ecosystem on Stellar.

Soroban Registry lets developers publish, discover, and verify Soroban contracts across Stellar networks, similar to how npm and crates.io serve JavaScript and Rust communities.[cite:352]

> Production: https://soroban-registry.vercel.app/

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)
![TypeScript](https://img.shields.io/badge/typescript-5.0%2B-blue.svg)

---

## Features

- **Registry & Discovery** – Search and browse contracts by network, tags, category, and publisher.
- **Source Verification** – Verify that on‑chain bytecode matches published source.
- **Versioning & Changelogs** – Track versions, semver compatibility, and breaking changes.
- **Multi‑Network Support** – Mainnet, Testnet, and Futurenet in a single registry.[cite:352]
- **Publisher Profiles** – Attach contracts to publishers and their deployment history.
- **Analytics** – Usage statistics and interaction metrics for contracts.
- **Web App + CLI** – Next.js frontend for browsing; Rust CLI for developer workflows.

---

## Project Layout

```text
soroban-registry/
├── backend/        # Rust backend services (Axum API, indexer, verifier)
├── frontend/       # Next.js web application
├── cli/            # Rust CLI tool
├── database/       # PostgreSQL migrations
└── examples/       # Example contracts
```

---

## Prerequisites

- **Rust** 1.75+ – https://rustup.rs/
- **Node.js** 20+ – https://nodejs.org/
- **PostgreSQL** 16+ – https://www.postgresql.org/download/
- **Docker** (optional, recommended for local all‑in‑one setup)[cite:352]

---

## Quick Start

### 1. Clone and configure

```bash
git clone https://github.com/ALIPHATICHYD/Soroban-Registry.git
cd Soroban-Registry

cp .env.example .env
```

### 2. Run everything with Docker (recommended)

```bash
docker-compose up -d

# API:      http://localhost:3001
# Frontend: http://localhost:3000
```

This starts the PostgreSQL HA pair behind `pgpool`, the backend API, and the Next.js frontend with sensible defaults.[cite:352]

The Docker stack now uses a primary/replica PostgreSQL pair behind `pgpool`, so the API and other database-backed services get read balancing plus automatic failover. If you want the operator runbook, see [docs/database-high-availability.md](docs/database-high-availability.md).

---

## Running From Source

### Database

The backend uses SQLx's compile-time-checked query macros (`sqlx::query!`,
`sqlx::query_as!`, `sqlx::query_scalar!`). They require **either** a live
database reachable at `DATABASE_URL` **or** prepared offline query data under
`backend/.sqlx/` (`SQLX_OFFLINE=true`). Without one of these, `cargo build`
fails with errors like *"set `DATABASE_URL` to use query macros online, or run
`cargo sqlx prepare` to update the query cache"*.

```bash
# 1. Create the database
createdb soroban_registry

# 2. Export DATABASE_URL — needed at compile time AND at runtime
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/soroban_registry"

# 3. Install the SQLx CLI (one-time)
cargo install sqlx-cli --version 0.8.6 --no-default-features --features rustls,postgres --locked

# 4. Apply migrations so the schema matches what the query macros expect
sqlx migrate run --source database/migrations

# 5. (Optional) Verify the environment is ready to build
./scripts/check-sqlx-env.sh
```

#### Building without a live database (offline mode)

If your build environment cannot reach a Postgres instance (e.g. some CI
runners, contributor laptops), generate offline query metadata once from a
machine that can:

```bash
# From the backend workspace, with DATABASE_URL pointing at a fully
# migrated database
cd backend
cargo sqlx prepare --workspace -- --all-targets
git add .sqlx
```

Then anyone (or any CI job) can build without a database by setting:

```bash
export SQLX_OFFLINE=true
```

Re-run `cargo sqlx prepare` whenever a query macro or migration changes.

### Persistent PostgreSQL

The repository's `docker-compose.yml` defines named volumes for the primary and replica PostgreSQL nodes. That means each database survives container restarts and `docker-compose down`; your data is only removed if you explicitly delete the volumes.

```bash
# Start or reattach to the HA database stack
docker-compose up -d postgres-primary postgres-replica pgpool

# Apply migrations against the persistent database
docker-compose exec postgres-primary psql -U postgres -d soroban_registry -c "SELECT 1"
sqlx migrate run --source database/migrations

# Stop services without deleting data
docker-compose down

# Remove the database data only if you want a clean slate
docker-compose down -v
```

Use the same `DATABASE_URL` on future runs so the backend, SQLx checks, and any local tools all connect through `pgpool` to the same replicated database cluster.

### Backend API

```bash
cd backend
cargo build --release
cargo run --bin api
```

The API server will listen on the address configured in your `.env` (commonly `http://localhost:3001`).[cite:352]

### Frontend

```bash
cd frontend
pnpm install
pnpm dev
```

Visit `http://localhost:3000` to browse the registry UI.

---

## Installing and Using the CLI

The CLI lets you interact with the registry directly from your terminal.[cite:352]

### Install from source

```bash
# From the repo root
cargo install --path cli
```

This installs a `soroban-registry` binary into your Cargo bin directory.

### Common commands

```bash
# Search for contracts
soroban-registry search "token" --category defi --verified-only --network testnet,futurenet

# Get contract details
soroban-registry info <contract-id>

# Publish a contract
soroban-registry publish --contract-path ./my-contract

# Verify a contract against source
soroban-registry verify <contract-id> --source ./src

# View registry analytics for contracts
soroban-registry contract stats --network testnet --top-n 5 --format table

# Export contract registry data for backup or migration
soroban-registry contract export contracts.jsonl --format jsonl --network testnet --compress
```

Configuration is stored at `~/.soroban-registry/config.toml`. A legacy `~/.soroban-registry.toml` file is migrated automatically if present.[cite:352]

---

## API Overview

The backend exposes a REST API suitable for integration with dashboards, bots, and CI:

- `GET /api/contracts` – List and search contracts
- `GET /api/contracts/:id` – Contract details
- `POST /api/contracts` – Publish a new contract
- `GET /api/contracts/:id/versions` – Version history
- `GET /api/contracts/:id/changelog` – Changelog with breaking‑change markers
- `GET /api/publishers/:id` – Publisher details
- `GET /api/publishers/:id/contracts` – Contracts by publisher
- `GET /api/stats` – Registry‑level stats
- `GET /health` – Health check endpoint[cite:352]

See the OpenAPI spec (coming soon) or the `backend/api` handlers for full details.

---

## Contributing

Contributions from the Stellar/Soroban community are welcome.

1. **Fork** the repository.
2. **Create a branch**: `git checkout -b feature/short-description`
3. **Make changes** and add tests where appropriate.
4. **Run checks**:
   ```bash
   # Rust
   cargo fmt --all
   cargo test --all

   # TypeScript
   cd frontend
   pnpm lint
   pnpm test
   ```
5. **Commit**: `git commit -m "feat: add <short description>"`
6. **Push and open a PR** against `main`.

Bug reports and feature requests can be filed as GitHub Issues:
https://github.com/ALIPHATICHYD/Soroban-Registry/issues

---

## Community & Support

- Soroban SDK – https://github.com/stellar/rs-soroban-sdk
- Stellar Docs – https://developers.stellar.org/
- Stellar Community Discord – https://discord.gg/stellar[cite:352]

---

## License

Soroban Registry is licensed under the MIT License. See [LICENSE](LICENSE) for details.[cite:352]
