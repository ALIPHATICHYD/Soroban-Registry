# PostgreSQL with pgBackRest installed alongside it.
#
# pgBackRest's archive_command runs inside the PostgreSQL server's own
# environment, so the binary has to be on the database host. A sidecar
# container cannot archive WAL for a database whose filesystem it does not
# share, which is why this image exists rather than a standalone pgBackRest
# service. Production installs the pgbackrest package on the database host the
# same way; this image is how the local stack reproduces that arrangement.

FROM postgres:16-alpine

RUN apk add --no-cache pgbackrest

# Repository, spool and log directories, owned by the postgres user so both
# archive_command and the backup commands can write to them.
# The lock directory matters as much as the others: pgBackRest takes a lock per
# stanza before it does anything, so a root-owned /tmp/pgbackrest stops every
# command that runs as postgres.
RUN mkdir -p /var/lib/pgbackrest /var/spool/pgbackrest /var/log/pgbackrest /tmp/pgbackrest \
    && chown -R postgres:postgres /var/lib/pgbackrest /var/spool/pgbackrest \
       /var/log/pgbackrest /tmp/pgbackrest

# Reference copy of the production archiving settings, for comparison.
COPY postgresql.backup.conf /etc/pgbackrest/postgresql.backup.conf
# Applied during cluster initialisation, after PostgreSQL writes postgresql.conf.
COPY docker-entrypoint-archiving.sh /docker-entrypoint-initdb.d/00-archiving.sh
