#![allow(unused_variables)]

mod backup;
mod batch_verify;
mod commands;
mod config;
mod conversions;
mod coverage;
mod events;
mod export;
mod formal_verification;
mod fuzz;
mod import;
mod incident;
mod io_utils;
mod manifest;
mod migration;
mod multisig;
mod package_signing;
mod patch;
mod release_notes;
mod profiler;
mod sla;
mod test_framework;
mod webhook;
mod wizard;

use anyhow::Result;
use clap::{Parser, Subcommand};
use patch::Severity;

/// Soroban Registry CLI — discover, publish, verify, and deploy Soroban contracts
#[derive(Debug, Parser)]
#[command(name = "soroban-registry", version, about, long_about = None)]
pub struct Cli {
    /// Registry API URL
    #[arg(
        long,
        env = "SOROBAN_REGISTRY_API_URL",
        default_value = "http://localhost:3001"
    )]
    pub api_url: String,

    /// Stellar network to use (mainnet | testnet | futurenet)
    #[arg(long, global = true)]
    pub network: Option<String>,

    /// Enable verbose output (shows HTTP requests, responses, and debug info)
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Search for contracts in the registry
    Search {
        /// Search query
        query: String,
        /// Only show verified contracts
        #[arg(long)]
        verified_only: bool,
        /// Filter by one or more networks (comma-separated: mainnet,testnet,futurenet)
        #[arg(long)]
        networks: Option<String>,
        /// Filter by contract category (e.g. DEX, token, lending, oracle)
        #[arg(long)]
        category: Option<String>,
        /// Maximum number of results to return
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Number of results to skip (for pagination)
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Get detailed information about a contract
    Info {
        /// Contract registry UUID (use --network for network-specific config)
        contract_id: String,
    },

    /// Publish a new contract to the registry
    Publish {
        /// On-chain contract ID
        #[arg(long)]
        contract_id: String,

        /// Human-readable contract name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Network (mainnet, testnet, futurenet)
        #[arg(long, default_value = "Testnet")]
        network: String,

        /// Category
        #[arg(long)]
        category: Option<String>,

        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,

        /// Publisher Stellar address
        #[arg(long)]
        publisher: String,

        /// Validate inputs and show the payload without creating records
        #[arg(long)]
        dry_run: bool,
    },

    /// List recent contracts
    List {
        /// Maximum number of contracts to show
        #[arg(long, default_value = "10")]
        limit: usize,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Detect breaking changes between contract versions
    BreakingChanges {
        /// Old contract identifier (UUID or contract_id@version)
        old_id: String,
        /// New contract identifier (UUID or contract_id@version)
        new_id: String,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Contract state migration assistant
    Migrate {
        #[command(subcommand)]
        action: MigrateCommands,
    },
    /// Analyze upgrades between two contract versions or schema files
    UpgradeAnalyze {
        /// Old contract version ID or local schema JSON file
        old: String,

        /// New contract version ID or local schema JSON file
        new: String,

        /// Output JSON
        #[arg(long)]
        json: bool,
    },

    /// Export a contract archive (.tar.gz)
    Export {
        /// Contract registry ID (UUID)
        #[arg(long)]
        id: String,

        /// Output archive path
        #[arg(long, default_value = "contract-export.tar.gz")]
        output: String,

        /// Path to contract source directory
        #[arg(long, default_value = ".")]
        contract_dir: String,
    },

    /// Import a contract from an archive
    Import {
        /// Path to the archive file
        archive: String,

        /// Directory to extract into
        #[arg(long, default_value = "./imported")]
        output_dir: String,
    },

    /// Generate documentation from a contract WASM
    Doc {
        /// Path to contract WASM file
        contract_path: String,

        /// Output directory
        #[arg(long, default_value = "docs")]
        output: String,
    },

    /// Generate OpenAPI 3.0 spec from contract ABI
    Openapi {
        /// Path to contract WASM file or ABI JSON file
        contract_path: String,

        /// Output file path
        #[arg(long, short = 'o', default_value = "openapi.yaml")]
        output: String,

        /// Output format: yaml, json, markdown, html
        #[arg(long, short = 'f', default_value = "yaml")]
        format: String,
    },

    /// Launch the interactive setup wizard
    Wizard {},

    /// Show command history
    History {
        /// Filter by search term
        #[arg(long)]
        search: Option<String>,

        /// Maximum number of entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Security patch management
    Patch {
        #[command(subcommand)]
        action: PatchCommands,
    },

    /// Incident response management
    Incident {
        #[command(subcommand)]
        action: IncidentCommands,
    },

    /// Multi-signature contract deployment workflow
    Multisig {
        #[command(subcommand)]
        action: MultisigCommands,
    },

    /// Fuzz testing for contracts
    Fuzz {
        #[arg(long)]
        contract_path: String,
        #[arg(long)]
        duration: u64,
        #[arg(long)]
        timeout: u64,
        #[arg(long)]
        threads: u32,
        #[arg(long)]
        max_cases: u32,
        #[arg(long)]
        output: String,
        #[arg(long)]
        minimize: bool,
    },

    /// Profile contract execution performance
    Profile {
        /// Path to contract file
        contract_path: String,

        /// Method to profile
        #[arg(long)]
        method: Option<String>,

        /// Output JSON file
        #[arg(long)]
        output: Option<String>,

        /// Generate flame graph
        #[arg(long)]
        flamegraph: Option<String>,

        /// Compare with baseline profile
        #[arg(long)]
        compare: Option<String>,

        /// Show recommendations
        #[arg(long, default_value = "true")]
        recommendations: bool,
    },

    /// Run integration tests
    Test {
        /// Path to test file (YAML or JSON)
        test_file: String,

        /// Path to contract directory or file
        #[arg(long)]
        contract_path: Option<String>,

        /// Output JUnit XML report
        #[arg(long)]
        junit: Option<String>,

        /// Show coverage report
        #[arg(long, default_value = "true")]
        coverage: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// SLA compliance monitoring
    Sla {
        #[command(subcommand)]
        action: SlaCommands,
    },

    Config {
        #[command(subcommand)]
        action: ConfigSubcommands,
    },

    /// Run formal verification analysis against a deployed or local contract
    VerifyFormal {
        /// Path to contract file
        contract_path: String,

        /// Path to properties DSL file
        #[arg(long)]
        properties: String,

        /// Output format (json or text)
        #[arg(long, default_value = "text")]
        output: String,

        /// Post results back to registry
        #[arg(long)]
        post: bool,
    },

    ScanDeps {
        #[arg(long)]
        contract_id: String,
        #[arg(long, default_value = ",")]
        dependencies: String,
        #[arg(long, default_value_t = false)]
        fail_on_high: bool,
    },

    /// Measure and report code coverage for contract tests
    Coverage {
        /// Path to contract directory
        contract_path: String,

        /// Path to test directory or file
        #[arg(long)]
        tests: String,

        /// Fail if coverage is below this threshold (0-100)
        #[arg(long, default_value_t = 0.0)]
        threshold: f64,

        /// Output directory for HTML reports
        #[arg(long, default_value = "coverage_report")]
        output: String,
    },

    /// Sign a contract package with your private key
    Sign {
        /// Path to the package file to sign
        package: String,

        /// Private key (base64-encoded Ed25519)
        #[arg(long)]
        private_key: String,

        /// Contract ID
        #[arg(long)]
        contract_id: String,

        /// Package version
        #[arg(long)]
        version: String,

        /// Signature expiration (RFC3339 format)
        #[arg(long)]
        expires_at: Option<String>,
    },

    /// Verify a signed contract package
    Verify {
        /// Path to the package file to verify
        package: String,

        /// Contract ID
        #[arg(long)]
        contract_id: String,

        /// Package version (optional)
        #[arg(long)]
        version: Option<String>,

        /// Signature (base64, optional - will lookup from registry if not provided)
        #[arg(long)]
        signature: Option<String>,
    },

    /// Verify a contract binary against an Ed25519 signature locally
    VerifyContract {
        /// Path to the contract WASM/binary file
        wasm_path: String,

        /// Contract ID used when signing
        #[arg(long)]
        contract_id: String,

        /// Contract version used when signing
        #[arg(long)]
        version: String,

        /// Ed25519 signature (base64)
        #[arg(long)]
        signature: String,

        /// Ed25519 public key (base64)
        #[arg(long)]
        public_key: String,
    },

    /// Manage signing keys and signatures
    Keys {
        #[command(subcommand)]
        action: KeysCommands,
    },

    /// Verify multiple contracts in a single atomic batch (all succeed or all rollback)
    BatchVerify {
        /// Comma-separated list of contract IDs to verify.
        /// Optionally suffix with @version (e.g. abc123@1.0.0,def456)
        #[arg(long)]
        contracts: String,

        /// Stellar address or username initiating the batch (recorded in audit log)
        #[arg(long)]
        initiated_by: String,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Manage webhooks for contract lifecycle events
    Webhook {
        #[command(subcommand)]
        action: WebhookCommands,
    },

    /// Auto-generate and manage release notes for contract versions
    ReleaseNotes {
        #[command(subcommand)]
        action: ReleaseNotesCommands,
    },
}

/// Sub-commands for the `release-notes` group
#[derive(Debug, Subcommand)]
pub enum ReleaseNotesCommands {
    /// Auto-generate release notes from code diff and changelog
    Generate {
        /// Contract registry ID (UUID or on-chain ID)
        #[arg(long)]
        contract_id: String,

        /// Version to generate notes for (semver, e.g. 1.2.0)
        #[arg(long)]
        version: String,

        /// Previous version to diff against (auto-detected if omitted)
        #[arg(long)]
        previous_version: Option<String>,

        /// Path to CHANGELOG.md file (auto-detected if present in cwd)
        #[arg(long)]
        changelog: Option<String>,

        /// On-chain contract address to include in notes
        #[arg(long)]
        contract_address: Option<String>,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// View generated release notes for a version
    View {
        /// Contract registry ID
        #[arg(long)]
        contract_id: String,

        /// Version to view
        #[arg(long)]
        version: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Edit draft release notes before publishing
    Edit {
        /// Contract registry ID
        #[arg(long)]
        contract_id: String,

        /// Version to edit
        #[arg(long)]
        version: String,

        /// Path to a file containing the new release notes text
        #[arg(long)]
        file: Option<String>,

        /// Inline text for the release notes
        #[arg(long)]
        text: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Publish (finalize) release notes
    Publish {
        /// Contract registry ID
        #[arg(long)]
        contract_id: String,

        /// Version to publish
        #[arg(long)]
        version: String,

        /// Skip updating the contract_versions.release_notes column
        #[arg(long)]
        skip_version_update: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List all release notes for a contract
    List {
        /// Contract registry ID
        #[arg(long)]
        contract_id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommands {
    Get {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
    },
    Set {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
        #[arg(long)]
        config_data: String,
        #[arg(long)]
        secrets_data: Option<String>,
        #[arg(long)]
        created_by: String,
    },
    History {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
    },
    Rollback {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
        #[arg(long)]
        version: i32,
        #[arg(long)]
        created_by: String,
    },
}

/// Sub-commands for the `sla` group
#[derive(Debug, Subcommand)]
pub enum SlaCommands {
    /// Record hourly SLA metrics for a contract
    Record {
        /// Contract identifier
        id: String,
        /// Uptime percentage (0-100)
        uptime: f64,
        /// Average latency in milliseconds
        latency: f64,
        /// Error rate percentage (0-100)
        error_rate: f64,
    },
    /// Show real-time SLA compliance dashboard
    Status {
        /// Contract identifier
        id: String,
    },
}

/// Sub-commands for the `multisig` group
#[derive(Debug, Subcommand)]
pub enum MultisigCommands {
    /// Create a new multi-sig policy (defines signers and required threshold)
    CreatePolicy {
        #[arg(long)]
        name: String,
        #[arg(long)]
        threshold: u32,
        #[arg(long)]
        signers: String,
        #[arg(long)]
        expiry_secs: Option<u32>,
        #[arg(long)]
        created_by: String,
    },

    /// Create an unsigned deployment proposal
    CreateProposal {
        #[arg(long)]
        contract_name: String,
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        wasm_hash: String,
        #[arg(long, default_value = "testnet")]
        network: String,
        #[arg(long)]
        policy_id: String,
        #[arg(long)]
        proposer: String,
        #[arg(long)]
        description: Option<String>,
    },

    /// Sign a deployment proposal (add your approval)
    Sign {
        proposal_id: String,
        #[arg(long)]
        signer: String,
        #[arg(long)]
        signature_data: Option<String>,
    },

    /// Execute an approved deployment proposal
    Execute { proposal_id: String },

    /// Show full info for a proposal (signatures, policy, status)
    Info { proposal_id: String },

    /// List deployment proposals
    ListProposals {
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

/// Sub-commands for the `incident` group
#[derive(Debug, Subcommand)]
pub enum IncidentCommands {
    /// Trigger a new incident for a contract
    Trigger {
        /// On-chain contract ID
        contract_id: String,
        /// Incident severity (critical|high|medium|low)
        #[arg(long)]
        severity: String,
    },
    /// Update the state of an existing incident
    Update {
        /// Incident UUID returned by trigger
        incident_id: String,
        /// New state (detected|responding|contained|recovered|post_review)
        #[arg(long)]
        state: String,
    },
}

/// Sub-commands for the `patch` group
#[derive(Debug, Subcommand)]
pub enum PatchCommands {
    /// Create a new security patch
    Create {
        #[arg(long)]
        version: String,
        #[arg(long)]
        hash: String,
        #[arg(long)]
        severity: String,
        #[arg(long, default_value = "100")]
        rollout: u8,
    },
    /// Notify subscribers about a patch
    Notify {
        #[arg(long)]
        patch_id: String,
    },
    /// Apply a patch to a specific contract
    Apply {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        patch_id: String,
    },
    /// Manage contract dependencies
    Deps {
        #[command(subcommand)]
        command: DepsCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum DepsCommands {
    /// List dependencies for a contract
    List {
        /// Contract ID
        contract_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum KeysCommands {
    /// Generate a new Ed25519 keypair for signing
    Generate {},

    /// Revoke a signature
    Revoke {
        /// Signature ID to revoke
        signature_id: String,
        /// Address of the revoker
        #[arg(long)]
        revoked_by: String,
        /// Reason for revocation
        #[arg(long)]
        reason: String,
    },

    /// Show chain of custody for a contract
    Custody {
        /// Contract ID
        contract_id: String,
    },

    /// View transparency log
    Log {
        /// Filter by contract ID
        #[arg(long)]
        contract_id: Option<String>,
        /// Filter by entry type
        #[arg(long)]
        entry_type: Option<String>,
        /// Maximum entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

/// Sub-commands for the `webhook` group
#[derive(Debug, Subcommand)]
pub enum WebhookCommands {
    /// Register a new webhook subscription
    Create {
        /// Endpoint URL to receive events (must be HTTPS in production)
        #[arg(long)]
        url: String,

        /// Comma-separated list of events to subscribe to.
        /// Valid: contract.published, contract.verified,
        ///        contract.failed_verification, version.created
        #[arg(long)]
        events: String,

        /// Optional HMAC-SHA256 secret key (auto-generated if omitted)
        #[arg(long)]
        secret: Option<String>,
    },

    /// List all registered webhooks
    List {},

    /// Delete a webhook by ID
    Delete {
        /// Webhook ID to delete
        webhook_id: String,
    },

    /// Send a test event to a webhook
    Test {
        /// Webhook ID to test
        webhook_id: String,
    },

    /// View delivery logs for a webhook
    Logs {
        /// Webhook ID
        webhook_id: String,

        /// Maximum number of log entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Manually retry a dead-letter delivery
    Retry {
        /// Delivery ID to retry
        delivery_id: String,
    },

    /// Verify a webhook payload signature locally
    VerifySig {
        /// HMAC secret key used for signing
        #[arg(long)]
        secret: String,

        /// Raw JSON payload body
        #[arg(long)]
        payload: String,

        /// Signature header value (e.g. sha256=abc123...)
        #[arg(long)]
        signature: String,
    },
}

/// Sub-commands for the `migrate` group
#[derive(Debug, Subcommand)]
pub enum MigrateCommands {
    /// Preview migration outcome (dry-run)
    Preview {
        old_id: String,
        new_id: String,
    },
    /// Analyze schema differences between versions
    Analyze {
        old_id: String,
        new_id: String,
    },
    /// Generate migration script template (rust|js)
    Generate {
        old_id: String,
        new_id: String,
        #[arg(long, default_value = "rust")]
        language: String,
        #[arg(long)]
        output: Option<String>,
    },
    /// Validate migration for data loss risks
    Validate {
        old_id: String,
        new_id: String,
    },
    /// Apply migration and record history
    Apply {
        old_id: String,
        new_id: String,
    },
    /// Rollback a migration by migration ID
    Rollback { migration_id: String },
    /// Show migration history
    History {
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // ── Initialise logger ─────────────────────────────────────────────────────
    // --verbose / -v  →  DEBUG level (shows HTTP calls, payloads, timing)
    // default         →  WARN level  (only errors and warnings)
    let log_level = if cli.verbose { "debug" } else { "warn" };
    env_logger::Builder::new()
        .parse_filters(log_level)
        .format_timestamp(None) // no timestamps in CLI output
        .format_module_path(cli.verbose) // show module path only in verbose
        .init();

    log::debug!("Verbose mode enabled");
    log::debug!("API URL: {}", cli.api_url);

    // ── Resolve network ───────────────────────────────────────────────────────
    let cfg_network = config::resolve_network(cli.network)?;
    let mut net_str = cfg_network.to_string();
    if net_str == "auto" { net_str = "mainnet".to_string(); }
    let network: commands::Network = net_str.parse().unwrap();
    log::debug!("Network: {:?}", network);

    match cli.command {
        Commands::Search {
            query,
            verified_only,
            networks,
            category,
            limit,
            offset,
            json,
        } => {
            let networks_vec: Vec<String> = networks
                .map(|n| n.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            log::debug!(
                "Command: search | query={:?} verified_only={} networks={:?} category={:?}",
                query,
                verified_only,
                networks_vec,
                category
            );
            commands::search(
                &cli.api_url,
                &query,
                network,
                verified_only,
                networks_vec,
                category.as_deref(),
                limit,
                offset,
                json,
            )
            .await?;
        }
        Commands::Info { contract_id } => {
            log::debug!("Command: info | contract_id={}", contract_id);
            commands::info(&cli.api_url, &contract_id, cfg_network).await?;
        }
        Commands::Publish {
            contract_id,
            name,
            description,
            network: _publish_network,
            category,
            tags,
            publisher,
            dry_run,
        } => {
            let tags_vec = tags
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            log::debug!(
                "Command: publish | contract_id={} name={} tags={:?} dry_run={}",
                contract_id,
                name,
                tags_vec,
                dry_run
            );
            commands::publish(
                &cli.api_url,
                &contract_id,
                &name,
                description.as_deref(),
                network,
                category.as_deref(),
                tags_vec,
                &publisher,
                dry_run,
            )
            .await?;
        }
        Commands::List { limit, json } => {
            log::debug!("Command: list | limit={}", limit);
            commands::list(&cli.api_url, limit, network, json).await?;
        }
        Commands::BreakingChanges { old_id, new_id, json } => {
            log::debug!("Command: breaking-changes | old={} new={}", old_id, new_id);
            commands::breaking_changes(&cli.api_url, &old_id, &new_id, json).await?;
        }
        Commands::UpgradeAnalyze { old, new, json } => {
            log::debug!("Command: upgrade analyze | old={} new={}", old, new);
            commands::upgrade_analyze(&cli.api_url, &old, &new, json).await?;
        }
        Commands::Migrate { action } => match action {
            MigrateCommands::Preview { old_id, new_id } => {
                log::debug!("Command: migrate preview | old_id={} new_id={}", old_id, new_id);
                migration::preview(&old_id, &new_id)?;
            }
            MigrateCommands::Analyze { old_id, new_id } => {
                log::debug!("Command: migrate analyze | old_id={} new_id={}", old_id, new_id);
                migration::analyze(&old_id, &new_id)?;
            }
            MigrateCommands::Generate {
                old_id,
                new_id,
                language,
                output,
            } => {
                log::debug!(
                    "Command: migrate generate | old_id={} new_id={} language={}",
                    old_id,
                    new_id,
                    language
                );
                migration::generate_template(&old_id, &new_id, &language, output.as_deref())?;
            }
            MigrateCommands::Validate { old_id, new_id } => {
                log::debug!("Command: migrate validate | old_id={} new_id={}", old_id, new_id);
                migration::validate(&old_id, &new_id)?;
            }
            MigrateCommands::Apply { old_id, new_id } => {
                log::debug!("Command: migrate apply | old_id={} new_id={}", old_id, new_id);
                migration::apply(&old_id, &new_id)?;
            }
            MigrateCommands::Rollback { migration_id } => {
                log::debug!("Command: migrate rollback | migration_id={}", migration_id);
                migration::rollback(&migration_id)?;
            }
            MigrateCommands::History { limit } => {
                log::debug!("Command: migrate history | limit={}", limit);
                migration::history(limit)?;
            }
        },
        Commands::Export {
            id,
            output,
            contract_dir,
        } => {
            log::debug!("Command: export | id={} output={}", id, output);
            commands::export(&cli.api_url, &id, &output, &contract_dir).await?;
        }
        Commands::Import {
            archive,
            output_dir,
        } => {
            log::debug!(
                "Command: import | archive={} output_dir={}",
                archive,
                output_dir
            );
            commands::import(&cli.api_url, &archive, network, &output_dir).await?;
        }
        Commands::Doc {
            contract_path,
            output,
        } => {
            log::debug!(
                "Command: doc | contract_path={} output={}",
                contract_path,
                output
            );
            commands::doc(&contract_path, &output)?;
        }
        Commands::Openapi {
            contract_path,
            output,
            format,
        } => {
            log::debug!(
                "Command: openapi | contract_path={} output={} format={}",
                contract_path,
                output,
                format
            );
            commands::openapi(&contract_path, &output, &format)?;
        }
        Commands::Wizard {} => {
            log::debug!("Command: wizard");
            wizard::run(&cli.api_url).await?;
        }
        Commands::History { search, limit } => {
            log::debug!("Command: history | search={:?} limit={}", search, limit);
            wizard::show_history(search.as_deref(), limit)?;
        }
        Commands::Incident { action } => match action {
            IncidentCommands::Trigger {
                contract_id,
                severity,
            } => {
                log::debug!(
                    "Command: incident trigger | contract_id={} severity={}",
                    contract_id,
                    severity
                );
                commands::incident_trigger(&contract_id, &severity)?;
            }
            IncidentCommands::Update { incident_id, state } => {
                log::debug!(
                    "Command: incident update | incident_id={} state={}",
                    incident_id,
                    state
                );
                commands::incident_update(&incident_id, &state)?;
            }
        },
        Commands::Patch { action } => match action {
            PatchCommands::Create {
                version,
                hash,
                severity,
                rollout,
            } => {
                let sev = severity.parse::<Severity>()?;
                log::debug!(
                    "Command: patch create | version={} rollout={}",
                    version,
                    rollout
                );
                commands::patch_create(&cli.api_url, &version, &hash, sev, rollout).await?;
            }
            PatchCommands::Notify { patch_id } => {
                log::debug!("Command: patch notify | patch_id={}", patch_id);
                commands::patch_notify(&cli.api_url, &patch_id).await?;
            }
            PatchCommands::Apply {
                contract_id,
                patch_id,
            } => {
                log::debug!(
                    "Command: patch apply | contract_id={} patch_id={}",
                    contract_id,
                    patch_id
                );
                commands::patch_apply(&cli.api_url, &contract_id, &patch_id).await?;
            }
            PatchCommands::Deps { command } => match command {
                DepsCommands::List { contract_id } => {
                    commands::deps_list(&cli.api_url, &contract_id).await?;
                }
            },
        },
        // ── Multi-sig commands (issue #47) ───────────────────────────────────
        Commands::Multisig { action } => match action {
            MultisigCommands::CreatePolicy {
                name,
                threshold,
                signers,
                expiry_secs,
                created_by,
            } => {
                let signer_vec: Vec<String> =
                    signers.split(',').map(|s| s.trim().to_string()).collect();
                log::debug!(
                    "Command: multisig create-policy | name={} threshold={} signers={:?}",
                    name,
                    threshold,
                    signer_vec
                );
                multisig::create_policy(
                    &cli.api_url,
                    &name,
                    threshold,
                    signer_vec,
                    expiry_secs,
                    &created_by,
                )
                .await?;
            }
            MultisigCommands::CreateProposal {
                contract_name,
                contract_id,
                wasm_hash,
                network: net_str,
                policy_id,
                proposer,
                description,
            } => {
                log::debug!(
                    "Command: multisig create-proposal | contract_id={} policy_id={}",
                    contract_id,
                    policy_id
                );
                multisig::create_proposal(
                    &cli.api_url,
                    &contract_name,
                    &contract_id,
                    &wasm_hash,
                    &net_str,
                    &policy_id,
                    &proposer,
                    description.as_deref(),
                )
                .await?;
            }
            MultisigCommands::Sign {
                proposal_id,
                signer,
                signature_data,
            } => {
                log::debug!("Command: multisig sign | proposal_id={}", proposal_id);
                multisig::sign_proposal(
                    &cli.api_url,
                    &proposal_id,
                    &signer,
                    signature_data.as_deref(),
                )
                .await?;
            }
            MultisigCommands::Execute { proposal_id } => {
                log::debug!("Command: multisig execute | proposal_id={}", proposal_id);
                multisig::execute_proposal(&cli.api_url, &proposal_id).await?;
            }
            MultisigCommands::Info { proposal_id } => {
                log::debug!("Command: multisig info | proposal_id={}", proposal_id);
                multisig::proposal_info(&cli.api_url, &proposal_id).await?;
            }
            MultisigCommands::ListProposals { status, limit } => {
                log::debug!(
                    "Command: multisig list-proposals | status={:?} limit={}",
                    status,
                    limit
                );
                multisig::list_proposals(&cli.api_url, status.as_deref(), limit).await?;
            }
        },
        Commands::Fuzz {
            contract_path,
            duration,
            timeout,
            threads,
            max_cases,
            output,
            minimize,
        } => {
            fuzz::run_fuzzer(
                &contract_path,
                &duration.to_string(),
                &timeout.to_string(),
                threads as usize,
                max_cases as u64,
                &output,
                minimize,
            )
            .await?;
        }
        Commands::Profile {
            contract_path,
            method,
            output,
            flamegraph,
            compare,
            recommendations,
        } => {
            log::debug!(
                "Command: profile | contract_path={} method={:?} output={:?} flamegraph={:?} compare={:?} recommendations={}",
                contract_path,
                method,
                output,
                flamegraph,
                compare,
                recommendations
            );
            commands::profile(
                &contract_path,
                method.as_deref(),
                output.as_deref(),
                flamegraph.as_deref(),
                compare.as_deref(),
                recommendations,
            )?;
        }
        Commands::Test {
            test_file,
            contract_path,
            junit,
            coverage,
            verbose,
        } => {
            commands::run_tests(
                &test_file,
                contract_path.as_deref(),
                junit.as_deref(),
                coverage,
                verbose,
            )
            .await?;
        }
        Commands::Sla { action } => match action {
            SlaCommands::Record {
                id,
                uptime,
                latency,
                error_rate,
            } => {
                log::debug!(
                    "Command: sla record | id={} uptime={} latency={} error_rate={}",
                    id,
                    uptime,
                    latency,
                    error_rate
                );
                commands::sla_record(&id, uptime, latency, error_rate)?;
            }
            SlaCommands::Status { id } => {
                log::debug!("Command: sla status | id={}", id);
                commands::sla_status(&id)?;
            }
        },
        Commands::Config { action } => match action {
            ConfigSubcommands::Get {
                contract_id,
                environment,
            } => {
                commands::config_get(&cli.api_url, &contract_id, &environment).await?;
            }
            ConfigSubcommands::Set {
                contract_id,
                environment,
                config_data,
                secrets_data,
                created_by,
            } => {
                commands::config_set(
                    &cli.api_url,
                    &contract_id,
                    &environment,
                    &config_data,
                    secrets_data.as_deref(),
                    &created_by,
                )
                .await?;
            }
            ConfigSubcommands::History {
                contract_id,
                environment,
            } => {
                commands::config_history(&cli.api_url, &contract_id, &environment).await?;
            }
            ConfigSubcommands::Rollback {
                contract_id,
                environment,
                version,
                created_by,
            } => {
                commands::config_rollback(
                    &cli.api_url,
                    &contract_id,
                    &environment,
                    version,
                    &created_by,
                )
                .await?;
            }
        },
        Commands::VerifyFormal {
            contract_path,
            properties,
            output,
            post,
        } => {
            formal_verification::run(&cli.api_url, &contract_path, &properties, &output, post)
                .await?;
        }
        Commands::ScanDeps {
            contract_id,
            dependencies,
            fail_on_high,
        } => {
            commands::scan_deps(&cli.api_url, &contract_id, &dependencies, fail_on_high).await?;
        }
        Commands::Coverage {
            contract_path,
            tests,
            threshold,
            output,
        } => {
            coverage::run(&contract_path, &tests, threshold, &output).await?;
        }
        Commands::Sign {
            package,
            private_key,
            contract_id,
            version,
            expires_at,
        } => {
            log::debug!(
                "Command: sign | package={} contract_id={} version={}",
                package,
                contract_id,
                version
            );
            package_signing::sign_package(
                &cli.api_url,
                &package,
                &private_key,
                &contract_id,
                &version,
                expires_at.as_deref(),
            )
            .await?;
        }
        Commands::Verify {
            package,
            contract_id,
            version,
            signature,
        } => {
            log::debug!(
                "Command: verify | package={} contract_id={}",
                package,
                contract_id
            );
            package_signing::verify_package(
                &cli.api_url,
                &package,
                &contract_id,
                version.as_deref(),
                signature.as_deref(),
            )
            .await?;
        }
        Commands::VerifyContract {
            wasm_path,
            contract_id,
            version,
            signature,
            public_key,
        } => {
            log::debug!(
                "Command: verify-contract | wasm_path={} contract_id={} version={}",
                wasm_path,
                contract_id,
                version
            );
            package_signing::verify_contract_local(
                &wasm_path,
                &contract_id,
                &version,
                &signature,
                &public_key,
            )?;
        }
        Commands::Keys { action } => match action {
            KeysCommands::Generate {} => {
                log::debug!("Command: keys generate");
                package_signing::generate_keypair()?;
            }
            KeysCommands::Revoke {
                signature_id,
                revoked_by,
                reason,
            } => {
                log::debug!("Command: keys revoke | signature_id={}", signature_id);
                package_signing::revoke_signature(
                    &cli.api_url,
                    &signature_id,
                    &revoked_by,
                    &reason,
                )
                .await?;
            }
            KeysCommands::Custody { contract_id } => {
                log::debug!("Command: keys custody | contract_id={}", contract_id);
                package_signing::get_chain_of_custody(&cli.api_url, &contract_id).await?;
            }
            KeysCommands::Log {
                contract_id,
                entry_type,
                limit,
            } => {
                log::debug!("Command: keys log");
                package_signing::get_transparency_log(
                    &cli.api_url,
                    contract_id.as_deref(),
                    entry_type.as_deref(),
                    limit,
                )
                .await?;
            }
        },
        Commands::BatchVerify {
            contracts,
            initiated_by,
            json,
        } => {
            log::debug!(
                "Command: batch-verify | contracts={} initiated_by={}",
                contracts,
                initiated_by
            );
            batch_verify::run_batch_verify(&cli.api_url, &contracts, &initiated_by, json).await?;
        }
        Commands::Webhook { action } => match action {
            WebhookCommands::Create { url, events, secret } => {
                let event_list: Vec<String> =
                    events.split(',').map(|s| s.trim().to_string()).collect();
                log::debug!("Command: webhook create | url={} events={:?}", url, event_list);
                webhook::create_webhook(&cli.api_url, &url, event_list, secret.as_deref())
                    .await?;
            }
            WebhookCommands::List {} => {
                log::debug!("Command: webhook list");
                webhook::list_webhooks(&cli.api_url).await?;
            }
            WebhookCommands::Delete { webhook_id } => {
                log::debug!("Command: webhook delete | id={}", webhook_id);
                webhook::delete_webhook(&cli.api_url, &webhook_id).await?;
            }
            WebhookCommands::Test { webhook_id } => {
                log::debug!("Command: webhook test | id={}", webhook_id);
                webhook::test_webhook(&cli.api_url, &webhook_id).await?;
            }
            WebhookCommands::Logs { webhook_id, limit } => {
                log::debug!("Command: webhook logs | id={} limit={}", webhook_id, limit);
                webhook::webhook_logs(&cli.api_url, &webhook_id, limit).await?;
            }
            WebhookCommands::Retry { delivery_id } => {
                log::debug!("Command: webhook retry | delivery_id={}", delivery_id);
                webhook::retry_delivery(&cli.api_url, &delivery_id).await?;
            }
            WebhookCommands::VerifySig { secret, payload, signature } => {
                log::debug!("Command: webhook verify-sig");
                webhook::verify_signature_cmd(&secret, &payload, &signature)?;
            }
        },
        // ── Release Notes commands ───────────────────────────────────────────
        Commands::ReleaseNotes { action } => match action {
            ReleaseNotesCommands::Generate {
                contract_id,
                version,
                previous_version,
                changelog,
                contract_address,
                json,
            } => {
                log::debug!(
                    "Command: release-notes generate | contract_id={} version={}",
                    contract_id,
                    version
                );
                release_notes::generate(
                    &cli.api_url,
                    &contract_id,
                    &version,
                    previous_version.as_deref(),
                    changelog.as_deref(),
                    contract_address.as_deref(),
                    json,
                )
                .await?;
            }
            ReleaseNotesCommands::View {
                contract_id,
                version,
                json,
            } => {
                log::debug!(
                    "Command: release-notes view | contract_id={} version={}",
                    contract_id,
                    version
                );
                release_notes::view(&cli.api_url, &contract_id, &version, json).await?;
            }
            ReleaseNotesCommands::Edit {
                contract_id,
                version,
                file,
                text,
                json,
            } => {
                log::debug!(
                    "Command: release-notes edit | contract_id={} version={}",
                    contract_id,
                    version
                );
                release_notes::edit(
                    &cli.api_url,
                    &contract_id,
                    &version,
                    file.as_deref(),
                    text.as_deref(),
                    json,
                )
                .await?;
            }
            ReleaseNotesCommands::Publish {
                contract_id,
                version,
                skip_version_update,
                json,
            } => {
                log::debug!(
                    "Command: release-notes publish | contract_id={} version={}",
                    contract_id,
                    version
                );
                release_notes::publish(
                    &cli.api_url,
                    &contract_id,
                    &version,
                    skip_version_update,
                    json,
                )
                .await?;
            }
            ReleaseNotesCommands::List {
                contract_id,
                json,
            } => {
                log::debug!(
                    "Command: release-notes list | contract_id={}",
                    contract_id
                );
                release_notes::list(&cli.api_url, &contract_id, json).await?;
            }
        },
    }

    Ok(())
}
