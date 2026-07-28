-- Track chain-origin imports for automated testnet indexing.

ALTER TABLE contracts
    ADD COLUMN IF NOT EXISTS chain_imported BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS import_source VARCHAR(64),
    ADD COLUMN IF NOT EXISTS imported_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS imported_ledger_sequence BIGINT,
    ADD COLUMN IF NOT EXISTS imported_tx_id VARCHAR(128),
    ADD COLUMN IF NOT EXISTS last_seen_on_chain_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_contracts_chain_imported
    ON contracts(chain_imported);

CREATE INDEX IF NOT EXISTS idx_contracts_last_seen_on_chain_at
    ON contracts(last_seen_on_chain_at DESC);

COMMENT ON COLUMN contracts.chain_imported IS 'Whether this contract record originated from chain indexing';
COMMENT ON COLUMN contracts.import_source IS 'Origin of chain import (e.g. soroban_testnet_indexer)';
COMMENT ON COLUMN contracts.imported_ledger_sequence IS 'Ledger sequence where the contract deployment was detected';
COMMENT ON COLUMN contracts.imported_tx_id IS 'Transaction hash from which this contract import was derived';
