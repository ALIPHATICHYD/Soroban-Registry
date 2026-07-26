use crate::cached_http;
use crate::table_format::render_table;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;

/// Exit codes for automation (#970).
pub const EXIT_IDENTICAL: i32 = 0;
pub const EXIT_DIFFERENCES: i32 = 1;
pub const EXIT_ERROR: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffFormat {
    None,
    Unified,
    SideBySide,
}

impl DiffFormat {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw.to_lowercase().as_str() {
            "none" => Ok(Self::None),
            "unified" | "diff" => Ok(Self::Unified),
            "side-by-side" | "side_by_side" | "side" => Ok(Self::SideBySide),
            other => anyhow::bail!(
                "Unknown diff format '{other}'. Expected none, unified, or side-by-side."
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FieldGroup {
    Metadata,
    Verification,
    Deployment,
    Abi,
}

impl FieldGroup {
    pub fn parse_list(raw: Option<&str>) -> Result<Vec<Self>> {
        let Some(raw) = raw else {
            return Ok(vec![
                Self::Metadata,
                Self::Verification,
                Self::Deployment,
                Self::Abi,
            ]);
        };
        let mut groups = Vec::new();
        for part in raw.split(',') {
            match part.trim().to_lowercase().as_str() {
                "metadata" | "meta" => groups.push(Self::Metadata),
                "verification" | "verify" => groups.push(Self::Verification),
                "deployment" | "deploy" => groups.push(Self::Deployment),
                "abi" => groups.push(Self::Abi),
                "all" => {
                    groups = vec![
                        Self::Metadata,
                        Self::Verification,
                        Self::Deployment,
                        Self::Abi,
                    ];
                    break;
                }
                other => anyhow::bail!("Unknown field group '{other}'"),
            }
        }
        groups.sort();
        groups.dedup();
        Ok(groups)
    }
}

#[derive(Debug, Clone)]
struct CompareField {
    group: FieldGroup,
    name: &'static str,
    extract: fn(&Value) -> String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct FieldDiff {
    field: String,
    group: String,
    values: BTreeMap<String, String>,
    identical: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CompareSummary {
    pub compared_contracts: Vec<String>,
    pub identical: bool,
    pub diff_count: usize,
    pub diffs: Vec<FieldDiff>,
}

pub struct CompareOptions {
    pub exit_code: bool,
    pub diff_format: DiffFormat,
    pub fields: Option<String>,
}

pub async fn run(
    api_url: &str,
    ids: Vec<String>,
    json: bool,
    export_path: Option<&str>,
    format_opt: Option<&str>,
    options: CompareOptions,
) -> Result<i32> {
    if ids.len() < 2 || ids.len() > 4 {
        anyhow::bail!("You must specify between 2 and 4 contract IDs to compare.");
    }

    let groups = FieldGroup::parse_list(options.fields.as_deref())?;
    let fields = active_fields(&groups);

    let contracts = match fetch_contracts(api_url, &ids).await {
        Ok(data) => data,
        Err(err) => {
            if options.exit_code {
                return Ok(EXIT_ERROR);
            }
            return Err(err);
        }
    };

    let summary = build_summary(&ids, &contracts, &fields);

    if json || format_opt == Some("json") || export_path.map_or(false, |p| p.ends_with(".json")) {
        let out = serde_json::json!({
            "compared_contracts": summary.compared_contracts,
            "identical": summary.identical,
            "diff_count": summary.diff_count,
            "diffs": summary.diffs,
            "data": contracts,
        });

        if let Some(path) = export_path {
            fs::write(path, serde_json::to_string_pretty(&out)?)
                .with_context(|| format!("Failed to write export to {path}"))?;
            println!("{} Comparison exported to {}", "✓".green(), path);
        } else {
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        return Ok(exit_from_summary(&summary, options.exit_code));
    }

    if format_opt == Some("csv") || export_path.map_or(false, |p| p.ends_with(".csv")) {
        export_csv(&ids, &contracts, &fields, export_path)?;
        return Ok(exit_from_summary(&summary, options.exit_code));
    }

    print_table(&ids, &contracts, &fields, &summary);

    if options.diff_format != DiffFormat::None {
        print_diff(&ids, &summary, options.diff_format);
    }

    Ok(exit_from_summary(&summary, options.exit_code))
}

fn exit_from_summary(summary: &CompareSummary, exit_code: bool) -> i32 {
    if !exit_code {
        return EXIT_IDENTICAL;
    }
    if summary.identical {
        EXIT_IDENTICAL
    } else {
        EXIT_DIFFERENCES
    }
}

async fn fetch_contracts(api_url: &str, ids: &[String]) -> Result<Vec<Value>> {
    let mut contracts = Vec::new();
    for id in ids {
        let url = format!("{}/api/contracts/{}", api_url.trim_end_matches('/'), id);
        let (status, body) = cached_http::cached_get_simple(&url).await?;
        if status == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("Contract not found: {id}");
        }
        if !status.is_success() {
            anyhow::bail!("API returned error {status} for contract {id}");
        }
        let data: Value = serde_json::from_str(&body).context("Invalid contract JSON")?;
        contracts.push(data);
    }
    Ok(contracts)
}

fn active_fields(groups: &[FieldGroup]) -> Vec<CompareField> {
    let all = vec![
        CompareField {
            group: FieldGroup::Metadata,
            name: "Name",
            extract: |c| c["name"].as_str().unwrap_or("N/A").to_string(),
        },
        CompareField {
            group: FieldGroup::Metadata,
            name: "Category",
            extract: |c| c["category"].as_str().unwrap_or("None").to_string(),
        },
        CompareField {
            group: FieldGroup::Metadata,
            name: "Description",
            extract: |c| c["description"].as_str().unwrap_or("").to_string(),
        },
        CompareField {
            group: FieldGroup::Verification,
            name: "Network",
            extract: |c| c["network"].as_str().unwrap_or("N/A").to_string(),
        },
        CompareField {
            group: FieldGroup::Verification,
            name: "Verified",
            extract: |c| {
                if c["is_verified"].as_bool().unwrap_or(false) {
                    "Yes".into()
                } else {
                    "No".into()
                }
            },
        },
        CompareField {
            group: FieldGroup::Verification,
            name: "WASM Hash",
            extract: |c| shorten_hash(c["wasm_hash"].as_str().unwrap_or("N/A")),
        },
        CompareField {
            group: FieldGroup::Abi,
            name: "ABI Size",
            extract: |c| {
                if let Some(a) = c["abi"].as_array() {
                    format!("{} methods", a.len())
                } else if let Some(o) = c["abi"].as_object() {
                    format!("{} methods", o.len())
                } else {
                    "N/A".into()
                }
            },
        },
        CompareField {
            group: FieldGroup::Deployment,
            name: "Deployments",
            extract: |c| {
                c["deployments"]
                    .as_array()
                    .map_or("0".into(), |a| format!("{}", a.len()))
            },
        },
        CompareField {
            group: FieldGroup::Deployment,
            name: "Health Score",
            extract: |c| {
                c["health_score"]
                    .as_f64()
                    .map_or("N/A".into(), |f| format!("{f:.1}"))
            },
        },
    ];

    all.into_iter()
        .filter(|field| groups.contains(&field.group))
        .collect()
}

fn build_summary(ids: &[String], contracts: &[Value], fields: &[CompareField]) -> CompareSummary {
    let mut diffs = Vec::new();

    for field in fields {
        let mut values = BTreeMap::new();
        let mut extracted = Vec::new();
        for (id, contract) in ids.iter().zip(contracts.iter()) {
            let value = (field.extract)(contract);
            extracted.push(value.clone());
            values.insert(id.clone(), value);
        }
        let identical = extracted.iter().all(|v| v == &extracted[0]);
        diffs.push(FieldDiff {
            field: field.name.to_string(),
            group: format!("{:?}", field.group).to_lowercase(),
            values,
            identical,
        });
    }

    let diff_count = diffs.iter().filter(|d| !d.identical).count();
    CompareSummary {
        compared_contracts: ids.to_vec(),
        identical: diff_count == 0,
        diff_count,
        diffs,
    }
}

fn print_table(
    ids: &[String],
    contracts: &[Value],
    fields: &[CompareField],
    summary: &CompareSummary,
) {
    println!("\n{}", "Contract Comparison".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if summary.identical {
        println!("  {} All compared fields are identical.", "✔".green());
    } else {
        println!(
            "  {} {} field(s) differ.",
            "≠".yellow().bold(),
            summary.diff_count
        );
    }

    let mut headers = vec!["Field".to_string()];
    headers.extend(ids.iter().cloned());

    let mut rows = Vec::new();
    for field in fields {
        let mut row = vec![field.name.to_string()];
        let mut values = Vec::new();
        for contract in contracts {
            values.push((field.extract)(contract));
        }
        let all_same = values.iter().all(|v| v == &values[0]);
        for val in values {
            if all_same {
                row.push(val.bright_black().to_string());
            } else {
                row.push(val.yellow().to_string());
            }
        }
        rows.push(row);
    }

    let mut col_widths = vec![15];
    for i in 0..ids.len() {
        let max_w = rows
            .iter()
            .map(|r| r[i + 1].len())
            .max()
            .unwrap_or(10)
            .max(ids[i].len());
        col_widths.push(max_w.min(35));
    }

    let header_strs: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
    print!("{}", render_table(&header_strs, &col_widths, &rows));
    println!();
}

fn print_diff(ids: &[String], summary: &CompareSummary, format: DiffFormat) {
    let differing: Vec<_> = summary.diffs.iter().filter(|d| !d.identical).collect();
    if differing.is_empty() {
        return;
    }

    println!("{}", "Diff Summary".bold().cyan());
    match format {
        DiffFormat::Unified => {
            for diff in differing {
                println!("--- {}", diff.field);
                for id in ids {
                    let value = diff.values.get(id).cloned().unwrap_or_default();
                    println!("  {id}: {value}");
                }
                println!();
            }
        }
        DiffFormat::SideBySide => {
            for diff in differing {
                println!("{}", diff.field.bold());
                let mut columns: Vec<_> = ids
                    .iter()
                    .map(|id| (id.clone(), diff.values.get(id).cloned().unwrap_or_default()))
                    .collect();
                columns.sort_by(|a, b| a.1.cmp(&b.1));
                for (id, value) in columns {
                    println!("  {:<24} {}", id.cyan(), value.yellow());
                }
                println!();
            }
        }
        DiffFormat::None => {}
    }
}

fn export_csv(
    ids: &[String],
    contracts: &[Value],
    fields: &[CompareField],
    export_path: Option<&str>,
) -> Result<()> {
    let mut csv_data = String::new();
    let headers: Vec<String> = std::iter::once("Field".into())
        .chain(ids.iter().cloned())
        .collect();
    csv_data.push_str(&headers.join(","));
    csv_data.push('\n');

    for field in fields {
        let mut row = vec![field.name.to_string()];
        for contract in contracts {
            row.push((field.extract)(contract));
        }
        csv_data.push_str(&row.join(","));
        csv_data.push('\n');
    }

    if let Some(path) = export_path {
        fs::write(path, csv_data)
            .with_context(|| format!("Failed to write CSV export to {path}"))?;
        println!("{} Comparison exported to {}", "✓".green(), path);
    } else {
        println!("{csv_data}");
    }
    Ok(())
}

fn shorten_hash(hash: &str) -> String {
    if hash.len() > 10 {
        format!("{}...{}", &hash[..6], &hash[hash.len() - 4..])
    } else {
        hash.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn field_groups_parse_all() {
        let groups = FieldGroup::parse_list(Some("all")).unwrap();
        assert_eq!(groups.len(), 4);
    }

    #[test]
    fn summary_marks_identical_contracts() {
        let ids = vec!["a".into(), "b".into()];
        let contracts = vec![
            json!({"name": "X", "category": "defi", "description": "", "network": "testnet", "is_verified": true, "wasm_hash": "abc", "deployments": [], "health_score": 1.0, "abi": []}),
            json!({"name": "X", "category": "defi", "description": "", "network": "testnet", "is_verified": true, "wasm_hash": "abc", "deployments": [], "health_score": 1.0, "abi": []}),
        ];
        let fields = active_fields(&FieldGroup::parse_list(None).unwrap());
        let summary = build_summary(&ids, &contracts, &fields);
        assert!(summary.identical);
        assert_eq!(summary.diff_count, 0);
    }

    #[test]
    fn summary_detects_differences() {
        let ids = vec!["a".into(), "b".into()];
        let contracts = vec![
            json!({"name": "X", "category": "defi", "description": "", "network": "testnet", "is_verified": true, "wasm_hash": "abc", "deployments": [], "health_score": 1.0, "abi": []}),
            json!({"name": "Y", "category": "defi", "description": "", "network": "testnet", "is_verified": true, "wasm_hash": "abc", "deployments": [], "health_score": 1.0, "abi": []}),
        ];
        let fields = active_fields(&FieldGroup::parse_list(Some("metadata")).unwrap());
        let summary = build_summary(&ids, &contracts, &fields);
        assert!(!summary.identical);
        assert_eq!(summary.diff_count, 1);
    }

    #[test]
    fn exit_code_mapping() {
        let identical = CompareSummary {
            compared_contracts: vec![],
            identical: true,
            diff_count: 0,
            diffs: vec![],
        };
        assert_eq!(exit_from_summary(&identical, true), EXIT_IDENTICAL);
        let diff = CompareSummary {
            compared_contracts: vec![],
            identical: false,
            diff_count: 1,
            diffs: vec![],
        };
        assert_eq!(exit_from_summary(&diff, true), EXIT_DIFFERENCES);
    }
}
