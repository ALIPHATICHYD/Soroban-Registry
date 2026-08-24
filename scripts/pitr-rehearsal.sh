#!/usr/bin/env bash
#
# pitr-rehearsal.sh
#
# Rehearses point-in-time recovery end to end using only PostgreSQL's own
# tools, on a throwaway cluster. No Docker, no pgBackRest, no cloud account.
#
# It exists so the recovery procedure can be proven on a laptop and in CI, on
# every change, rather than only during a quarterly drill against real
# infrastructure. The mechanics it exercises - continuous archiving, base
# backup, WAL replay, recovery to a chosen timestamp - are exactly the ones
# pgBackRest drives in production.
#
# What it proves:
#
#   1. Writes made BEFORE the recovery target survive recovery.
#   2. Writes made AFTER the recovery target do not.
#   3. The recovery completes inside the RTO target.
#
# Usage:
#   scripts/pitr-rehearsal.sh [--port PORT] [--keep] [--json]
#
#   --keep  Leave the rehearsal directory in place for inspection.
#
# Exit codes:
#   0 - recovery landed exactly on the target and met the RTO target.
#   1 - recovery failed, lost committed data, or replayed past the target.

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/lib/backup-env.sh
source "$repo_root/scripts/lib/backup-env.sh"

port="${PITR_PORT:-55432}"
keep_workdir=false
json_output=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      port="${2:?--port needs a value}"
      shift 2
      ;;
    --keep)
      keep_workdir=true
      shift
      ;;
    --json)
      json_output=true
      shift
      ;;
    -h | --help)
      sed -n '2,28p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "pitr-rehearsal.sh: unknown argument '$1'" >&2
      exit 1
      ;;
  esac
done

# A fixed locale keeps the rehearsal reproducible across machines, and macOS
# refuses to start a postmaster at all when the inherited locale pulls in a
# multithreaded ICU path.
export LC_ALL=C
export LANG=C

for binary in initdb pg_ctl pg_basebackup psql; do
  if ! command -v "$binary" >/dev/null 2>&1; then
    echo "[pitr] $binary is not on PATH; install PostgreSQL client and server tools" >&2
    exit 1
  fi
done

workdir=$(mktemp -d "${TMPDIR:-/tmp}/soroban-pitr.XXXXXX")
primary_dir="$workdir/primary"
archive_dir="$workdir/wal-archive"
basebackup_dir="$workdir/basebackup"
restored_dir="$workdir/restored"
logfile="$workdir/postgres.log"
restore_logfile="$workdir/restored.log"

cleanup() {
  pg_ctl -D "$restored_dir" -m immediate stop >/dev/null 2>&1 || true
  pg_ctl -D "$primary_dir" -m immediate stop >/dev/null 2>&1 || true
  if [[ "$keep_workdir" == true ]]; then
    echo "[pitr] --keep given; rehearsal directory left at $workdir"
  else
    rm -rf "$workdir"
  fi
}
trap cleanup EXIT

mkdir -p "$archive_dir"

# ── 1. A cluster that archives WAL, like production ──────────────────────────
echo "[pitr] creating a throwaway cluster in $primary_dir"
initdb -D "$primary_dir" --auth=trust --username=postgres >/dev/null

cat >> "$primary_dir/postgresql.conf" <<CONF
port = $port
listen_addresses = 'localhost'
# Keep the socket inside the rehearsal directory. The compiled-in default on
# Debian and Ubuntu is /var/run/postgresql, which only the postgres user can
# write to, so an unprivileged run (a CI runner, a developer laptop) would fail
# to start the postmaster at all.
unix_socket_directories = '$workdir'
wal_level = replica
archive_mode = on
# Copy to a temporary name and rename into place: a rename is atomic, so
# recovery can never observe a half-written segment. pgBackRest's archive-push
# does the same thing, which is one reason production does not hand-roll this.
archive_command = 'test ! -f $archive_dir/%f && cp %p $archive_dir/%f.tmp && mv $archive_dir/%f.tmp $archive_dir/%f'
archive_timeout = 5s
CONF

pg_ctl -D "$primary_dir" -l "$logfile" -w start >/dev/null
run_sql() { psql -h localhost -p "$port" -U postgres -d postgres -tAc "$1"; }

# ── 2. Data that must survive, then a base backup ────────────────────────────
run_sql "CREATE TABLE contracts (id bigserial PRIMARY KEY, name text NOT NULL, created_at timestamptz NOT NULL DEFAULT now())" >/dev/null
run_sql "INSERT INTO contracts (name) SELECT 'before-backup-' || generate_series(1, 50)" >/dev/null

echo "[pitr] taking a base backup"
pg_basebackup -h localhost -p "$port" -U postgres -D "$basebackup_dir" -X stream -c fast >/dev/null

# ── 3. More writes, then the moment we will recover to ───────────────────────
run_sql "INSERT INTO contracts (name) SELECT 'after-backup-' || generate_series(1, 25)" >/dev/null
expected_rows=$(run_sql "SELECT count(*) FROM contracts")

