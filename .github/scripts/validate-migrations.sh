#!/usr/bin/env bash

set -euo pipefail

root_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
migrations_dir="$root_dir/database/migrations"

if [[ ! -d "$migrations_dir" ]]; then
  echo "ERROR: migrations directory not found: $migrations_dir" >&2
  exit 1
fi

shopt -s nullglob
migration_files=("$migrations_dir"/*.sql)

if [[ ${#migration_files[@]} -eq 0 ]]; then
  echo "ERROR: no migration files found in $migrations_dir" >&2
  exit 1
fi

status=0
prev_name=""

for file_path in "${migration_files[@]}"; do
  file_name=$(basename "$file_path")

  if [[ ! "$file_name" =~ ^([0-9]{3}|[0-9]{14})_[a-z0-9][a-z0-9_]*\.sql$ ]]; then
    echo "ERROR: invalid migration filename: $file_name" >&2
    echo "  Expected <NNN>_name.sql or <YYYYMMDDHHMMSS>_name.sql" >&2
    status=1
    continue
  fi

  if [[ -n "$prev_name" && "$file_name" < "$prev_name" ]]; then
    echo "ERROR: migration files are not in lexicographic order:" >&2
    echo "  $prev_name" >&2
    echo "  $file_name" >&2
    status=1
  fi

  prev_name="$file_name"
done

if [[ $status -ne 0 ]]; then
  exit $status
fi

# Determine the base revision for immutable migration validation.
# This allows CI to reject edits or removals of already-merged migration files.
determine_base_revision() {
  local base_revision=""

  if [[ -n "${GITHUB_EVENT_NAME:-}" && "${GITHUB_EVENT_NAME}" == "pull_request" ]]; then
    if [[ -n "${GITHUB_BASE_REF:-}" && git rev-parse --verify "origin/${GITHUB_BASE_REF}" >/dev/null 2>&1 ]]; then
      base_revision="origin/${GITHUB_BASE_REF}"
    fi
  elif [[ -n "${GITHUB_EVENT_BEFORE:-}" && "${GITHUB_EVENT_BEFORE}" != "0000000000000000000000000000000000000000" ]]; then
    base_revision="${GITHUB_EVENT_BEFORE}"
  fi

  if [[ -z "$base_revision" && git rev-parse --verify origin/main >/dev/null 2>&1 ]]; then
    base_revision="origin/main"
  fi

  if [[ -z "$base_revision" && git rev-parse --verify main >/dev/null 2>&1 ]]; then
    base_revision="main"
  fi

  if [[ -z "$base_revision" ]]; then
    base_revision=$(git merge-base HEAD origin/main 2>/dev/null || true)
  fi

  printf "%s" "$base_revision"
}

base_revision=$(determine_base_revision)
if [[ -n "$base_revision" ]]; then
  diff_output=$(git diff --name-status --no-renames "$base_revision" -- "$migrations_dir" || true)
  if [[ -n "$diff_output" ]]; then
    while IFS=$'\t' read -r status file _; do
      if [[ "$status" != A* ]]; then
        echo "ERROR: Existing migration file modified or removed compared to $base_revision" >&2
        echo "  Existing migration files in database/migrations are immutable once merged." >&2
        echo "  Changed path: $file" >&2
        echo "  If you need to alter schema or data, add a new migration file instead." >&2
        echo
        git diff --name-status --no-renames "$base_revision" -- "$migrations_dir" >&2
        exit 1
      fi
    done <<< "$diff_output"
  fi
else
  echo "WARNING: Unable to determine base revision for immutable migration validation; skipping this check." >&2
fi

echo "✓ Migration filenames are valid"