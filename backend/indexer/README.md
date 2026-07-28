# Stellar Blockchain Indexer Service

A production-grade Rust service that continuously monitors the Stellar blockchain, detects smart contract deployments, and automatically synchronizes them into the Soroban Registry database. This eliminates the need for manual contract registration and keeps the registry comprehensive and self-maintaining.

## Architecture

The indexer is built with a modular architecture with clear separation of concerns:

- **RPC Client** (`rpc.rs`) - Stellar RPC endpoint polling (30-second configurable intervals)
- **Detector** (`detector.rs`) - Identifies createContract operations and extracts metadata
- **Database Writer** (`db.rs`) - Inserts contract records with `is_verified = false`
- **State Manager** (`state.rs`) - Tracks last indexed ledger for safe resume after restarts
- **Backoff Handler** (`backoff.rs`) - Exponential backoff for RPC failures
- **Reorg Handler** (`reorg.rs`) - Detects and recovers from ledger reorganizations
- **Configuration** (`config.rs`) - Network and database configuration management

## Features

✅ **Continuous Operation** - Runs as a long-lived background service
✅ **Automatic Recovery** - Resumes from last checkpoint after restarts or crashes
✅ **Multi-Network Support** - Mainnet, Testnet, and Futurenet via environment configuration
✅ **Exponential Backoff** - Handles RPC failures gracefully without crashing
✅ **Reorg Handling** - Detects and recovers from blockchain reorganizations
✅ **Structured Logging** - All operations logged with timestamps and context
✅ **Duplicate Prevention** - Gracefully handles re-processing without errors
✅ **2-Minute Latency** - New contracts appear in database within 2 minutes under normal conditions

## Quick Start

### Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- Access to a Stellar RPC endpoint (Mainnet, Testnet, or Futurenet)

### Installation

1. **Clone and navigate to the indexer**:

   ```bash
   cd backend/indexer
   ```

2. **Set up environment variables** (see Configuration section below)

3. **Build the service**:

   ```bash
   cargo build --release
   ```

4. **Run the service**:
   ```bash
   ./target/release/indexer
   ```

## Configuration

All configuration is externalized through environment variables. No code changes required to switch networks.

### Required Environment Variables

```bash
# Database connection
DATABASE_URL="postgresql://user:password@localhost:5432/soroban_registry"

# Stellar network selection (mainnet, testnet, or futurenet)
STELLAR_NETWORK="testnet"
```

### Optional Environment Variables

```bash
# Custom RPC endpoints (defaults to public endpoints if not set)
STELLAR_RPC_MAINNET="https://rpc-mainnet.stellar.org"
STELLAR_RPC_TESTNET="https://rpc-testnet.stellar.org"
STELLAR_RPC_FUTURENET="https://rpc-futurenet.stellar.org"

# Polling interval in seconds (default: 30, min: 1, max: 300)
STELLAR_POLL_INTERVAL_SECS="30"

# Database connection pooling
DB_MAX_CONNECTIONS="10"

# Exponential backoff configuration
INDEXER_BACKOFF_BASE_SECS="1"      # Initial backoff interval
INDEXER_BACKOFF_MAX_SECS="600"     # Maximum backoff interval (10 minutes)

# Ledger reorg handling
INDEXER_REORG_CHECKPOINT_DEPTH="100"  # Update checkpoint every N ledgers

# Logging configuration
RUST_LOG="indexer=info"  # Set to 'debug' for verbose logging
```

### Example .env File

```bash
# .env
DATABASE_URL=postgresql://postgres:password@localhost:5432/soroban_registry
STELLAR_NETWORK=testnet
STELLAR_POLL_INTERVAL_SECS=30
DB_MAX_CONNECTIONS=10
RUST_LOG=indexer=info
```

## Operational Runbook

### Starting the Service

```bash
# Run directly
./target/release/indexer

# Run with custom log level
RUST_LOG=indexer=debug ./target/release/indexer

# Run in background with systemd (example)
sudo systemctl start soroban-indexer
```

### Monitoring

**Check current state:**

```sql
SELECT * FROM indexer_state WHERE network = 'testnet';
```

**View recent contracts:**

