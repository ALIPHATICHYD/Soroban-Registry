//! Policy CLI command handler (Issue #1148).
//!
//! Provides CLI policy check evaluation, explain reports, and policy dry-run mode.

use anyhow::{bail, Context, Result};
use colored::Colorize;
use shared::policy::{AdmissionContext, ArtifactContext, PolicyDefinition, PolicyEvaluator};
use std::fs;
use std::path::Path;

pub async fn run_policy_check(
    wasm_path: Option<String>,
    policy_path: String,
    explain: bool,
    json: bool,
    dry_run: bool,
) -> Result<()> {
    if !Path::new(&policy_path).exists() {
        bail!("Policy file not found: {}", policy_path);
    }

    let policy_content = fs::read_to_string(&policy_path)
        .with_context(|| format!("Failed to read policy file: {}", policy_path))?;

    let policy_def = PolicyDefinition::from_yaml(&policy_content)
        .or_else(|_| PolicyDefinition::from_json(&policy_content))
        .with_context(|| format!("Failed to parse policy file: {}", policy_path))?;

    let mut context = AdmissionContext::default();

    if let Some(ref path_str) = wasm_path {
        let path = Path::new(path_str);
        if path.exists() {
            let metadata = fs::metadata(path)?;
            context.artifact = ArtifactContext {
                signature_verified: false,
                size: metadata.len(),
                hash: "local-artifact-hash".to_string(),
                is_wasm: path_str.ends_with(".wasm"),
            };
        }
    }

    let result = PolicyEvaluator::evaluate(&policy_def, &context)
        .map_err(|e| anyhow::anyhow!("Policy evaluation error: {}", e))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", "Policy Evaluation Result".bold().cyan());
        println!("========================");
        println!("Policy Version : {}", result.policy_version);
        println!(
            "Decision       : {}",
            match result.decision {
                shared::policy::PolicyDecision::Allow => "ALLOW".green().bold(),
                shared::policy::PolicyDecision::Warn => "WARN".yellow().bold(),
                shared::policy::PolicyDecision::Deny => "DENY".red().bold(),
            }
        );
        println!("Allowed        : {}", result.allowed);

        if explain || !result.evaluated_rules.is_empty() {
            println!("\n{}", "Evaluated Rules:".bold());
            for rule in &result.evaluated_rules {
                let status_str = if rule.matched {
                    match rule.decision {
                        shared::policy::PolicyDecision::Allow => "[ALLOW]".green().bold(),
                        shared::policy::PolicyDecision::Warn => "[WARN]".yellow().bold(),
                        shared::policy::PolicyDecision::Deny => "[DENY]".red().bold(),
                    }
                } else {
                    "[SKIPPED]".dimmed()
                };

                println!(
                    "  {} {} (Matched: {})",
                    status_str,
                    rule.name.bold(),
                    rule.matched
                );
                println!("         Condition: {}", rule.condition.italic());
                if let Some(ref reason) = rule.reason {
                    println!("         Reason   : {}", reason);
                }
                if !rule.input_values.is_empty() {
                    println!(
                        "         Inputs   : {}",
                        serde_json::to_string(&rule.input_values).unwrap_or_default()
                    );
                }
            }
        }

        if !result.reasons.is_empty() {
            println!("\n{}", "Denial Reasons:".red().bold());
            for r in &result.reasons {
                println!("  • {}", r);
            }
        }

        if !result.warnings.is_empty() {
            println!("\n{}", "Warnings:".yellow().bold());
            for w in &result.warnings {
                println!("  • {}", w);
            }
        }

        if dry_run {
            println!(
                "\n{}",
                "[DRY-RUN] No artifacts were submitted.".blue().bold()
            );
        }
    }

    if !result.allowed {
        bail!("Contract publishing rejected by policy check");
    }

    Ok(())
}
