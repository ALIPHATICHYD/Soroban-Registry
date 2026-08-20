//! Pure types and rules for contract dependency graphs and transitive risk
//! (Issue #1147).
//!
//! Everything here is deterministic and free of I/O, so the rules that decide
//! *what counts as a risk* and *how risks combine* can be tested exhaustively
//! without a database. The SQL traversal in `api::dependency_graph` and the
//! propagation in `api::dependency_risk` supply rows; this module decides what
//! they mean and in what order they are reported.
//!
//! Like [`crate::contract_spec`], this module is deliberately dependency-light
//! so every workspace member -- API, CLI, verifier -- can share one definition
//! of the wire shapes and one definition of the rules.
//!
//! ## Two arrays, not one
//!
//! Findings carry a severity; diagnostics do not. A cycle, an unresolved edge,
//! or a truncated traversal is a fact about the *graph*, not a security
//! condition, and giving it a severity would drag `max()` over every cyclic
//! graph up to at least Low -- making "zero findings" unreachable and the whole
//! report useless as a gate. See [`Diagnostic`].

use crate::models::IssueSeverity;
use crate::semver::{SemVer, VersionConstraint};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

// ── Traversal budget ────────────────────────────────────────────────────────

/// Default traversal depth when the caller does not ask for one.
///
/// One constant, referenced everywhere. Before Issue #1147 there were four
/// different limits in three modules (20, 5, and 8), so the same graph reported
/// a different shape depending on which endpoint you asked.
pub const DEFAULT_MAX_DEPTH: u32 = 10;

/// Hard ceiling on `depth`, regardless of what the caller requests.
pub const MAX_DEPTH_CAP: u32 = 32;

/// Default cap on distinct nodes returned by one traversal.
pub const DEFAULT_MAX_NODES: usize = 1_000;

/// Hard ceiling on `max_nodes`. Also the reason offset pagination is
/// defensible here: the result set can never exceed this.
pub const MAX_NODES_CAP: usize = 10_000;

/// Wall-clock budget applied as `SET LOCAL statement_timeout` around a
/// traversal.
///
/// Dropping an axum future does **not** cancel an in-flight sqlx query: the
/// Postgres backend keeps running and the pooled connection stays checked out.
/// A client that disconnects mid-traversal would otherwise cost a connection
/// for as long as the query takes. This makes the bound enforceable rather than
/// aspirational.
pub const TRAVERSAL_STATEMENT_TIMEOUT_MS: u32 = 10_000;

/// Clamp a caller-supplied depth into `1..=MAX_DEPTH_CAP`.
pub fn clamp_depth(requested: Option<u32>) -> u32 {
    requested
        .unwrap_or(DEFAULT_MAX_DEPTH)
        .clamp(1, MAX_DEPTH_CAP)
}

/// Clamp a caller-supplied node budget into `1..=MAX_NODES_CAP`.
pub fn clamp_max_nodes(requested: Option<usize>) -> usize {
    requested
        .unwrap_or(DEFAULT_MAX_NODES)
        .clamp(1, MAX_NODES_CAP)
}

// ── Edge vocabulary ─────────────────────────────────────────────────────────

/// Where an edge came from. Mirrors the `dependency_edge_source` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeSource {
    /// Declared by the publisher through the dependencies endpoint.
    Declared,
    /// Observed on chain, derived live from daily call aggregates.
    Telemetry,
}

/// Whether an edge's declared reference bound to a registry contract.
/// Mirrors the `dependency_edge_state` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeState {
    Resolved,
    Unresolved,
    NetworkMismatch,
}

impl EdgeState {
    /// Only a resolved edge is walked for risk. An unresolved reference names
    /// nothing whose risk could be inherited.
    pub fn is_traversable(self) -> bool {
        matches!(self, EdgeState::Resolved)
    }
}

// ── Diagnostics ─────────────────────────────────────────────────────────────

