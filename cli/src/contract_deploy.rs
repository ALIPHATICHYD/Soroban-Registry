use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::io_utils::compute_sha256_streaming;

/// Represents metadata for a contract deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentMetadata {
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub network: String,
    pub tags: Vec<String>,
    pub icon_path: Option<String>,
}

/// Response from contract deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentResponse {
    pub id: String,
    pub contract_id: String,
    pub wasm_hash: String,
    pub name: String,
    pub network: String,
    pub verification_status: String,
    pub created_at: String,
    pub confirmation_code: String,
}

/// Represents contract ABI information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAbiInfo {
    pub functions: Vec<Function>,
    pub custom_types: Vec<CustomType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub name: String,
    pub type_name: String,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub type_name: String,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomType {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub type_name: String,
}

/// Validates WASM file integrity and format
pub fn validate_wasm_file(wasm_path: &str) -> Result<String> {
    let path = Path::new(wasm_path);

    if !path.exists() {
        bail!("WASM file not found: {}", wasm_path);
    }

    if !path.is_file() {
        bail!("Path is not a file: {}", wasm_path);
    }

    // Check file size (max 10 MB for WASM)
    let metadata = fs::metadata(path).context("failed to get file metadata")?;
    if metadata.len() > 10 * 1024 * 1024 {
        bail!("WASM file exceeds maximum size of 10 MB");
    }

    // Verify it's a valid WASM binary by checking magic bytes
    let file_content = fs::read(path).context("failed to read WASM file")?;
    if file_content.len() < 4 {
        bail!("WASM file is too small (less than 4 bytes)");
    }

    // WASM files must start with magic bytes 0x00 0x61 0x73 0x6d ("\0asm")
    if file_content[0..4] != [0x00, 0x61, 0x73, 0x6d] {
        bail!("Invalid WASM file: incorrect magic bytes. Expected '\\0asm' at the start.");
    }

    println!("✓ WASM file validation passed",);

    Ok(wasm_path.to_string())
}

/// Computes SHA-256 hash of WASM file
pub fn compute_contract_hash(wasm_path: &str) -> Result<String> {
    let hash =
        compute_sha256_streaming(Path::new(wasm_path)).context("failed to compute WASM hash")?;
    println!("✓ Contract hash computed: {}", hash.cyan());
    Ok(hash)
}

/// Extracts contract ABI from WASM file
pub fn extract_abi_from_wasm(wasm_path: &str) -> Result<ContractAbiInfo> {
    println!("\n📋 Extracting contract ABI...");

    // Try using soroban CLI if available
    let output = Command::new("soroban")
        .args(&["contract", "bindings", "json", "--wasm", wasm_path])
        .output()
        .context("failed to run soroban contract bindings command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("⚠ Warning: Could not extract ABI using soroban CLI");
        println!("  Error: {}", stderr);

        // Return a basic ABI structure if soroban is not available
        return Ok(ContractAbiInfo {
            functions: vec![],
            custom_types: vec![],
        });
    }

    let abi_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("failed to parse ABI JSON")?;

    // Parse the ABI JSON into our structure
    let abi_info = parse_abi_json(&abi_json)?;
    println!(
        "✓ ABI extracted successfully ({} functions found)",
        abi_info.functions.len()
    );

    Ok(abi_info)
}

