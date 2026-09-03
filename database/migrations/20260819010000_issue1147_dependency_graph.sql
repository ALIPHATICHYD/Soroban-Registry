-- Issue #1147: canonical contract dependency edges, plus a compatibility view.
--
-- ## Why a new table
--
-- Eleven call sites in `backend/api` query a table named `contract_dependencies`
-- that no migration has ever created, under two mutually incompatible schemas:
-- a "static" shape (contract_id / dependency_name / dependency_contract_id /
-- version_constraint) and a "call" shape (caller_id / callee_contract_id /
-- call_volume). Migration 062 documents the phantom in its own comment and
-- guards its ALTER behind `to_regclass(...) IS NOT NULL`.
--
-- This migration introduces one canonical edge model and, separately, ships a
-- VIEW literally named `contract_dependencies` projecting the legacy static
-- shape, so the existing static-shaped call sites work unchanged.
--
-- ## Landmine: migration 062 vs. this view
--
-- `062_add_dependency_type.sql` runs `ALTER TABLE contract_dependencies ...`
-- when `to_regclass('public.contract_dependencies')` is non-NULL. ALTER TABLE
-- against a *view* fails. Lexicographic ordering puts `062_` before
-- `20260819010000_`, so on a fresh database 062 sees nothing and no-ops, and we
-- are safe. Anyone who reorders, renumbers, or squashes migrations such that 062
-- runs after this file will break their deploy. 062's import/call/data taxonomy
-- survives here as `dep_kind`.
--
-- ## What is NOT changed
--
-- `contract_static_dependencies` (migration 006) stays the write-through source
-- for declared edges. It backs six live, working endpoints through
-- `dependency::build_dependency_graph`, and this table is an additive read model
-- backfilled from it, so the working path carries no regression risk.
--
-- `contract_call_dependencies` (migration 007) is NOT backfilled: it has zero
-- readers and zero writers, so it would contribute a permanently empty subtree.
-- Telemetry edges are instead read live from
-- `contract_call_edge_daily_aggregates`, which stores both endpoints as UUID
-- foreign keys, is already network-scoped, and is already the historical record.

-- ── Enums ───────────────────────────────────────────────────────────────────

-- Two values, not three. Spec-derived data never produces an edge (dependencies
-- are never inferred from ABI strings; see Issue #1147 D3), so a 'spec' value
-- could never be emitted -- and Postgres enum values are painful to remove.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'dependency_edge_source') THEN
        CREATE TYPE dependency_edge_source AS ENUM ('declared', 'telemetry');
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'dependency_edge_state') THEN
        CREATE TYPE dependency_edge_state AS ENUM ('resolved', 'unresolved', 'network_mismatch');
    END IF;
END $$;

-- ── Canonical edge table ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS contract_dependency_edges (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_contract_id    UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    -- NULL means the reference did not resolve to a registered contract. The
    -- row is retained anyway: the operator declared something real, and
    -- dropping it would silently shrink the graph.
    --
    -- CASCADE, not SET NULL. SET NULL would blank this column while leaving
    -- edge_state = 'resolved', violating
    -- contract_dependency_edges_state_matches_target and making it impossible
    -- to delete any contract that something depends on. CASCADE also matches
    -- the source-side FK and contract_call_edge_daily_aggregates, which
    -- cascades on both endpoints.
    target_contract_id    UUID REFERENCES contracts(id) ON DELETE CASCADE,
    -- Exactly what was declared, preserved verbatim so an unresolved edge can
    -- still be reported and can resolve later without re-declaring.
    target_ref            TEXT NOT NULL,
    network               network_type NOT NULL,
    edge_source           dependency_edge_source NOT NULL,
    edge_state            dependency_edge_state NOT NULL,
    -- Migration 062's import|call|data taxonomy.
    dep_kind              TEXT,
    version_constraint    TEXT,
    -- The target's interface fingerprint and wasm hash *at the time the edge was
    -- recorded*. Current state is always JOINed from `contracts`, never
    -- denormalized here: an incompatibility is precisely the drift between these
    -- recorded values and the target's current ones.
    expected_interface_id TEXT,
    expected_wasm_hash    TEXT,
    recorded_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- NULL means this row is the current state of the edge. Bitemporal history
    -- applies to declared edges only; telemetry is already day-bucketed at the
    -- source and is read live.
    superseded_at         TIMESTAMPTZ,
    CONSTRAINT contract_dependency_edges_no_self_edge
        CHECK (source_contract_id IS DISTINCT FROM target_contract_id),
    -- A resolved edge must name its target; an unresolved or cross-network one
    -- must not, or it would be indistinguishable from a real dependency.
    CONSTRAINT contract_dependency_edges_state_matches_target
        CHECK (
            (edge_state = 'resolved' AND target_contract_id IS NOT NULL)
            OR (edge_state <> 'resolved' AND target_contract_id IS NULL)
        )
);

