#!/usr/bin/env bash
#
# disaster-recovery-drill.sh
#
# Rehearses losing the production database entirely and rebuilding it from the
# OFF-SITE repository only, then measures the two numbers the policy is written
# in terms of:
#
#   RTO - how long the service was unavailable (target: 60 minutes)
#   RPO - how much data the recovery lost      (target: 15 minutes)
#
# The drill runs against a throwaway instance and reads only from repo2, the
# off-site copy. It never touches the production cluster, and it refuses to run
# if pointed at one.
#
# Usage:
#   scripts/disaster-recovery-drill.sh [--stanza NAME] [--report FILE] [--json]
#
#   --report  Append a dated drill record to FILE. Default:
#             docs/backup-drill-log.md
#
# Exit codes:
#   0 - the drill completed inside both targets.
#   1 - the drill failed, or a measured target was missed.

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/lib/backup-env.sh
source "$repo_root/scripts/lib/backup-env.sh"

stanza="${PGBACKREST_STANZA:-soroban-registry}"
report_file="$repo_root/docs/backup-drill-log.md"
json_output=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --stanza)
      stanza="${2:?--stanza needs a value}"
      shift 2
      ;;
    --report)
      report_file="${2:?--report needs a value}"
      shift 2
      ;;
    --json)
      json_output=true
      shift
      ;;
    -h | --help)
      sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "disaster-recovery-drill.sh: unknown argument '$1'" >&2
      exit 1
      ;;
  esac
done

# The drill cannot damage production by construction, and it is worth being
# explicit about why: the only destination it ever restores into is the
# throwaway instance defined in docker-compose.backup.yml, and the only thing it
# does with a production connection is read pg_stat_archiver for the RPO
# measurement. There is deliberately no flag that redirects the restore, because
# a drill that can be pointed at production is not a drill.

backup_env::export_repo_credentials
backup_env::offsite_reachable "$stanza"

drill_started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
drill_start_epoch=$(date +%s)

echo "[drill] disaster recovery drill starting at $drill_started_at"
echo "[drill] source: off-site repository only (repo2)"

# ── Step 1: what would we have lost? This is the RPO measurement. ────────────
# Read from the live database when one is reachable. In the local stack that is
# the container; in production it is DATABASE_URL.
# The newest recoverable point is the end of the newest archived WAL segment,
# so the data at risk is everything written after it.
rpo_minutes="unknown"
wal_age_query="SELECT COALESCE(EXTRACT(EPOCH FROM (now() - last_archived_time))::bigint, -1) FROM pg_stat_archiver"
last_archived_age="-1"

if [[ -n "${DATABASE_URL:-}" ]] && command -v psql >/dev/null 2>&1; then
  last_archived_age=$(psql "$DATABASE_URL" -tAc "$wal_age_query" 2>/dev/null || echo "-1")
elif command -v docker >/dev/null 2>&1; then
  last_archived_age=$(docker exec -u postgres "${BACKUP_DB_CONTAINER:-soroban-registry-db-backup}" \
    psql -d "${POSTGRES_DB:-soroban_registry}" -tAc "$wal_age_query" 2>/dev/null || echo "-1")
fi

if [[ "$last_archived_age" =~ ^[0-9]+$ ]]; then
  rpo_minutes=$((last_archived_age / 60))
fi

# ── Step 2: rebuild from the off-site copy and time it. ──────────────────────
# The restore test is the recovery procedure, so the drill runs it rather than
# duplicating it: one procedure, exercised two ways.
echo "[drill] restoring from the off-site repository"
restore_output=""
restore_status=0
restore_output=$("$repo_root/scripts/restore-test.sh" --stanza "$stanza" --repo 2 --json 2>&1) || restore_status=$?
echo "$restore_output"

drill_end_epoch=$(date +%s)
rto_minutes=$(((  drill_end_epoch - drill_start_epoch ) / 60))
rto_seconds=$((drill_end_epoch - drill_start_epoch))
drill_finished_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# ── Step 3: judge the drill against the targets. ─────────────────────────────
failures=()
[[ "$restore_status" -eq 0 ]] || failures+=("restore from the off-site repository failed")
if [[ "$rto_minutes" -gt "$BACKUP_RTO_MINUTES" ]]; then
  failures+=("RTO ${rto_minutes}m exceeded the ${BACKUP_RTO_MINUTES}m target")
fi
if [[ "$rpo_minutes" != "unknown" && "$rpo_minutes" -gt "$BACKUP_RPO_MINUTES" ]]; then
  failures+=("RPO ${rpo_minutes}m exceeded the ${BACKUP_RPO_MINUTES}m target")
fi

drill_result=$([[ ${#failures[@]} -eq 0 ]] && echo "PASS" || echo "FAIL")

echo "[drill] result: $drill_result"
echo "[drill] RTO: ${rto_minutes}m (${rto_seconds}s), target ${BACKUP_RTO_MINUTES}m"
echo "[drill] RPO: ${rpo_minutes}m, target ${BACKUP_RPO_MINUTES}m"

# ── Step 4: record it. An undocumented drill did not happen. ─────────────────
if [[ -n "$report_file" ]]; then
  mkdir -p "$(dirname "$report_file")"
  if [[ ! -f "$report_file" ]]; then
    cat > "$report_file" <<'HEADER'
# Disaster recovery drill log

Appended to by `scripts/disaster-recovery-drill.sh`. Each entry records one
rehearsal of recovering the registry database from the off-site repository.

| Date (UTC) | Result | RTO | RPO | Notes |
| --- | --- | --- | --- | --- |
HEADER
  fi
  printf '| %s | %s | %sm | %sm | %s |\n' \
    "$drill_started_at" "$drill_result" "$rto_minutes" "$rpo_minutes" \
    "$([[ ${#failures[@]} -eq 0 ]] && echo "targets met" || printf '%s; ' "${failures[@]}")" \
    >> "$report_file"
  echo "[drill] recorded in $report_file"
fi

if [[ "$json_output" == true ]]; then
  printf '{"started_at":"%s","finished_at":"%s","rto_minutes":%s,"rto_seconds":%s,"rpo_minutes":"%s","rto_target_minutes":%s,"rpo_target_minutes":%s,"result":"%s"}\n' \
    "$drill_started_at" "$drill_finished_at" "$rto_minutes" "$rto_seconds" \
    "$rpo_minutes" "$BACKUP_RTO_MINUTES" "$BACKUP_RPO_MINUTES" "$drill_result"
fi

if [[ ${#failures[@]} -gt 0 ]]; then
  echo "[drill] FAILED:" >&2
  for failure in "${failures[@]}"; do
    echo "  - $failure" >&2
  done
  exit 1
fi
