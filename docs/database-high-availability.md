# Database High Availability

This repository uses a three-part PostgreSQL setup in `docker-compose.yml`:

1. `postgres-primary` runs the writable primary.
2. `postgres-replica` follows the primary through streaming replication.
3. `pgpool` sits in front of both nodes and provides read balancing plus automatic failover.

The application services connect to `pgpool` through `DATABASE_URL`, while the API also receives direct primary and replica URLs for lag monitoring.

## Monitoring

The API exports:

- `db_replication_lag_ms`
- `db_replication_wal_lag_bytes`
- `db_replication_health`

Prometheus alerts fire when lag exceeds `100 ms` or replication health drops to an unhealthy state.

## Manual failover

Automatic failover should handle the common case. If an operator needs to step in:

1. Confirm the primary is unhealthy and the replica is current enough to take over.
2. Promote the replica inside the replica container.
3. Restart `pgpool` so new connections route to the promoted node.
4. Update any direct connection strings that still point at the old primary.
5. Verify writes succeed through `DATABASE_URL` and that replication lag returns to normal.

Example commands:

```bash
docker compose exec postgres-replica repmgr standby promote
docker compose restart pgpool
```

## Consistency checks

The API periodically compares the primary WAL position with the replica replay position and warns if the lag target is exceeded. This gives a second signal beyond connection health, which helps catch cases where the replica is alive but no longer close enough to the primary.
