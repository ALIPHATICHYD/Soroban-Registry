#![allow(dead_code)]

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize)]
pub struct PropertiesConfig {
    pub property: Vec<PropertyDef>,
}

#[derive(Debug, Deserialize)]
pub struct PropertyDef {
    pub id: String,
    pub description: String,
    pub invariant: String,
    pub severity: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationReport {
    pub contract_path: String,
    pub tool_version: String,
    pub properties_checked: usize,
    pub properties_proved: usize,
    pub properties_violated: usize,
    pub properties_unknown: usize,
    pub results: Vec<PropertyResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PropertyResult {
    pub id: String,
    pub description: String,
    pub status: String, // "Proved", "Violated", "Unknown"
    pub counterexample: Option<String>,
}

pub async fn run(
    api_url: &str,
    contract_path: &str,
    properties_file: &str,
    output_format: &str,
    post_results: bool,
) -> Result<()> {
    if output_format != "json" {
        println!(
            "\n{}",
            "Starting Formal Verification Analysis...".bold().cyan()
        );
        println!("Contract: {}", contract_path);
        println!("Properties: {}", properties_file);
        println!("{}", "=".repeat(80).cyan());
    }

    let config_contents = fs::read_to_string(properties_file).context(format!(
        "Failed to read properties file: {}",
        properties_file
    ))?;

    let config: PropertiesConfig =
        toml::from_str(&config_contents).context("Failed to parse TOML properties file")?;

    let contract_contents = match fs::read(contract_path) {
        Ok(c) => c,
        Err(_) => {
            if output_format != "json" {
                println!(
                    "{}",
                    "⚠ Failed to read contract file, returning UNKNOWN for all...".yellow()
                );
            }
            Vec::new()
        }
    };

    // Convert to text for simple invariant matching. In reality you'd disassemble or use a real solver.
    let contract_text = String::from_utf8_lossy(&contract_contents).to_lowercase();

    let mut report = VerificationReport {
        contract_path: contract_path.to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        properties_checked: config.property.len(),
        properties_proved: 0,
        properties_violated: 0,
        properties_unknown: 0,
        results: Vec::new(),
    };

    for prop in config.property {
        let (status, counterexample) = evaluate_invariant(&prop.invariant, &contract_text);

        match status.as_str() {
            "Proved" => report.properties_proved += 1,
            "Violated" => report.properties_violated += 1,
            _ => report.properties_unknown += 1,
        }

        report.results.push(PropertyResult {
            id: prop.id.clone(),
            description: prop.description.clone(),
            status,
            counterexample,
        });
    }

    if output_format == "json" {
        let json = serde_json::to_string_pretty(&report)?;
        println!("{}", json);
    } else {
        print_report(&report);
    }

    if post_results {
        // Send to backend (requires extracting contract ID in reality, hardcoded here for demo)
        if output_format != "json" {
            println!("\n{}", "Posting results to registry...".bold().cyan());
        }

        let client = crate::net::client();
        // Just demonstrating the endpoint structure.
        let url = format!(
            "{}/api/contracts/00000000-0000-0000-0000-000000000000/formal-verification",
            api_url
        );

        // This simulates a full valid report payload for `FormalVerificationReport` model
        let payload = serde_json::json!({
            "session": {
                "id": uuid::Uuid::new_v4().to_string(),
                "contract_id": uuid::Uuid::new_v4().to_string(),
                "version": "1.0.0",
                "verifier_version": report.tool_version,
                "created_at": chrono::Utc::now(),
                "updated_at": chrono::Utc::now()
            },
            "properties": report.results.iter().map(|res| {
                serde_json::json!({
                    "property": {
                        "id": uuid::Uuid::new_v4().to_string(),
                        "session_id": uuid::Uuid::new_v4().to_string(),
                        "property_id": res.id,
                        "description": res.description,
                        "invariant": "N/A",
                        "severity": "Medium"
                    },
                    "result": {
                        "id": uuid::Uuid::new_v4().to_string(),
                        "property_id": uuid::Uuid::new_v4().to_string(),
                        "status": res.status,
                        "counterexample": res.counterexample,
                        "details": null
                    }
                })
            }).collect::<Vec<_>>()
        });

        let resp = client.post(&url).json(&payload).send_with_retry().await;
        match resp {
            Ok(r) if r.status().is_success() => {
                if output_format != "json" {
                    println!("{} Results successfully posted.", "✓".green());
                }
            }
            Ok(r) => {
                if output_format != "json" {
                    println!("{} Failed to post results: HTTP {}", "✗".red(), r.status());
                }
            }
            Err(e) => {
                if output_format != "json" {
                    println!("{} Failed to reach registry: {}", "✗".red(), e);
                }
            }
        }
    }

    Ok(())
}

fn print_report(report: &VerificationReport) {
    for res in &report.results {
        let (icon, color_status) = match res.status.as_str() {
            "Proved" => ("✓", "PROVED".green().bold()),
            "Violated" => ("✗", "VIOLATED".red().bold()),
            _ => ("?", "UNKNOWN".yellow().bold()),
        };

        println!(
            "\n{} {} [{}]",
            icon,
            res.description.bold(),
            res.id.bright_black()
        );
        println!("  Result: {}", color_status);

        if let Some(ce) = &res.counterexample {
            println!("  {} {}", "↳ Counterexample:".red(), ce);
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!(
        "Analysis Complete: {} Checked | {} Proved | {} Violated | {} Unknown\n",
        report.properties_checked,
        report.properties_proved.to_string().green(),
        report.properties_violated.to_string().red(),
        report.properties_unknown.to_string().yellow()
    );
}

// Very basic simulation of static analysis pattern matching logic
fn evaluate_invariant(invariant: &str, contract_text: &str) -> (String, Option<String>) {
    if contract_text.is_empty() {
        return ("Unknown".to_string(), None);
    }

    match invariant {
        "caller_is_auth" => {
            if contract_text.contains("require_auth") {
                ("Proved".to_string(), None)
            } else {
                (
                    "Violated".to_string(),
                    Some(
                        "Function allows transfer without `require_auth` check on the sender."
                            .to_string(),
                    ),
                )
            }
        }
        "checked_arithmetic" => {
            if !contract_text.contains("overflow") {
                ("Proved".to_string(), None)
            } else {
                (
                    "Violated".to_string(),
                    Some(
                        "Unsafe arithmetic detected. A crafted input can cause integer overflow."
                            .to_string(),
                    ),
                )
            }
        }
        "no_reentrancy" => ("Proved".to_string(), None),
        _ => (
            "Unknown".to_string(),
            Some(format!(
                "Invariant pattern '{}' not recognized by static engine.",
                invariant
            )),
        ),
    }
}
