-- Issue #886: Data integrity verification and checksums.
--
-- Provides the storage layer for:
--   • Per-resource SHA-256 checksums of contract data and ABIs.
--   • Periodic integrity verification job runs and their outcomes.
--   • An append-style log of detected integrity issues with severity and
--     repair status, used for alerting and reporting.
--
-- Design notes:
--   • `data_checksums` holds one baseline row per (resource_type, resource_id).
--     Verification recomputes the checksum from live data and compares it to the
--     stored baseline to detect corruption.
--   • `integrity_verification_runs` records each verification job (ad-hoc or the
--     periodic full-system sweep) so status can be reported over time.
--   • `integrity_issues` is the durable record of every mismatch / missing
--     checksum / missing data event, enabling alerting, reporting, and repair
--     tracking.

-- ── Checksum baselines ────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS data_checksums (
    id              BIGSERIAL    PRIMARY KEY,
    -- What the checksum covers: 'contract' (core contract data) or 'abi'.
    resource_type   TEXT         NOT NULL,
    -- Stable identifier of the resource (e.g. contract UUID, ABI row UUID).
    resource_id     TEXT         NOT NULL,
    -- Owning contract, when applicable, for convenient grouping/reporting.
    contract_id     UUID         REFERENCES contracts(id) ON DELETE CASCADE,
    -- Hash algorithm; sha256 today, kept explicit for future migration.
    algorithm       TEXT         NOT NULL DEFAULT 'sha256',
    -- Hex-encoded digest of the canonicalized resource bytes.
    checksum        TEXT         NOT NULL,
    -- Size in bytes of the canonicalized payload that was hashed.
    byte_size       BIGINT       NOT NULL DEFAULT 0,
    -- Latest verification verdict: 'valid' | 'mismatch' | 'missing'.
    status          TEXT         NOT NULL DEFAULT 'valid',
    last_verified_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (resource_type, resource_id)
);

CREATE INDEX IF NOT EXISTS idx_data_checksums_contract_id ON data_checksums (contract_id);
CREATE INDEX IF NOT EXISTS idx_data_checksums_resource    ON data_checksums (resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_data_checksums_status      ON data_checksums (status);

-- ── Verification job runs ─────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS integrity_verification_runs (
    id              BIGSERIAL    PRIMARY KEY,
    -- Scope of the run: 'full' (system-wide), 'contract', or 'abi'.
    scope           TEXT         NOT NULL,
    -- Optional contract the run targeted (NULL for full-system sweeps).
    contract_id     UUID         REFERENCES contracts(id) ON DELETE SET NULL,
    -- 'running' | 'completed' | 'failed'.
    status          TEXT         NOT NULL DEFAULT 'running',
    total_checked   BIGINT       NOT NULL DEFAULT 0,
    valid_count     BIGINT       NOT NULL DEFAULT 0,
    mismatch_count  BIGINT       NOT NULL DEFAULT 0,
    missing_count   BIGINT       NOT NULL DEFAULT 0,
    repaired_count  BIGINT       NOT NULL DEFAULT 0,
    error_message   TEXT,
    started_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_integrity_runs_status     ON integrity_verification_runs (status);
CREATE INDEX IF NOT EXISTS idx_integrity_runs_started_at ON integrity_verification_runs (started_at DESC);

-- ── Detected integrity issues ─────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS integrity_issues (
    id                BIGSERIAL   PRIMARY KEY,
    -- The verification run that surfaced this issue (NULL for on-access checks).
    run_id            BIGINT      REFERENCES integrity_verification_runs(id) ON DELETE SET NULL,
    resource_type     TEXT        NOT NULL,
    resource_id       TEXT        NOT NULL,
    contract_id       UUID        REFERENCES contracts(id) ON DELETE CASCADE,
    -- 'checksum_mismatch' | 'missing_checksum' | 'missing_data'.
    issue_type        TEXT        NOT NULL,
    expected_checksum TEXT,
    actual_checksum   TEXT,
    -- 'minor' | 'major' | 'critical'.
    severity          TEXT        NOT NULL DEFAULT 'major',
    -- 'open' | 'repaired' | 'resolved' | 'ignored'.
    status            TEXT        NOT NULL DEFAULT 'open',
    details           JSONB       NOT NULL DEFAULT '{}',
    detected_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at       TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_integrity_issues_status      ON integrity_issues (status);
CREATE INDEX IF NOT EXISTS idx_integrity_issues_contract_id ON integrity_issues (contract_id);
CREATE INDEX IF NOT EXISTS idx_integrity_issues_resource    ON integrity_issues (resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_integrity_issues_detected_at ON integrity_issues (detected_at DESC);
