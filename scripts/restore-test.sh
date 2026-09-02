#!/usr/bin/env bash
#
# restore-test.sh
#
# Proves a backup can be restored, which is the only evidence that matters.
# "A backup file exists" is not a backup; "we restored it and the data was
# there" is.
#
# The pipeline:
#
#   create a throwaway PostgreSQL -> restore the newest backup -> replay WAL ->
#   run integrity checks -> run application checks -> destroy it -> report
#
# The throwaway instance is a container, so this never touches the production
# cluster and leaves nothing behind.
#
# Usage:
#   scripts/restore-test.sh [--stanza NAME] [--repo N]
#                           [--target-time "YYYY-MM-DD HH:MM:SS"] [--keep] [--json]
#
#   --repo         Restore from this repository: 1 is local disk, 2 is off-site.
#                  Defaults to pgBackRest's own choice, the first that has the
#                  backup. The disaster recovery drill passes 2 to prove the
#                  off-site copy is usable on its own.
#   --target-time  Restore to a point in time instead of the end of the WAL
#                  stream. Use it to rehearse point-in-time recovery.
#   --keep         Leave the restored instance running for inspection.
#
# Exit codes:
#   0 - the backup restored and every check passed.
#   1 - the restore or a check failed.

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/lib/backup-env.sh
source "$repo_root/scripts/lib/backup-env.sh"

stanza="${PGBACKREST_STANZA:-soroban-registry}"
repo=""
target_time=""
keep_instance=false
json_output=false
compose_files=(-f "$repo_root/docker-compose.backup.yml")
# How long to let WAL replay run before calling the restore stuck.
RESTORE_RECOVERY_TIMEOUT="${RESTORE_RECOVERY_TIMEOUT:-300}"
restore_service="postgres-restore-target"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --stanza)
      stanza="${2:?--stanza needs a value}"
      shift 2
      ;;
    --repo)
      repo="${2:?--repo needs a value}"
      shift 2
      ;;
    --target-time)
      target_time="${2:?--target-time needs a value}"
      shift 2
      ;;
    --keep)
      keep_instance=true
      shift
      ;;
    --json)
      json_output=true
      shift
      ;;
    -h | --help)
      sed -n '2,34p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "restore-test.sh: unknown argument '$1'" >&2
      exit 1
      ;;
  esac
done

if ! docker compose version >/dev/null 2>&1; then
  echo "[restore-test] docker compose is required to create the throwaway instance" >&2
  exit 1
fi

compose() {
  docker compose "${compose_files[@]}" "$@"
}

cleanup() {
  if [[ "$keep_instance" == true ]]; then
    echo "[restore-test] --keep given; leaving '$restore_service' running"
    return
  fi
  echo "[restore-test] destroying the throwaway instance"
  compose --profile restore-test rm --stop --force --volumes "$restore_service" >/dev/null 2>&1 || true
}
trap cleanup EXIT

start_epoch=$(date +%s)

# ── 1. Throwaway PostgreSQL, from nothing ────────────────────────────────────
# Removing the volume first is what makes this a real test: the restore has to
# rebuild the cluster, not patch up an existing one.
echo "[restore-test] discarding any previous restore target"
compose --profile restore-test rm --stop --force --volumes "$restore_service" >/dev/null 2>&1 || true

# `compose rm --volumes` only removes anonymous volumes, so the target's named
# data volume survives with whatever the last run left in it - including a stale
# postmaster.pid, which makes pgBackRest refuse to restore over a cluster it
# believes is running. Empty the directory so this really is a restore from
# nothing, which is what the test claims to prove.
compose --profile restore-test run --rm --user postgres \
  --entrypoint sh "$restore_service" \
  -c 'rm -rf /var/lib/postgresql/data/..?* /var/lib/postgresql/data/.[!.]* /var/lib/postgresql/data/*' \
  >/dev/null 2>&1 || true

# ── 2. Restore the newest backup into the target's data directory ────────────
# --archive-mode=off is not optional for a test restore. The throwaway instance
# promotes when it finishes recovery, and a promoted cluster with archiving on
# pushes its new timeline into the same repository the production database backs
# up to. That pollutes the archive and makes later restores fail with "target
# timeline N forked from backup timeline 1". A verification restore must be
# read-only with respect to the repository.
#
# --target-timeline=current restores the timeline the backup was taken on rather
# than following whatever timeline is newest, which is what a verification wants.
restore_args=(--stanza="$stanza" --delta --archive-mode=off --target-timeline=current)
if [[ -n "$repo" ]]; then
  # Passed on the command line, not through the environment: the restore runs
  # inside a container, which does not inherit the caller's environment.
  restore_args+=(--repo="$repo")
  echo "[restore-test] restoring from repo${repo}"
fi
if [[ -n "$target_time" ]]; then
  # --target-action=promote so the instance opens for writes once it reaches the
  # target, which is what a real recovery does.
  #
  restore_args+=(--type=time --target="$target_time" --target-action=promote)
  echo "[restore-test] restoring to point in time: $target_time (timeline: current)"
