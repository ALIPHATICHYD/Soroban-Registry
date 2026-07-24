//! contract_dependency.rs — `soroban-registry contract dependency <ADDRESS>` (#836, #1008)
//!
//! Analyze a contract's dependencies: contracts it depends on, contracts that
//! depend on it, and a dependency tree with configurable `--depth`.
//!
//! Supports human-readable table output, JSON output, and a compact `--summary`
//! mode that shows only aggregate counts for large dependency graphs.

use crate::net::RequestBuilderExt;
use crate::output_format::{self, OutputFormat};
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;

const MAX_DEPENDENCY_DEPTH: u32 = 20;

/// `soroban-registry contract dependency <ADDRESS> [--depth N] [--format json|table] [--summary]`
///
/// Analyzes a contract's on-chain dependencies and dependents. When `--summary`
/// is set, output is condensed to aggregate counts (useful for contracts with
/// hundreds of dependents). The `--format` flag controls machine-readable output.
pub async fn run(
    api_url: &str,
    address: &str,
    depth: u32,
    format: OutputFormat,
    summary: bool,
) -> Result<()> {
    validate_depth(depth)?;

    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/{}/dependencies?depth={}",
        api_url.trim_end_matches('/'),
        address,
        depth
    );
    log::debug!("GET {}", url);

    let resp = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to reach the registry API. Is the registry running?")?;
    let status = resp.status();
    let value: Value = resp.json().await.unwrap_or(Value::Null);
    if status.as_u16() == 404 {
        anyhow::bail!("no dependency data found for {}; the registry has no dependency records for this contract", address);
    }
    if !status.is_success() {
        anyhow::bail!("contract dependency failed ({}): {}", status, value);
    }
    if !has_dependency_data(&value) {
        anyhow::bail!(
            "missing dependency data for {}; response did not include dependsOn, dependents, or tree/dependencies fields",
            address
        );
    }

    // Machine-readable formats: dump the full API response
    if matches!(
        format,
        OutputFormat::Json | OutputFormat::Yaml | OutputFormat::Csv
    ) {
        let rendered = match format {
            OutputFormat::Json => output_format::render_json(&value)?,
            OutputFormat::Yaml => output_format::render_yaml(&value)?,
            OutputFormat::Csv => {
                // For CSV, extract the flat lists
                let flat = flatten_for_csv(&value);
                output_format::render_csv(&flat)?
            }
            _ => unreachable!(),
        };
        println!("{}", rendered);
        return Ok(());
    }

    // ── Summary mode: aggregate counts only ──────────────────────────────
    if summary {
        let depends_on = value
            .get("dependsOn")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let dependents = value
            .get("dependents")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let tree_nodes = count_tree_nodes(value.get("tree"));
        println!("{} {}", "Dependencies for".bold(), address.cyan());
        println!(
            "  Depends on:   {} contracts",
            depends_on.to_string().bold()
        );
        println!(
            "  Depended by:  {} contracts",
            dependents.to_string().bold()
        );
        if tree_nodes > 0 {
            println!(
                "  Tree nodes:   {} (depth {})",
                tree_nodes.to_string().bold(),
                depth
            );
        }
        return Ok(());
    }

    // ── Full table mode ──────────────────────────────────────────────────
    println!("{} {}", "Dependencies for".bold(), address.cyan());

    let depends_on = value
        .get("dependsOn")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    println!("\n  {} ({})", "Depends on:".bold(), depends_on.len());
    for d in &depends_on {
        let addr = d.get("address").and_then(Value::as_str).unwrap_or("?");
        let name = d.get("name").and_then(Value::as_str).unwrap_or("");
        println!("    → {} {}", addr.cyan(), name.dimmed());
    }

    let dependents = value
        .get("dependents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    println!("\n  {} ({})", "Depended on by:".bold(), dependents.len());
    for d in &dependents {
        let addr = d.get("address").and_then(Value::as_str).unwrap_or("?");
        let name = d.get("name").and_then(Value::as_str).unwrap_or("");
        println!("    ← {} {}", addr.cyan(), name.dimmed());
    }

    if let Some(tree) = value.get("tree") {
        println!("\n  {} (depth {})", "Dependency tree:".bold(), depth);
        print_tree(tree, 0);
    }
    Ok(())
}