/// A fact about the shape of the graph. Carries **no** severity, deliberately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    /// Contracts involved, root first. Empty when the diagnostic is about the
    /// traversal as a whole rather than a specific path.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<Uuid>,
    /// Human-readable explanation. Never a severity.
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticKind {
    /// The traversal re-encountered a contract already on the current path.
    Cycle,
    /// A declared reference that does not name a registered contract.
    UnresolvedEdge,
    /// A declared reference that names a contract on a different network.
    NetworkMismatch,
    /// The traversal hit a depth, node, or time budget.
    Truncated,
    /// Two edges target the same contract under constraints no published
    /// version satisfies.
    VersionConflict,
    /// A node the caller may not see, counted but not named.
    RedactedNode,
}

/// Why a traversal stopped early. Always reported alongside `truncated: true`
/// so a consumer can tell a genuinely small graph from a clipped one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TruncationReason {
    DepthLimit,
    NodeLimit,
    TimeLimit,
}

impl TruncationReason {
    pub fn detail(self, limit: u64) -> String {
        match self {
            Self::DepthLimit => format!("traversal stopped at the depth limit of {limit}"),
            Self::NodeLimit => format!("traversal stopped at the node limit of {limit}"),
            Self::TimeLimit => format!("traversal exceeded its {limit}ms time budget"),
        }
    }
}

// ── Risk rules ──────────────────────────────────────────────────────────────

/// A security condition that can be attached to a contract.
///
/// Each rule declares its own direct and inherited severities. Inherited
/// severity is separate because most conditions matter less at a distance: a
/// deprecated *dependency* is a planning problem, whereas a deprecated contract
/// you are calling directly is an immediate one. Some conditions do not
/// propagate at all -- an interface incompatibility is a fact about one specific
/// edge, so re-reporting it two hops away would be meaningless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskRule {
    /// An open entry in `security_issues` or a matched CVE.
    OpenVulnerability,
    /// The package signature for the current artifact was revoked.
    SignatureRevoked,
    /// The published artifact failed its scan and is quarantined.
    ArtifactQuarantined,
    /// No valid signature exists for the current wasm hash.
    ArtifactUnsigned,
    /// Deprecated with no replacement, or superseded.
    DeprecatedNoReplacement,
    /// Deprecated but with a named replacement and still inside its grace
    /// period. A migration task, not a live hazard, so it does not propagate.
    DeprecatedWithReplacement,
    /// The target's current interface id differs from the one recorded on the
    /// edge when the dependency was declared.
    InterfaceIncompatibility,
}

impl RiskRule {
    /// Stable wire identifier. Used as a sort key and as part of the dedup
    /// identity, so it must never change for an existing rule.
    pub fn id(self) -> &'static str {
        match self {
            Self::OpenVulnerability => "open_vulnerability",
            Self::SignatureRevoked => "signature_revoked",
            Self::ArtifactQuarantined => "artifact_quarantined",
            Self::ArtifactUnsigned => "artifact_unsigned",
            Self::DeprecatedNoReplacement => "deprecated_no_replacement",
            Self::DeprecatedWithReplacement => "deprecated_with_replacement",
            Self::InterfaceIncompatibility => "interface_incompatibility",
        }
    }

    /// Severity when the condition is on the contract being asked about.
    ///
    /// `OpenVulnerability` returns `None`: its severity comes from the recorded
    /// issue, not from this table, and inventing one here would overwrite what
    /// a scanner actually found.
    pub fn direct_severity(self) -> Option<IssueSeverity> {
        match self {
            Self::OpenVulnerability => None,
            Self::SignatureRevoked => Some(IssueSeverity::High),
            Self::ArtifactQuarantined => Some(IssueSeverity::High),
            Self::ArtifactUnsigned => Some(IssueSeverity::Medium),
            Self::DeprecatedNoReplacement => Some(IssueSeverity::Medium),
            Self::DeprecatedWithReplacement => Some(IssueSeverity::Low),
            Self::InterfaceIncompatibility => Some(IssueSeverity::High),
        }
    }

    /// Severity when the condition is on a transitive dependency.
    /// `None` means the condition does not propagate at all.
    pub fn inherited_severity(self, direct: IssueSeverity) -> Option<IssueSeverity> {
        match self {
            // A vulnerable dependency is exactly as exploitable through the
            // caller as it is directly, so severity is carried unchanged.
            Self::OpenVulnerability => Some(direct),
            Self::SignatureRevoked => Some(IssueSeverity::High),
            Self::ArtifactQuarantined => Some(IssueSeverity::Medium),
            Self::ArtifactUnsigned => Some(IssueSeverity::Low),
            Self::DeprecatedNoReplacement => Some(IssueSeverity::Low),
            Self::DeprecatedWithReplacement => None,
            Self::InterfaceIncompatibility => None,
        }
    }
}

