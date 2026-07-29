# Data Retention

Proposal history tables grow without bound. Every deploy proposal that expires and
every governance proposal that gets rejected stays in the live table forever,
inflating the indexes on `status`, `updated_at`, and `expires_at` that normal
queries depend on.

The retention task moves those records into archive tables once they fall outside
a configurable window.

## What gets archived

| Live table | Archive table | Retention anchor |
|---|---|---|
| `deploy_proposals` | `deploy_proposals_archive` | `updated_at` |
| `proposal_signatures` | `proposal_signatures_archive` | parent proposal |
| `governance_proposals` | `governance_proposals_archive` | `COALESCE(executed_at, created_at)` |
| `governance_votes` | `governance_votes_archive` | parent proposal |

Records are moved, never hard-deleted. Each table is archived in its own
transaction, so a partial failure never leaves a proposal deleted without its
archive copy. A run stops at the first table that errors; committed work stands
and the next run retries the remainder.

`proposal_signatures` and `governance_votes` are `ON DELETE CASCADE` children.
They are archived alongside their parent, otherwise purging a proposal would
silently destroy its approval trail or voting record.

## Terminal states

Only records in a state they can no longer transition out of are eligible:

| Table | Terminal (eligible) | Active (never archived) |
|---|---|---|
| `deploy_proposals` | `executed`, `expired`, `rejected` | `pending`, `approved` |
| `governance_proposals` | `executed`, `rejected`, `cancelled` | `pending`, `active`, `passed` |

`approved` and `passed` are deliberately excluded. An approved deploy proposal has
collected its signatures but has not been executed; a passed governance proposal
is still waiting out `execution_delay_hours`. Both are live workflow state.

Active records are never archived regardless of age.

### Known limitation on the governance anchor

`governance_proposals` has no `updated_at` column, so a proposal that never
executed falls back to `created_at`. A long-running proposal created 100 days ago
and rejected yesterday is therefore eligible immediately, even though it only just
became terminal. `deploy_proposals` is unaffected — its trigger-maintained
`updated_at` marks the actual transition.

Fixing this properly means adding `updated_at` plus an
`update_updated_at_column()` trigger to `governance_proposals`, which is a schema
change to an existing table and out of scope here.

## Boundary semantics

The cutoff is `now - RETENTION_DAYS`. A record is archived only when its anchor
timestamp is **strictly older** than the cutoff, so a record sitting exactly on
the boundary is still inside the window and is retained.

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| `RETENTION_ENABLED` | `true` | Set to `false` to stop the task from starting |
| `RETENTION_DAYS` | `90` | Length of the retention window |
| `RETENTION_INTERVAL_SECONDS` | `86400` | How often the task runs |
| `RETENTION_BATCH_SIZE` | `1000` | Max records per table per run |

Invalid or zero values fall back to the default with a warning, matching the
behaviour of the rate limiter and cache config.

Batching bounds transaction size on the first run against a large backlog. A
backlog bigger than one batch drains over subsequent runs; lower
`RETENTION_INTERVAL_SECONDS` temporarily to drain it faster.

## Recovering archived records

Archive tables carry no foreign keys and store enum-backed columns as `TEXT`, so
archived rows survive deletion of the contracts, publishers, and policies they
referenced, and stay readable if the live enums change. Restoring means selecting
from the archive table and re-inserting with the appropriate casts.

## Tests

Boundary conditions are covered in `backend/api/src/retention.rs`:

- Pure unit tests for the cutoff predicate — exactly at, just under, and just
  over the threshold, plus active-state exclusion. These run with `cargo test`.
- DB-backed tests that exercise the purge SQL itself. These are `#[ignore]`d
  because they need a migrated Postgres:

  ```bash
  RETENTION_TEST_DATABASE_URL=postgresql://postgres:postgres@localhost/soroban_registry_test \
      cargo test --bin api retention::db_tests -- --ignored --test-threads=1
  ```
