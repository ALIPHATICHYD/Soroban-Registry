//! `soroban-registry contract update` — update contract metadata (#828).

use crate::contract_deploy::{upload_icon_to_backend, validate_and_process_icon};
use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

#[derive(Debug, Clone, Serialize)]
struct MetadataPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
}

impl MetadataPatch {
    /// The registry's own request type. `user_id` is server-derived from the
    /// caller's session, so the CLI never sets it.
    fn to_request(&self) -> registry_client::UpdateContractMetadataRequest {
        registry_client::UpdateContractMetadataRequest {
            name: self.name.clone(),
            description: self.description.clone(),
            category: self.category.clone(),
            tags: self.tags.clone(),
            user_id: None,
        }
    }
}

/// A key that is identical for the same patch applied to the same contract, so
/// a retried update collapses into one version-history entry instead of two.
fn metadata_idempotency_key(contract_id: &str, patch: &MetadataPatch) -> String {
    use sha2::{Digest, Sha256};
    let canonical = serde_json::to_string(patch).unwrap_or_default();
    let digest = Sha256::digest(format!("{contract_id}:{canonical}").as_bytes());
    format!("cli-metadata-{}", hex::encode(&digest[..16]))
}

#[derive(Debug, Deserialize)]
struct ContractRecord {
    id: String,
    contract_id: String,
    name: String,
    description: Option<String>,
    category: Option<String>,
    #[serde(default)]
    tags: Vec<TagRecord>,
}

#[derive(Debug, Deserialize)]
struct TagRecord {
    name: String,
}

pub struct UpdateArgs<'a> {
    pub api_url: &'a str,
    pub address: &'a str,
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub icon: Option<String>,
    pub homepage: Option<String>,
    pub dry_run: bool,
    pub yes: bool,
    pub json: bool,
}

pub async fn run(args: UpdateArgs<'_>) -> Result<()> {
    if args.name.is_none()
        && args.description.is_none()
        && args.category.is_none()
        && args.tags.is_none()
        && args.icon.is_none()
        && args.homepage.is_none()
    {
        bail!("Provide at least one field to update (--name, --description, --category, --tags, --icon, or --homepage)");
    }

    if args.homepage.is_some() {
        eprintln!(
            "{} Homepage updates are not yet supported by the registry API; the field will be ignored.",
            "⚠".yellow()
        );
    }

    let current = fetch_contract(args.api_url, args.address).await?;
    let patch = MetadataPatch {
        name: args.name.clone(),
        description: args.description.clone(),
        category: args.category.clone(),
        tags: args.tags.clone(),
    };

    let diffs = build_diffs(&current, &patch, args.icon.is_some());
    if diffs.is_empty() && args.icon.is_none() {
        println!("No changes detected.");
        return Ok(());
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "contract_id": current.contract_id,
                "registry_id": current.id,
                "dry_run": args.dry_run,
                "changes": diffs,
                "icon_update": args.icon.is_some(),
            }))?
        );
        if args.dry_run {
            return Ok(());
        }
    } else {
        print_diff_table(&current.contract_id, &diffs, args.icon.is_some());
        if args.dry_run {
            println!("\n{} Dry run — no changes were submitted.", "·".dimmed());
            return Ok(());
        }
        if !args.yes && !confirm("Apply these metadata changes?")? {
            println!("Update cancelled.");
            return Ok(());
        }
    }

    // A metadata update appends a version-history entry, so it is only retried
    // under an idempotency key derived from the contract and the exact patch.
    let idempotency_key = metadata_idempotency_key(&current.id, &patch);
    let updated = crate::registry::uncached_client(args.api_url)
        .await?
        .update_contract_metadata(&current.id, &patch.to_request(), Some(idempotency_key))
        .await
        .map_err(|err| anyhow::anyhow!("Metadata update failed: {err}"))?;
    let updated = serde_json::to_value(&updated).context("Invalid update response")?;

    if let Some(icon_path) = &args.icon {
        let icon_data = validate_and_process_icon(icon_path)?;
        let extension = std::path::Path::new(icon_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        upload_icon_to_backend(args.api_url, &current.id, &icon_data, extension).await?;
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&updated)?);
    } else {
        println!(
            "\n{} Contract metadata updated successfully.",
            "✔".green().bold()
        );
        println!("  Version history is preserved automatically by the registry.");
    }

    Ok(())
}

