-- Add logical_id and network_configs for network-specific contract configuration (Issue #43).
-- logical_id: groups rows that represent the same logical contract across networks.
-- network_configs: JSONB per row with shape { "mainnet": { "contract_id", "is_verified", "min_version", "max_version" }, ... }.

ALTER TABLE contracts
  ADD COLUMN IF NOT EXISTS logical_id UUID,
  ADD COLUMN IF NOT EXISTS network_configs JSONB DEFAULT '{}';

-- Backfill: each existing row is its own logical contract; network_configs has one key for this row's network.
UPDATE contracts
SET
  logical_id = id,
  network_configs = jsonb_build_object(
    network::text,
    jsonb_build_object(
      'contract_id', contract_id,
      'is_verified', is_verified,
      'min_version', null,
      'max_version', null
    )
  )
WHERE network_configs IS NULL OR network_configs = '{}';

-- Ensure default for new rows
ALTER TABLE contracts ALTER COLUMN network_configs SET DEFAULT '{}';

CREATE INDEX IF NOT EXISTS idx_contracts_logical_id ON contracts(logical_id);
CREATE INDEX IF NOT EXISTS idx_contracts_network_configs ON contracts USING GIN (network_configs);
