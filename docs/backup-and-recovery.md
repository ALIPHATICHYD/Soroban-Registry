# Backup and Recovery

The database is backed up by [pgBackRest](https://pgbackrest.org) to two
repositories, and every backup is proven by restoring it.

Replication ([database-high-availability.md](./database-high-availability.md))
protects against a node dying. Backups protect against what replication cannot:
a bad migration, a wrong `DELETE`, corruption replicated faithfully to the
replica, or losing the environment.

## Targets

| Target | Value |
| --- | --- |
| RTO (service restored) | 1 hour |
| RPO (data lost) | 15 minutes |

Both are asserted, not just documented: `backup-verify.sh` fails when WAL lag
exceeds the RPO, and `disaster-recovery-drill.sh` fails when a rehearsed
recovery exceeds the RTO. The values live in `.env.example` and are read from
one place, `scripts/lib/backup-env.sh`.

## Architecture

```text
              postgres (pgBackRest installed alongside it)
                               |
                     archive_command, async
                               |
              +----------------+----------------+
              |                                 |
       repo1: local disk               repo2: Cloudflare R2
       fast restores                   encrypted, survives the host
```

`archive_command` runs inside PostgreSQL's environment, so pgBackRest must be on
the database host - not in a sidecar.

WAL archiving fans out to both repositories on its own, but `pgbackrest backup`
writes to **one** repository per invocation, so `backup.sh` runs it once per
repository and reports success only when all of them have it. A failure on one
does not abort the others.

## Schedule

| When | What |
| --- | --- |
| Sunday | Full backup |
| Mon-Sat | Incremental backup |
| Continuously | WAL archived as segments fill, and at least every 60s |
| Daily | `backup-verify.sh` |
| Weekly | `restore-test.sh` |
| Monthly | `disaster-recovery-drill.sh` |

`archive_timeout=60s` bounds the RPO: an idle database still ships a segment
every minute.

```cron
17 2 * * * /opt/soroban-registry/scripts/backup.sh --json >> /var/log/backup.log 2>&1
47 3 * * * /opt/soroban-registry/scripts/backup-verify.sh --json >> /var/log/backup.log 2>&1
```

## Configuration

| File | Purpose |
| --- | --- |
| `database/pgbackrest/pgbackrest.conf` | Repositories, retention, compression, encryption |
| `database/pgbackrest/postgresql.backup.conf` | `wal_level`, `archive_mode`, `archive_command`, `archive_timeout` |
| `docker-compose.backup.yml` | Local stack: PostgreSQL with pgBackRest, plus MinIO standing in for R2 |
| `.env.example` | Credentials and targets |

Secrets come from the environment as `PGBACKREST_<OPTION>`;
`scripts/lib/backup-env.sh` maps this repository's `CLOUDFLARE_*` variables onto
those names. Required in production: `CLOUDFLARE_ACCESS_KEY_ID`,
`CLOUDFLARE_SECRET_ACCESS_KEY`, `CLOUDFLARE_R2_BUCKET`, `CLOUDFLARE_R2_ENDPOINT`,
`BACKUP_ENCRYPTION_KEY`.

`BACKUP_ENCRYPTION_KEY` encrypts the off-site repository. Losing it means those
backups cannot be restored, so keep it in a secret manager.

`repo2-bundle=y` is not optional. A data directory is ~1300 mostly-tiny files;
unbundled, each becomes one object, one round trip and one billed write.
Measured against R2: 375s and 1290 objects unbundled, 23s and 7 bundled.

## Scripts

All take `--json`, and document their exit codes in their own header.

| Script | Runs | Does |
| --- | --- | --- |
| `backup.sh` | database host | Full or incremental into every repository |
| `backup-verify.sh` | database host | Checksums, backup age, WAL lag against RPO |
| `restore-test.sh` | anywhere with Docker | Restores into a throwaway instance and validates it |
| `disaster-recovery-drill.sh` | anywhere with Docker | Rebuilds from off-site only, measures RTO and RPO |
| `pitr-rehearsal.sh` | anywhere with PostgreSQL | Proves point-in-time recovery with no Docker or cloud account |

## Verification

A backup that has never been restored is a hypothesis. `restore-test.sh` builds
a throwaway PostgreSQL, restores into it, waits for WAL replay, then checks
tables, foreign keys, indexes, migrations and row counts before destroying it. A
failure there means the backups are not backups.

It restores with `--archive-mode=off`: the throwaway promotes when recovery
finishes, and a promoted cluster with archiving on would push its new timeline
into the repository production backs up to.

## Local stack

```bash
docker compose -f docker-compose.backup.yml up -d --wait
docker exec -u postgres soroban-registry-db-backup pgbackrest --stanza=soroban-registry stanza-create
docker exec -u postgres soroban-registry-db-backup bash /opt/scripts/backup.sh --type full
./scripts/restore-test.sh --repo 2 --json
./scripts/disaster-recovery-drill.sh --json
```

MinIO stands in for R2 and serves HTTPS with a self-signed certificate, because
pgBackRest always speaks TLS to an S3 endpoint. To point the local stack at real
R2, create `database/pgbackrest/pgbackrest.local.env` (gitignored) with the
`PGBACKREST_REPO2_*` values; it overrides the MinIO defaults.

### Exercising the whole cycle

Fifteen minutes, touching nothing outside its own containers. `backup.sh` and
`backup-verify.sh` run inside the database container, where pgBackRest is;
`restore-test.sh` and the drill run on your machine, because they create and
destroy containers.

```bash
export POSTGRES_PASSWORD=localdev
db() { docker exec -u postgres soroban-registry-db-backup "$@"; }

# 1. Start, and create the repository layout
docker compose -f docker-compose.backup.yml up -d --wait
db pgbackrest --stanza=soroban-registry stanza-create

# 2. Give it something to lose
db psql -d soroban_registry -c "CREATE TABLE IF NOT EXISTS contracts (id bigserial PRIMARY KEY, name text NOT NULL, created_at timestamptz NOT NULL DEFAULT now()); INSERT INTO contracts (name) SELECT 'contract-' || g FROM generate_series(1,120) g; SELECT count(*) FROM contracts;"

# 3. Back up, inspect, verify. The WAL lag reported here is your RPO.
db bash /opt/scripts/backup.sh --type full --json
db pgbackrest --stanza=soroban-registry info
db bash /opt/scripts/backup-verify.sh --json

# 4. Prove it restores, from local and from off-site alone
./scripts/restore-test.sh --json
./scripts/restore-test.sh --repo 2 --json

# 5. Point-in-time recovery: note the time, break something, rewind past it
TARGET=$(db psql -d soroban_registry -tAc "SELECT now()" | cut -d. -f1)
db psql -d soroban_registry -c "INSERT INTO contracts (name) SELECT 'accident-' || g FROM generate_series(1,300) g;"
db bash /opt/scripts/backup.sh --type incr
./scripts/restore-test.sh --target-time "$TARGET" --json   # expect 120, not 420

# 6. Full drill, then clean up
./scripts/disaster-recovery-drill.sh --json
docker compose -f docker-compose.backup.yml --profile restore-test down -v
```

## Before relying on this in production

Run once from a host resembling production, because four things only appear
against the real thing: credentials and bucket permissions, R2's endpoint
format, TLS verification (on in production, off for the local self-signed
certificate), and real transfer times.

```bash
pgbackrest --stanza=soroban-registry stanza-create
pgbackrest --stanza=soroban-registry --repo=2 info   # credentials, endpoint, passphrase
./scripts/backup.sh --type full --repos 2
./scripts/restore-test.sh --repo 2 --json
./scripts/disaster-recovery-drill.sh --json
```

The RTO from that last run is the one to trust. If it exceeds an hour, that is a
finding about bandwidth or database size, not a reason to revise the target.

## Restoring

[backup-recovery-runbook.md](./backup-recovery-runbook.md) has the three
procedures: damaged data, undoing a specific change, and rebuilding a lost host.

`disaster-recovery-drill.sh` runs monthly, reads only from the off-site
repository, and appends its result to
[backup-drill-log.md](./backup-drill-log.md). A drill that is not recorded did
not happen.

## Troubleshooting

**WAL lag over the RPO.** Archiving is behind or broken. Check
`pg_stat_archiver.failed_count` and the pgBackRest log; usually expired
object-storage credentials or a full spool directory.

**`role "root" does not exist`.** pgBackRest connected as the OS user running
it. Run as postgres (`docker exec -u postgres`) and set `pg1-user=postgres`.

**`unable to acquire lock ... Permission denied`.** Its lock, log and spool
directories must be writable by the user running the command.

**Backup to object storage takes minutes for a few megabytes.** Object count,
not bandwidth - see `repo2-bundle=y` above.

**`backup and archive info files exist but do not match the database`.** The
repository belongs to a different cluster; `initdb` mints a new system
identifier, so this follows a rebuilt host or a `down -v` against a stack whose
off-site repository persists. If the backups are still wanted, restore *from*
the repository instead. If not, reset the stanza - **this deletes every backup
in it**:

```bash
pgbackrest --stanza=soroban-registry stop
pgbackrest --stanza=soroban-registry --repo=1 stanza-delete --force
pgbackrest --stanza=soroban-registry stop          # each delete consumes the stop file
pgbackrest --stanza=soroban-registry --repo=2 stanza-delete --force
pgbackrest --stanza=soroban-registry start
pgbackrest --stanza=soroban-registry stanza-create
```

**`target timeline N forked from backup timeline 1`.** Something restored from
this repository, promoted, and archived its new timeline back into it. A restore
that promotes must not write to the repository it restored from: pass
`--archive-mode=off`.

**Recovery ends before reaching the target time.** Either the target is beyond
the archived WAL, or recovery followed a newer timeline that branched before it.
Pin the backup's own timeline with `--target-timeline=current`.

**Off-site restore fails to decrypt.** `BACKUP_ENCRYPTION_KEY` does not match
the passphrase the backup was written with. There is no recovery from a lost
passphrase.
