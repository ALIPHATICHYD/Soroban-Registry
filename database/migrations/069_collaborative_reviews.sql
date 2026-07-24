-- Migration 052: Collaborative Reviews
-- Build interface for team collaboration on contract reviews with annotations.

-- Custom types for review status
DO $$ BEGIN
    CREATE TYPE collaborative_review_status AS ENUM ('pending', 'approved', 'changes_requested');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Main review session for a contract version
CREATE TABLE collaborative_reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    status collaborative_review_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_collab_reviews_contract_id ON collaborative_reviews(contract_id);

-- Individual reviewers assigned or participating in a review
CREATE TABLE collaborative_reviewers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    review_id UUID NOT NULL REFERENCES collaborative_reviews(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    status collaborative_review_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(review_id, user_id)
);

CREATE INDEX idx_collab_reviewers_review_id ON collaborative_reviewers(review_id);
CREATE INDEX idx_collab_reviewers_user_id ON collaborative_reviewers(user_id);

-- Comments on the review (supports inline annotations)
CREATE TABLE collaborative_review_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    review_id UUID NOT NULL REFERENCES collaborative_reviews(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    -- For inline code comments
    line_number INT,
    file_path TEXT,
    -- For ABI method comments
    abi_path TEXT, -- e.g. "functions.balance_of"
    -- For threading
    parent_id UUID REFERENCES collaborative_review_comments(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_collab_comments_review_id ON collaborative_review_comments(review_id);
CREATE INDEX idx_collab_comments_parent_id ON collaborative_review_comments(parent_id);

-- Add updated_at triggers
CREATE TRIGGER update_collab_reviews_updated_at BEFORE UPDATE ON collaborative_reviews
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_collab_reviewers_updated_at BEFORE UPDATE ON collaborative_reviewers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_collab_comments_updated_at BEFORE UPDATE ON collaborative_review_comments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Seed Notification Templates
INSERT INTO notification_templates (name, subject, message_template, channel)
VALUES 
('review_assigned', 'New Contract Review Assigned', 'You have been assigned as a reviewer for contract {{contract_name}} ({{version}}).', 'email'),
('review_comment', 'New Comment on Contract Review', '{{user_name}} commented on {{contract_name}}: "{{comment_excerpt}}"', 'email'),
('review_completed', 'Contract Review Completed', 'The review for {{contract_name}} ({{version}}) has been marked as {{status}}.', 'email')
ON CONFLICT (name) DO UPDATE SET 
    subject = EXCLUDED.subject,
    message_template = EXCLUDED.message_template;