fn print_tree(node: &Value, indent: usize) {
    let pad = "  ".repeat(indent + 2);
    let addr = node_label(node);
    println!("{}{}", pad, addr);
    if let Some(children) = node_children(node) {
        for child in children {
            print_tree(child, indent + 1);
        }
    }
}

/// Count all nodes in a recursive dependency tree.
fn count_tree_nodes(node: Option<&Value>) -> usize {
    let node = match node {
        Some(n) => n,
        None => return 0,
    };
    let mut count = 1;
    if let Some(children) = node_children(node) {
        for child in children {
            count += count_tree_nodes(Some(child));
        }
    }
    count
}

fn validate_depth(depth: u32) -> Result<()> {
    if depth > MAX_DEPENDENCY_DEPTH {
        anyhow::bail!(
            "dependency depth {} exceeds the supported maximum of {}",
            depth,
            MAX_DEPENDENCY_DEPTH
        );
    }
    Ok(())
}

fn has_dependency_data(value: &Value) -> bool {
    value.get("dependsOn").is_some()
        || value.get("dependents").is_some()
        || value.get("tree").is_some()
        || value.get("dependencies").is_some()
}

fn node_children(node: &Value) -> Option<&Vec<Value>> {
    node.get("children")
        .and_then(Value::as_array)
        .or_else(|| node.get("dependencies").and_then(Value::as_array))
}

fn node_label(node: &Value) -> &str {
    node.get("address")
        .and_then(Value::as_str)
        .or_else(|| node.get("contract_id").and_then(Value::as_str))
        .or_else(|| node.get("id").and_then(Value::as_str))
        .unwrap_or("?")
}

/// Flatten the dependency API response into an array of uniform objects for CSV rendering.
fn flatten_for_csv(value: &Value) -> Value {
    let mut rows = Vec::new();
    if let Some(depends_on) = value.get("dependsOn").and_then(Value::as_array) {
        for d in depends_on {
            let mut row = serde_json::Map::new();
            row.insert("relation".into(), Value::String("depends_on".into()));
            row.insert(
                "address".into(),
                d.get("address").cloned().unwrap_or(Value::Null),
            );
            row.insert("name".into(), d.get("name").cloned().unwrap_or(Value::Null));
            rows.push(Value::Object(row));
        }
    }
    if let Some(dependents) = value.get("dependents").and_then(Value::as_array) {
        for d in dependents {
            let mut row = serde_json::Map::new();
            row.insert("relation".into(), Value::String("depended_by".into()));
            row.insert(
                "address".into(),
                d.get("address").cloned().unwrap_or(Value::Null),
            );
            row.insert("name".into(), d.get("name").cloned().unwrap_or(Value::Null));
            rows.push(Value::Object(row));
        }
    }
    Value::Array(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_zero_is_valid() {
        validate_depth(0).unwrap();
    }

    #[test]
    fn depth_above_backend_limit_is_rejected() {
        let err = validate_depth(MAX_DEPENDENCY_DEPTH + 1)
            .unwrap_err()
            .to_string();

        assert!(err.contains("exceeds the supported maximum"));
    }

    #[test]
    fn counts_nested_dependencies_field() {
        let tree = serde_json::json!({
            "contract_id": "root",
            "dependencies": [
                {"contract_id": "child-a", "dependencies": []},
                {"contract_id": "child-b", "dependencies": [
                    {"contract_id": "grandchild"}
                ]}
            ]
        });

        assert_eq!(count_tree_nodes(Some(&tree)), 4);
    }

    #[test]
    fn missing_dependency_payload_is_detected() {
        let value = serde_json::json!({ "contract_id": "root" });

        assert!(!has_dependency_data(&value));
    }
}
