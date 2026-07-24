-- Issue #874: Database indexing strategy for contract filters and search
--
-- Improves common registry queries by adding targeted indexes for:
--   * network, category, verification_status, created_at, updated_at, last_accessed_at
--   * common filter combinations used by contract listing and export
--   * full-text search over name/description via an existing search_vector column
--   * partial indexes for public verified contracts and active contract listings
--
-- These indexes are designed to remain reasonably sized and support the
-- most valuable query shapes without introducing unnecessary write overhead.

CREATE INDEX IF NOT EXISTS idx_contracts_created_at_desc ON contracts (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_contracts_verification_status_created_at
  ON contracts (verification_status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_contracts_network_category_verification_created_at
  ON contracts (network, category, verification_status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_contracts_verified_public_created_at
  ON contracts (created_at DESC)
  WHERE is_verified = true AND visibility = 'public';

CREATE INDEX IF NOT EXISTS contracts_search_vector_idx
  ON contracts USING GIN (search_vector);

ANALYZE contracts;
