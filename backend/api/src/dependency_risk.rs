//! Transitive risk propagation with provenance (Issue #1147).
//!
//! Answers "what is wrong with this contract, and what is wrong with the things
//! it depends on" -- and, critically, *by which path*.
//!
//! ## Why this is a new module
//!
//! [`crate::graph_analysis::propagate_vulnerability`] is live and stays
//! untouched. It answers a different question with a float-decay model: how much
//! influence a vulnerability exerts over a network, decaying with distance. That
//! is useful for ranking blast radius, but it cannot say "this specific finding
//! reaches you through this specific chain", and its float scores are not a
//! basis for a deterministic, byte-stable report that a CLI can gate a deploy
//! on. Retrofitting provenance onto it would change what its existing consumers
//! see.
//!
//! ## Findings and diagnostics are separate
//!
//! Only severity-bearing security conditions become findings. Cycles, unresolved
//! edges, truncation, and version conflicts are diagnostics with no severity --
//! see [`shared::dependency_graph`]. Folding them into `findings` would make
//! `max()` rank every cyclic graph at Low or above, so "zero findings" could
//! never be reached and the report would be useless as a gate.
//!
//! ## Cost
//!
//! One traversal, then one batched query per risk source over the whole
//! reachable set -- not one query per node. The set is already bounded by the
//! traversal's node budget, so the total query count is constant in the size of
//! the graph.

use crate::dependency_graph::{self, Direction, TraversalRequest, TraversedEdgeRow};
use crate::error::ApiResult;
use crate::handlers::db_internal_error;
use crate::state::AppState;
use shared::dependency_graph::{
    constraints_conflict, dedup_findings, sort_diagnostics, Diagnostic, DiagnosticKind,
    EffectiveRisk, Finding, RiskRule,
};
use shared::models::IssueSeverity;
use shared::Network;
use sqlx::PgPool;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

/// The full risk report for one contract.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DependencyRiskReport {
    pub contract_id: Uuid,
    pub network: Network,
    /// Severity-bearing conditions on the contract itself.
    pub direct_findings: Vec<Finding>,
    /// Severity-bearing conditions reached through dependencies, each carrying
    /// the shortest path that reaches it.
    pub inherited_findings: Vec<Finding>,
    /// Graph facts with no severity.
    pub diagnostics: Vec<Diagnostic>,
    /// `(effective_severity, counts)` over direct findings only.
    pub direct_risk: EffectiveRisk,
    /// The same over direct and inherited findings together. This is what a
    /// `--fail-on` gate compares against.
    pub overall_risk: EffectiveRisk,
    pub total_dependencies: usize,
    pub max_depth: usize,
    pub truncated: bool,
}

/// Build the risk report for `root`.
pub async fn assess(
    state: &AppState,
    root: Uuid,
    network: Network,
    max_depth: u32,
    caller_org: Option<Uuid>,
) -> ApiResult<DependencyRiskReport> {
    let request = TraversalRequest {
        root,
        network,
        direction: Direction::Dependencies,
        transitive: true,
        max_depth,
        max_nodes: shared::dependency_graph::DEFAULT_MAX_NODES,
        as_of: None,
        caller_org,
        include_telemetry: false,
    };

    let traversal = dependency_graph::traverse(&state.db, &request).await?;

    // Shortest path to each reachable contract. Computed once here so every rule
    // reports the same provenance for the same contract, instead of each rule
    // picking whichever path its own query happened to return.
    let reach = shortest_paths(&traversal.rows);

    let mut subjects: Vec<Uuid> = reach.keys().copied().collect();
    subjects.push(root);

    let mut findings = collect_findings(&state.db, &subjects, root, &reach).await?;
    findings.extend(interface_findings(&traversal.rows, root, &reach));

    let mut diagnostics = traversal.diagnostics.clone();
    diagnostics.extend(version_conflicts(&state.db, &traversal.rows).await?);
    sort_diagnostics(&mut diagnostics);

    let all = dedup_findings(findings);
    let (direct_findings, inherited_findings): (Vec<Finding>, Vec<Finding>) =
        all.iter().cloned().partition(Finding::is_direct);

    Ok(DependencyRiskReport {
        contract_id: root,
        network,
        direct_risk: EffectiveRisk::from_findings(&direct_findings),
        overall_risk: EffectiveRisk::from_findings(&all),
        direct_findings,
        inherited_findings,
        diagnostics,
        total_dependencies: reach.len(),
        max_depth: traversal
            .rows
            .iter()
            .map(|r| r.depth as usize)
            .max()
            .unwrap_or(0),
        truncated: traversal.truncated,
    })
}

