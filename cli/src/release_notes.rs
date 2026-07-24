#![allow(dead_code)]

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionChange {
    pub name: String,
    pub change_type: String,
    pub old_signature: Option<String>,
    pub new_signature: Option<String>,
    pub is_breaking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub files_changed: i32,
    pub lines_added: i32,
    pub lines_removed: i32,
    pub function_changes: Vec<FunctionChange>,
    pub has_breaking_changes: bool,
    pub features_count: i32,
    pub fixes_count: i32,
    pub breaking_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseNotesResponse {
    pub id: String,
    pub contract_id: String,
    pub version: String,
    pub previous_version: Option<String>,
    pub diff_summary: DiffSummary,
    pub changelog_entry: Option<String>,
    pub notes_text: String,
    pub status: String,
    pub generated_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
}

/// Generate release notes for a contract version
pub async fn generate(
    api_url: &str,
    contract_id: &str,
    version: &str,
    previous_version: Option<&str>,
    changelog_file: Option<&str>,
    contract_address: Option<&str>,
    json_output: bool,
) -> Result<()> {
    println!("\n{}", "Generating release notes...".bold().cyan());
    println!("{}", "=".repeat(60).cyan());

    // Read changelog file if provided
    let changelog_content = if let Some(path) = changelog_file {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read changelog file: {}", path))?;
        Some(content)
    } else {
        // Try to auto-detect CHANGELOG.md in current directory
        if std::path::Path::new("CHANGELOG.md").exists() {
            let content =
                fs::read_to_string("CHANGELOG.md").context("Failed to read CHANGELOG.md")?;
            println!("{} Auto-detected CHANGELOG.md", "ℹ".blue());
            Some(content)
        } else {
            None
        }
    };

    let body = serde_json::json!({
        "version": version,
        "previous_version": previous_version,
        "changelog_content": changelog_content,
        "contract_address": contract_address,
    });

    let client = crate::net::client();
    let resp = client
        .post(format!(
            "{}/api/contracts/{}/release-notes/generate",
            api_url, contract_id
        ))
        .json(&body)
        .send_with_retry()
        .await
        .context("Failed to connect to registry API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("API returned {}: {}", status, text);
    }

    let notes: ReleaseNotesResponse = resp.json().await.context("Failed to parse API response")?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&notes)?);
    } else {
        print_release_notes(&notes);
    }

    Ok(())
}

/// View existing release notes for a contract version
pub async fn view(
    api_url: &str,
    contract_id: &str,
    version: &str,
    json_output: bool,
) -> Result<()> {
    let client = crate::net::client();
    let resp = client
        .get(format!(
            "{}/api/contracts/{}/release-notes/{}",
            api_url, contract_id, version
        ))
        .send_with_retry()
        .await
        .context("Failed to connect to registry API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("API returned {}: {}", status, text);
    }

    let notes: ReleaseNotesResponse = resp.json().await.context("Failed to parse API response")?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&notes)?);
    } else {
        print_release_notes(&notes);
    }

    Ok(())
}

/// Edit release notes (opens content from file or --text, then PUTs to API)
pub async fn edit(
    api_url: &str,
    contract_id: &str,
    version: &str,
    notes_file: Option<&str>,
    notes_text: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let text = if let Some(path) = notes_file {
        fs::read_to_string(path).with_context(|| format!("Failed to read notes file: {}", path))?
    } else if let Some(t) = notes_text {
        t.to_string()
    } else {
        anyhow::bail!("Either --file or --text must be provided for editing release notes");
    };

    println!("{}", "Updating release notes...".bold().cyan());

    let body = serde_json::json!({
        "notes_text": text,
    });

    let client = crate::net::client();
    let resp = client
        .put(format!(
            "{}/api/contracts/{}/release-notes/{}",
            api_url, contract_id, version
        ))
        .json(&body)
        .send_with_retry()
        .await
        .context("Failed to connect to registry API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("API returned {}: {}", status, text);
    }

    let notes: ReleaseNotesResponse = resp.json().await.context("Failed to parse API response")?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&notes)?);
    } else {
        println!(
            "{} Release notes updated for v{}",
            "✓".green(),
            notes.version
        );
        println!("{}: {}", "Status".bold(), notes.status);
    }

    Ok(())
}

