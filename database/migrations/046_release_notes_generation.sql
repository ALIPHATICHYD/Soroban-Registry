-- Release Notes Generation (auto-generated release notes for contract versions)
-- The `release_notes` TEXT column already exists on `contract_versions`.
-- This migration adds:
--   1. A `release_notes_generated` table to track generation metadata
--   2. A `release_notes_status` enum for draft/published workflow

-- Status enum for release notes (draft allows manual editing before publishing)
DO $$ BEGIN
    CREATE TYPE release_notes_status AS ENUM ('draft', 'published');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

-- Tracks auto-generation metadata for each contract version's release notes
CREATE TABLE IF NOT EXISTS release_notes_generated (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id     UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    version         VARCHAR(50) NOT NULL,
    -- The version this was compared against (NULL for first version)
    previous_version VARCHAR(50),
    -- Structured diff summary as JSON
    diff_summary    JSONB NOT NULL DEFAULT '{}',
    -- Changelog section extracted from CHANGELOG.md (if any)
    changelog_entry TEXT,
    -- The final generated/edited release notes text
    notes_text      TEXT NOT NULL DEFAULT '',
    -- Whether the notes are in draft (editable) or published (immutable)
    status          release_notes_status NOT NULL DEFAULT 'draft',
    -- Who/what generated the notes ('auto' or a publisher address)
    generated_by    VARCHAR(200) NOT NULL DEFAULT 'auto',
    -- Timestamp tracking
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    published_at    TIMESTAMPTZ,
    -- One set of release notes per contract+version
    UNIQUE(contract_id, version)
);

-- Index for quick lookups by contract
CREATE INDEX IF NOT EXISTS idx_release_notes_generated_contract
    ON release_notes_generated(contract_id);

-- Index for filtering by status
CREATE INDEX IF NOT EXISTS idx_release_notes_generated_status
    ON release_notes_generated(status);