# Everything committed up to this instant must survive recovery.
recovery_target=$(run_sql "SELECT now()")
sleep 1

# ── 4. The damage we are recovering from ─────────────────────────────────────
# This models the common reason to reach for PITR: a bad migration or an
# accidental DELETE that has to be undone by rewinding the database to just
# before it happened.
run_sql "INSERT INTO contracts (name) SELECT 'lost-write-' || generate_series(1, 40)" >/dev/null
rows_before_failure=$(run_sql "SELECT count(*) FROM contracts")

# Archiving is what makes any of this recoverable: WAL that never reaches the
# archive is exactly the data a recovery loses, which is what the RPO measures.
# Wait for the segment holding the recovery target to be archived before
# pulling the plug, then confirm it landed.
segment_to_archive=$(run_sql "SELECT pg_walfile_name(pg_current_wal_lsn())")
run_sql "SELECT pg_switch_wal()" >/dev/null
archived=false
for _ in $(seq 1 60); do
  if [[ -f "$archive_dir/$segment_to_archive" ]]; then
    archived=true
    break
  fi
  sleep 0.5
done
if [[ "$archived" != true ]]; then
  echo "[pitr] WAL segment $segment_to_archive was never archived; check archive_command" >&2
  exit 1
fi

# ── 5. Disaster ──────────────────────────────────────────────────────────────
echo "[pitr] simulating total loss of the primary"
pg_ctl -D "$primary_dir" -m immediate stop >/dev/null
recovery_start_epoch=$(date +%s)

# ── 6. Recover to the chosen point in time ───────────────────────────────────
echo "[pitr] recovering to $recovery_target"
cp -R "$basebackup_dir" "$restored_dir"
chmod 700 "$restored_dir"
rm -f "$restored_dir/postmaster.pid"

cat >> "$restored_dir/postgresql.conf" <<CONF
port = $((port + 1))
unix_socket_directories = '$workdir'
archive_mode = off
restore_command = 'cp $archive_dir/%f %p'
recovery_target_time = '$recovery_target'
recovery_target_action = 'promote'
CONF
touch "$restored_dir/recovery.signal"

pg_ctl -D "$restored_dir" -l "$restore_logfile" -w -t 120 start >/dev/null

restored_sql() { psql -h localhost -p "$((port + 1))" -U postgres -d postgres -tAc "$1"; }

# Wait for recovery to finish and the instance to open for writes.
for _ in $(seq 1 60); do
  if [[ "$(restored_sql 'SELECT pg_is_in_recovery()' 2>/dev/null || echo t)" == "f" ]]; then
    break
  fi
  sleep 1
done

recovery_seconds=$(($(date +%s) - recovery_start_epoch))

# ── 7. Did recovery land exactly where we asked? ─────────────────────────────
failures=()
recovered_rows=$(restored_sql "SELECT count(*) FROM contracts" 2>/dev/null || echo "0")
lost_writes=$(restored_sql "SELECT count(*) FROM contracts WHERE name LIKE 'lost-write-%'" 2>/dev/null || echo "-1")

if [[ "$(restored_sql 'SELECT pg_is_in_recovery()' 2>/dev/null || echo t)" != "f" ]]; then
  failures+=("recovered instance never left recovery")
fi
if [[ "$recovered_rows" != "$expected_rows" ]]; then
  failures+=("expected $expected_rows row(s) at the recovery target, found $recovered_rows")
fi
if [[ "$lost_writes" != "0" ]]; then
  failures+=("recovery replayed $lost_writes write(s) from after the target")
fi
if [[ "$recovery_seconds" -gt $((BACKUP_RTO_MINUTES * 60)) ]]; then
  failures+=("recovery took ${recovery_seconds}s, over the ${BACKUP_RTO_MINUTES}m RTO target")
fi

echo "[pitr] rows at target: expected $expected_rows, recovered $recovered_rows"
echo "[pitr] writes after the target correctly discarded: $((rows_before_failure - recovered_rows))"
echo "[pitr] recovery took ${recovery_seconds}s (RTO target $((BACKUP_RTO_MINUTES * 60))s)"

if [[ "$json_output" == true ]]; then
  printf '{"recovery_target":"%s","expected_rows":%s,"recovered_rows":%s,"discarded_writes":%s,"recovery_seconds":%s,"rto_target_seconds":%s,"status":"%s"}\n' \
    "$recovery_target" "$expected_rows" "$recovered_rows" \
    "$((rows_before_failure - recovered_rows))" "$recovery_seconds" \
    "$((BACKUP_RTO_MINUTES * 60))" \
    "$([[ ${#failures[@]} -eq 0 ]] && echo ok || echo failed)"
fi

if [[ ${#failures[@]} -gt 0 ]]; then
  echo "[pitr] FAILED:" >&2
  for failure in "${failures[@]}"; do
    echo "  - $failure" >&2
  done
  exit 1
fi

echo "[pitr] point-in-time recovery verified"
