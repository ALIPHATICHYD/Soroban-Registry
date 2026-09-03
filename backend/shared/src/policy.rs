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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PolicyRule {
    pub name: String,
    pub when: String,
    pub decision: PolicyDecision,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct ArtifactContext {
    pub signature_verified: bool,
    pub size: u64,
    pub hash: String,
    pub is_wasm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct VerificationContext {
    pub state: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct ProvenanceContext {
    pub present: bool,
    pub builder_image: Option<String>,
    pub reproducible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct VulnerabilityContext {
    pub count: usize,
    pub critical_count: usize,
    pub high_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct RiskContext {
    pub max_severity: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct DependencyContext {
    pub revoked_count: usize,
    pub has_revoked: bool,
    pub risk_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct InterfaceContext {
    pub compatible: bool,
    pub breaking_changes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct NetworkContext {
    pub identity: String,
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct MetadataContext {
    pub complete: bool,
    pub completeness: f64,
    pub has_readme: bool,
    pub has_license: bool,
    pub has_repository: bool,
    pub has_version: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EvaluatedRule {
    pub name: String,
    pub condition: String,
    pub matched: bool,
    pub decision: PolicyDecision,
    pub reason: Option<String>,
    pub input_values: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
}

pub struct PolicyEvaluator;

impl PolicyEvaluator {
    pub fn evaluate(
        policy: &PolicyDefinition,
        context: &AdmissionContext,
    ) -> Result<PolicyEvaluationResult, PolicyError> {
        policy.validate()?;

        let ctx_val = serde_json::to_value(context)
            .map_err(|e| PolicyError::EvaluationFailed(format!("Failed to serialize context: {e}")))?;

        let mut evaluated_rules = Vec::new();
        let mut overall_decision = PolicyDecision::Allow;
        let mut reasons = Vec::new();
        let mut warnings = Vec::new();

        for rule in &policy.rules {
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
}
