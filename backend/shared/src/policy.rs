//! Policy-as-code admission checks for contract publishing (Issue #1148).
//!
//! Provides deterministic evaluation of verification state, signatures,
//! provenance, vulnerability findings, dependency risk, interface compatibility,
//! network identity, metadata completeness, and artifact size before publishing
//! or accepting registry artifacts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyDecision {
    Allow = 0,
    Warn = 1,
    Deny = 2,
}

impl PolicyDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
        }
    }
}

impl std::fmt::Display for PolicyDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    pub when: String,
    pub decision: PolicyDecision,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDefinition {
    pub version: u32,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
}

impl PolicyDefinition {
    pub fn from_yaml(content: &str) -> Result<Self, PolicyError> {
        let def: Self = serde_yaml::from_str(content)
            .map_err(|e| PolicyError::MalformedPolicy(format!("Failed to parse YAML: {e}")))?;
        def.validate()?;
        Ok(def)
    }

    pub fn from_json(content: &str) -> Result<Self, PolicyError> {
        let def: Self = serde_json::from_str(content)
            .map_err(|e| PolicyError::MalformedPolicy(format!("Failed to parse JSON: {e}")))?;
        def.validate()?;
        Ok(def)
    }

    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.version != 1 {
            return Err(PolicyError::UnsupportedVersion(self.version));
        }
        if self.rules.is_empty() {
            return Err(PolicyError::MalformedPolicy(
                "Policy contains no rules".to_string(),
            ));
        }
        for rule in &self.rules {
            if rule.name.trim().is_empty() {
                return Err(PolicyError::MalformedPolicy(
                    "Rule name cannot be empty".to_string(),
                ));
            }
            if rule.when.trim().is_empty() {
                return Err(PolicyError::MalformedPolicy(format!(
                    "Rule '{}' has an empty 'when' condition",
                    rule.name
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactContext {
    pub signature_verified: bool,
    pub size: u64,
    pub hash: String,
    pub is_wasm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerificationContext {
    pub state: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvenanceContext {
    pub present: bool,
    pub builder_image: Option<String>,
    pub reproducible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VulnerabilityContext {
    pub count: usize,
    pub critical_count: usize,
    pub high_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskContext {
    pub max_severity: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependencyContext {
    pub revoked_count: usize,
    pub has_revoked: bool,
    pub risk_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InterfaceContext {
    pub compatible: bool,
    pub breaking_changes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkContext {
    pub identity: String,
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataContext {
    pub complete: bool,
    pub completeness: f64,
    pub has_readme: bool,
    pub has_license: bool,
    pub has_repository: bool,
    pub has_version: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdmissionContext {
    pub artifact: ArtifactContext,
    pub verification: VerificationContext,
    pub provenance: ProvenanceContext,
    pub vulnerabilities: VulnerabilityContext,
    pub risk: RiskContext,
    pub dependency: DependencyContext,
    pub interface: InterfaceContext,
    pub network: NetworkContext,
    pub metadata: MetadataContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedRule {
    pub name: String,
    pub condition: String,
    pub matched: bool,
    pub decision: PolicyDecision,
    pub reason: Option<String>,
    pub input_values: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluationResult {
    pub policy_version: u32,
    pub decision: PolicyDecision,
    pub allowed: bool,
    pub evaluated_rules: Vec<EvaluatedRule>,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub evidence: serde_json::Value,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PolicyError {
    #[error("Unsupported policy version: {0}. Only version 1 is supported.")]
    UnsupportedVersion(u32),

    #[error("Malformed policy: {0}")]
    MalformedPolicy(String),

    #[error("Ambiguous or invalid rule result for rule '{rule_name}': {details}")]
    AmbiguousRuleResult { rule_name: String, details: String },

    #[error("Policy evaluation failed: {0}")]
    EvaluationFailed(String),

    #[error("Default policy parse error: {0}")]
    DefaultPolicyError(String),
}

/// Maximum number of rules a single policy may contain.
/// Policies exceeding this limit fail closed to keep evaluation bounded.
pub const MAX_POLICY_RULES: usize = 100;

/// Default maximum evaluation duration (seconds).
/// Evaluation aborts with an error if it exceeds this budget.
const DEFAULT_EVAL_TIMEOUT_SECS: u64 = 5;

pub struct PolicyEvaluator;

impl PolicyEvaluator {
    pub fn evaluate(
        policy: &PolicyDefinition,
        context: &AdmissionContext,
    ) -> Result<PolicyEvaluationResult, PolicyError> {
        Self::evaluate_with_timeout(policy, context, DEFAULT_EVAL_TIMEOUT_SECS)
    }

    /// Evaluate with an explicit timeout in seconds.
    pub fn evaluate_with_timeout(
        policy: &PolicyDefinition,
        context: &AdmissionContext,
        timeout_secs: u64,
    ) -> Result<PolicyEvaluationResult, PolicyError> {
        policy.validate()?;

        if policy.rules.len() > MAX_POLICY_RULES {
            return Err(PolicyError::EvaluationFailed(format!(
                "Policy contains {} rules, exceeding the maximum of {}",
                policy.rules.len(),
                MAX_POLICY_RULES
            )));
        }

        let ctx_val = serde_json::to_value(context)
            .map_err(|e| PolicyError::EvaluationFailed(format!("Failed to serialize context: {e}")))?;

        let mut evaluated_rules = Vec::new();
        let mut overall_decision = PolicyDecision::Allow;
        let mut reasons = Vec::new();
        let mut warnings = Vec::new();
        let start = std::time::Instant::now();
        let deadline = std::time::Duration::from_secs(timeout_secs);

        for rule in &policy.rules {
            if start.elapsed() > deadline {
                return Err(PolicyError::EvaluationFailed(format!(
                    "Policy evaluation exceeded {timeout_secs}s timeout"
                )));
            }
            let (matched, input_values) =
                evaluate_expression(&rule.when, &ctx_val).map_err(|details| {
                    PolicyError::AmbiguousRuleResult {
                        rule_name: rule.name.clone(),
                        details,
                    }
                })?;

            if matched {
                if rule.decision > overall_decision {
                    overall_decision = rule.decision;
                }
                let r_reason = rule
                    .reason
                    .clone()
                    .unwrap_or_else(|| format!("Rule '{}' triggered with decision {}", rule.name, rule.decision));

                if rule.decision == PolicyDecision::Deny {
                    reasons.push(r_reason.clone());
                } else if rule.decision == PolicyDecision::Warn {
                    warnings.push(r_reason.clone());
                }
            }

            evaluated_rules.push(EvaluatedRule {
                name: rule.name.clone(),
                condition: rule.when.clone(),
                matched,
                decision: rule.decision,
                reason: rule.reason.clone(),
                input_values,
            });
        }

        let allowed = overall_decision != PolicyDecision::Deny;

        Ok(PolicyEvaluationResult {
            policy_version: policy.version,
            decision: overall_decision,
            allowed,
            evaluated_rules,
            reasons,
            warnings,
            evidence: ctx_val,
            evaluated_at: Utc::now(),
        })
    }
}

fn evaluate_expression(
    expr: &str,
    ctx_val: &serde_json::Value,
) -> Result<(bool, HashMap<String, serde_json::Value>), String> {
    let expr = expr.trim();
    if expr.contains(" && ") {
        let parts: Vec<&str> = expr.split(" && ").collect();
        let mut all_matched = true;
        let mut values = HashMap::new();
        for part in parts {
            let (matched, part_vals) = evaluate_single_expression(part, ctx_val)?;
            values.extend(part_vals);
            if !matched {
                all_matched = false;
            }
        }
        return Ok((all_matched, values));
    }
    if expr.contains(" || ") {
        let parts: Vec<&str> = expr.split(" || ").collect();
        let mut any_matched = false;
        let mut values = HashMap::new();
        for part in parts {
            let (matched, part_vals) = evaluate_single_expression(part, ctx_val)?;
            values.extend(part_vals);
            if matched {
                any_matched = true;
            }
        }
        return Ok((any_matched, values));
    }
    evaluate_single_expression(expr, ctx_val)
}

fn evaluate_single_expression(
    expr: &str,
    ctx_val: &serde_json::Value,
) -> Result<(bool, HashMap<String, serde_json::Value>), String> {
    let expr = expr.trim();
    let op_opt = if let Some(pos) = expr.find("==") {
        Some(("==", pos, 2))
    } else if let Some(pos) = expr.find("!=") {
        Some(("!=", pos, 2))
    } else if let Some(pos) = expr.find(">=") {
        Some((">=", pos, 2))
    } else if let Some(pos) = expr.find("<=") {
        Some(("<=", pos, 2))
    } else if let Some(pos) = expr.find('>') {
        Some((">", pos, 1))
    } else if let Some(pos) = expr.find('<') {
        Some(("<", pos, 1))
    } else if let Some(pos) = expr.find(" in ") {
        Some(("in", pos, 4))
    } else if let Some(pos) = expr.find(" contains ") {
        Some(("contains", pos, 10))
    } else {
        None
    };

    if let Some((op, pos, op_len)) = op_opt {
        let lhs_str = expr[..pos].trim();
        let rhs_str = expr[pos + op_len..].trim();

        let lhs_val = get_path_value(ctx_val, lhs_str)?;
        let mut input_vals = HashMap::new();
        input_vals.insert(lhs_str.to_string(), lhs_val.clone());

        let matched = match op {
            "==" => compare_values(&lhs_val, rhs_str, |a, b| a == b)?,
            "!=" => compare_values(&lhs_val, rhs_str, |a, b| a != b)?,
            ">" => compare_numeric(&lhs_val, rhs_str, |a, b| a > b)?,
            "<" => compare_numeric(&lhs_val, rhs_str, |a, b| a < b)?,
            ">=" => compare_numeric(&lhs_val, rhs_str, |a, b| a >= b)?,
            "<=" => compare_numeric(&lhs_val, rhs_str, |a, b| a <= b)?,
            "in" => check_in(&lhs_val, rhs_str)?,
            "contains" => check_contains(&lhs_val, rhs_str)?,
            _ => return Err(format!("Unsupported operator '{op}'")),
        };
        Ok((matched, input_vals))
    } else if let Some(stripped) = expr.strip_prefix('!') {
        let path = stripped.trim();
        let val = get_path_value(ctx_val, path)?;
        let mut input_vals = HashMap::new();
        input_vals.insert(path.to_string(), val.clone());
        let b = val
            .as_bool()
            .ok_or_else(|| format!("Field '{path}' is not a boolean"))?;
        Ok((!b, input_vals))
    } else {
        let path = expr.trim();
        let val = get_path_value(ctx_val, path)?;
        let mut input_vals = HashMap::new();
        input_vals.insert(path.to_string(), val.clone());
        let b = val
            .as_bool()
            .ok_or_else(|| format!("Field '{path}' is not a boolean"))?;
        Ok((b, input_vals))
    }
}

fn get_path_value(root: &serde_json::Value, path: &str) -> Result<serde_json::Value, String> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = root;
    for part in parts {
        match current.get(part) {
            Some(v) => current = v,
            None => return Err(format!("Field '{path}' not found in admission context")),
        }
    }
    Ok(current.clone())
}

fn parse_rhs_literal(rhs: &str) -> serde_json::Value {
    let trimmed = rhs.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        let inner = &trimmed[1..trimmed.len() - 1];
        serde_json::Value::String(inner.to_string())
    } else if trimmed == "true" {
        serde_json::Value::Bool(true)
    } else if trimmed == "false" {
        serde_json::Value::Bool(false)
    } else if let Ok(n) = trimmed.parse::<i64>() {
        serde_json::Value::Number(n.into())
    } else if let Ok(f) = trimmed.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            serde_json::Value::Number(n)
        } else {
            serde_json::Value::String(trimmed.to_string())
        }
    } else {
        serde_json::Value::String(trimmed.to_string())
    }
}

fn compare_values<F>(lhs: &serde_json::Value, rhs_str: &str, comp: F) -> Result<bool, String>
where
    F: Fn(&serde_json::Value, &serde_json::Value) -> bool,
{
    let rhs_val = parse_rhs_literal(rhs_str);
    match (lhs, &rhs_val) {
        (serde_json::Value::String(a), serde_json::Value::String(b)) => Ok(comp(
            &serde_json::Value::String(a.clone()),
            &serde_json::Value::String(b.clone()),
        )),
        (serde_json::Value::Bool(a), serde_json::Value::Bool(b)) => Ok(comp(
            &serde_json::Value::Bool(*a),
            &serde_json::Value::Bool(*b),
        )),
        (serde_json::Value::Number(a), serde_json::Value::Number(b)) => Ok(comp(
            &serde_json::Value::Number(a.clone()),
            &serde_json::Value::Number(b.clone()),
        )),
        _ => {
            let l_str = match lhs {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => return Err(format!("Cannot compare value {:?} with '{}'", lhs, rhs_str)),
            };
            let r_str = match &rhs_val {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => rhs_str.trim().trim_matches('"').trim_matches('\'').to_string(),
            };
            Ok(comp(
                &serde_json::Value::String(l_str),
                &serde_json::Value::String(r_str),
            ))
        }
    }
}

fn compare_numeric<F>(lhs: &serde_json::Value, rhs_str: &str, comp: F) -> Result<bool, String>
where
    F: Fn(f64, f64) -> bool,
{
    let lhs_num = lhs
        .as_f64()
        .ok_or_else(|| format!("Expected numeric field, found {:?}", lhs))?;
    let rhs_num = rhs_str
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("Invalid numeric literal: '{rhs_str}'"))?;
    Ok(comp(lhs_num, rhs_num))
}

fn check_in(lhs: &serde_json::Value, rhs_str: &str) -> Result<bool, String> {
    let rhs_trimmed = rhs_str.trim();
    if rhs_trimmed.starts_with('[') && rhs_trimmed.ends_with(']') {
        let arr: Vec<serde_json::Value> = serde_json::from_str(rhs_trimmed)
            .map_err(|e| format!("Invalid array in expression '{rhs_str}': {e}"))?;
        Ok(arr.contains(lhs))
    } else {
        let items: Vec<&str> = rhs_trimmed
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\''))
            .collect();
        let lhs_str = match lhs {
            serde_json::Value::String(s) => s.as_str(),
            serde_json::Value::Bool(b) => {
                if *b {
                    "true"
                } else {
                    "false"
                }
            }
            serde_json::Value::Number(n) => return Ok(items.contains(&n.to_string().as_str())),
            _ => return Ok(false),
        };
        Ok(items.contains(&lhs_str))
    }
}

fn check_contains(lhs: &serde_json::Value, rhs_str: &str) -> Result<bool, String> {
    let rhs_val = parse_rhs_literal(rhs_str);
    match lhs {
        serde_json::Value::Array(arr) => Ok(arr.contains(&rhs_val)),
        serde_json::Value::String(s) => {
            let needle = rhs_val.as_str().unwrap_or(rhs_str.trim());
            Ok(s.contains(needle))
        }
        _ => Err(format!(
            "'contains' operator requires array or string field, found {:?}",
            lhs
        )),
    }
}

/// Resolve effective policy by merging an override with an optional default.
///
/// The `override_policy` takes full precedence when present; the
/// `default_policy` is used as a fallback. When both are `None`, the
/// function returns `None` (no policy enforcement).
///
/// This supports the requirement that repository or organization defaults
/// can be set, with explicit CLI/API overrides.
pub fn resolve_effective_policy(
    override_policy: Option<&PolicyDefinition>,
    default_policy: Option<&PolicyDefinition>,
) -> Result<Option<PolicyDefinition>, PolicyError> {
    if let Some(ov) = override_policy {
        ov.validate()?;
        Ok(Some(ov.clone()))
    } else if let Some(def) = default_policy {
        def.validate()?;
        Ok(Some(def.clone()))
    } else {
        Ok(None)
    }
}

/// Parse a policy from an optional YAML/JSON string, returning None if absent.
pub fn parse_optional_policy(
    content: Option<&str>,
) -> Result<Option<PolicyDefinition>, PolicyError> {
    match content {
        Some(c) => {
            let trimmed = c.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            let def = PolicyDefinition::from_yaml(trimmed)
                .or_else(|_| PolicyDefinition::from_json(trimmed))?;
            Ok(Some(def))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_context() -> AdmissionContext {
        AdmissionContext {
            artifact: ArtifactContext {
                signature_verified: false,
                size: 1024,
                hash: "a1b2c3d4".to_string(),
                is_wasm: true,
            },
            verification: VerificationContext {
                state: "unverified".to_string(),
                verified: false,
            },
            provenance: ProvenanceContext {
                present: false,
                builder_image: None,
                reproducible: false,
            },
            vulnerabilities: VulnerabilityContext {
                count: 1,
                critical_count: 1,
                high_count: 0,
            },
            risk: RiskContext {
                max_severity: "critical".to_string(),
                score: 9.5,
            },
            dependency: DependencyContext {
                revoked_count: 1,
                has_revoked: true,
                risk_score: 8.0,
            },
            interface: InterfaceContext {
                compatible: false,
                breaking_changes: 2,
            },
            network: NetworkContext {
                identity: "testnet".to_string(),
                passphrase: Some("Test Stellar Network ; July 2015".to_string()),
            },
            metadata: MetadataContext {
                complete: false,
                completeness: 0.5,
                has_readme: true,
                has_license: false,
                has_repository: true,
                has_version: true,
            },
        }
    }

    /// A permissive context where everything passes.
    fn clean_context() -> AdmissionContext {
        AdmissionContext {
            artifact: ArtifactContext {
                signature_verified: true,
                size: 2048,
                hash: "deadbeef".to_string(),
                is_wasm: true,
            },
            verification: VerificationContext {
                state: "verified".to_string(),
                verified: true,
            },
            provenance: ProvenanceContext {
                present: true,
                builder_image: Some("soroban/contract:latest".to_string()),
                reproducible: true,
            },
            vulnerabilities: VulnerabilityContext {
                count: 0,
                critical_count: 0,
                high_count: 0,
            },
            risk: RiskContext {
                max_severity: "none".to_string(),
                score: 0.0,
            },
            dependency: DependencyContext {
                revoked_count: 0,
                has_revoked: false,
                risk_score: 0.0,
            },
            interface: InterfaceContext {
                compatible: true,
                breaking_changes: 0,
            },
            network: NetworkContext {
                identity: "mainnet".to_string(),
                passphrase: Some("Public Global Stellar Network ; September 2015".to_string()),
            },
            metadata: MetadataContext {
                complete: true,
                completeness: 1.0,
                has_readme: true,
                has_license: true,
                has_repository: true,
                has_version: true,
            },
        }
    }

    // ── Rule evaluation ────────────────────────────────────────────────────

    #[test]
    fn test_sample_policy_yaml_parsing_and_evaluation() {
        let yaml = r#"
version: 1
name: example-policy
description: Pre-publish security checks
rules:
  - name: require-signed-artifact
    when: artifact.signature_verified != true
    decision: deny
    reason: "Unsigned artifacts are not allowed"
  - name: block-critical-risk
    when: risk.max_severity == "critical"
    decision: deny
    reason: "Critical risk detected"
  - name: require-provenance
    when: provenance.present != true
    decision: warn
    reason: "Missing provenance details"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).expect("YAML parsing failed");
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).expect("Evaluation failed");

        assert_eq!(res.decision, PolicyDecision::Deny);
        assert!(!res.allowed);
        assert_eq!(res.evaluated_rules.len(), 3);
        assert!(res.reasons.contains(&"Unsigned artifacts are not allowed".to_string()));
        assert!(res.reasons.contains(&"Critical risk detected".to_string()));
    }

    #[test]
    fn test_clean_context_allows() {
        let yaml = r#"
version: 1
rules:
  - name: require-signed
    when: artifact.signature_verified != true
    decision: deny
    reason: "Unsigned artifacts are not allowed"
  - name: require-provenance
    when: provenance.present != true
    decision: warn
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = clean_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();

        assert_eq!(res.decision, PolicyDecision::Allow);
        assert!(res.allowed);
        assert!(res.reasons.is_empty());
        assert!(res.warnings.is_empty());
    }

    // ── Precedence ─────────────────────────────────────────────────────────

    #[test]
    fn test_precedence_deny_over_warn_over_allow() {
        let yaml = r#"
version: 1
rules:
  - name: warning-rule
    when: metadata.has_license == false
    decision: warn
    reason: "License missing"
  - name: deny-rule
    when: dependency.has_revoked == true
    decision: deny
    reason: "Revoked dependency"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();

        assert_eq!(res.decision, PolicyDecision::Deny);
        assert!(!res.allowed);
        assert_eq!(res.warnings.len(), 1);
        assert_eq!(res.reasons.len(), 1);
    }

    #[test]
    fn test_warn_only_decision() {
        let yaml = r#"
version: 1
rules:
  - name: missing-license
    when: metadata.has_license == false
    decision: warn
    reason: "License file not found"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();

        assert_eq!(res.decision, PolicyDecision::Warn);
        assert!(res.allowed);
        assert!(res.warnings.contains(&"License file not found".to_string()));
    }

    #[test]
    fn test_multiple_deny_reasons_collected() {
        let yaml = r#"
version: 1
rules:
  - name: block-critical
    when: risk.max_severity == "critical"
    decision: deny
    reason: "Critical risk"
  - name: block-revoked
    when: dependency.has_revoked == true
    decision: deny
    reason: "Revoked dependency"
  - name: block-incompatible
    when: interface.compatible == false
    decision: deny
    reason: "Incompatible interface"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();

        assert_eq!(res.decision, PolicyDecision::Deny);
        assert_eq!(res.reasons.len(), 3);
    }

    // ── Fails-closed ───────────────────────────────────────────────────────

    #[test]
    fn test_unsupported_version_fails_closed() {
        let yaml = r#"
version: 2
rules:
  - name: test-rule
    when: artifact.is_wasm == true
    decision: allow
"#;
        let res = PolicyDefinition::from_yaml(yaml);
        assert!(matches!(res, Err(PolicyError::UnsupportedVersion(2))));
    }

    #[test]
    fn test_malformed_policy_fails_closed() {
        let yaml = r#"
version: 1
rules: []
"#;
        let res = PolicyDefinition::from_yaml(yaml);
        assert!(matches!(res, Err(PolicyError::MalformedPolicy(_))));
    }

    #[test]
    fn test_malformed_empty_rule_name_fails_closed() {
        let yaml = r#"
version: 1
rules:
  - name: ""
    when: artifact.is_wasm == true
    decision: allow
"#;
        let res = PolicyDefinition::from_yaml(yaml);
        assert!(matches!(res, Err(PolicyError::MalformedPolicy(_))));
    }

    #[test]
    fn test_malformed_empty_when_condition_fails_closed() {
        let yaml = r#"
version: 1
rules:
  - name: empty-cond
    when: ""
    decision: deny
"#;
        let res = PolicyDefinition::from_yaml(yaml);
        assert!(matches!(res, Err(PolicyError::MalformedPolicy(_))));
    }

    #[test]
    fn test_ambiguous_field_fails_closed() {
        let yaml = r#"
version: 1
rules:
  - name: invalid-field
    when: non_existent.field == true
    decision: deny
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx);
        assert!(matches!(res, Err(PolicyError::AmbiguousRuleResult { .. })));
    }

    #[test]
    fn test_exceeding_max_rules_fails_closed() {
        let mut rules = Vec::new();
        for i in 0..=MAX_POLICY_RULES {
            rules.push(format!(
                "  - name: rule-{i}\n    when: artifact.is_wasm == true\n    decision: allow"
            ));
        }
        let yaml = format!("version: 1\nrules:\n{}", rules.join("\n"));
        let def = PolicyDefinition::from_yaml(&yaml).unwrap();
        let ctx = clean_context();
        let res = PolicyEvaluator::evaluate(&def, &ctx);
        assert!(matches!(res, Err(PolicyError::EvaluationFailed(_))));
    }

    // ── Timeout / bounded evaluation ────────────────────────────────────────

    #[test]
    fn test_evaluation_timeout_fails_closed() {
        let yaml = r#"
version: 1
rules:
  - name: check-size
    when: artifact.size > 0
    decision: allow
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = clean_context();
        // Timeout with 0 seconds should fail immediately
        let res = PolicyEvaluator::evaluate_with_timeout(&policy, &ctx, 0);
        assert!(matches!(res, Err(PolicyError::EvaluationFailed(ref msg)) if msg.contains("timeout")));
    }

    #[test]
    fn test_evaluate_with_timeout_succeeds_normally() {
        let yaml = r#"
version: 1
rules:
  - name: check
    when: artifact.is_wasm == true
    decision: allow
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = clean_context();
        let res = PolicyEvaluator::evaluate_with_timeout(&policy, &ctx, 5);
        assert!(res.is_ok());
    }

    // ── Explain output ─────────────────────────────────────────────────────

    #[test]
    fn test_explain_output_contains_all_rule_details() {
        let yaml = r#"
version: 1
rules:
  - name: require-signed
    when: artifact.signature_verified != true
    decision: deny
    reason: "Unsigned artifacts are not allowed"
  - name: require-provenance
    when: provenance.present != true
    decision: warn
    reason: "Missing provenance"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();

        // Every evaluated rule should have condition, matched, decision, reason, input_values
        for rule in &res.evaluated_rules {
            assert!(!rule.condition.is_empty());
            assert!(rule.matched, "rule {} should match", rule.name);
            assert!(rule.input_values.contains_key(&rule.condition.split_whitespace().next().unwrap().to_string()));
        }

        // Evidence should be present
        assert!(res.evidence.is_object());
        assert!(res.evidence.get("artifact").is_some());
    }

    // ── JSON serialization ─────────────────────────────────────────────────

    #[test]
    fn test_policy_check_response_json_roundtrip() {
        use crate::models::{PolicyCheckRequest, PolicyCheckResponse};

        let yaml = r#"
version: 1
rules:
  - name: check
    when: artifact.is_wasm == true
    decision: warn
    reason: "WASM artifact"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = clean_context();
        let eval = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        let resp = PolicyCheckResponse { evaluation: eval };

        let json_str = serde_json::to_string(&resp).unwrap();
        let parsed: PolicyCheckResponse = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.evaluation.decision, PolicyDecision::Warn);
        assert!(parsed.evaluation.allowed);
        assert_eq!(parsed.evaluation.policy_version, 1);
    }

    // ── Specific scenarios from acceptance criteria ─────────────────────────

    #[test]
    fn scenario_unsigned_artifact_denied() {
        let yaml = r#"
version: 1
rules:
  - name: require-signed
    when: artifact.signature_verified != true
    decision: deny
    reason: "Unsigned artifacts are not allowed"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let mut ctx = clean_context();
        ctx.artifact.signature_verified = false;
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert!(!res.allowed);
        assert!(res.reasons.iter().any(|r| r.contains("Unsigned")));
    }

    #[test]
    fn scenario_critical_vulnerability_denied() {
        let yaml = r#"
version: 1
rules:
  - name: block-critical
    when: risk.max_severity == "critical"
    decision: deny
    reason: "Critical risk"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert!(!res.allowed);
    }

    #[test]
    fn scenario_revoked_dependency_denied() {
        let yaml = r#"
version: 1
rules:
  - name: block-revoked
    when: dependency.has_revoked == true
    decision: deny
    reason: "Revoked dependency"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert!(!res.allowed);
        assert!(res.reasons.iter().any(|r| r.contains("Revoked")));
    }

    #[test]
    fn scenario_wrong_network_denied() {
        let yaml = r#"
version: 1
rules:
  - name: require-mainnet
    when: network.identity != "mainnet"
    decision: deny
    reason: "Wrong network"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert!(!res.allowed);
        assert!(res.reasons.iter().any(|r| r.contains("Wrong network")));
    }

    #[test]
    fn scenario_missing_provenance_warned() {
        let yaml = r#"
version: 1
rules:
  - name: require-provenance
    when: provenance.present != true
    decision: warn
    reason: "Missing provenance"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert!(res.allowed);
        assert!(res.warnings.iter().any(|w| w.contains("provenance")));
    }

    #[test]
    fn scenario_incompatible_interface_denied() {
        let yaml = r#"
version: 1
rules:
  - name: require-compatible
    when: interface.compatible == false
    decision: deny
    reason: "Incompatible interface"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert!(!res.allowed);
        assert!(res.reasons.iter().any(|r| r.contains("Incompatible")));
    }

    // ── Operator coverage ──────────────────────────────────────────────────

    #[test]
    fn test_numeric_comparisons() {
        let yaml = r#"
version: 1
rules:
  - name: size-check
    when: artifact.size > 1000000
    decision: deny
    reason: "Artifact too large"
  - name: score-low
    when: risk.score < 1.0
    decision: allow
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context(); // size=1024, score=9.5
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        // size(1024) > 1000000 is false, score(9.5) < 1.0 is false => no rules match => Allow
        assert_eq!(res.decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_in_operator() {
        let yaml = r#"
version: 1
rules:
  - name: network-check
    when: network.identity in ["testnet", "futurenet"]
    decision: warn
    reason: "Non-production network"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context(); // network.identity = "testnet"
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert!(res.warnings.iter().any(|w| w.contains("Non-production")));
    }

    #[test]
    fn test_contains_operator() {
        let yaml = r#"
version: 1
rules:
  - name: hash-check
    when: artifact.hash contains "a1b2"
    decision: warn
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context(); // hash = "a1b2c3d4"
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert_eq!(res.warnings.len(), 1);
    }

    #[test]
    fn test_negation_operator() {
        // `!` must be quoted in YAML to avoid being parsed as a YAML tag.
        let yaml = "version: 1\nrules:\n  - name: not-wasm\n    when: \"!artifact.is_wasm\"\n    decision: deny\n";
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context(); // is_wasm = true
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        // !true = false => rule does not match => Allow
        assert_eq!(res.decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_bool_field_truthy() {
        let yaml = r#"
version: 1
rules:
  - name: is-wasm
    when: artifact.is_wasm
    decision: allow
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context(); // is_wasm = true
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert_eq!(res.decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_and_or_operators() {
        let yaml = r#"
version: 1
rules:
  - name: combined
    when: artifact.signature_verified != true && dependency.has_revoked == true
    decision: deny
    reason: "Unsigned with revoked dep"
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert!(!res.allowed);
    }

    #[test]
    fn test_or_operator() {
        let yaml = r#"
version: 1
rules:
  - name: either
    when: network.identity == "mainnet" || risk.score >= 9.0
    decision: warn
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context(); // network=testnet, score=9.5
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert_eq!(res.warnings.len(), 1);
    }

    // ── JSON policy parsing ────────────────────────────────────────────────

    #[test]
    fn test_json_policy_parsing() {
        let json = r#"{
  "version": 1,
  "rules": [
    {
      "name": "check-size",
      "when": "artifact.size > 100",
      "decision": "deny",
      "reason": "Artifact too large"
    }
  ]
}"#;
        let policy = PolicyDefinition::from_json(json).unwrap();
        let ctx = sample_context(); // size=1024
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();
        assert!(!res.allowed);
    }

    // ── Default policy resolution ──────────────────────────────────────────

    #[test]
    fn test_resolve_effective_policy_override_takes_precedence() {
        let override_yaml = r#"
version: 1
rules:
  - name: override-rule
    when: artifact.is_wasm == true
    decision: deny
"#;
        let default_yaml = r#"
version: 1
rules:
  - name: default-rule
    when: artifact.is_wasm == true
    decision: allow
"#;
        let ov = PolicyDefinition::from_yaml(override_yaml).unwrap();
        let def = PolicyDefinition::from_yaml(default_yaml).unwrap();
        let effective = resolve_effective_policy(Some(&ov), Some(&def)).unwrap().unwrap();
        assert_eq!(effective.rules[0].name, "override-rule");
    }

    #[test]
    fn test_resolve_effective_policy_fallback_to_default() {
        let default_yaml = r#"
version: 1
rules:
  - name: default-rule
    when: artifact.is_wasm == true
    decision: warn
"#;
        let def = PolicyDefinition::from_yaml(default_yaml).unwrap();
        let effective = resolve_effective_policy(None, Some(&def)).unwrap().unwrap();
        assert_eq!(effective.rules[0].name, "default-rule");
    }

    #[test]
    fn test_resolve_effective_policy_none_when_both_absent() {
        let effective = resolve_effective_policy(None, None).unwrap();
        assert!(effective.is_none());
    }

    #[test]
    fn test_parse_optional_policy_none() {
        let result = parse_optional_policy(None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_optional_policy_empty_string() {
        let result = parse_optional_policy(Some("   ")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_optional_policy_valid_yaml() {
        let yaml = r#"
version: 1
rules:
  - name: test
    when: artifact.is_wasm == true
    decision: allow
"#;
        let result = parse_optional_policy(Some(yaml)).unwrap();
        assert!(result.is_some());
    }

    // ── Evidence / input values ────────────────────────────────────────────

    #[test]
    fn test_evaluated_rules_capture_input_values() {
        let yaml = r#"
version: 1
rules:
  - name: check-risk
    when: risk.max_severity == "critical"
    decision: deny
"#;
        let policy = PolicyDefinition::from_yaml(yaml).unwrap();
        let ctx = sample_context();
        let res = PolicyEvaluator::evaluate(&policy, &ctx).unwrap();

        let rule = &res.evaluated_rules[0];
        assert!(rule.matched);
        // Should have captured the input value for risk.max_severity
        assert!(rule.input_values.contains_key("risk.max_severity"));
        let val = &rule.input_values["risk.max_severity"];
        assert_eq!(val.as_str(), Some("critical"));
    }

    // ── Backward compatibility ─────────────────────────────────────────────

    #[test]
    fn test_no_policy_allows_publish() {
        // When no policy is configured, publishing should work without error.
        // This tests the backward compatibility requirement.
        let ctx = clean_context();
        // No policy evaluation = backward compatible
        let effective = resolve_effective_policy(None, None).unwrap();
        assert!(effective.is_none());
    }

    // ── Malformed JSON policy ──────────────────────────────────────────────

    #[test]
    fn test_malformed_json_fails_closed() {
        let res = PolicyDefinition::from_json("not valid json");
        assert!(matches!(res, Err(PolicyError::MalformedPolicy(_))));
    }

    #[test]
    fn test_malformed_yaml_fails_closed() {
        let res = PolicyDefinition::from_yaml(
            "version: 1\nrules:\n  - invalid yaml structure!!!",
        );
        assert!(matches!(res, Err(PolicyError::MalformedPolicy(_))));
    }
}