/// One severity-bearing security condition, with provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: IssueSeverity,
    /// The contract the condition is actually on.
    pub origin_contract_id: Uuid,
    /// Stable identity of the underlying record (a `security_issues` row, a CVE
    /// id, or the rule id when the condition is a property of the contract
    /// itself). Part of the dedup key.
    pub finding_id: String,
    /// Root-to-origin path. `[root]` for a direct finding.
    pub path: Vec<Uuid>,
    /// Hops from the root. `0` for a direct finding. Always emitted so
    /// consumers can gate on distance.
    pub inherited_via_depth: u32,
    pub detail: String,
}

impl Finding {
    pub fn is_direct(&self) -> bool {
        self.inherited_via_depth == 0
    }

    /// Identity for deduplication: the same underlying condition on the same
    /// contract, however many paths reach it.
    fn dedup_key(&self) -> (&str, Uuid, &str) {
        (
            self.rule_id.as_str(),
            self.origin_contract_id,
            self.finding_id.as_str(),
        )
    }
}

// ── Aggregation ─────────────────────────────────────────────────────────────

/// Count of findings at each severity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
}

impl SeverityCounts {
    pub fn tally(findings: &[Finding]) -> Self {
        let mut counts = Self::default();
        for finding in findings {
            match finding.severity {
                IssueSeverity::Critical => counts.critical += 1,
                IssueSeverity::High => counts.high += 1,
                IssueSeverity::Medium => counts.medium += 1,
                IssueSeverity::Low => counts.low += 1,
            }
        }
        counts
    }

    pub fn total(&self) -> u32 {
        self.critical + self.high + self.medium + self.low
    }

    /// Ordering key: most severe bucket first, then counts within it.
    /// Float-free and total, so two identical inputs always compare equal.
    fn ordering_key(&self) -> (u32, u32, u32, u32) {
        (self.critical, self.high, self.medium, self.low)
    }
}

/// The overall risk of a contract and its dependency closure.
///
/// Compared as `(effective_severity, counts)` lexicographically rather than a
/// bare `max()`. `max()` alone ranks one High identically to forty Highs, which
/// is exactly the distinction an operator triaging an upgrade needs; and
/// severity is ordinal, so summing or averaging it is meaningless. The tuple is
/// deterministic, float-free, and is also what the CLI table renders.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveRisk {
    /// `None` means no findings at all -- a state that stays reachable only
    /// because diagnostics are kept out of `findings`.
    pub effective_severity: Option<IssueSeverity>,
    pub counts: SeverityCounts,
}

impl EffectiveRisk {
    pub fn from_findings(findings: &[Finding]) -> Self {
        Self {
            effective_severity: findings.iter().map(|f| f.severity).max(),
            counts: SeverityCounts::tally(findings),
        }
    }

    /// True when this risk meets or exceeds `threshold`. This is what a CLI
    /// `--fail-on` gate calls.
    pub fn meets_threshold(&self, threshold: IssueSeverity) -> bool {
        self.effective_severity
            .is_some_and(|severity| severity >= threshold)
    }
}

impl PartialOrd for EffectiveRisk {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EffectiveRisk {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.effective_severity
            .cmp(&other.effective_severity)
            .then_with(|| self.counts.ordering_key().cmp(&other.counts.ordering_key()))
    }
}

// ── Dedup and ordering ──────────────────────────────────────────────────────

