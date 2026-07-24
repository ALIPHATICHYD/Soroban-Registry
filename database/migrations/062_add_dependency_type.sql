-- Add dep_type column to contract_dependencies for filtering by import/call/data (issue #726)
--
-- NOTE: No migration in this repo creates a table literally named
-- `contract_dependencies` (migration 006 creates `contract_static_dependencies`,
-- and the call-graph lives in `contract_call_dependencies`). The original form of
-- this migration did an unguarded `ALTER TABLE contract_dependencies`, which fails
-- on every fresh database with: relation "contract_dependencies" does not exist.
-- Guard the change so the migration applies cleanly; when/if the table is
-- introduced, the column and index will be added.
DO $$
BEGIN
  IF to_regclass('public.contract_dependencies') IS NOT NULL THEN
    ALTER TABLE contract_dependencies
      ADD COLUMN IF NOT EXISTS dep_type VARCHAR(20) NOT NULL DEFAULT 'call';

    CREATE INDEX IF NOT EXISTS idx_contract_dependencies_dep_type
      ON contract_dependencies(dep_type);
  END IF;
END $$;
