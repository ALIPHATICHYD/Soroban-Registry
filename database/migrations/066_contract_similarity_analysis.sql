CREATE TYPE similarity_match_type AS ENUM ('exact_clone', 'near_duplicate', 'similar');
CREATE TYPE similarity_review_status AS ENUM ('none', 'pending', 'reviewed', 'dismissed');

CREATE TABLE contract_similarity_signatures (
    contract_id UUID PRIMARY KEY REFERENCES contracts(id) ON DELETE CASCADE,
    representation_type VARCHAR(32) NOT NULL,
    exact_hash VARCHAR(64) NOT NULL,
    simhash BIGINT NOT NULL,
    token_count INTEGER NOT NULL DEFAULT 0,
    source_length INTEGER NOT NULL DEFAULT 0,
    wasm_hash VARCHAR(64) NOT NULL,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_contract_similarity_signatures_exact_hash
    ON contract_similarity_signatures(exact_hash);

CREATE INDEX idx_contract_similarity_signatures_wasm_hash
    ON contract_similarity_signatures(wasm_hash);

CREATE TABLE contract_similarity_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    similar_contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    similarity_score DECIMAL(5,4) NOT NULL,
    exact_clone BOOLEAN NOT NULL DEFAULT FALSE,
    match_type similarity_match_type NOT NULL,
    suspicious BOOLEAN NOT NULL DEFAULT FALSE,
    flagged_for_review BOOLEAN NOT NULL DEFAULT FALSE,
    review_status similarity_review_status NOT NULL DEFAULT 'none',
    reasons JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, similar_contract_id)
);

CREATE INDEX idx_contract_similarity_reports_contract_id
    ON contract_similarity_reports(contract_id, similarity_score DESC);

CREATE INDEX idx_contract_similarity_reports_similar_contract_id
    ON contract_similarity_reports(similar_contract_id, similarity_score DESC);

CREATE INDEX idx_contract_similarity_reports_flagged
    ON contract_similarity_reports(flagged_for_review, suspicious, updated_at DESC)
    WHERE flagged_for_review = TRUE;

CREATE TRIGGER update_contract_similarity_reports_updated_at
    BEFORE UPDATE ON contract_similarity_reports
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