/// Collapse findings that describe the same condition reached by several paths,
/// keeping the **shortest** path.
///
/// Without this the output is non-deterministic: a diamond reaches the same
/// vulnerable contract twice, and which path survives depends on traversal
/// order. Ties break on the path itself (lexicographically), so the result is a
/// total function of the input set rather than of the order it arrived in.
///
/// The shortest path is kept because it is the most actionable: it is the
/// closest route from the contract you asked about to the problem.
pub fn dedup_findings(findings: Vec<Finding>) -> Vec<Finding> {
    let mut best: BTreeMap<(String, Uuid, String), Finding> = BTreeMap::new();

    for finding in findings {
        let key = {
            let (rule, origin, id) = finding.dedup_key();
            (rule.to_string(), origin, id.to_string())
        };

        match best.get(&key) {
            Some(existing)
                if (existing.path.len(), &existing.path) <= (finding.path.len(), &finding.path) => {
            }
            _ => {
                best.insert(key, finding);
            }
        }
    }

    let mut out: Vec<Finding> = best.into_values().collect();
    sort_findings(&mut out);
    out
}

/// Deterministic report order: most severe first, then rule id, then path.
///
/// No floats anywhere in the key, and every component is total, so two calls
/// with the same input serialize to identical bytes.
pub fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.rule_id.cmp(&b.rule_id))
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.origin_contract_id.cmp(&b.origin_contract_id))
            .then_with(|| a.finding_id.cmp(&b.finding_id))
    });
}

/// Deterministic diagnostic order, on the same principle as [`sort_findings`].
pub fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.detail.cmp(&b.detail))
    });
}

// ── Cycles ──────────────────────────────────────────────────────────────────

/// Extract the cyclic segment of a path that has revisited `repeated`.
///
/// The recursive CTE stops descending when the next target is already on the
/// path; this turns "we stopped here" into the actual cycle, so the diagnostic
/// names the loop rather than the whole path leading into it.
/// Returns `None` when `repeated` is not on `path` (no cycle).
pub fn cycle_segment(path: &[Uuid], repeated: Uuid) -> Option<Vec<Uuid>> {
    let start = path.iter().position(|node| *node == repeated)?;
    let mut segment = path[start..].to_vec();
    segment.push(repeated);
    Some(segment)
}

// ── Version conflicts ───────────────────────────────────────────────────────