/// Read the contract through the shared client, so a missing contract, an
/// expired session, and a server fault stay distinguishable.
async fn fetch_contract(api_url: &str, address: &str) -> Result<ContractRecord> {
    let response = crate::registry::uncached_client(api_url)
        .await?
        .get_contract(address)
        .await
        .map_err(|err| match err {
            registry_client::Error::NotFound(_) => anyhow::anyhow!("Contract not found: {address}"),
            other => anyhow::anyhow!("Failed to fetch contract: {other}"),
        })?;

    let contract = response.contract;
    Ok(ContractRecord {
        id: contract.id.to_string(),
        contract_id: contract.contract_id,
        name: contract.name,
        description: contract.description,
        category: contract.category,
        tags: contract
            .tags
            .into_iter()
            .map(|tag| TagRecord { name: tag.name })
            .collect(),
    })
}

fn current_tags(contract: &ContractRecord) -> Vec<String> {
    contract.tags.iter().map(|t| t.name.clone()).collect()
}

fn build_diffs(
    current: &ContractRecord,
    patch: &MetadataPatch,
    icon_update: bool,
) -> Vec<(String, String, String)> {
    let mut diffs = Vec::new();

    if let Some(name) = &patch.name {
        if name != &current.name {
            diffs.push(("name".into(), current.name.clone(), name.clone()));
        }
    }
    if let Some(description) = &patch.description {
        let before = current.description.clone().unwrap_or_default();
        if description != &before {
            diffs.push(("description".into(), before, description.clone()));
        }
    }
    if let Some(category) = &patch.category {
        let before = current.category.clone().unwrap_or_default();
        if category != &before {
            diffs.push(("category".into(), before, category.clone()));
        }
    }
    if let Some(tags) = &patch.tags {
        let before = current_tags(current).join(", ");
        let after = tags.join(", ");
        if after != before {
            diffs.push(("tags".into(), before, after));
        }
    }
    if icon_update {
        diffs.push((
            "icon".into(),
            "(unchanged in preview)".into(),
            "(new file)".into(),
        ));
    }

    diffs
}

fn print_diff_table(contract_id: &str, diffs: &[(String, String, String)], icon_update: bool) {
    println!();
    println!("{}", "Contract Metadata Update Preview".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
    println!("  Contract: {}", contract_id.bold());
    println!();
    println!(
        "  {:<14} {:<22} {}",
        "Field".bold(),
        "Current".bold(),
        "New".bold()
    );
    println!("  {}", "-".repeat(58));
    for (field, before, after) in diffs {
        println!(
            "  {:<14} {:<22} {}",
            field,
            truncate(before, 22),
            truncate(after, 22)
        );
    }
    if icon_update && !diffs.iter().any(|(f, _, _)| f == "icon") {
        println!("  {:<14} {:<22} {}", "icon", "(existing)", "(new file)");
    }
    println!();
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        format!(
            "{}…",
            value
                .chars()
                .take(max.saturating_sub(1))
                .collect::<String>()
        )
    }
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{} [y/N]: ", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_diffs_detects_name_change() {
        let current = ContractRecord {
            id: "uuid".into(),
            contract_id: "C123".into(),
            name: "Old".into(),
            description: None,
            category: None,
            tags: vec![],
        };
        let patch = MetadataPatch {
            name: Some("New".into()),
            description: None,
            category: None,
            tags: None,
        };
        let diffs = build_diffs(&current, &patch, false);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].0, "name");
    }
}
