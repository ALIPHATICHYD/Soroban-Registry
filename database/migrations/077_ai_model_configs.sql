-- Migration: 077_ai_model_configs.sql
-- Issue #643: AI-Powered Contract Code Assistant
-- Purpose: Store AI model configurations and prompt templates

-- AI model configurations (for different use cases)
CREATE TABLE IF NOT EXISTS ai_model_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    provider VARCHAR(50) NOT NULL, -- openai, anthropic, local
    model_name VARCHAR(255) NOT NULL, -- gpt-4, claude-3-opus, etc.
    model_parameters JSONB DEFAULT '{}', -- temperature, max_tokens, etc.
    system_prompt_template TEXT,
    context_window INTEGER,
    cost_per_1k_tokens DECIMAL(10,6),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ai_model_configs_active ON ai_model_configs(is_active);
CREATE INDEX idx_ai_model_configs_provider ON ai_model_configs(provider);

-- Insert default configurations
INSERT INTO ai_model_configs (name, provider, model_name, is_default, system_prompt_template)
VALUES 
    ('Default GPT-4', 'openai', 'gpt-4-turbo-preview', true, 
     E'You are a smart contract expert for the Soroban platform (Stellar''s smart contract platform). 
     Provide accurate, helpful responses about smart contract development, security, and best practices.
     When analyzing code, explain concepts clearly and highlight potential issues.
     Always include relevant code examples when helpful.
     Format responses using markdown.'),
    ('Claude Opus', 'anthropic', 'claude-3-opus-20240229', false,
     E'You are an expert in Soroban smart contracts and Rust-based blockchain development.
     Provide detailed, nuanced analysis of contract code, security vulnerabilities, and optimization opportunities.
     Be thorough in explanations and always consider gas efficiency and security implications.')
ON CONFLICT (name) DO NOTHING;

-- AI prompt templates (for different scenarios)
CREATE TABLE IF NOT EXISTS ai_prompt_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_name VARCHAR(255) NOT NULL UNIQUE,
    prompt_template TEXT NOT NULL,
    description TEXT,
    category VARCHAR(100), -- code_analysis, vulnerability, code_gen, explanation
    variables JSONB DEFAULT '[]', -- list of required variables
    model_config_id UUID REFERENCES ai_model_configs(id),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ai_prompt_templates_category ON ai_prompt_templates(category);
CREATE INDEX idx_ai_prompt_templates_active ON ai_prompt_templates(is_active);

-- Insert default templates
INSERT INTO ai_prompt_templates (template_name, prompt_template, description, category, variables)
VALUES 
    ('contract_analysis', 
     E'Analyze the following Soroban smart contract:\n\n```rust\n{contract_code}\n```\n\nContract metadata:\n- Name: {contract_name}\n- Category: {contract_category}\n- Description: {contract_description}\n\nProvide:\n1. A high-level summary of what this contract does\n2. Key functions and their purposes\n3. Potential security concerns or best practice violations\n4. Gas optimization opportunities\n5. Any upgradeability considerations',
     'Analyzes a contract and provides insights', 'code_analysis', '["contract_code", "contract_name", "contract_category", "contract_description"]'),
    
    ('vulnerability_check',
     E'Review this Soroban contract for security vulnerabilities:\n\n```rust\n{contract_code}\n```\n\nFocus on:\n- Reentrancy risks\n- Integer overflow/underflow\n- Authorization issues\n- Unchecked return values\n- Logic errors\n- Access control problems\n\nList any findings with severity levels (critical, high, medium, low) and suggested fixes.',
     'Security vulnerability scan', 'vulnerability', '["contract_code"]'),
    
    ('code_explanation',
     E'Explain this Soroban contract code line by line or by function:\n\n```rust\n{contract_code}\n```\n\n{additional_context}',
     'Explains contract code', 'explanation', '["contract_code", "additional_context"]'),
    
    ('code_suggestion',
     E'I need help with this Soroban contract:\n\n```rust\n{contract_code}\n```\n\nRequest: {user_request}\n\nProvide code suggestions, patterns, or corrections.',
     'Generates code suggestions', 'code_gen', '["contract_code", "user_request"]')
ON CONFLICT (template_name) DO NOTHING;

-- Function to auto-archive old chat sessions after 90 days (configurable)
CREATE OR REPLACE FUNCTION archive_old_ai_sessions(retention_days INTEGER DEFAULT 90)
RETURNS INTEGER AS $$
DECLARE
    archived_count INTEGER;
BEGIN
    UPDATE ai_chat_sessions 
    SET is_active = FALSE
    WHERE is_active = TRUE 
      AND updated_at < NOW() - (retention_days || ' days')::INTERVAL;
    
    GET DIAGNOSTICS archived_count = ROW_COUNT;
    RETURN archived_count;
END;
$$ LANGUAGE plpgsql;

-- Grant permissions (adjust as needed based on your security model)
-- GRANT SELECT, INSERT ON ai_chat_sessions TO registry_user;
-- GRANT SELECT, INSERT ON ai_chat_messages TO registry_user;
-- GRANT SELECT ON ai_model_configs TO registry_user;
-- GRANT SELECT ON ai_prompt_templates TO registry_user;