/// Shortest root-to-contract path for every visible, resolved node.
///
/// Only visible nodes are included: a contract the caller may not see must not
/// have its vulnerabilities enumerated, even anonymously. The traversal already
/// stops at such a node, so its own dependencies never appear either.
fn shortest_paths(rows: &[TraversedEdgeRow]) -> BTreeMap<Uuid, Vec<Uuid>> {
    let mut best: BTreeMap<Uuid, Vec<Uuid>> = BTreeMap::new();

    for row in rows {
        let Some(target) = row.target_contract_id else {
            continue;
        };
        if !row.visible || row.cycle_with.is_some() {
            continue;
        }

        match best.get(&target) {
            // Ties break on the path itself so the result is a function of the
            // reachable set, not of row order.
            Some(existing) if (existing.len(), existing) <= (row.path.len(), &row.path) => {}
            _ => {
                best.insert(target, row.path.clone());
            }
        }
    }

    best
}

/// Row returned by every contract-keyed risk query.
#[derive(sqlx::FromRow)]
struct RiskRow {
    contract_id: Uuid,
    finding_id: String,
    severity: Option<String>,
    detail: String,
}

/// Run each risk source once over the whole reachable set.
async fn collect_findings(
    pool: &PgPool,
    subjects: &[Uuid],
    root: Uuid,
    reach: &BTreeMap<Uuid, Vec<Uuid>>,
) -> ApiResult<Vec<Finding>> {
    let mut findings = Vec::new();

    // Open vulnerabilities. Severity comes from the record, never from the rule
    // table: overwriting what a scanner actually found would be a lie.
    let rows: Vec<RiskRow> = sqlx::query_as(
        "SELECT contract_id,
                id::text AS finding_id,
                severity::text AS severity,
                title AS detail
         FROM security_issues
         WHERE contract_id = ANY($1) AND status = 'open'",
    )
    .bind(subjects)
    .fetch_all(pool)
    .await
    .map_err(|err| db_internal_error("collect open security issues", err))?;
    push_all(
        &mut findings,
        RiskRule::OpenVulnerability,
        rows,
        root,
        reach,
    );

    // Revoked signatures for the artifact currently published.
    let rows: Vec<RiskRow> = sqlx::query_as(
        "SELECT ps.contract_id,
                ps.id::text AS finding_id,
                NULL::text AS severity,
                'package signature revoked: ' || COALESCE(ps.revoked_reason, 'no reason recorded') AS detail
         FROM package_signatures ps
         JOIN contracts c ON c.id = ps.contract_id AND c.wasm_hash = ps.wasm_hash
         WHERE ps.contract_id = ANY($1) AND ps.status = 'revoked'",
    )
    .bind(subjects)
    .fetch_all(pool)
    .await
    .map_err(|err| db_internal_error("collect revoked signatures", err))?;
    push_all(&mut findings, RiskRule::SignatureRevoked, rows, root, reach);

    // Quarantined artifacts.
    let rows: Vec<RiskRow> = sqlx::query_as(
        "SELECT id AS contract_id,
                id::text AS finding_id,
                NULL::text AS severity,
                'published artifact is quarantined' AS detail
         FROM contracts
         WHERE id = ANY($1) AND artifact_scan_status = 'quarantined'",
    )
    .bind(subjects)
    .fetch_all(pool)
    .await
    .map_err(|err| db_internal_error("collect quarantined artifacts", err))?;
    push_all(
        &mut findings,
        RiskRule::ArtifactQuarantined,
        rows,
        root,
        reach,
    );

    // Unsigned artifacts: no *valid* signature for the wasm hash on the row now.
    // A signature for a previous artifact does not vouch for the current one,
    // which is why the join is on `wasm_hash` rather than on contract alone.
    let rows: Vec<RiskRow> = sqlx::query_as(
        "SELECT c.id AS contract_id,
                c.id::text AS finding_id,
                NULL::text AS severity,
                'no valid signature for the published wasm hash' AS detail
         FROM contracts c
         WHERE c.id = ANY($1)
           AND NOT EXISTS (
               SELECT 1 FROM package_signatures ps
               WHERE ps.contract_id = c.id
                 AND ps.wasm_hash = c.wasm_hash
                 AND ps.status = 'valid'
           )",
    )
    .bind(subjects)
    .fetch_all(pool)
    .await
    .map_err(|err| db_internal_error("collect unsigned artifacts", err))?;
    push_all(&mut findings, RiskRule::ArtifactUnsigned, rows, root, reach);

    // Deprecation. Split by whether a replacement exists and the grace period is
    // still running, because those are different problems: one is a live hazard,
    // the other a scheduled migration.
    let rows: Vec<RiskRow> = sqlx::query_as(
        "SELECT c.id AS contract_id,
                c.id::text AS finding_id,
                NULL::text AS severity,
                'contract is ' || c.deprecation_status
                    || COALESCE(': ' || c.deprecation_reason, '') AS detail
         FROM contracts c
         LEFT JOIN contract_deprecations d ON d.contract_id = c.id
         WHERE c.id = ANY($1)
           AND c.deprecated_at IS NOT NULL
           AND NOT (
               c.replacement_contract_id IS NOT NULL
               AND d.grace_period_days IS NOT NULL
               AND NOW() < c.deprecated_at + make_interval(days => d.grace_period_days)
           )",
    )
    .bind(subjects)
    .fetch_all(pool)
    .await
    .map_err(|err| db_internal_error("collect deprecated contracts", err))?;
    push_all(
        &mut findings,
        RiskRule::DeprecatedNoReplacement,
        rows,
        root,
        reach,
    );

    let rows: Vec<RiskRow> = sqlx::query_as(
        "SELECT c.id AS contract_id,
                c.id::text AS finding_id,
                NULL::text AS severity,
                'deprecated with a replacement; grace period ends '
                    || (c.deprecated_at + make_interval(days => d.grace_period_days))::text AS detail
         FROM contracts c
         JOIN contract_deprecations d ON d.contract_id = c.id
         WHERE c.id = ANY($1)
           AND c.deprecated_at IS NOT NULL
           AND c.replacement_contract_id IS NOT NULL
           AND d.grace_period_days IS NOT NULL
           AND NOW() < c.deprecated_at + make_interval(days => d.grace_period_days)",
    )
    .bind(subjects)
    .fetch_all(pool)
    .await
    .map_err(|err| db_internal_error("collect grace-period deprecations", err))?;
    push_all(
        &mut findings,
        RiskRule::DeprecatedWithReplacement,
        rows,
        root,
        reach,
    );

    Ok(findings)
}

