#!/usr/bin/env bash
#
# backup.sh
#
# Takes a pgBackRest backup of the registry database and writes it to both
# repositories: local disk (repo1) and off-site object storage (repo2).
#
# The schedule this is designed for:
#
#   Sunday      full
#   Mon-Sat     incremental
#
# Run it from cron or a systemd timer; see docs/backup-and-recovery.md.
#
# A backup command writes to ONE repository, so this runs once per repository:
# only WAL archiving fans out to all of them by itself. A run is not considered
# successful until every repository has the backup.
#
# Usage:
#   scripts/backup.sh [--type full|diff|incr] [--stanza NAME] [--repos "1 2"]
#                     [--json]
#
#   --type    Backup type. Defaults to "full" on BACKUP_FULL_DAY (default
#             Sunday) and "incr" otherwise, so a single daily cron entry
#             produces the weekly pattern above.
#   --stanza  pgBackRest stanza. Defaults to $PGBACKREST_STANZA or
#             "soroban-registry".
#   --repos   Space-separated repository numbers. Defaults to $BACKUP_REPOS or
#             "1 2": local disk and off-site.
#   --json    Print a machine-readable summary for monitoring.
#
# Credentials come from the environment, never from a config file. See
# .env.example for the CLOUDFLARE_* and BACKUP_* variables this reads.
#
# Exit codes:
#   0 - backup completed and is present in both repositories.
#   1 - backup failed, or required configuration is missing.

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/lib/backup-env.sh
source "$repo_root/scripts/lib/backup-env.sh"

backup_type=""
stanza="${PGBACKREST_STANZA:-soroban-registry}"
read -r -a repos <<< "${BACKUP_REPOS:-1 2}"
json_output=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --type)
      backup_type="${2:?--type needs a value}"
      shift 2
      ;;
    --stanza)
      stanza="${2:?--stanza needs a value}"
      shift 2
      ;;
    --repos)
      read -r -a repos <<< "${2:?--repos needs a value}"
      shift 2
      ;;
    --json)
      json_output=true
      shift
      ;;
    -h | --help)
      sed -n '2,38p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "backup.sh: unknown argument '$1'" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$backup_type" ]]; then
  # A full backup once a week keeps restore chains short; the rest of the week
  # is incremental, which is what keeps storage and bandwidth down.
  full_day="${BACKUP_FULL_DAY:-Sun}"
  if [[ "$(date +%a)" == "$full_day" ]]; then
    backup_type="full"
  else
    backup_type="incr"
  fi
fi

case "$backup_type" in
  full | diff | incr) ;;
  *)
    echo "backup.sh: --type must be full, diff or incr (got '$backup_type')" >&2
    exit 1
    ;;
esac

backup_env::require_pgbackrest
backup_env::export_repo_credentials

# A stanza has to exist before the first backup. Creating it is idempotent, so
# this also repairs a host that was rebuilt from scratch.
if ! pgbackrest --stanza="$stanza" info >/dev/null 2>&1; then
  echo "[backup] stanza '$stanza' not found; creating it"
  pgbackrest --stanza="$stanza" stanza-create
fi

started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
start_epoch=$(date +%s)

echo "[backup] $backup_type backup of stanza '$stanza' starting at $started_at"
echo "[backup] repositories: ${repos[*]}"

failed_repos=()
for repo in "${repos[@]}"; do
  echo "[backup] repo${repo}: $backup_type backup starting"
  if pgbackrest --stanza="$stanza" --repo="$repo" --type="$backup_type" backup; then
    echo "[backup] repo${repo}: done"
  else
    # Keep going: a backup on disk is still worth having when the off-site
    # upload fails, and the exit code below still reports the failure.
    echo "[backup] repo${repo}: FAILED" >&2
    failed_repos+=("$repo")
  fi
done

finished_epoch=$(date +%s)
duration=$((finished_epoch - start_epoch))
finished_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)

echo "[backup] $backup_type backup finished in ${duration}s"

# Report what is now on disk, so the log line after a backup always answers
# "what can we restore from right now".
pgbackrest --stanza="$stanza" info

if [[ "$json_output" == true ]]; then
  printf '{"stanza":"%s","type":"%s","repositories":"%s","failed_repositories":"%s","started_at":"%s","finished_at":"%s","duration_seconds":%d,"status":"%s"}\n' \
    "$stanza" "$backup_type" "${repos[*]}" "${failed_repos[*]-}" \
    "$started_at" "$finished_at" "$duration" \
    "$([[ ${#failed_repos[@]} -eq 0 ]] && echo ok || echo failed)"
fi

if [[ ${#failed_repos[@]} -gt 0 ]]; then
  echo "[backup] FAILED on repo(s): ${failed_repos[*]}" >&2
  exit 1
fi