/// Do two constraints on the same target have a commonly satisfying published
/// version?
///
/// `version_constraint` is opaque and defaults to `"*"`, for which
/// [`VersionConstraint::parse`] returns `None`; and the target has no version on
/// `contracts` at all -- versions live in `contract_versions`. So a conflict
/// cannot be decided from the constraints alone. It is defined against the
/// target's actually-published versions: two constraints conflict when **no**
/// published version satisfies both.
///
/// An unparseable constraint (including `"*"`) is treated as "matches
/// everything" and therefore never creates a conflict. Reporting a conflict
/// because a constraint could not be parsed would flag a parser gap as a
/// dependency problem.
pub fn constraints_conflict(a: &str, b: &str, published_versions: &[String]) -> bool {
    let (Some(ca), Some(cb)) = (VersionConstraint::parse(a), VersionConstraint::parse(b)) else {
        return false;
    };

    // A target with no published versions cannot be shown to conflict: there is
    // nothing to satisfy either constraint, so any claim would be speculation.
    if published_versions.is_empty() {
        return false;
    }

    !published_versions
        .iter()
        .filter_map(|version| SemVer::parse(version))
        .any(|version| ca.matches(&version) && cb.matches(&version))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid(n: u8) -> Uuid {
        Uuid::from_bytes([n; 16])
    }

    fn finding(rule: RiskRule, severity: IssueSeverity, origin: u8, path: &[u8]) -> Finding {
        Finding {
            rule_id: rule.id().to_string(),
            severity,
            origin_contract_id: uuid(origin),
            finding_id: format!("{}-{origin}", rule.id()),
            path: path.iter().copied().map(uuid).collect(),
            inherited_via_depth: path.len().saturating_sub(1) as u32,
            detail: String::new(),
        }
    }

    // ── Budget ──────────────────────────────────────────────────────────────

    #[test]
    fn depth_defaults_and_clamps() {
        assert_eq!(clamp_depth(None), DEFAULT_MAX_DEPTH);
        assert_eq!(clamp_depth(Some(3)), 3);
        assert_eq!(clamp_depth(Some(0)), 1, "zero depth would return nothing");
        assert_eq!(clamp_depth(Some(u32::MAX)), MAX_DEPTH_CAP);
    }

    #[test]
    fn node_budget_defaults_and_clamps() {
        assert_eq!(clamp_max_nodes(None), DEFAULT_MAX_NODES);
        assert_eq!(clamp_max_nodes(Some(50)), 50);
        assert_eq!(clamp_max_nodes(Some(0)), 1);
        assert_eq!(clamp_max_nodes(Some(usize::MAX)), MAX_NODES_CAP);
    }

    #[test]
    fn node_cap_justifies_offset_pagination() {
        // Offset pagination is only defensible because the result set is
        // bounded. If this ever grows unbounded, revisit the pagination mode.
        assert!(MAX_NODES_CAP <= 10_000);
    }

    // ── Rule table ──────────────────────────────────────────────────────────

    #[test]
    fn open_vulnerability_severity_comes_from_the_record_not_the_table() {
        assert_eq!(RiskRule::OpenVulnerability.direct_severity(), None);
        for severity in [
            IssueSeverity::Low,
            IssueSeverity::Medium,
            IssueSeverity::High,
            IssueSeverity::Critical,
        ] {
            assert_eq!(
                RiskRule::OpenVulnerability.inherited_severity(severity),
                Some(severity),
                "a vulnerable dependency is as exploitable through the caller"
            );
        }
    }

    #[test]
    fn inherited_severity_never_exceeds_direct() {
        for rule in [
            RiskRule::SignatureRevoked,
            RiskRule::ArtifactQuarantined,
            RiskRule::ArtifactUnsigned,
            RiskRule::DeprecatedNoReplacement,
            RiskRule::DeprecatedWithReplacement,
            RiskRule::InterfaceIncompatibility,
        ] {
            let direct = rule.direct_severity().expect("has a direct severity");
            if let Some(inherited) = rule.inherited_severity(direct) {
                assert!(
                    inherited <= direct,
                    "{} inherited {inherited:?} > direct {direct:?}",
                    rule.id()
                );
            }
        }
    }

    #[test]
    fn edge_specific_rules_do_not_propagate() {
        assert_eq!(
            RiskRule::InterfaceIncompatibility.inherited_severity(IssueSeverity::High),
            None,
            "incompatibility is a fact about one edge"
        );
        assert_eq!(
            RiskRule::DeprecatedWithReplacement.inherited_severity(IssueSeverity::Low),
            None,
            "an in-grace-period deprecation with a replacement is a migration task"
        );
    }

    #[test]
    fn rule_ids_are_unique_and_stable() {
        let rules = [
            RiskRule::OpenVulnerability,
            RiskRule::SignatureRevoked,
            RiskRule::ArtifactQuarantined,
            RiskRule::ArtifactUnsigned,
            RiskRule::DeprecatedNoReplacement,
            RiskRule::DeprecatedWithReplacement,
            RiskRule::InterfaceIncompatibility,
        ];
        let mut ids: Vec<&str> = rules.iter().map(|r| r.id()).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "rule ids must be unique");
    }

    // ── Combinator ──────────────────────────────────────────────────────────

    #[test]
    fn zero_findings_is_reachable() {
        let risk = EffectiveRisk::from_findings(&[]);
        assert_eq!(risk.effective_severity, None);
        assert_eq!(risk.counts.total(), 0);
        assert!(!risk.meets_threshold(IssueSeverity::Low));
    }

    #[test]
    fn many_highs_outrank_one_high() {
        let one = EffectiveRisk::from_findings(&[finding(
            RiskRule::SignatureRevoked,
            IssueSeverity::High,
            1,
            &[0, 1],
        )]);
        let many = EffectiveRisk::from_findings(&[
            finding(RiskRule::SignatureRevoked, IssueSeverity::High, 1, &[0, 1]),
            finding(
                RiskRule::ArtifactQuarantined,
                IssueSeverity::High,
                2,
                &[0, 2],
            ),
        ]);
        assert_eq!(one.effective_severity, many.effective_severity);
        assert!(many > one, "a bare max() would rank these equal");
    }

    #[test]
    fn one_critical_outranks_many_highs() {
        let critical = EffectiveRisk::from_findings(&[finding(
            RiskRule::OpenVulnerability,
            IssueSeverity::Critical,
            1,
            &[0, 1],
        )]);
        let highs = EffectiveRisk::from_findings(
            &(1..=40)
                .map(|n| finding(RiskRule::SignatureRevoked, IssueSeverity::High, n, &[0, n]))
                .collect::<Vec<_>>(),
        );
        assert!(
            critical > highs,
            "severity is ordinal; forty Highs must not sum into a Critical"
        );
    }

    #[test]
    fn threshold_gate_is_inclusive() {
        let risk = EffectiveRisk::from_findings(&[finding(
            RiskRule::SignatureRevoked,
            IssueSeverity::High,
            1,
            &[0, 1],
        )]);
        assert!(risk.meets_threshold(IssueSeverity::Low));
        assert!(risk.meets_threshold(IssueSeverity::High));
        assert!(!risk.meets_threshold(IssueSeverity::Critical));
    }

    // ── Dedup ───────────────────────────────────────────────────────────────

    #[test]
    fn same_finding_via_two_paths_keeps_the_shortest() {
        let short = finding(RiskRule::OpenVulnerability, IssueSeverity::High, 9, &[0, 9]);
        let long = finding(
            RiskRule::OpenVulnerability,
            IssueSeverity::High,
            9,
            &[0, 1, 2, 9],
        );

        let from_long_first = dedup_findings(vec![long.clone(), short.clone()]);
        let from_short_first = dedup_findings(vec![short.clone(), long]);

        assert_eq!(from_long_first.len(), 1);
        assert_eq!(from_long_first, from_short_first, "order must not matter");
        assert_eq!(from_long_first[0].path, short.path);
        assert_eq!(from_long_first[0].inherited_via_depth, 1);
    }

    #[test]
    fn equal_length_paths_break_ties_lexicographically() {
        let via_one = finding(
            RiskRule::OpenVulnerability,
            IssueSeverity::High,
            9,
            &[0, 1, 9],
        );
        let via_two = finding(
            RiskRule::OpenVulnerability,
            IssueSeverity::High,
            9,
            &[0, 2, 9],
        );

        let a = dedup_findings(vec![via_one.clone(), via_two.clone()]);
        let b = dedup_findings(vec![via_two, via_one.clone()]);
        assert_eq!(a, b);
        assert_eq!(a[0].path, via_one.path, "lower path sorts first");
    }

    #[test]
    fn distinct_conditions_on_one_contract_are_not_collapsed() {
        let out = dedup_findings(vec![
            finding(RiskRule::OpenVulnerability, IssueSeverity::High, 9, &[0, 9]),
            finding(RiskRule::ArtifactUnsigned, IssueSeverity::Low, 9, &[0, 9]),
        ]);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn same_rule_on_different_contracts_is_not_collapsed() {
        let out = dedup_findings(vec![
            finding(RiskRule::ArtifactUnsigned, IssueSeverity::Low, 8, &[0, 8]),
            finding(RiskRule::ArtifactUnsigned, IssueSeverity::Low, 9, &[0, 9]),
        ]);
        assert_eq!(out.len(), 2);
    }

    // ── Ordering ────────────────────────────────────────────────────────────

    #[test]
    fn findings_order_by_severity_then_rule_then_path() {
        let mut findings = vec![
            finding(RiskRule::ArtifactUnsigned, IssueSeverity::Low, 3, &[0, 3]),
            finding(
                RiskRule::OpenVulnerability,
                IssueSeverity::Critical,
                1,
                &[0, 1],
            ),
            finding(RiskRule::SignatureRevoked, IssueSeverity::High, 2, &[0, 2]),
            finding(
                RiskRule::ArtifactQuarantined,
                IssueSeverity::High,
                4,
                &[0, 4],
            ),
        ];
        sort_findings(&mut findings);

        let ids: Vec<&str> = findings.iter().map(|f| f.rule_id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "open_vulnerability",   // Critical
                "artifact_quarantined", // High, rule id sorts before signature_revoked
                "signature_revoked",    // High
                "artifact_unsigned",    // Low
            ]
        );
    }

    #[test]
    fn ordering_is_stable_across_input_permutations() {
        let base = vec![
            finding(RiskRule::SignatureRevoked, IssueSeverity::High, 2, &[0, 2]),
            finding(RiskRule::SignatureRevoked, IssueSeverity::High, 1, &[0, 1]),
            finding(RiskRule::ArtifactUnsigned, IssueSeverity::Low, 3, &[0, 3]),
        ];
        let mut forward = base.clone();
        let mut reverse: Vec<Finding> = base.into_iter().rev().collect();
        sort_findings(&mut forward);
        sort_findings(&mut reverse);
        assert_eq!(forward, reverse);
    }

    // ── Cycles ──────────────────────────────────────────────────────────────

    #[test]
    fn cycle_segment_names_the_loop_not_the_lead_in() {
        // 0 -> 1 -> 2 -> 3, and 3 depends on 1 again.
        let path = [uuid(0), uuid(1), uuid(2), uuid(3)];
        let segment = cycle_segment(&path, uuid(1)).expect("cycle");
        assert_eq!(segment, vec![uuid(1), uuid(2), uuid(3), uuid(1)]);
    }

    #[test]
    fn self_loop_is_a_cycle_of_one() {
        let segment = cycle_segment(&[uuid(0), uuid(5)], uuid(5)).expect("cycle");
        assert_eq!(segment, vec![uuid(5), uuid(5)]);
    }

    #[test]
    fn no_cycle_when_the_node_is_not_on_the_path() {
        assert_eq!(cycle_segment(&[uuid(0), uuid(1)], uuid(7)), None);
    }

    // ── Version conflicts ───────────────────────────────────────────────────

    #[test]
    fn disjoint_major_ranges_conflict() {
        let published = vec!["1.0.0".to_string(), "2.0.0".to_string()];
        assert!(constraints_conflict("^1.0.0", "^2.0.0", &published));
    }

    #[test]
    fn overlapping_ranges_do_not_conflict() {
        let published = vec!["1.2.0".to_string()];
        assert!(!constraints_conflict("^1.0.0", "~1.2.0", &published));
    }

    #[test]
    fn wildcard_never_conflicts() {
        // "*" does not parse as a constraint, and treating a parser gap as a
        // dependency problem would flood the report with false conflicts.
        let published = vec!["1.0.0".to_string()];
        assert!(!constraints_conflict("*", "^2.0.0", &published));
        assert!(!constraints_conflict("*", "*", &published));
    }

    #[test]
    fn no_published_versions_means_no_provable_conflict() {
        assert!(!constraints_conflict("^1.0.0", "^2.0.0", &[]));
    }

    #[test]
    fn unparseable_published_versions_are_skipped_not_matched() {
        let published = vec!["not-a-version".to_string()];
        assert!(constraints_conflict("^1.0.0", "^2.0.0", &published));
    }

    // ── Edge vocabulary ─────────────────────────────────────────────────────

    #[test]
    fn only_resolved_edges_are_traversed() {
        assert!(EdgeState::Resolved.is_traversable());
        assert!(!EdgeState::Unresolved.is_traversable());
        assert!(!EdgeState::NetworkMismatch.is_traversable());
    }

    #[test]
    fn edge_vocabulary_serializes_as_the_database_spells_it() {
        assert_eq!(
            serde_json::to_string(&EdgeSource::Telemetry).unwrap(),
            "\"telemetry\""
        );
        assert_eq!(
            serde_json::to_string(&EdgeState::NetworkMismatch).unwrap(),
            "\"network_mismatch\""
        );
    }
}
