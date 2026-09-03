#!/usr/bin/env bash
#
# backup-env.sh
#
# Shared helpers for the backup and recovery scripts: credential mapping,
# dependency checks, and the RTO/RPO targets the scripts assert against.
#
# Sourced, not executed.

# Recovery targets from the backup and recovery policy. Scripts compare their
# measurements against these, so the numbers live in exactly one place.
readonly BACKUP_RTO_MINUTES="${BACKUP_RTO_MINUTES:-60}"
readonly BACKUP_RPO_MINUTES="${BACKUP_RPO_MINUTES:-15}"
# A daily schedule plus slack: anything older means a backup run was missed.
readonly BACKUP_MAX_AGE_HOURS="${BACKUP_MAX_AGE_HOURS:-26}"

# Map the repository's CLOUDFLARE_*/BACKUP_* variables onto the PGBACKREST_*
# names pgBackRest reads. Keeping one set of names across services means an
# operator configures object storage once.
backup_env::export_repo_credentials() {
  if [[ -n "${CLOUDFLARE_ACCESS_KEY_ID:-}" ]]; then
    export PGBACKREST_REPO2_S3_KEY="$CLOUDFLARE_ACCESS_KEY_ID"
  fi
  if [[ -n "${CLOUDFLARE_SECRET_ACCESS_KEY:-}" ]]; then
    export PGBACKREST_REPO2_S3_KEY_SECRET="$CLOUDFLARE_SECRET_ACCESS_KEY"
  fi
  if [[ -n "${CLOUDFLARE_R2_BUCKET:-}" ]]; then
    export PGBACKREST_REPO2_S3_BUCKET="$CLOUDFLARE_R2_BUCKET"
  fi
  if [[ -n "${CLOUDFLARE_R2_ENDPOINT:-}" ]]; then
    # pgBackRest wants a host, with or without scheme; normalise like the other
    # services in this repository do.
    export PGBACKREST_REPO2_S3_ENDPOINT="${CLOUDFLARE_R2_ENDPOINT#http://}"
    export PGBACKREST_REPO2_S3_ENDPOINT="${PGBACKREST_REPO2_S3_ENDPOINT#https://}"
  fi
  if [[ -n "${CLOUDFLARE_R2_REGION:-}" ]]; then
    export PGBACKREST_REPO2_S3_REGION="$CLOUDFLARE_R2_REGION"
  fi
  if [[ -n "${BACKUP_ENCRYPTION_KEY:-}" ]]; then
    export PGBACKREST_REPO2_CIPHER_PASS="$BACKUP_ENCRYPTION_KEY"
  fi
}

