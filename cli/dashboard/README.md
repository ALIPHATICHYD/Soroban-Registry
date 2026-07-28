# Soroban Registry Dashboard (Terminal UI)

This folder contains the Node.js/TypeScript implementation of the interactive terminal dashboard (blessed + ws).

The main Soroban-Registry CLI is Rust. The Rust subcommand `soroban-registry dashboard` launches this Node dashboard by running `node dist/index.js ...`.

## Setup

From the repo root:

```bash
cd cli/dashboard
npm install
npm run build
```

## Run (with mock data)

Terminal 1:

```bash
cd cli/dashboard
npm run mock:server
```

Terminal 2 (Rust CLI wrapper):

```bash
cd cli
cargo run --bin soroban-registry -- dashboard --refresh-rate 100 --network testnet --category dex
```

## Run the Node dashboard directly

```bash
cd cli/dashboard
SOROBAN_REGISTRY_WS_URL=ws://127.0.0.1:8787 node dist/index.js dashboard --refresh-rate 100
```

## Environment variables

- `SOROBAN_REGISTRY_WS_URL`: WebSocket URL the dashboard connects to (default: `ws://127.0.0.1:8787`)
- `SOROBAN_REGISTRY_DASHBOARD_ENTRY`: Optional override for the Rust wrapper to locate the built entrypoint

## WebSocket event schema

The dashboard expects JSON messages like:

```json
{
  "type": "deployment_created",
  "payload": {
    "id": "uuid",
    "contractId": "C...",
    "network": "testnet",
    "category": "dex",
    "publisher": "G...",
    "timestamp": "2026-03-27T00:00:00.000Z"
  }
}
```

```json
{
  "type": "contract_interaction",
  "payload": {
    "id": "uuid",
    "contractId": "C...",
    "network": "testnet",
    "timestamp": "2026-03-27T00:00:00.000Z"
  }
}
```

```json
{
  "type": "network_status",
  "payload": {
    "network": "testnet",
    "status": "connected",
    "latencyMs": 42,
    "timestamp": "2026-03-27T00:00:00.000Z"
  }
}
```

The mock server supports basic client messages:

- `{"type":"subscribe","payload":{"filters":{...}}}`
- `{"type":"set_filters","payload":{"filters":{...}}}`
- `{"type":"refresh","payload":{}}`
