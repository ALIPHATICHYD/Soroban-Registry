-- Migration: 075_ai_chat_sessions.sql
-- Issue #643: AI-Powered Contract Code Assistant
-- Purpose: Store chat sessions and messages for AI contract assistance

-- AI chat sessions table
CREATE TABLE IF NOT EXISTS ai_chat_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES publishers(id) ON DELETE CASCADE,
    contract_id UUID REFERENCES contracts(id) ON DELETE SET NULL,
    session_title VARCHAR(255),
    context_type VARCHAR(50) DEFAULT 'general', -- general, contract_analysis, vulnerability, code_gen
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    message_count INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_ai_chat_sessions_user_id ON ai_chat_sessions(user_id);
CREATE INDEX idx_ai_chat_sessions_contract_id ON ai_chat_sessions(contract_id);
CREATE INDEX idx_ai_chat_sessions_created_at ON ai_chat_sessions(created_at DESC);

-- AI chat messages table
CREATE TABLE IF NOT EXISTS ai_chat_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES ai_chat_sessions(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL, -- user, assistant, system
    content TEXT NOT NULL,
    contract_code_snippet TEXT, -- optional code context
    token_count INTEGER,
    model_used VARCHAR(100), -- e.g., "gpt-4", "claude-3"
    response_time_ms INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB DEFAULT '{}' -- usage stats, cost, etc.
);

CREATE INDEX idx_ai_chat_messages_session_id ON ai_chat_messages(session_id);
CREATE INDEX idx_ai_chat_messages_created_at ON ai_chat_messages(created_at DESC);
CREATE INDEX idx_ai_chat_messages_role ON ai_chat_messages(role);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_ai_chat_session_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger to automatically update updated_at on sessions
CREATE TRIGGER update_ai_chat_session_updated_at 
    BEFORE UPDATE ON ai_chat_sessions 
    FOR EACH ROW EXECUTE FUNCTION update_ai_chat_session_timestamp();

-- Trigger to increment message_count on new message
CREATE OR REPLACE FUNCTION increment_session_message_count()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE ai_chat_sessions 
    SET message_count = message_count + 1 
    WHERE id = NEW.session_id;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER increment_ai_chat_session_message_count 
    AFTER INSERT ON ai_chat_messages 
    FOR EACH ROW EXECUTE FUNCTION increment_session_message_count();