/// Parses soroban ABI JSON output
fn parse_abi_json(abi_json: &serde_json::Value) -> Result<ContractAbiInfo> {
    let mut functions = Vec::new();
    let mut custom_types = Vec::new();

    if let Some(specs) = abi_json.as_array() {
        for spec in specs {
            if let Some(spec_type) = spec.get("specType").and_then(|v| v.as_str()) {
                match spec_type {
                    "function" => {
                        if let Ok(func) = parse_function_spec(spec) {
                            functions.push(func);
                        }
                    }
                    "type" => {
                        if let Ok(custom_type) = parse_type_spec(spec) {
                            custom_types.push(custom_type);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(ContractAbiInfo {
        functions,
        custom_types,
    })
}

/// Parses a single function spec
fn parse_function_spec(spec: &serde_json::Value) -> Result<Function> {
    let name = spec
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let mut inputs = Vec::new();
    if let Some(input_specs) = spec.get("inputs").and_then(|v| v.as_array()) {
        for input in input_specs {
            if let Some(input_name) = input.get("name").and_then(|v| v.as_str()) {
                let type_name = input
                    .get("value")
                    .and_then(|v| v.get("typeName"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                inputs.push(Input {
                    name: input_name.to_string(),
                    type_name,
                    doc: input
                        .get("doc")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
    }

    let mut outputs = Vec::new();
    if let Some(output_specs) = spec.get("outputs").and_then(|v| v.as_array()) {
        for output in output_specs {
            let type_name = output
                .get("typeName")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            outputs.push(Output {
                type_name,
                doc: output
                    .get("doc")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            });
        }
    }

    Ok(Function {
        name,
        inputs,
        outputs,
        doc: spec
            .get("doc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

/// Parses a type spec
fn parse_type_spec(spec: &serde_json::Value) -> Result<CustomType> {
    let name = spec
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let mut fields = Vec::new();
    if let Some(field_specs) = spec.get("fields").and_then(|v| v.as_array()) {
        for field in field_specs {
            if let Some(field_name) = field.get("name").and_then(|v| v.as_str()) {
                let type_name = field
                    .get("value")
                    .and_then(|v| v.get("typeName"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                fields.push(Field {
                    name: field_name.to_string(),
                    type_name,
                });
            }
        }
    }

    Ok(CustomType { name, fields })
}

/// Validates metadata
pub fn validate_metadata(metadata: &DeploymentMetadata) -> Result<()> {
    if metadata.name.trim().is_empty() {
        bail!("Contract name cannot be empty");
    }

    if metadata.name.len() > 255 {
        bail!("Contract name exceeds maximum length of 255 characters");
    }

    if let Some(desc) = &metadata.description {
        if desc.len() > 5000 {
            bail!("Description exceeds maximum length of 5000 characters");
        }
    }

    if let Some(category) = &metadata.category {
        let valid_categories = vec!["DeFi", "Token", "Oracle", "NFT", "Utility", "Other"];
        if !valid_categories.contains(&category.as_str()) {
            bail!(
                "Invalid category: {}. Valid categories: {:?}",
                category,
                valid_categories
            );
        }
    }

    let valid_networks = vec!["mainnet", "testnet", "futurenet"];
    if !valid_networks.contains(&metadata.network.as_str()) {
        bail!(
            "Invalid network: {}. Valid networks: {:?}",
            metadata.network,
            valid_networks
        );
    }

    println!("✓ Metadata validation passed");

    Ok(())
}

/// Validates and processes icon file
pub fn validate_and_process_icon(icon_path: &str) -> Result<Vec<u8>> {
    let path = Path::new(icon_path);

    if !path.exists() {
        bail!("Icon file not found: {}", icon_path);
    }

    // Check file size (max 2 MB for icon)
    let metadata = fs::metadata(path).context("failed to get icon metadata")?;
    if metadata.len() > 2 * 1024 * 1024 {
        bail!("Icon file exceeds maximum size of 2 MB");
    }

    // Check file type (only PNG, JPG, SVG allowed)
    let file_ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    if !["png", "jpg", "jpeg", "svg"].contains(&file_ext.as_str()) {
        bail!("Invalid icon format. Supported formats: PNG, JPG, SVG");
    }

    let icon_data = fs::read(path).context("failed to read icon file")?;

    // Verify file is not corrupted by checking magic bytes
    match file_ext.as_str() {
        "png" => {
            if icon_data.len() < 8 || &icon_data[0..8] != [137, 80, 78, 71, 13, 10, 26, 10] {
                bail!("Invalid PNG file: incorrect header");
            }
        }
        "jpg" | "jpeg" => {
            if icon_data.len() < 2 || icon_data[0] != 0xFF || icon_data[1] != 0xD8 {
                bail!("Invalid JPG file: incorrect header");
            }
        }
        "svg" => {
            if icon_data.len() < 5 || !String::from_utf8_lossy(&icon_data[0..5]).contains("<") {
                bail!("Invalid SVG file: does not appear to be valid SVG");
            }
        }
        _ => {}
    }

    println!(
        "✓ Icon file validation passed ({})",
        file_ext.to_uppercase()
    );

    Ok(icon_data)
}

/// Collects metadata interactively from user
pub fn collect_metadata_interactive(wasm_path: &str) -> Result<DeploymentMetadata> {
    use std::io::Write;

    println!("\n🚀 Interactive Contract Deployment Mode");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Contract name
    print!("Contract name: ");
    std::io::stdout().flush()?;
    let mut name = String::new();
    std::io::stdin().read_line(&mut name)?;
    let name = name.trim().to_string();

    if name.is_empty() {
        bail!("Contract name is required");
    }

    // Description
    print!("Contract description (optional): ");
    std::io::stdout().flush()?;
    let mut description = String::new();
    std::io::stdin().read_line(&mut description)?;
    let description = if description.trim().is_empty() {
        None
    } else {
        Some(description.trim().to_string())
    };

    // Category
    println!("\nAvailable categories:");
    println!("  1. DeFi");
    println!("  2. Token");
    println!("  3. Oracle");
    println!("  4. NFT");
    println!("  5. Utility");
    println!("  6. Other");
    print!("Select category (1-6): ");
    std::io::stdout().flush()?;
    let mut category_input = String::new();
    std::io::stdin().read_line(&mut category_input)?;

    let category = match category_input.trim() {
        "1" => Some("DeFi".to_string()),
        "2" => Some("Token".to_string()),
        "3" => Some("Oracle".to_string()),
        "4" => Some("NFT".to_string()),
        "5" => Some("Utility".to_string()),
        "6" | _ => Some("Other".to_string()),
    };

    // Network
    println!("\nAvailable networks:");
    println!("  1. mainnet");
    println!("  2. testnet");
    println!("  3. futurenet");
    print!("Select network (1-3): ");
    std::io::stdout().flush()?;
    let mut network_input = String::new();
    std::io::stdin().read_line(&mut network_input)?;

    let network = match network_input.trim() {
        "1" => "mainnet".to_string(),
        "2" => "testnet".to_string(),
        "3" | _ => "futurenet".to_string(),
    };

    // Tags
    print!("\nTags (comma-separated, optional): ");
    std::io::stdout().flush()?;
    let mut tags_input = String::new();
    std::io::stdin().read_line(&mut tags_input)?;
    let tags: Vec<String> = tags_input
        .trim()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Icon
    print!("\nIcon file path (optional): ");
    std::io::stdout().flush()?;
    let mut icon_input = String::new();
    std::io::stdin().read_line(&mut icon_input)?;
    let icon_path = if icon_input.trim().is_empty() {
        None
    } else {
        Some(icon_input.trim().to_string())
    };

    Ok(DeploymentMetadata {
        name,
        description,
        category,
        network,
        tags,
        icon_path,
    })
}

/// Uploads icon to storage backend
pub async fn upload_icon_to_backend(
    api_url: &str,
    contract_id: &str,
    icon_data: &[u8],
    file_extension: &str,
) -> Result<String> {
    println!("\n📤 Uploading icon...");

    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new()
        .part(
            "icon",
            reqwest::multipart::Part::bytes(icon_data.to_vec())
                .file_name(format!("icon.{}", file_extension)),
        )
        .text("contract_id", contract_id.to_string());

    let response = client
        .post(&format!("{}/api/contracts/{}/icon", api_url, contract_id))
        .multipart(form)
        .send()
        .await
        .context("failed to upload icon")?;

    if !response.status().is_success() {
        bail!(
            "icon upload failed: {} - {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
    }

    let result: serde_json::Value = response
        .json()
        .await
        .context("failed to parse icon upload response")?;

    let icon_url = result
        .get("icon_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| contract_id.to_string());

    println!("✓ Icon uploaded successfully");

    Ok(icon_url)
}

/// Submits contract deployment to the registry API
pub async fn submit_contract_to_registry(
    api_url: &str,
    wasm_path: &str,
    contract_hash: &str,
    metadata: &DeploymentMetadata,
    publisher_address: &str,
) -> Result<DeploymentResponse> {
    println!("\n📝 Submitting contract to registry...");

    // Build deployment payload
    let payload = json!({
        "wasm_hash": contract_hash,
        "name": metadata.name,
        "description": metadata.description,
        "category": metadata.category,
        "network": metadata.network,
        "tags": metadata.tags,
        "publisher_address": publisher_address,
        "wasm_file_size": fs::metadata(wasm_path)?.len(),
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/api/contracts/deploy", api_url))
        .json(&payload)
        .send()
        .await
        .context("failed to submit contract to registry")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        bail!("contract deployment failed: {}", error_text);
    }

    let deployment_response: DeploymentResponse = response
        .json()
        .await
        .context("failed to parse deployment response")?;

    println!(
        "✓ Contract registered with ID: {}",
        deployment_response.id.cyan()
    );

    Ok(deployment_response)
}

/// Displays deployment summary
pub fn display_deployment_summary(
    response: &DeploymentResponse,
    metadata: &DeploymentMetadata,
    abi_info: &ContractAbiInfo,
) {
    println!(
        "\n{}",
        "═══════════════════════════════════════════════════"
            .green()
            .bold()
    );
    println!(
        "{}",
        "         ✓ CONTRACT DEPLOYMENT SUCCESSFUL".green().bold()
    );
    println!(
        "{}",
        "═══════════════════════════════════════════════════"
            .green()
            .bold()
    );

    println!("\n📋 Deployment Details:");
    println!("  Deployment ID:     {}", response.id.cyan());
    println!(
        "  Confirmation Code: {}",
        response.confirmation_code.yellow()
    );
    println!("  Contract Name:     {}", metadata.name);
    println!("  Network:           {}", metadata.network);
    println!(
        "  Category:          {}",
        metadata.category.as_ref().unwrap_or(&"N/A".to_string())
    );
    println!("  Verification:      {}", response.verification_status);
    println!("  Created At:        {}", response.created_at);

    println!("\n🔗 Contract Hash:");
    println!("  {}", response.wasm_hash);

    if !abi_info.functions.is_empty() {
        println!(
            "\n📚 Contract Functions ({} total):",
            abi_info.functions.len()
        );
        for func in &abi_info.functions {
            println!(
                "  • {}({})",
                func.name.cyan(),
                func.inputs
                    .iter()
                    .map(|i| i.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    if !metadata.tags.is_empty() {
        println!("\n🏷️  Tags:");
        for tag in &metadata.tags {
            println!("  • {}", tag);
        }
    }

    println!(
        "\n{}",
        "═══════════════════════════════════════════════════".green()
    );
    println!(
        "\n✨ Next steps:\n  • Monitor verification at: https://registry.soroban.org/contracts/{}\n  • Share your deployment ID: {}\n",
        response.id.cyan(),
        response.confirmation_code.yellow()
    );
}

/// Main entry point for contract deployment
pub async fn run_deploy(
    api_url: &str,
    wasm_path: &str,
    name: Option<&str>,
    description: Option<&str>,
    category: Option<&str>,
    network: &str,
    icon: Option<&str>,
    interactive: bool,
    publisher: Option<&str>,
    tags: Option<&str>,
    skip_abi: bool,
    json_output: bool,
) -> Result<()> {
    println!("\n🚀 Soroban Contract Deployment Manager");
    println!(
        "{}",
        "═══════════════════════════════════════════════════".cyan()
    );

    // Step 1: Validate WASM file
    println!("\n📦 Step 1/6: Validating WASM file...");
    validate_wasm_file(wasm_path)?;

    // Step 2: Compute contract hash
    println!("\n#️⃣  Step 2/6: Computing contract hash...");
    let contract_hash = compute_contract_hash(wasm_path)?;

    // Step 3: Collect or prepare metadata
    println!("\n📋 Step 3/6: Preparing contract metadata...");
    let metadata = if interactive {
        collect_metadata_interactive(wasm_path)?
    } else {
        // Validate that required metadata is provided
        let name =
            name.ok_or_else(|| anyhow::anyhow!("--name is required when not using --interactive"))?;

        DeploymentMetadata {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            category: category.map(|s| s.to_string()),
            network: network.to_string(),
            tags: tags
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default(),
            icon_path: icon.map(|s| s.to_string()),
        }
    };

    // Validate metadata
    validate_metadata(&metadata)?;

    // Step 4: Extract ABI (if not skipped)
    println!("\n📚 Step 4/6: Extracting contract ABI...");
    let abi_info = if skip_abi {
        println!("⚠️  Skipping ABI extraction");
        ContractAbiInfo {
            functions: vec![],
            custom_types: vec![],
        }
    } else {
        extract_abi_from_wasm(wasm_path).unwrap_or_else(|e| {
            println!("⚠️  Warning: ABI extraction failed - {}", e);
            ContractAbiInfo {
                functions: vec![],
                custom_types: vec![],
            }
        })
    };

    // Step 5: Get publisher address
    println!("\n👤 Step 5/6: Preparing publisher information...");
    let publisher_address = publisher.unwrap_or("unknown_publisher");
    println!("  Publisher: {}", publisher_address);

    // Step 6: Submit contract to registry
    println!("\n✉️  Step 6/6: Submitting contract to registry...");
    let deployment_response = submit_contract_to_registry(
        api_url,
        wasm_path,
        &contract_hash,
        &metadata,
        publisher_address,
    )
    .await?;

    // Upload icon if provided
    if let Some(icon_path) = &metadata.icon_path {
        if !icon_path.is_empty() {
            match validate_and_process_icon(icon_path) {
                Ok(icon_data) => {
                    let file_ext = std::path::Path::new(icon_path)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("png")
                        .to_lowercase();

                    match upload_icon_to_backend(
                        api_url,
                        &deployment_response.id,
                        &icon_data,
                        &file_ext,
                    )
                    .await
                    {
                        Ok(_) => {
                            println!("✓ Icon uploaded successfully");
                        }
                        Err(e) => {
                            println!("⚠️  Warning: Failed to upload icon - {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  Warning: Icon validation failed - {}", e);
                }
            }
        }
    }

    // Display summary
    if json_output {
        // Output as JSON
        let summary = json!({
            "status": "success",
            "deployment": deployment_response,
            "abi_functions_count": abi_info.functions.len(),
            "abi_types_count": abi_info.custom_types.len(),
            "network": metadata.network,
            "contract_name": metadata.name,
        });
        println!("\n{}", serde_json::to_string_pretty(&summary)?);
    } else {
        // Display formatted summary
        display_deployment_summary(&deployment_response, &metadata, &abi_info);
    }

    Ok(())
}
