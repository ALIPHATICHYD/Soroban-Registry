#!/usr/bin/env bash
#
# Turns on continuous archiving in the local backup stack, mirroring what
# database/pgbackrest/postgresql.backup.conf does on a production host.
#
# Runs once, during cluster initialisation, before the server is opened to
# clients.

set -euo pipefail

stanza="${PGBACKREST_STANZA:-soroban-registry}"

cat >> "$PGDATA/postgresql.conf" <<CONF

# Continuous archiving for pgBackRest; see docs/backup-and-recovery.md.
wal_level = replica
archive_mode = on
archive_command = 'pgbackrest --stanza=$stanza archive-push %p'
# Bounds RPO: an idle database still produces a recoverable segment each minute.
archive_timeout = 60s
max_wal_senders = 10
CONF

echo "[archiving] enabled for stanza '$stanza'"
