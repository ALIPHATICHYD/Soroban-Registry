CREATE TABLE protocol_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug VARCHAR(100) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    required_functions TEXT[] NOT NULL DEFAULT '{}',
    optional_functions TEXT[] NOT NULL DEFAULT '{}',
    bridge_indicators TEXT[] NOT NULL DEFAULT '{}',
    adapter_indicators TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_protocol_definitions_slug ON protocol_definitions(slug);

INSERT INTO protocol_definitions (
    slug,
    name,
    description,
    required_functions,
    optional_functions,
    bridge_indicators,
    adapter_indicators
)
VALUES
    (
        'token-interface',
        'Token Interface',
        'Core fungible-token surface for transfer, allowance, and approval flows.',
        ARRAY['balance', 'transfer', 'approve', 'allowance', 'transfer_from'],
        ARRAY['decimals', 'symbol', 'name', 'mint', 'burn', 'clawback'],
        ARRAY[]::TEXT[],
        ARRAY['wrap', 'unwrap', 'quote', 'route']
    ),
    (
        'ownership-admin',
        'Ownership Admin',
        'Administrative ownership hooks used by managed or upgradeable contracts.',
        ARRAY['owner'],
        ARRAY['set_owner', 'transfer_ownership', 'renounce_ownership', 'admin', 'set_admin'],
        ARRAY[]::TEXT[],
        ARRAY[]::TEXT[]
    ),
    (
        'pause-control',
        'Pause Control',
        'Operational controls for pausing and resuming contract behavior.',
        ARRAY['pause', 'unpause'],
        ARRAY['paused', 'is_paused'],
        ARRAY[]::TEXT[],
        ARRAY[]::TEXT[]
    ),
    (
        'oracle-read',
        'Oracle Read',
        'Read-oriented pricing or oracle access patterns.',
        ARRAY['get_price'],
        ARRAY['latest_price', 'read_price', 'quote', 'price', 'last_price'],
        ARRAY[]::TEXT[],
        ARRAY['quote', 'convert']
    ),
    (
        'bridge-settlement',
        'Bridge Settlement',
        'Lock, release, mint, burn, and proof-submission flows for bridge settlement.',
        ARRAY['lock', 'release'],
        ARRAY['mint_wrapped', 'burn_wrapped', 'submit_proof', 'relay_message', 'claim'],
        ARRAY['lock', 'release', 'claim', 'submit_proof', 'relay_message', 'mint_wrapped', 'burn_wrapped'],
        ARRAY[]::TEXT[]
    ),
    (
        'adapter-execution',
        'Adapter Execution',
        'Routing or wrapping contracts that normalize calls into another protocol surface.',
        ARRAY['execute'],
        ARRAY['quote', 'wrap', 'unwrap', 'convert', 'route', 'swap'],
        ARRAY[]::TEXT[],
        ARRAY['execute', 'quote', 'wrap', 'unwrap', 'convert', 'route', 'swap']
    )
ON CONFLICT (slug) DO NOTHING;
