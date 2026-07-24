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

echo "✓ Migration filenames are valid"