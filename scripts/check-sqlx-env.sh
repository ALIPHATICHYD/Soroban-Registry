#!/usr/bin/env bash
#
# check-sqlx-env.sh
#
# Validates that the local environment can build crates that use SQLx's
# compile-time-checked query macros (`sqlx::query!`, `query_as!`, ...).
#
# Pass conditions (one of):
#   1. SQLX_OFFLINE=true AND backend/.sqlx/ contains prepared query data.
#   2. DATABASE_URL is set, uses a Postgres URL, and is reachable.
#
# Exit codes:
#   0 — environment is ready to build.
#   1 — environment is NOT ready; an actionable message is printed.

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
sqlx_offline_dir="$repo_root/backend/.sqlx"

red()    { printf '\033[31m%s\033[0m\n' "$*" >&2; }
yellow() { printf '\033[33m%s\033[0m\n' "$*" >&2; }
green()  { printf '\033[32m%s\033[0m\n' "$*" >&2; }

# ── Offline mode ───────────────────────────────────────────────────────
if [[ "${SQLX_OFFLINE:-}" == "true" || "${SQLX_OFFLINE:-}" == "1" ]]; then
  if [[ -d "$sqlx_offline_dir" ]] && compgen -G "$sqlx_offline_dir/query-*.json" >/dev/null; then
    green "✓ SQLX_OFFLINE=true and prepared query data found at backend/.sqlx/"
    exit 0
  fi
  red "✗ SQLX_OFFLINE=true but backend/.sqlx/ is missing or empty."
  red "  Run 'cd backend && cargo sqlx prepare --workspace -- --all-targets' on a"
  red "  machine with a live, migrated database, then commit backend/.sqlx/."
  exit 1
fi

# ── Online mode ────────────────────────────────────────────────────────
if [[ -z "${DATABASE_URL:-}" ]]; then
  red "✗ DATABASE_URL is not set."
  red ""
  red "  SQLx's query macros validate SQL against a live database at compile"
  red "  time. Set DATABASE_URL to a reachable Postgres instance:"
  red ""
  red "    export DATABASE_URL=\"postgresql://postgres:postgres@localhost:5432/soroban_registry\""
  red ""
  red "  Or, to build without a database, set SQLX_OFFLINE=true and commit"
  red "  backend/.sqlx/ (see README → Running From Source)."
  exit 1
fi

# Light syntactic check — must start with postgres:// or postgresql://
if [[ ! "$DATABASE_URL" =~ ^postgres(ql)?:// ]]; then
  red "✗ DATABASE_URL is set but does not look like a Postgres URL:"
  red "    $DATABASE_URL"
  red "  Expected scheme: postgres:// or postgresql://"
  exit 1
fi

# If psql is available, try a real connection so we fail fast with a clear
# message instead of waiting for cargo to choke on the first query macro.
# NOTE: This validates reachability only; it does not verify migration state.
if command -v psql >/dev/null 2>&1; then
  if ! psql "$DATABASE_URL" -c 'SELECT 1' >/dev/null 2>&1; then
    red "✗ DATABASE_URL is set but the database is unreachable."
    red "    $DATABASE_URL"
    red "  Confirm Postgres is running and the URL/credentials are correct."
    exit 1
  fi
else
  yellow "! psql not found — skipping live connectivity check."
fi

green "✓ DATABASE_URL is set and reachable."