/// Turn raw rows into findings, applying the direct/inherited severity rule.
///
/// A rule whose `inherited_severity` is `None` simply produces no finding for a
/// non-root contract -- the condition is real but does not propagate.
fn push_all(
    out: &mut Vec<Finding>,
    rule: RiskRule,
    rows: Vec<RiskRow>,
    root: Uuid,
    reach: &BTreeMap<Uuid, Vec<Uuid>>,
) {
    for row in rows {
        let recorded = row.severity.as_deref().and_then(parse_severity);
        let is_direct = row.contract_id == root;

        // `direct_severity()` is None only for OpenVulnerability, whose severity
        // is on the record. If a record somehow has neither, there is nothing
        // honest to report, so it is skipped rather than assigned a default.
        let Some(direct) = recorded.or_else(|| rule.direct_severity()) else {
            continue;
        };

        let severity = if is_direct {
            direct
        } else {
            match rule.inherited_severity(direct) {
                Some(severity) => severity,
                None => continue,
            }
        };

        let path = if is_direct {
            vec![root]
        } else {
            match reach.get(&row.contract_id) {
                Some(path) => path.clone(),
                // Unreachable in practice: subjects come from `reach`. Skipping
                // is still the right response -- a finding with no provenance is
                // exactly what this feature exists to avoid emitting.
                None => continue,
            }
        };

        out.push(Finding {
            rule_id: rule.id().to_string(),
            severity,
            origin_contract_id: row.contract_id,
            finding_id: row.finding_id,
            inherited_via_depth: path.len().saturating_sub(1) as u32,
            path,
            detail: row.detail,
        });
    }
}

