-- Marketplace Phase 3 — indexer scanner state.
--
-- The USDC scanner inside the indexer crate polls Horizon's
-- /accounts/{addr}/payments endpoint and POSTs observed payments to
-- /api/marketplace/usdc/confirm. To resume safely after restart it
-- needs a paging cursor; Horizon's paging tokens are opaque strings
-- (lexicographically orderable but should be treated as a blob).
--
-- One row per (network, receiving_address). Composite primary key
-- because a single deployment can rotate receiving addresses or run
-- against both testnet and public.

CREATE TABLE marketplace_usdc_scanner_state (
    network           VARCHAR(20) NOT NULL CHECK (network IN ('testnet', 'public')),
    receiving_address VARCHAR(56) NOT NULL,
    cursor            TEXT,
    last_seen_tx      VARCHAR(64),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (network, receiving_address)
);