# Fail early and specifically when the off-site repository is not configured.
backup_env::require_offsite_config() {
  local missing=()
  [[ -n "${CLOUDFLARE_ACCESS_KEY_ID:-}" ]] || missing+=("CLOUDFLARE_ACCESS_KEY_ID")
  [[ -n "${CLOUDFLARE_SECRET_ACCESS_KEY:-}" ]] || missing+=("CLOUDFLARE_SECRET_ACCESS_KEY")
  [[ -n "${CLOUDFLARE_R2_BUCKET:-}" ]] || missing+=("CLOUDFLARE_R2_BUCKET")
  [[ -n "${CLOUDFLARE_R2_ENDPOINT:-}" ]] || missing+=("CLOUDFLARE_R2_ENDPOINT")
  [[ -n "${BACKUP_ENCRYPTION_KEY:-}" ]] || missing+=("BACKUP_ENCRYPTION_KEY")

  if [[ ${#missing[@]} -gt 0 ]]; then
    echo "[backup] off-site repository is not configured; missing: ${missing[*]}" >&2
    echo "[backup] see .env.example and docs/backup-and-recovery.md" >&2
    return 1
  fi
}

# Confirm the off-site repository actually answers, which is the thing a drill
# depends on. This is deliberately a behavioural check rather than a check that
# some environment variables are set: the credentials may live on this host, or
# in the database container's environment, and what matters either way is that
# repo2 responds.
backup_env::offsite_reachable() {
  local stanza="$1" container="${BACKUP_DB_CONTAINER:-soroban-registry-db-backup}"

  if command -v pgbackrest >/dev/null 2>&1 &&
    pgbackrest --stanza="$stanza" --repo=2 info >/dev/null 2>&1; then
    return 0
  fi

  if command -v docker >/dev/null 2>&1 &&
    docker exec -u postgres "$container" \
      pgbackrest --stanza="$stanza" --repo=2 info >/dev/null 2>&1; then
    return 0
  fi

  echo "[backup] the off-site repository (repo2) did not respond" >&2
  echo "[backup] check CLOUDFLARE_* credentials and BACKUP_ENCRYPTION_KEY, or" >&2
  echo "[backup] start the local stack: docker compose -f docker-compose.backup.yml up -d" >&2
  return 1
}

backup_env::require_pgbackrest() {
  if ! command -v pgbackrest >/dev/null 2>&1; then
    echo "[backup] pgbackrest is not installed or not on PATH" >&2
    echo "[backup] install it on the database host, or run the containerised" >&2
    echo "[backup] stack: docker compose -f docker-compose.backup.yml up -d" >&2
    return 1
  fi
}

# Newest backup's stop time, as a unix timestamp, or empty when there are none.
#
# Three readers, in order of preference, because a database host is often
# minimal: jq, then python3, then pgBackRest's own text output parsed with awk.
# The last one needs nothing that is not already there to run pgBackRest.
backup_env::latest_backup_epoch() {
  local stanza="$1" info stop_text

  if command -v jq >/dev/null 2>&1; then
    info=$(pgbackrest --stanza="$stanza" info --output=json 2>/dev/null) || return 0
    printf '%s' "$info" | jq -r '.[0].backup[-1].timestamp.stop // empty'
    return 0
  fi

  if command -v python3 >/dev/null 2>&1; then
    info=$(pgbackrest --stanza="$stanza" info --output=json 2>/dev/null) || return 0
    printf '%s' "$info" | python3 -c '
import json, sys
try:
    stanzas = json.load(sys.stdin)
except (json.JSONDecodeError, ValueError):
    sys.exit(0)
for stanza in stanzas:
    backups = stanza.get("backup") or []
    if backups:
        print(backups[-1]["timestamp"]["stop"])
'
    return 0
  fi

  # Text form: "timestamp start/stop: 2026-08-24 05:36:02+00 / 2026-08-24 05:36:14+00"
  stop_text=$(pgbackrest --stanza="$stanza" info 2>/dev/null |
    awk -F' / ' '/timestamp start\/stop:/ { stop = $2 } END { if (stop != "") print stop }')
  [[ -n "$stop_text" ]] || return 0
  backup_env::to_epoch "${stop_text%%+*}"
}

# Unix timestamp for a "YYYY-MM-DD HH:MM:SS" instant read as UTC, on both GNU
# and BSD date.
backup_env::to_epoch() {
  local instant="${1% }"
  date -u -d "$instant" +%s 2>/dev/null ||
    date -u -j -f "%Y-%m-%d %H:%M:%S" "$instant" +%s 2>/dev/null ||
    return 1
}

backup_env::require_psql() {
  if ! command -v psql >/dev/null 2>&1; then
    echo "[backup] psql is not installed or not on PATH" >&2
    return 1
  fi
}

# Minutes between an ISO-8601 UTC timestamp and now, on both GNU and BSD date.
backup_env::minutes_since() {
  local timestamp="$1" then_epoch
  if then_epoch=$(date -u -d "$timestamp" +%s 2>/dev/null); then
    :
  elif then_epoch=$(date -u -j -f "%Y-%m-%d %H:%M:%S" "${timestamp%%.*}" +%s 2>/dev/null); then
    :
  else
    echo "[backup] could not parse timestamp '$timestamp'" >&2
    return 1
  fi

  echo $(((  $(date -u +%s) - then_epoch ) / 60))
}