```sql
SELECT contract_id, created_at FROM contracts
WHERE is_verified = false AND network = 'testnet'
ORDER BY created_at DESC
LIMIT 10;
```

**Check for errors:**

```sql
SELECT network, consecutive_failures, error_message, updated_at
FROM indexer_state
ORDER BY updated_at DESC;
```

### Handling a Lagging Indexer

If the indexer falls significantly behind:

1. **Check the logs for errors:**

   ```bash
   tail -f indexer.log | grep ERROR
   ```

2. **Verify RPC endpoint is responsive:**

   ```bash
   curl https://rpc-testnet.stellar.org/health
   ```

3. **Check database connection:**

   ```bash
   psql $DATABASE_URL -c "SELECT 1"
   ```

4. **Manual state adjustment (if needed):**

   ```sql
   -- WARNING: Only do this if you understand the implications
   UPDATE indexer_state
   SET last_indexed_ledger_height = 12345
   WHERE network = 'testnet';
   ```

5. **Restart the service:**
   ```bash
   systemctl restart soroban-indexer
   ```

### Handling a Ledger Reorganization (Reorg)

When a reorg occurs:

1. **Service detects it automatically** - logged as `Reorg detected`
2. **Falls back to last checkpoint** - no data loss
3. **Resumes processing cleanly** - from checkpoint height
4. **Example log output:**
   ```
   WARN: Reorg detected, recovering to checkpoint
   WARN: Recovery completed from ledger 4567
   ```

**Manual recovery if automatic fails:**

```sql
-- View current checkpoint
SELECT last_checkpoint_ledger_height FROM indexer_state
WHERE network = 'testnet';

-- Update to safe checkpoint (example: 100 ledgers back)
UPDATE indexer_state
SET last_indexed_ledger_height = last_checkpoint_ledger_height - 100
WHERE network = 'testnet';
```

### Database Initialization

The service automatically runs migrations on startup. If you need to initialize:

```bash
# Run migrations manually
sqlx migrate run --database-url "$DATABASE_URL"
```

### Graceful Shutdown

Send SIGTERM or SIGINT to allow the service to finish processing:

```bash
# Graceful shutdown
kill -TERM $(pidof indexer)

# Or
pkill -TERM indexer

# Service logs will show: "Received shutdown signal, gracefully exiting..."
```

### Service Health Check

Monitor service health with:

```bash
# Using systemd
systemctl status soroban-indexer

# Check recent logs
journalctl -u soroban-indexer -n 100

# Check database state
SELECT network, consecutive_failures, indexed_at
FROM indexer_state;
```

## Performance Tuning

### Polling Interval

- **Faster indexing (10s):** `STELLAR_POLL_INTERVAL_SECS=10` - More RPC calls
- **Balanced (30s):** `STELLAR_POLL_INTERVAL_SECS=30` - Recommended default
- **Lower load (60s):** `STELLAR_POLL_INTERVAL_SECS=60` - Less frequent indexing

### Database Connections

```bash
# Small deployments
DB_MAX_CONNECTIONS=5

# Large deployments
DB_MAX_CONNECTIONS=20
```

### Backoff Strategy

```bash
# Aggressive recovery (retry quickly)
INDEXER_BACKOFF_BASE_SECS=1
INDEXER_BACKOFF_MAX_SECS=60

# Conservative (avoid hammering failing RPC)
INDEXER_BACKOFF_BASE_SECS=5
INDEXER_BACKOFF_MAX_SECS=600
```

## Logs

The service outputs structured, machine-parseable logs to stdout with the following information:

- **Timestamp** - When the event occurred
- **Level** - INFO, WARN, ERROR
- **Message** - Human-readable description
- **Context** - Ledger height, contract ID, network, attempt count, etc.

**Example log entries:**

```
INFO: Stellar Blockchain Indexer Service starting...
INFO: Network configuration loaded: network=testnet, endpoint=https://rpc-testnet.stellar.org, poll_interval=30s
INFO: Loaded indexer state: last_indexed_ledger=45678
INFO: RPC endpoint health check passed
INFO: Poll cycle started: network=testnet, latest_ledger=45680, next_ledger=45679, lag=1
INFO: Fetched ledger operations: ledger=45679, operations=8
INFO: Found contract deployments: ledger=45679, contracts=1
INFO: Contracts written to database: ledger=45679, new=1, duplicates=0
INFO: Poll cycle completed successfully: network=testnet, processed=1, new_contracts=1
WARN: Reorg detected, recovering to checkpoint
ERROR: Failed to fetch ledger operations: RPC timeout
WARN: attempt=2, backoff_secs=2, Backing off before retry
```

