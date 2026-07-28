use crate::io_utils::compute_sha256_streaming;
use crate::net::RequestBuilderExt;
use crate::wizard::{confirm, prompt, prompt_with_validation};
use anyhow::{Context, Result};
use colored::Colorize;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{DependencyDeclaration, Network, PublishRequest};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use stellar_strkey::{Contract as ContractStrkey, Strkey};
use stellar_xdr::curr::{
    ContractDataDurability, ContractId, Hash, LedgerKey, LedgerKeyContractData, Limits, ScAddress,
    ScVal, WriteXdr,
};

const MAX_BATCH_SIZE: usize = 50;
const REGISTER_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RegistrationDraft {
    #[serde(default)]
    contract_id: Option<String>,
    #[serde(default)]
    wasm_hash: Option<String>,
    #[serde(default)]
    wasm_path: Option<PathBuf>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    network: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    publisher_address: Option<String>,
    #[serde(default)]
    dependencies: Option<Vec<DependencyDeclaration>>,
    #[serde(default)]
    is_cicd: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RegistrationFileWrapper {
    publisher: Option<String>,
    network: Option<String>,
    contracts: Vec<RegistrationDraft>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RegistrationFile {
    Single(RegistrationDraft),
    Batch(Vec<RegistrationDraft>),
    Wrapper(RegistrationFileWrapper),
}

#[derive(Debug, Clone, Serialize)]
struct ResolvedRegistration {
    request: PublishRequest,
    registry_id: Option<String>,
    registry_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct RegistrationOutcome {
    contract_id: String,
    name: String,
    network: String,
    status: String,
    registry_id: Option<String>,
    registry_url: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct RegistrationSummary {
    total: usize,
    registered: usize,
    failed: usize,
    skipped: usize,
    results: Vec<RegistrationOutcome>,
}

#[derive(Debug, Clone)]
struct NetworkDef {
    network: Network,
    rpc_endpoint: &'static str,
}

const NETWORKS: &[NetworkDef] = &[
    NetworkDef {
        network: Network::Mainnet,
        rpc_endpoint: "https://mainnet.sorobanrpc.com",
    },
    NetworkDef {
        network: Network::Testnet,
        rpc_endpoint: "https://soroban-testnet.stellar.org",
    },
    NetworkDef {
        network: Network::Futurenet,
        rpc_endpoint: "https://rpc-futurenet.stellar.org",
    },
];

pub async fn run(
    api_url: &str,
    config_network: crate::config::Network,
    file: Option<&str>,
    batch: bool,
    json: bool,
) -> Result<()> {
    let defaults = if config_network == crate::config::Network::Auto {
        None
    } else {
        Some(config_network.to_string())
    };

    let (drafts, file_publisher, file_network) = match file {
        Some(path) => load_file(path)?,
        None => (Vec::new(), None, None),
    };

    let mut drafts = if drafts.is_empty() {
        collect_interactive_entries(batch, defaults.clone()).await?
    } else {
        drafts
    };

    if drafts.is_empty() {
        anyhow::bail!("No contract metadata supplied.");
    }

    let resolved = resolve_drafts(
        &mut drafts,
        defaults.as_deref(),
        file_publisher.as_deref(),
        file_network.as_deref(),
        file.is_none(),
    )
    .await?;

    let (resolved, skipped_duplicates) = deduplicate(resolved);
    if resolved.is_empty() {
        anyhow::bail!("No valid contracts found.");
    }
    if resolved.len() > MAX_BATCH_SIZE {
        anyhow::bail!(
            "Batch size {} exceeds the maximum of {}.",
            resolved.len(),
            MAX_BATCH_SIZE
        );
    }

    validate_all(&resolved)?;

    let summary = submit_all(api_url, resolved, skipped_duplicates, json).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_summary(&summary);
    }

    if summary.failed > 0 {
        anyhow::bail!("{} contract(s) failed to register.", summary.failed);
    }

    Ok(())
}

fn load_file(path: &str) -> Result<(Vec<RegistrationDraft>, Option<String>, Option<String>)> {
    let ext = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "yaml" | "yml" | "json" => {}
        other => anyhow::bail!(
            "Unsupported metadata extension '.{}'. Use .yaml, .yml, or .json.",
            other
        ),
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read metadata file: {}", path))?;

    match ext.as_str() {
        "yaml" | "yml" => load_from_text(&content, false, path),
        "json" => load_from_text(&content, true, path),
        _ => unreachable!(),
    }
}

fn load_from_text(
    content: &str,
    is_json: bool,
    path: &str,
) -> Result<(Vec<RegistrationDraft>, Option<String>, Option<String>)> {
    let file: RegistrationFile = if is_json {
        serde_json::from_str(content)
            .with_context(|| format!("Failed to parse JSON metadata file: {}", path))?
    } else {
        serde_yaml::from_str(content)
            .with_context(|| format!("Failed to parse YAML metadata file: {}", path))?
    };

    Ok(match file {
        RegistrationFile::Single(entry) => (vec![entry], None, None),
        RegistrationFile::Batch(entries) => (entries, None, None),
        RegistrationFile::Wrapper(wrapper) => {
            (wrapper.contracts, wrapper.publisher, wrapper.network)
        }
    })
}

async fn collect_interactive_entries(
    batch: bool,
    default_network: Option<String>,
) -> Result<Vec<RegistrationDraft>> {
    let mut entries = Vec::new();

    loop {
        entries.push(prompt_for_entry(default_network.as_deref()).await?);

        if !batch {
            break;
        }

        if !confirm("Add another contract? [y/N]", false)? {
            break;
        }
    }

    Ok(entries)
}

async fn prompt_for_entry(default_network: Option<&str>) -> Result<RegistrationDraft> {
    println!("\n{}", "Contract Registration".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let contract_id = prompt_with_validation(
        "Contract ID",
        None::<String>,
        |value| is_valid_contract_id(value),
        "Enter a valid Stellar contract ID starting with C.",
    )?;

    let detected_network = detect_network(&contract_id).await.ok().flatten();
    if let Some(network) = &detected_network {
        println!(
            "{} {}",
            "Detected network:".bold().bright_black(),
            network.to_string().bright_blue()
        );
    }

    let network_default = default_network
        .map(str::to_string)
        .or_else(|| detected_network.map(|network| network.to_string()))
        .unwrap_or_else(|| "testnet".to_string());

    let network = prompt_with_validation(
        "Network [mainnet|testnet|futurenet]",
        Some(network_default),
        |value| {
            matches!(
                value.to_lowercase().as_str(),
                "mainnet" | "testnet" | "futurenet"
            )
        },
        "Choose mainnet, testnet, or futurenet.",
    )?;

    let wasm_path_raw = prompt("Path to WASM file (optional)", Some(String::new()))?;
    let wasm_hash = if wasm_path_raw.trim().is_empty() {
        prompt_with_validation(
            "WASM hash (64 hex chars)",
            None::<String>,
            |value| is_valid_wasm_hash(value),
            "Enter a valid 64-character hexadecimal SHA-256 hash.",
        )?
    } else {
        compute_sha256_streaming(Path::new(wasm_path_raw.trim()))
            .with_context(|| format!("Failed to hash WASM file: {}", wasm_path_raw.trim()))?
    };

    let name = prompt_with_validation(
        "Name",
        None::<String>,
        |value| !value.trim().is_empty() && value.chars().count() <= 255,
        "Enter a non-empty name up to 255 characters.",
    )?;

    let publisher_address = prompt_with_validation(
        "Publisher Stellar address",
        None::<String>,
        |value| is_valid_stellar_address(value),
        "Enter a valid Stellar address starting with G.",
    )?;

    let description = optional_prompt("Description");
    let category = optional_prompt("Category");
    let tags = optional_tags();
    let source_url = optional_prompt("Source URL");
    let slug = optional_prompt("Slug");

    Ok(RegistrationDraft {
        contract_id: Some(contract_id),
        wasm_hash: Some(wasm_hash),
        wasm_path: None,
        name: Some(name),
        slug,
        description,
        network: Some(network),
        category,
        tags,
        source_url,
        publisher_address: Some(publisher_address),
        dependencies: None,
        is_cicd: None,
    })
}

fn optional_prompt(label: &str) -> Option<String> {
    match prompt(label, Some(String::new())) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn optional_tags() -> Option<Vec<String>> {
    let raw = prompt("Tags (comma-separated)", Some(String::new())).unwrap_or_default();
    let tags: Vec<String> = raw
        .split(',')
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();

    if tags.is_empty() {
        None
    } else {
        Some(tags)
    }
}

async fn resolve_drafts(
    drafts: &mut [RegistrationDraft],
    default_network: Option<&str>,
    file_publisher: Option<&str>,
    file_network: Option<&str>,
    interactive: bool,
) -> Result<Vec<ResolvedRegistration>> {
    let mut resolved = Vec::with_capacity(drafts.len());

    for draft in drafts.iter_mut() {
        let mut contract_id = take_required(&mut draft.contract_id, "contract_id")?;
        contract_id = contract_id.trim().to_uppercase();

        let wasm_hash = resolve_wasm_hash(draft)?;
        let name = take_required(&mut draft.name, "name")?;
        let publisher_address = draft
            .publisher_address
            .clone()
            .or_else(|| file_publisher.map(str::to_string))
            .ok_or_else(|| anyhow::anyhow!("publisher_address is required"))?;
        let publisher_address = publisher_address.trim().to_uppercase();

        let network = resolve_network(
            draft.network.as_deref().or(file_network),
            default_network,
            &contract_id,
            interactive,
        )
        .await?;

        let description = trim_optional(draft.description.take());
        let category = trim_optional(draft.category.take());
        let source_url = trim_optional(draft.source_url.take());
        let slug = trim_optional(draft.slug.take());
        let tags = normalize_tags(draft.tags.take().unwrap_or_default());
        let dependencies = draft.dependencies.take().unwrap_or_default();
        let is_cicd = draft.is_cicd.unwrap_or(false);

        resolved.push(ResolvedRegistration {
            request: PublishRequest {
                contract_id,
                wasm_hash,
                name,
                slug,
                description,
                network,
                category,
                tags,
                source_url,
                publisher_address,
                dependencies,
                is_cicd,
            },
            registry_id: None,
            registry_url: None,
        });
    }

    Ok(resolved)
}

fn resolve_wasm_hash(draft: &RegistrationDraft) -> Result<String> {
    if let Some(path) = draft.wasm_path.as_ref() {
        return compute_sha256_streaming(path)
            .with_context(|| format!("Failed to hash WASM file: {}", path.display()));
    }

    let wasm_hash = draft
        .wasm_hash
        .clone()
        .ok_or_else(|| anyhow::anyhow!("wasm_hash is required"))?;

    if !is_valid_wasm_hash(&wasm_hash) {
        anyhow::bail!("wasm_hash must be a 64-character hexadecimal SHA-256 hash");
    }

    Ok(wasm_hash.trim().to_lowercase())
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn resolve_network(
    explicit_network: Option<&str>,
    default_network: Option<&str>,
    contract_id: &str,
    interactive: bool,
) -> Result<Network> {
    if let Some(network) = explicit_network {
        return parse_network(network);
    }

    if let Some(detected) = detect_network(contract_id).await? {
        return Ok(detected);
    }

    if let Some(default_network) = default_network {
        return parse_network(default_network);
    }

    if interactive {
        return parse_network(&prompt_with_validation(
            "Network [mainnet|testnet|futurenet]",
            Some("testnet".to_string()),
            |value| {
                matches!(
                    value.to_lowercase().as_str(),
                    "mainnet" | "testnet" | "futurenet"
                )
            },
            "Choose mainnet, testnet, or futurenet.",
        )?);
    }

    anyhow::bail!(
        "Unable to detect network for contract {}. Provide a network in the file or pass --network.",
        contract_id
    );
}

fn parse_network(value: &str) -> Result<Network> {
    match value.trim().to_ascii_lowercase().as_str() {
        "mainnet" => Ok(Network::Mainnet),
        "testnet" => Ok(Network::Testnet),
        "futurenet" => Ok(Network::Futurenet),
        _ => anyhow::bail!("Invalid network value: {}", value),
    }
}

async fn detect_network(contract_id: &str) -> Result<Option<Network>> {
    parse_contract_strkey(contract_id)?;

    let mut matches = Vec::new();
    for def in NETWORKS {
        match contract_exists_on_network(def, contract_id).await {
            Ok(true) => matches.push(def.network.clone()),
            Ok(false) => {}
            Err(err) => {
                log::debug!("Network probe for {} failed: {}", def.network, err);
            }
        }
    }

    match matches.as_slice() {
        [] => Ok(None),
        [network] => Ok(Some(network.clone())),
        _ => anyhow::bail!(
            "Contract address {} appears on multiple networks",
            contract_id
        ),
    }
}

async fn contract_exists_on_network(def: &NetworkDef, contract_id: &str) -> Result<bool> {
    let client = crate::net::client();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": "getLedgerEntries",
        "method": "getLedgerEntries",
        "params": {
            "keys": [build_contract_instance_ledger_key(contract_id)?],
            "xdrFormat": "base64"
        }
    });

    let response = client
        .post(def.rpc_endpoint)
        .json(&payload)
        .send()
        .await
        .with_context(|| format!("Failed to query {}", def.network))?;

    if !response.status().is_success() {
        return Ok(false);
    }

    let body: RpcEnvelope<GetLedgerEntriesResult> = response
        .json()
        .await
        .with_context(|| format!("Failed to parse {} response", def.network))?;

    Ok(body
        .result
        .map(|result| !result.entries.is_empty())
        .unwrap_or(false))
}

fn build_contract_instance_ledger_key(contract_id: &str) -> Result<String> {
    let contract = parse_contract_strkey(contract_id)?;
    let key = LedgerKey::ContractData(LedgerKeyContractData {
        contract: ScAddress::Contract(ContractId(Hash(contract.0))),
        key: ScVal::LedgerKeyContractInstance,
        durability: ContractDataDurability::Persistent,
    });
    key.to_xdr_base64(Limits::none())
        .context("Failed to encode contract ledger key")
}

fn parse_contract_strkey(contract_id: &str) -> Result<ContractStrkey> {
    match Strkey::from_string(contract_id.trim()).context("Invalid contract address")? {
        Strkey::Contract(contract) => Ok(contract),
        _ => anyhow::bail!("contract_id must be a Stellar contract address"),
    }
}

fn take_required(value: &mut Option<String>, field: &str) -> Result<String> {
    value
        .take()
        .ok_or_else(|| anyhow::anyhow!("{} is required", field))
        .map(|value| value.trim().to_string())
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .take(10)
        .collect()
}

fn is_valid_contract_id(value: &str) -> bool {
    matches!(Strkey::from_string(value.trim()), Ok(Strkey::Contract(_)))
}

fn is_valid_stellar_address(value: &str) -> bool {
    matches!(
        Strkey::from_string(value.trim()),
        Ok(Strkey::PublicKeyEd25519(_))
    )
}

fn is_valid_wasm_hash(value: &str) -> bool {
    let value = value.trim();
    value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn validate_all(resolved: &[ResolvedRegistration]) -> Result<()> {
    let mut errors = Vec::new();
    for item in resolved {
        let request = &item.request;
        if !is_valid_contract_id(&request.contract_id) {
            errors.push(format!("{}: invalid contract_id", request.contract_id));
        }
        if !is_valid_wasm_hash(&request.wasm_hash) {
            errors.push(format!("{}: invalid wasm_hash", request.contract_id));
        }
        if request.name.trim().is_empty() || request.name.chars().count() > 255 {
            errors.push(format!("{}: invalid name", request.contract_id));
        }
        if !is_valid_stellar_address(&request.publisher_address) {
            errors.push(format!(
                "{}: invalid publisher_address",
                request.contract_id
            ));
        }
        if !matches!(
            request.network,
            Network::Mainnet | Network::Testnet | Network::Futurenet
        ) {
            errors.push(format!("{}: invalid network", request.contract_id));
        }
        if let Some(category) = request.category.as_deref() {
            if !ALLOWED_CATEGORIES.contains(&category) {
                errors.push(format!(
                    "{}: invalid category '{}'",
                    request.contract_id, category
                ));
            }
        }
        if request.tags.len() > 10 {
            errors.push(format!("{}: too many tags", request.contract_id));
        }
        if let Some(url) = request.source_url.as_deref() {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                errors.push(format!("{}: invalid source_url", request.contract_id));
            }
        }
        for dep in &request.dependencies {
            if dep.name.trim().is_empty() || dep.version_constraint.trim().is_empty() {
                errors.push(format!(
                    "{}: invalid dependency declaration",
                    request.contract_id
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("Validation failed:\n  {}", errors.join("\n  "));
    }
}

fn deduplicate(entries: Vec<ResolvedRegistration>) -> (Vec<ResolvedRegistration>, usize) {
    let total = entries.len();
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for entry in entries {
        let key = format!("{}:{}", entry.request.contract_id, entry.request.network);
        if seen.insert(key) {
            deduped.push(entry);
        }
    }

    let skipped = total - deduped.len();
    (deduped, skipped)
}

async fn submit_all(
    api_url: &str,
    entries: Vec<ResolvedRegistration>,
    skipped_duplicates: usize,
    json: bool,
) -> Result<RegistrationSummary> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REGISTER_TIMEOUT_SECS))
        .build()?;

    let url = format!("{}/api/contracts", api_url.trim_end_matches('/'));
    let total = entries.len();
    let mut results = Vec::with_capacity(total);
    let mut registered = 0usize;
    let mut failed = 0usize;
    let mut skipped = skipped_duplicates;

    for (index, mut entry) in entries.into_iter().enumerate() {
        if !json {
            print!(
                "  [{}/{}] Registering {} ... ",
                index + 1,
                total,
                entry.request.contract_id.bold()
            );
        }

        match submit_one(&client, &url, &mut entry).await {
            Ok(outcome) => {
                if !json {
                    println!("{}", "registered".green());
                }
                if outcome.status == "registered" {
                    registered += 1;
                } else if outcome.status == "skipped" {
                    skipped += 1;
                }
                results.push(outcome);
            }
            Err(err) => {
                if !json {
                    println!("{} — {}", "failed".red(), err.to_string().red());
                }
                failed += 1;
                results.push(RegistrationOutcome {
                    contract_id: entry.request.contract_id,
                    name: entry.request.name,
                    network: entry.request.network.to_string(),
                    status: "failed".to_string(),
                    registry_id: None,
                    registry_url: None,
                    error: Some(err.to_string()),
                });
            }
        }
    }

    Ok(RegistrationSummary {
        total: results.len(),
        registered,
        failed,
        skipped,
        results,
    })
}

async fn submit_one(
    client: &reqwest::Client,
    url: &str,
    entry: &mut ResolvedRegistration,
) -> Result<RegistrationOutcome> {
    let response = client
        .post(url)
        .json(&entry.request)
        .send_with_retry()
        .await
        .context("Failed to reach registry API")?;

    if response.status() == StatusCode::CONFLICT {
        return Ok(RegistrationOutcome {
            contract_id: entry.request.contract_id.clone(),
            name: entry.request.name.clone(),
            network: entry.request.network.to_string(),
            status: "skipped".to_string(),
            registry_id: None,
            registry_url: None,
            error: None,
        });
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {}: {}", status, body);
    }

    let body: serde_json::Value = response
        .json()
        .await
        .context("Invalid JSON from registry")?;
    let registry_id = body["id"]
        .as_str()
        .or_else(|| body["contract_id"].as_str())
        .map(str::to_string);
    let registry_url = registry_id.as_deref().map(|id| {
        format!(
            "{}/contracts/{}",
            url.trim_end_matches("/api/contracts"),
            id
        )
    });

    entry.registry_id = registry_id.clone();
    entry.registry_url = registry_url.clone();

    Ok(RegistrationOutcome {
        contract_id: entry.request.contract_id.clone(),
        name: entry.request.name.clone(),
        network: entry.request.network.to_string(),
        status: "registered".to_string(),
        registry_id,
        registry_url,
        error: None,
    })
}

fn print_summary(summary: &RegistrationSummary) {
    println!("\n{}", "Contract Registration Summary".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!(
        "  {}: {} registered, {} failed, {} skipped",
        "Summary".bold(),
        summary.registered.to_string().green(),
        summary.failed.to_string().red(),
        summary.skipped.to_string().bright_black()
    );

    println!("\n{}", "Per-contract results:".bold());
    for result in &summary.results {
        let status = match result.status.as_str() {
            "registered" => result.status.green(),
            "failed" => result.status.red(),
            "skipped" => result.status.yellow(),
            other => other.normal(),
        };
        println!(
            "  {} {} — {} ({})",
            result.contract_id.bold(),
            result.name.bright_black(),
            status,
            result.network.bright_blue()
        );
        if let Some(id) = &result.registry_id {
            println!("    Registry ID: {}", id.bright_black());
        }
        if let Some(url) = &result.registry_url {
            println!("    Registry URL: {}", url.bright_black());
        }
        if let Some(error) = &result.error {
            println!("    Error: {}", error.red());
        }
    }

    println!("\n{}\n", "=".repeat(60).cyan());
}

const ALLOWED_CATEGORIES: &[&str] = &["DEX", "Lending", "Bridge", "Oracle", "Token", "Other"];

#[derive(Debug, Deserialize)]
struct RpcEnvelope<T> {
    result: Option<T>,
    #[allow(dead_code)]
    error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GetLedgerEntriesResult {
    entries: Vec<LedgerEntryResponse>,
}

#[derive(Debug, Deserialize)]
struct LedgerEntryResponse {
    #[allow(dead_code)]
    xdr: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_contract_id() -> String {
        format!("C{}", "A".repeat(55))
    }

    fn valid_stellar_address() -> String {
        format!("G{}", "A".repeat(55))
    }

    #[test]
    fn validates_known_formats() {
        assert!(is_valid_contract_id(&valid_contract_id()));
        assert!(is_valid_stellar_address(&valid_stellar_address()));
        assert!(is_valid_wasm_hash(&"a".repeat(64)));
    }

    #[test]
    fn normalize_tags_trims_and_limits() {
        let tags = normalize_tags(vec![
            " alpha ".to_string(),
            "".to_string(),
            "beta".to_string(),
        ]);
        assert_eq!(tags, vec!["alpha", "beta"]);
    }

    #[test]
    fn load_file_single_json() {
        let file: RegistrationFile = serde_json::from_str(&format!(
            r#"{{"contract_id":"{}","name":"Token","wasm_hash":"{}","publisher_address":"{}"}}"#,
            valid_contract_id(),
            "a".repeat(64),
            valid_stellar_address()
        ))
        .unwrap();
        match file {
            RegistrationFile::Single(entry) => {
                assert_eq!(entry.name.as_deref(), Some("Token"));
            }
            _ => panic!("expected single entry"),
        }
    }

    #[test]
    fn load_file_wrapper_yaml() {
        let publisher = valid_stellar_address();
        let contract_id = valid_contract_id();
        let yaml = r#"
publisher: __PUBLISHER__
network: testnet
contracts:
  - contract_id: __CONTRACT__
    name: Token
    wasm_hash: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
"#;
        let yaml = yaml
            .replace("__PUBLISHER__", &publisher)
            .replace("__CONTRACT__", &contract_id);
        let file: RegistrationFile = serde_yaml::from_str(&yaml).unwrap();
        match file {
            RegistrationFile::Wrapper(wrapper) => {
                assert_eq!(wrapper.publisher.as_deref(), Some(publisher.as_str()));
                assert_eq!(wrapper.network.as_deref(), Some("testnet"));
                assert_eq!(wrapper.contracts.len(), 1);
            }
            _ => panic!("expected wrapper"),
        }
    }
}
