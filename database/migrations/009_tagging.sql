CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    prefix VARCHAR(100) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    usage_count INTEGER NOT NULL DEFAULT 0,
    is_trending BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(prefix, name)
);

CREATE INDEX idx_tags_prefix ON tags(prefix);
CREATE INDEX idx_tags_usage_count ON tags(usage_count DESC);
CREATE INDEX idx_tags_is_trending ON tags(is_trending) WHERE is_trending = TRUE;
CREATE INDEX idx_tags_prefix_name ON tags(prefix, name);

CREATE TABLE tag_aliases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alias VARCHAR(255) NOT NULL UNIQUE,
    canonical_tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tag_aliases_alias ON tag_aliases(alias);
CREATE INDEX idx_tag_aliases_canonical ON tag_aliases(canonical_tag_id);

CREATE TABLE tag_usage_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    usage_count INTEGER NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tag_usage_log_tag_id ON tag_usage_log(tag_id);
CREATE INDEX idx_tag_usage_log_recorded_at ON tag_usage_log(recorded_at DESC);

CREATE TRIGGER update_tags_updated_at BEFORE UPDATE ON tags
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