fn parse_severity(value: &str) -> Option<IssueSeverity> {
    match value {
        "low" => Some(IssueSeverity::Low),
        "medium" => Some(IssueSeverity::Medium),
        "high" => Some(IssueSeverity::High),
        "critical" => Some(IssueSeverity::Critical),
        _ => None,
    }
}

/// Interface incompatibility: the target's interface id now differs from the one
/// recorded on the edge when the dependency was declared.
///
/// Derived from the traversal rows rather than a query, because the comparison
/// is between an edge column and a contract column that the traversal already
/// joined. It does not propagate: it is a fact about one specific edge, so
/// re-reporting it further down the chain would be meaningless.
///
/// A NULL on either side means "unknown", not "different". Reporting drift
/// against an unknown would flag every artifact-less contract as incompatible.
fn interface_findings(
    rows: &[TraversedEdgeRow],
    root: Uuid,
    reach: &BTreeMap<Uuid, Vec<Uuid>>,
) -> Vec<Finding> {
    let mut out = Vec::new();

    for row in rows {
        let (Some(expected), Some(current)) = (
            row.expected_interface_id.as_deref(),
            row.current_interface_id.as_deref(),
        ) else {
            continue;
        };
        if expected == current {
            continue;
        }
        let Some(target) = row.target_contract_id else {
            continue;
        };

        let path = if row.source_contract_id == root {
            vec![root, target]
        } else {
            match reach.get(&target) {
                Some(path) => path.clone(),
                None => continue,
            }
        };

        out.push(Finding {
            rule_id: RiskRule::InterfaceIncompatibility.id().to_string(),
            severity: RiskRule::InterfaceIncompatibility
                .direct_severity()
                .unwrap_or(IssueSeverity::High),
            origin_contract_id: target,
            // Keyed by the edge, not the contract: two dependents can each have
            // recorded a different expectation of the same target.
            finding_id: format!("{}:{}", row.source_contract_id, target),
            inherited_via_depth: path.len().saturating_sub(1) as u32,
            path,
            detail: format!(
                "interface changed since this dependency was declared (expected {expected}, now {current})"
            ),
        });
    }

    out
}

/// Two edges targeting the same contract under constraints that no published
/// version satisfies.
///
/// A diagnostic, not a finding: it is a fact about how the graph was declared,
/// not a security condition, and it has no defensible severity.
async fn version_conflicts(pool: &PgPool, rows: &[TraversedEdgeRow]) -> ApiResult<Vec<Diagnostic>> {
    // Group the declared constraints per target.
    let mut by_target: BTreeMap<Uuid, BTreeSet<String>> = BTreeMap::new();
    for row in rows {
        let (Some(target), Some(constraint)) =
            (row.target_contract_id, row.version_constraint.as_deref())
        else {
            continue;
        };
        by_target
            .entry(target)
            .or_default()
            .insert(constraint.to_string());
    }

    // Only targets with at least two distinct constraints can conflict.
    let contested: Vec<Uuid> = by_target
        .iter()
        .filter(|(_, constraints)| constraints.len() > 1)
        .map(|(target, _)| *target)
        .collect();

    if contested.is_empty() {
        return Ok(Vec::new());
    }

    // The target carries no version on `contracts` -- versions live in
    // `contract_versions` -- so the conflict can only be decided against what is
    // actually published.
    let version_rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT contract_id, version FROM contract_versions WHERE contract_id = ANY($1)",
    )
    .bind(&contested)
    .fetch_all(pool)
    .await
    .map_err(|err| db_internal_error("collect versions for conflict check", err))?;

    let mut published: BTreeMap<Uuid, Vec<String>> = BTreeMap::new();
    for (contract_id, version) in version_rows {
        published.entry(contract_id).or_default().push(version);
    }

    let mut diagnostics = Vec::new();
    for target in contested {
        let constraints: Vec<&String> = by_target[&target].iter().collect();
        let versions = published.get(&target).cloned().unwrap_or_default();

        for (i, a) in constraints.iter().enumerate() {
            for b in &constraints[i + 1..] {
                if constraints_conflict(a, b, &versions) {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::VersionConflict,
                        path: vec![target],
                        detail: format!(
                            "constraints '{a}' and '{b}' on the same dependency have no commonly satisfying published version"
                        ),
                    });
                }
            }
        }
    }

    Ok(diagnostics)
}
