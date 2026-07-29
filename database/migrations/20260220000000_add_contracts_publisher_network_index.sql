-- no-transaction
-- CREATE INDEX CONCURRENTLY cannot share a file with other statements: sqlx sends
-- a migration as a single simple query, which Postgres wraps in an implicit
-- transaction. One concurrent index per migration.

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_contracts_publisher_network
    ON contracts (publisher_id, network);