-- One current row per (source, declared reference, network, source-of-truth).
-- Partial, so superseded history rows are exempt.
CREATE UNIQUE INDEX IF NOT EXISTS uq_contract_dependency_edges_current
    ON contract_dependency_edges (source_contract_id, target_ref, network, edge_source)
    WHERE superseded_at IS NULL;

-- Forward traversal: the recursive CTE walks source -> target over current rows.
CREATE INDEX IF NOT EXISTS idx_contract_dependency_edges_forward
    ON contract_dependency_edges (source_contract_id, network)
    WHERE superseded_at IS NULL;

-- Reverse traversal for /dependents.
CREATE INDEX IF NOT EXISTS idx_contract_dependency_edges_reverse
    ON contract_dependency_edges (target_contract_id, network)
    WHERE superseded_at IS NULL AND target_contract_id IS NOT NULL;

-- `as_of` replay reads history by recording time.
CREATE INDEX IF NOT EXISTS idx_contract_dependency_edges_recorded_at
    ON contract_dependency_edges (source_contract_id, recorded_at DESC);

COMMENT ON TABLE contract_dependency_edges IS
    'Canonical, network-scoped, bitemporal contract dependency edges (Issue #1147). superseded_at IS NULL selects current state.';
COMMENT ON COLUMN contract_dependency_edges.target_ref IS
    'The dependency reference exactly as declared. Retained even when it does not resolve.';
COMMENT ON COLUMN contract_dependency_edges.expected_interface_id IS
    'contracts.interface_id of the target when this edge was recorded. Drift from the target current value is an interface incompatibility.';

-- ── Backfill from the existing declared-dependency table ────────────────────
--
-- Additive and idempotent. `contract_static_dependencies` has no network column
-- of its own; an edge belongs to the declaring contract's network, which is also
-- the only network its target could legitimately have been resolved against.
--
-- Legacy rows that resolved by *name* (dropped in Issue #1147 D3, because
-- contracts.name has no UNIQUE constraint) are re-recorded as 'unresolved'
-- rather than silently rebound: the recorded binding cannot be trusted, and
-- inventing a new one from the same ambiguous name would repeat the defect.
INSERT INTO contract_dependency_edges (
    source_contract_id, target_contract_id, target_ref, network,
    edge_source, edge_state, version_constraint,
    expected_interface_id, expected_wasm_hash, recorded_at
)
SELECT
    csd.contract_id,
    CASE WHEN target.id IS NOT NULL THEN target.id END,
    csd.dependency_name,
    src.network,
    'declared'::dependency_edge_source,
    CASE WHEN target.id IS NOT NULL
         THEN 'resolved'::dependency_edge_state
         ELSE 'unresolved'::dependency_edge_state
    END,
    csd.version_constraint,
    target.interface_id,
    target.wasm_hash,
    csd.created_at
FROM contract_static_dependencies csd
JOIN contracts src ON src.id = csd.contract_id
-- Network-qualified join. Without it, a strkey deployed on two networks yields
-- two rows, producing duplicate edges and silent cross-network contamination.
LEFT JOIN contracts target
       ON target.id = csd.dependency_contract_id
      AND target.network = src.network
WHERE csd.contract_id <> COALESCE(csd.dependency_contract_id, '00000000-0000-0000-0000-000000000000'::uuid)
ON CONFLICT DO NOTHING;

-- ── Compatibility view ──────────────────────────────────────────────────────
--
-- Named exactly `contract_dependencies` and projecting the legacy static shape,
-- so the static-shaped call sites resolve against real data with no Rust change.
-- Only current, declared edges are exposed: the legacy shape has no vocabulary
-- for supersession or for telemetry-derived edges.
CREATE OR REPLACE VIEW contract_dependencies AS
SELECT
    e.id,
    e.source_contract_id AS contract_id,
    e.target_ref         AS dependency_name,
    e.target_contract_id AS dependency_contract_id,
    COALESCE(e.version_constraint, '*') AS version_constraint,
    e.recorded_at        AS created_at
FROM contract_dependency_edges e
WHERE e.superseded_at IS NULL
  AND e.edge_source = 'declared';

COMMENT ON VIEW contract_dependencies IS
    'Compatibility projection of contract_dependency_edges in the legacy static shape (Issue #1147). Read-only; write through contract_dependency_edges.';