else
  echo "[restore-test] restoring to the end of the WAL stream"
fi
restore_args+=(restore)

# A one-off container of the restore-target service: same volumes, but with
# PostgreSQL not running, which is the only state a restore can write into.
# pgBackRest lives in this image because archive_command needs it on the
# database host, so the restore runs here rather than in a sidecar.
compose --profile restore-test run --rm --user postgres \
  --entrypoint pgbackrest "$restore_service" "${restore_args[@]}"

# ── 3. Replay WAL by starting the instance ───────────────────────────────────
echo "[restore-test] starting the restored instance so it replays WAL"
compose --profile restore-test up -d --wait "$restore_service"

restore_epoch=$(date +%s)
restore_seconds=$((restore_epoch - start_epoch))
echo "[restore-test] restore and replay took ${restore_seconds}s"

# ── 4. Integrity checks ──────────────────────────────────────────────────────
psql_in_target() {
  compose --profile restore-test exec -T --user postgres "$restore_service" \
    psql -d "${POSTGRES_DB:-soroban_registry}" -tAc "$1"
}

failures=()

# The container is healthy as soon as it accepts connections, but PostgreSQL
# accepts read-only connections while it is still replaying WAL. Wait for
# recovery to actually finish before judging the restore, or the checks race
# the replay and report a database that is merely a few seconds early.
echo "[restore-test] waiting for WAL replay to finish"
recovery_deadline=$((SECONDS + RESTORE_RECOVERY_TIMEOUT))
while [[ "$(psql_in_target 'SELECT pg_is_in_recovery()' 2>/dev/null || echo t)" != "f" ]]; do
  if [[ $SECONDS -ge $recovery_deadline ]]; then
    failures+=("restored instance was still in recovery after ${RESTORE_RECOVERY_TIMEOUT}s")
    break
  fi
  sleep 2
done

echo "[restore-test] running integrity checks"

# Core tables must exist. These are the ones whose loss would mean losing the
# registry itself.
for table in contracts publishers contract_versions tags; do
  if [[ "$(psql_in_target "SELECT to_regclass('public.$table') IS NOT NULL")" != "t" ]]; then
    failures+=("table '$table' is missing from the restored database")
  fi
done

# Referential integrity and indexes must have survived.
constraint_count=$(psql_in_target "SELECT count(*) FROM pg_constraint WHERE contype = 'f'")
index_count=$(psql_in_target "SELECT count(*) FROM pg_indexes WHERE schemaname = 'public'")
[[ "${constraint_count:-0}" -gt 0 ]] || failures+=("no foreign keys in the restored database")
[[ "${index_count:-0}" -gt 0 ]] || failures+=("no indexes in the restored database")

# Migrations must be present and complete, or the application will not start.
if [[ "$(psql_in_target "SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")" == "t" ]]; then
  dirty=$(psql_in_target "SELECT count(*) FROM _sqlx_migrations WHERE success = false")
  [[ "${dirty:-0}" -eq 0 ]] || failures+=("${dirty} migration(s) recorded as failed")
  applied=$(psql_in_target "SELECT count(*) FROM _sqlx_migrations")
  echo "[restore-test] ${applied} migration(s) present"
else
  failures+=("migration table is missing from the restored database")
fi

contract_count=$(psql_in_target "SELECT count(*) FROM contracts" || echo "0")
publisher_count=$(psql_in_target "SELECT count(*) FROM publishers" || echo "0")
echo "[restore-test] contracts=${contract_count} publishers=${publisher_count}"

# ── 5. Application check ─────────────────────────────────────────────────────
# A restored database that the application cannot open is not a recovery. This
# is deliberately a connection and query round-trip rather than a full boot, so
# the test stays fast enough to run on every backup.
if ! psql_in_target "SELECT 1" >/dev/null; then
  failures+=("application-style connection to the restored database failed")
fi

total_seconds=$(($(date +%s) - start_epoch))

if [[ "$json_output" == true ]]; then
  printf '{"stanza":"%s","repo":"%s","target_time":"%s","restore_seconds":%d,"total_seconds":%d,"contracts":%s,"publishers":%s,"failures":%d,"status":"%s"}\n' \
    "$stanza" "${repo:-auto}" "${target_time:-latest}" "$restore_seconds" "$total_seconds" \
    "${contract_count:-0}" "${publisher_count:-0}" "${#failures[@]}" \
    "$([[ ${#failures[@]} -eq 0 ]] && echo ok || echo failed)"
fi

if [[ ${#failures[@]} -gt 0 ]]; then
  echo "[restore-test] FAILED after ${total_seconds}s:" >&2
  for failure in "${failures[@]}"; do
    echo "  - $failure" >&2
  done
  exit 1
fi

echo "[restore-test] backup restored and verified in ${total_seconds}s"
