# Runbook: recovering the database

Three procedures. Pick by what is broken:

- [Data is damaged, host is fine](#data-is-damaged-host-is-fine) - corruption, a
  bad bulk update, a dropped table.
- [Undo a specific change](#undo-a-specific-change) - a bad migration or an
  accidental `DELETE` with a knowable time.
- [The host is gone](#the-host-is-gone) - dead machine, destroyed volume, lost
  region.

Targets throughout: service back inside 1 hour, at most 15 minutes of writes
lost. Background and troubleshooting are in
[backup-and-recovery.md](./backup-and-recovery.md).

---

## Data is damaged, host is fine

### 1. Stop writes and note the time

RTO is measured from the failure, not from when you started.

```bash
docker compose stop postgres-primary postgres-replica
```

The replica is stopped too: a replica of a damaged primary is also damaged.

### 2. Check what you can recover to

```bash
pgbackrest --stanza=soroban-registry info
```

The newest recoverable point is the end of the newest archived segment.

### 3. Restore and start

```bash
docker compose exec -u postgres postgres-backup \
  pgbackrest --stanza=soroban-registry --delta restore

docker compose start postgres-primary
docker compose logs -f postgres-primary
```

`--delta` restores only files that differ, which is what keeps this in minutes.
Add `--repo=2` for the off-site copy. Wait for `database system is ready to
accept connections`.

### 4. Verify, then restore service

Run the [verification checklist](#verification-checklist), then:

```bash
docker compose up -d postgres-replica
docker compose exec postgres-replica repmgr standby clone --force
docker compose up -d api indexer
curl -fsS http://localhost:3001/health
```

Finish with [afterwards](#afterwards).

---

## Undo a specific change

Everything written after the target is discarded. Be sure of the target.

### 1. Pin down the target

Recover to just *before* the damage, not when you noticed it.

```sql
SELECT created_at, action, actor FROM audit_logs ORDER BY created_at DESC LIMIT 20;
SELECT version, description, installed_on FROM _sqlx_migrations ORDER BY installed_on DESC LIMIT 5;
```

Write it down in UTC, a few seconds early: `2026-08-24 14:36:50`. It must fall
between a backup and the end of the newest archived WAL (`pgbackrest info`); if
the covering WAL is not archived yet, take an incremental backup first.

### 2. Rehearse it

A few minutes on a throwaway instance tells you the target is right before you
discard anything:

```bash
./scripts/restore-test.sh --target-time "2026-08-24 14:36:50" --keep
```

### 3. Recover

```bash
docker compose stop postgres-primary postgres-replica

docker compose exec -u postgres postgres-backup pgbackrest \
  --stanza=soroban-registry --delta \
  --type=time --target="2026-08-24 14:36:50" --target-action=promote \
  --target-timeline=current \
  restore

docker compose start postgres-primary
docker compose logs -f postgres-primary
```

`--target-action=promote` opens the database for writes at the target.

`--target-timeline=current` is not optional in practice: every previous
promotion created a timeline, PostgreSQL follows the newest by default, and if
that one branched before your target, recovery stops with "recovery ended before
configured recovery target was reached".

### 4. Verify, then restore service

Run the [verification checklist](#verification-checklist), and also confirm the
newest surviving row sits just before the target:

```bash
docker compose exec -u postgres postgres-primary psql -d soroban_registry -c \
  "SELECT max(created_at) FROM contracts;"
```

A wrong target can be redone: restores do not change the backups or the WAL.
Then bring up the replica and services as above, and tell whoever depends on the
discarded window what was lost.

---

## The host is gone

The local repository died with the host, so everything comes from the off-site
copy. This is what the monthly drill rehearses.

### What you need

- [ ] `CLOUDFLARE_ACCESS_KEY_ID`, `CLOUDFLARE_SECRET_ACCESS_KEY`,
      `CLOUDFLARE_R2_BUCKET`, `CLOUDFLARE_R2_ENDPOINT`
- [ ] `BACKUP_ENCRYPTION_KEY` - the passphrase the off-site repository was
      written with. Without it, nothing here works.
- [ ] A host with Docker, or PostgreSQL 16 and pgBackRest

If those live only on the machine that died, fix that first: it is the single
point of failure that makes every other step pointless.

### 1. Provision and confirm the repository answers

```bash
git clone https://github.com/ALIPHATICHYD/Soroban-Registry.git
cd Soroban-Registry
cp .env.example .env      # fill from the secret manager

docker compose -f docker-compose.backup.yml up -d --wait postgres-backup
docker compose exec -u postgres postgres-backup \
  pgbackrest --stanza=soroban-registry --repo=2 info
```

Nothing else is needed from the old host: the repository holds the database,
this repo holds the configuration. An error here is credentials, endpoint, or
passphrase, in that order of likelihood.

### 2. Restore

```bash
docker compose exec -u postgres postgres-backup pgbackrest \
  --stanza=soroban-registry --repo=2 --delta restore

docker compose start postgres-backup
docker compose logs -f postgres-backup
```

This is the long step, bounded by how fast the host can pull the backup. It is
the number to watch when the RTO is at risk.

### 3. Verify, then bring the stack up

Run the [verification checklist](#verification-checklist), then:

```bash
docker compose up -d
curl -fsS http://localhost:3001/health
```

- [ ] Replica cloned, replication lag normal
- [ ] DNS or the load balancer points here

### 4. Re-establish protection

The new host has no local repository and is not archiving yet. Until this is
done, the recovery you just performed cannot be repeated.

```bash
./scripts/backup.sh --type full
./scripts/backup-verify.sh
```

---

## Verification checklist

Before letting traffic in, whichever procedure you ran:

```bash
docker compose exec -u postgres postgres-primary psql -d soroban_registry -c \
  "SELECT pg_is_in_recovery(); SELECT count(*) FROM contracts; SELECT count(*) FROM publishers;"
docker compose exec -u postgres postgres-primary psql -d soroban_registry -c \
  "SELECT count(*) FROM _sqlx_migrations WHERE success = false;"
```

- [ ] `pg_is_in_recovery()` is `f`
- [ ] Core tables present, row counts plausible
- [ ] No failed migrations

## Afterwards

- [ ] Take a fresh full backup: the recovered cluster starts a new WAL timeline.
- [ ] Record the incident, its timings and the measured RTO and RPO in
      [backup-drill-log.md](./backup-drill-log.md).
- [ ] Note which step took longest; that is what to optimise before the next
      drill.
- [ ] If recovery exceeded 1 hour, treat the gap as a defect with an owner, not
      as a revised target.

```bash
./scripts/backup.sh --type full
```
