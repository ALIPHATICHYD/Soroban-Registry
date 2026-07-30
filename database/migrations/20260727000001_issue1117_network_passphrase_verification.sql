-- Migration: Issue #1117 – Store network passphrase alongside verification records
--
-- Soroban network identity is tied to a network passphrase.  The enum
-- (mainnet/testnet/futurenet) is a convenience label; the authoritative
-- identifier is the passphrase that is hashed into every transaction.
-- Storing the passphrase at publish / verification time lets us detect
-- silent mismatches when a custom or private network reuses an enum label.
--
-- Design decisions:
--   • NULL means "passphrase not supplied" – treated as a soft-warning, not
--     an outright failure, to keep existing rows backward-compatible.
--   • A non-NULL passphrase must match any previous non-NULL passphrase for
--     the same contract; a mismatch is a hard reject.
--   • The three well-known passphrases are stored as constants in application
--     code (verifier crate) so DB rows can be validated offline.

-- 1. Add network_passphrase to the verifications table so every verification
--    attempt records which passphrase was used.
ALTER TABLE verifications
    ADD COLUMN IF NOT EXISTS network_passphrase TEXT;

-- 2. Add network_passphrase to the contracts table so the authoritative
--    passphrase established at first publish is available for comparison on
--    subsequent verification attempts.
ALTER TABLE contracts
    ADD COLUMN IF NOT EXISTS network_passphrase TEXT;

-- 3. Backfill well-known passphrases for existing rows based on the network
--    enum value.  Custom / private networks cannot be backfilled and stay NULL.
UPDATE contracts
SET network_passphrase = CASE network
    WHEN 'mainnet'   THEN 'Public Global Stellar Network ; September 2015'
    WHEN 'testnet'   THEN 'Test SDF Network ; September 2015'
    WHEN 'futurenet' THEN 'Test SDF Future Network ; October 2022'
    ELSE NULL
END
WHERE network_passphrase IS NULL;

UPDATE verifications v
SET network_passphrase = CASE c.network
    WHEN 'mainnet'   THEN 'Public Global Stellar Network ; September 2015'
    WHEN 'testnet'   THEN 'Test SDF Network ; September 2015'
    WHEN 'futurenet' THEN 'Test SDF Future Network ; October 2022'
    ELSE NULL
END
FROM contracts c
WHERE v.contract_id = c.id
  AND v.network_passphrase IS NULL;

-- 4. Index to speed up passphrase-based look-ups and consistency checks.
CREATE INDEX IF NOT EXISTS idx_contracts_network_passphrase
    ON contracts (network_passphrase)
    WHERE network_passphrase IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_verifications_network_passphrase
    ON verifications (contract_id, network_passphrase)
    WHERE network_passphrase IS NOT NULL;
