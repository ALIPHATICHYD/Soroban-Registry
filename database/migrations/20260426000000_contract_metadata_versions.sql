-- Contract Metadata Versioning (#729)
-- Tracks changes to contract name, description, category, and tags.

CREATE TABLE contract_metadata_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    user_id UUID, -- Optional: track which user made the change
    name VARCHAR(255) NOT NULL,
    description TEXT,
    category VARCHAR(100),
    tags TEXT[] DEFAULT '{}',
    change_summary TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_contract_metadata_versions_contract_id ON contract_metadata_versions(contract_id);
CREATE INDEX idx_contract_metadata_versions_created_at ON contract_metadata_versions(created_at);

-- Trigger to prune history to last 50 versions per contract
CREATE OR REPLACE FUNCTION prune_contract_metadata_versions()
RETURNS TRIGGER AS $$
BEGIN
    DELETE FROM contract_metadata_versions
    WHERE id NOT IN (
        SELECT id
        FROM contract_metadata_versions
        WHERE contract_id = NEW.contract_id
        ORDER BY created_at DESC
        LIMIT 50
    ) AND contract_id = NEW.contract_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_prune_metadata_versions
AFTER INSERT ON contract_metadata_versions
FOR EACH ROW EXECUTE FUNCTION prune_contract_metadata_versions();

-- Initial capture of current metadata for all contracts
INSERT INTO contract_metadata_versions (contract_id, name, description, category, tags, change_summary)
SELECT id, name, description, category, tags, 'Initial version'
FROM (
    SELECT c.id, c.name, c.description, c.category, 
           array_agg(t.name) FILTER (WHERE t.name IS NOT NULL) as tags
    FROM contracts c
    LEFT JOIN contract_tags ct ON c.id = ct.contract_id
    LEFT JOIN tags t ON ct.tag_id = t.id
    GROUP BY c.id
) sub;