/// Publish (finalize) release notes
pub async fn publish(
    api_url: &str,
    contract_id: &str,
    version: &str,
    skip_version_update: bool,
    json_output: bool,
) -> Result<()> {
    println!("{}", "Publishing release notes...".bold().cyan());

    let body = serde_json::json!({
        "update_version_record": !skip_version_update,
    });

    let client = crate::net::client();
    let resp = client
        .post(format!(
            "{}/api/contracts/{}/release-notes/{}/publish",
            api_url, contract_id, version
        ))
        .json(&body)
        .send_with_retry()
        .await
        .context("Failed to connect to registry API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("API returned {}: {}", status, text);
    }

    let notes: ReleaseNotesResponse = resp.json().await.context("Failed to parse API response")?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&notes)?);
    } else {
        println!(
            "{} Release notes v{} published!",
            "✓".green().bold(),
            notes.version
        );
        if !skip_version_update {
            println!("{} contract_versions.release_notes updated", "✓".green());
        }
    }

    Ok(())
}

/// List all release notes for a contract
pub async fn list(api_url: &str, contract_id: &str, json_output: bool) -> Result<()> {
    let client = crate::net::client();
    let resp = client
        .get(format!(
            "{}/api/contracts/{}/release-notes",
            api_url, contract_id
        ))
        .send_with_retry()
        .await
        .context("Failed to connect to registry API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("API returned {}: {}", status, text);
    }

    let all_notes: Vec<ReleaseNotesResponse> =
        resp.json().await.context("Failed to parse API response")?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&all_notes)?);
        return Ok(());
    }

    if all_notes.is_empty() {
        println!("{}", "No release notes found for this contract.".yellow());
        return Ok(());
    }

    println!(
        "\n{} release notes for contract {}",
        all_notes.len().to_string().bold(),
        contract_id.bold()
    );
    println!("{}", "─".repeat(60));

    for rn in &all_notes {
        let status_badge = match rn.status.as_str() {
            "published" => "✓ published".green().to_string(),
            "draft" => "◉ draft".yellow().to_string(),
            _ => rn.status.clone(),
        };
        let breaking = if rn.diff_summary.has_breaking_changes {
            " ⚠ BREAKING".red().bold().to_string()
        } else {
            String::new()
        };
        println!(
            "  v{:<12} [{}]{}  ({})",
            rn.version,
            status_badge,
            breaking,
            &rn.created_at[..10],
        );
    }
    println!();

    Ok(())
}

fn print_release_notes(notes: &ReleaseNotesResponse) {
    let status_badge = match notes.status.as_str() {
        "published" => "✓ published".green().to_string(),
        "draft" => "◉ draft".yellow().to_string(),
        _ => notes.status.clone(),
    };

    println!();
    println!(
        "  {} v{}  [{}]",
        "Release Notes".bold(),
        notes.version.bold(),
        status_badge,
    );
    println!("  {}", "─".repeat(50));

    if let Some(ref prev) = notes.previous_version {
        println!("  {} v{} → v{}", "Comparing:".dimmed(), prev, notes.version);
    } else {
        println!("  {}", "Initial release".dimmed());
    }

    println!("  {} {}", "Generated by:".dimmed(), notes.generated_by);
    println!();

    // Diff summary
    let ds = &notes.diff_summary;
    println!("  {}", "Diff Summary".bold().underline());
    println!(
        "    Files changed: {}  |  +{} -{} lines",
        ds.files_changed, ds.lines_added, ds.lines_removed
    );

    if ds.has_breaking_changes {
        println!(
            "    {} {} breaking change(s) detected",
            "⚠".red().bold(),
            ds.breaking_count,
        );
    }

    let added_count = ds
        .function_changes
        .iter()
        .filter(|c| c.change_type == "added")
        .count();
    let removed_count = ds
        .function_changes
        .iter()
        .filter(|c| c.change_type == "removed")
        .count();
    let modified_count = ds
        .function_changes
        .iter()
        .filter(|c| c.change_type == "modified")
        .count();

    if added_count > 0 || removed_count > 0 || modified_count > 0 {
        println!(
            "    Functions: +{} added, -{} removed, ~{} modified",
            added_count, removed_count, modified_count
        );
    }

    // Show function details
    for fc in &ds.function_changes {
        let symbol = match fc.change_type.as_str() {
            "added" => "+".green().to_string(),
            "removed" => "-".red().to_string(),
            "modified" => "~".yellow().to_string(),
            _ => " ".to_string(),
        };
        let breaking_mark = if fc.is_breaking {
            " BREAKING".red().bold().to_string()
        } else {
            String::new()
        };
        println!("      {} {}{}  ", symbol, fc.name, breaking_mark);
    }

    println!();

    // Full text
    println!("  {}", "Full Release Notes".bold().underline());
    for line in notes.notes_text.lines() {
        println!("  {}", line);
    }
    println!();
}
