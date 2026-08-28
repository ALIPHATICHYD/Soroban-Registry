#!/usr/bin/env bash
#
# backup-verify.sh
#
# Checks that the backups which exist are usable and current. It answers three
# questions a monitoring system should ask continuously:
#
#   1. Is the repository internally consistent? (pgbackrest verify)
#   2. Is the newest backup recent enough? (a missed run is a silent failure)
#   3. Is WAL archiving keeping up? (this is what bounds RPO)
#
# It does NOT prove a backup restores; that is scripts/restore-test.sh.
#
# Usage:
#   scripts/backup-verify.sh [--stanza NAME] [--skip-verify] [--json]
#
#   --skip-verify  Skip the checksum pass over the repository, which is slow on
#                  large repositories. Age and WAL checks still run.
#
# Exit codes:
#   0 - every check passed.
#   1 - a check failed; the failing check is named on stderr.

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/lib/backup-env.sh
source "$repo_root/scripts/lib/backup-env.sh"

stanza="${PGBACKREST_STANZA:-soroban-registry}"
skip_verify=false
json_output=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --stanza)
      stanza="${2:?--stanza needs a value}"
      shift 2
      ;;
    --skip-verify)
      skip_verify=true
      shift
      ;;
    --json)
      json_output=true
      shift
      ;;
    -h | --help)
      sed -n '2,23p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "backup-verify.sh: unknown argument '$1'" >&2
      exit 1
      ;;
  esac
done

backup_env::require_pgbackrest
backup_env::export_repo_credentials

failures=()
repo_status="skipped"
backup_age_minutes="unknown"
wal_age_minutes="unknown"

# ── 1. Repository integrity ──────────────────────────────────────────────────
if [[ "$skip_verify" == false ]]; then
  echo "[verify] checking repository checksums for stanza '$stanza'"
  if pgbackrest --stanza="$stanza" verify; then
    repo_status="ok"
  else
    repo_status="failed"
    failures+=("repository checksum verification failed")
  fi
fi

# ── 2. Backup freshness ──────────────────────────────────────────────────────
# pgBackRest reports the newest backup's stop time as a unix timestamp.
latest_stop=$(backup_env::latest_backup_epoch "$stanza")

if [[ -z "$latest_stop" ]]; then
  failures+=("no backup found in stanza '$stanza'")
else
  backup_age_minutes=$(((  $(date -u +%s) - latest_stop ) / 60))
  max_age_minutes=$((BACKUP_MAX_AGE_HOURS * 60))
  echo "[verify] newest backup is ${backup_age_minutes} minute(s) old"
  if [[ "$backup_age_minutes" -gt "$max_age_minutes" ]]; then
    failures+=("newest backup is ${backup_age_minutes}m old, over the ${max_age_minutes}m limit")
  fi
fi

# ── 3. WAL archiving lag, which is what bounds RPO ───────────────────────────
# Connect through DATABASE_URL when there is one, otherwise through the local
# socket, which is the case when this runs on the database host. Skipping this
# check quietly would hide the one measurement that bounds RPO.
psql_target=""
if command -v psql >/dev/null 2>&1; then
  if [[ -n "${DATABASE_URL:-}" ]] && psql "$DATABASE_URL" -tAc "SELECT 1" >/dev/null 2>&1; then
    psql_target="$DATABASE_URL"
  elif psql -d "${PGDATABASE:-${POSTGRES_DB:-postgres}}" -tAc "SELECT 1" >/dev/null 2>&1; then
    psql_target="${PGDATABASE:-${POSTGRES_DB:-postgres}}"
  fi
fi

if [[ -n "$psql_target" ]]; then
  last_archived=$(psql "$psql_target" -tAc \
    "SELECT COALESCE(EXTRACT(EPOCH FROM (now() - last_archived_time))::bigint, -1) FROM pg_stat_archiver" 2>/dev/null || echo "-1")
  failed_count=$(psql "$psql_target" -tAc "SELECT failed_count FROM pg_stat_archiver" 2>/dev/null || echo "0")

  if [[ "$last_archived" == "-1" ]]; then
    failures+=("no WAL segment has been archived yet; check archive_command")
  else
    wal_age_minutes=$((last_archived / 60))
    echo "[verify] newest archived WAL is ${wal_age_minutes} minute(s) old (RPO target ${BACKUP_RPO_MINUTES}m)"
    if [[ "$wal_age_minutes" -gt "$BACKUP_RPO_MINUTES" ]]; then
      failures+=("WAL lag ${wal_age_minutes}m exceeds the ${BACKUP_RPO_MINUTES}m RPO target")
    fi
  fi

  if [[ "$failed_count" -gt 0 ]]; then
    failures+=("pg_stat_archiver reports ${failed_count} failed archive attempt(s)")
  fi
else
  # Not a soft skip: without this check the RPO target is unverified, and an
  # unverified target is the one that fails during an incident.
  failures+=("could not reach the database to measure WAL lag; set DATABASE_URL")
fi

# ── Report ───────────────────────────────────────────────────────────────────
if [[ "$json_output" == true ]]; then
  printf '{"stanza":"%s","repository":"%s","backup_age_minutes":"%s","wal_age_minutes":"%s","rpo_target_minutes":%s,"failures":%d,"status":"%s"}\n' \
    "$stanza" "$repo_status" "$backup_age_minutes" "$wal_age_minutes" \
    "$BACKUP_RPO_MINUTES" "${#failures[@]}" \
    "$([[ ${#failures[@]} -eq 0 ]] && echo ok || echo failed)"
fi

if [[ ${#failures[@]} -gt 0 ]]; then
  echo "[verify] FAILED:" >&2
  for failure in "${failures[@]}"; do
    echo "  - $failure" >&2
  done
  exit 1
fi

echo "[verify] all checks passed"
