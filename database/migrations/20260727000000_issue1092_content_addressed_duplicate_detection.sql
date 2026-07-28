-- Add unique constraint for content-addressed duplicate detection (Issue #1092)
ALTER TABLE contracts ADD CONSTRAINT contracts_wasm_hash_key UNIQUE (wasm_hash);