## Database Schema

The indexer uses these tables:

### indexer_state

Tracks indexing progress per network:

```sql
CREATE TABLE indexer_state (
    id SERIAL PRIMARY KEY,
    network network_type UNIQUE,
    last_indexed_ledger_height BIGINT,
    last_checkpoint_ledger_height BIGINT,
    indexed_at TIMESTAMPTZ,
    checkpoint_at TIMESTAMPTZ,
    error_message TEXT,
    consecutive_failures INT,
    updated_at TIMESTAMPTZ
);
```

### contracts

Discovered contract records:

```sql
CREATE TABLE contracts (
    id UUID PRIMARY KEY,
    contract_id VARCHAR(56),
    network network_type,
    is_verified BOOLEAN,  -- false for auto-discovered contracts
    created_at TIMESTAMPTZ,
    ...
);
```

## Testing

Run all tests:

```bash
cargo test
```

Run specific module tests:

```bash
cargo test backoff::tests
cargo test detector::tests
cargo test state::tests
```

Run with output:

```bash
cargo test -- --nocapture
```

## Acceptance Criteria Verification

### ✅ Service Stability (24+ hours)

- [x] No unwrap() calls in production code paths
- [x] All errors handled explicitly with Result types
- [x] Graceful shutdown via SIGTERM/SIGINT
- [x] Exponential backoff prevents RPC hammering

### ✅ Polling Interval (30 seconds)

- [x] Configurable via STELLAR_POLL_INTERVAL_SECS
- [x] Validated at startup (1-300 second range)
- [x] No hardcoded intervals

### ✅ Contract Detection

- [x] Identifies createContract operations (type_code 110)
- [x] Extracts contract ID and deployer address
- [x] Validates contract ID format (56 chars, starts with 'C')

### ✅ Database Records

- [x] Contracts inserted with is_verified = false
- [x] Duplicate handling via existence checks
- [x] Automatic publisher record creation

### ✅ State Persistence

- [x] Last ledger height stored in indexer_state table
- [x] Resume on restart without re-processing
- [x] Atomic updates - state only advanced after successful DB write

### ✅ Exponential Backoff

- [x] Triggered on RPC failures
- [x] Doubles interval on each retry
- [x] Capped at configurable maximum
- [x] Every retry logged with timestamp and reason

### ✅ Reorg Handling

- [x] Detects ledger reorgs
- [x] Falls back to checkpoint
- [x] Resumes cleanly without state corruption

### ✅ Multi-Network Support

- [x] Mainnet, Testnet, Futurenet via STELLAR_NETWORK
- [x] No hardcoded values
- [x] No code changes required for network switch

### ✅ Structured Logging

- [x] Timestamps on all entries
- [x] Ledger height included
- [x] Contract IDs logged
- [x] Error and backoff events logged

### ✅ 2-Minute Latency

- [x] 30-second polling interval
- [x] <10 seconds processing
- [x] <20 seconds DB write
- [x] Total ~110 seconds at worst case (well under 2 minutes)

## Troubleshooting

### "Failed to load indexer state"

- Check database connection string
- Verify migrations have run
- Check database permissions

### "RPC endpoint health check failed"

- Verify endpoint URL is correct
- Check network connectivity
- Try alternative RPC endpoint

### "Reorg detected repeatedly"

- Check if RPC node is synced
- Consider longer checkpoint depth
- May indicate network instability

### "High consecutive failures"

- Check logs for specific errors
- Verify database is accessible
- Check RPC endpoint availability

## Contributing

To add new features or modify the indexer:

1. Keep modules focused on single responsibility
2. Add appropriate error handling (no unwrap in production paths)
3. Write tests for new logic
4. Update configuration docs
5. Run full test suite before PR

## License

MIT - See LICENSE file in repository
