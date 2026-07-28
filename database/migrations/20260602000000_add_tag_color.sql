-- Add the missing `color` column to `tags`.
--
-- The API selects `t.color` in seven places (contract tag lookups in
-- handlers.rs, v1_contract_handlers.rs, ...), but no migration ever created the
-- column. Any request that returns a contract therefore fails with
-- `column t.color does not exist`, which only surfaces once the contracts table
-- is non-empty.
--
-- Nullable: existing tags have no colour, and the API already models it as an
-- optional value.
ALTER TABLE tags
    ADD COLUMN IF NOT EXISTS color VARCHAR(32);
