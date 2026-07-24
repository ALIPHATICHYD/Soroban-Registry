#![allow(unused_variables)]

mod analytics;
mod analyze;
mod api_key;
mod auth;
mod audit_command;
mod backup;
mod batch_ops;
mod batch_audit;
mod batch_deploy;
mod batch_export;
mod batch_import;
mod batch_migrate;
mod batch_notify;
mod batch_register;
mod batch_update;
mod batch_verify;
mod cache;
mod cached_http;
mod cicd;
mod codegen;
mod commands;
mod compare;
mod completion;
mod config;
mod contract_dependency;
mod contract_deploy;
mod contract_highlight;
mod contract_interaction;
mod contract_register;
mod contract_risk;
mod contract_update;
mod contract_verify;
mod contracts;
mod conversions;
mod coverage;
mod dashboard;
mod deploy;
mod env;
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
mod net;
mod network;
mod notification;
mod package_signing;
mod patch;
mod plugins;
mod profiler;
mod release_notes;
mod shell;
mod sla;
mod table_format;
mod test_framework;
mod track_deployment;
mod upgrade;
mod user_config;
mod user_profile;
mod verification;
mod version;
mod webhook;
mod wizard;

mod diagnostic;
mod output_format;
mod search;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use colored::Colorize;
use patch::Severity;

/// Soroban Registry CLI — discover, publish, verify, and deploy Soroban contracts
#[derive(Debug, Parser)]
#[command(name = "soroban-registry", version, about, long_about = None)]
pub struct Cli {
    /// Registry API URL
    #[arg(long, global = true, default_value = "")]
    pub api_url: String,

    /// Stellar network to use (mainnet | testnet | futurenet)
    #[arg(long, global = true)]
    pub network: Option<String>,

    /// Global timeout for network/API operations (seconds)
    #[arg(long, global = true)]
    pub timeout: Option<u64>,

    /// Registry configuration profile to use
    #[arg(long, global = true)]
    pub profile: Option<String>,

    /// Skip local response cache and always fetch fresh data
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Enable verbose output. Repeat to increase verbosity (-v, -vv, -vvv).
    #[arg(
        long,
        short = 'v',
        global = true,
        action = ArgAction::Count,
        long_help = "Enable verbose output. Repeat the flag to raise the log level:\n  \
                     (none)  warn   — errors and warnings only (default)\n  \
                     -v      info   — high-level operations\n  \
                     -vv     debug  — HTTP requests, responses, and timing\n  \
                     -vvv+   trace  — full internal tracing"
    )]
    pub verbose: u8,

    /// Check for CLI updates before running the command.
    #[arg(long, global = true)]
    pub check_updates: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Query contract analytics and statistics
    Analytics {
        /// Query type: top-contracts, trending, by-category, by-network
        query: String,
        /// Time period: 7d, 30d, 90d, or RFC3339 range start..end
        #[arg(long, default_value = "30d")]
        period: String,
        /// Output format: table, json, csv, yaml
        #[arg(long, default_value = "table")]
        format: String,
        /// Sort mode: value_desc, value_asc, key_asc, key_desc
        #[arg(long)]
        sort: Option<String>,
        /// Export output to a file
        #[arg(long)]
        export: Option<String>,
    },

    /// Get comprehensive registry statistics
    Stats {
        /// Timeframe: 7d, 30d, or all (default: all)
        #[arg(long, default_value = "all")]
        timeframe: String,
        /// Output format: table, json, yaml
        #[arg(long, default_value = "table")]
        format: String,
        /// Export to file
        #[arg(long)]
        output: Option<String>,
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

        /// Path to contract project directory for preflight testing
        #[arg(long, default_value = ".")]
        contract_path: String,

        /// Custom test command to run before submission
        #[arg(long)]
        test_command: Option<String>,

        /// Require coverage data and fail if unavailable
        #[arg(long)]
        require_coverage: bool,

        /// Minimum required coverage percentage (0-100)
        #[arg(long, default_value_t = 0.0)]
        coverage_threshold: f64,

        /// Skip pre-submission contract tests
        #[arg(long)]
        skip_tests: bool,
    },

    /// List contracts in the registry
    List {
        /// Max number of contracts to list
        #[arg(long, short, default_value = "20")]
        limit: usize,

        /// Number of contracts to skip
        #[arg(long, short, default_value = "0")]
        offset: usize,

        /// Filter by network (comma-separated: mainnet,testnet,futurenet). The
        /// field is `networks`, not `network`: clap derives an arg id from the
        /// field name, and the global `--network` (global = true) shares that
        /// id, so a subcommand-local `network` field would collide with it and
        /// panic trying to downcast the matched value.
        #[arg(long)]
        networks: Option<String>,

        /// Filter by category (comma-separated for multiple: DeFi,NFT)
        #[arg(long, short)]
        category: Option<String>,

        /// Output format (table, json, csv, yaml)
        #[arg(long, short, default_value = "table")]
        format: String,
    },

    /// Show detailed info for a specific contract
    Info {
        /// Contract ID or slug
        id: String,

        #[arg(long)]
        json: bool,

        #[arg(long)]
        raw: bool,
    },

    /// Search for contracts in the registry
    Search {
        /// Search query
        query: String,

        /// Only show verified contracts
        #[arg(long)]
        verified_only: bool,

        /// Filter by network (comma-separated: mainnet,testnet,futurenet).
        /// The field is `networks`, not `network`: clap derives an arg id from
        /// the field name, and the global `--network` (global = true) shares
        /// that id, so clap would populate both from either flag.
        #[arg(long)]
        networks: Option<String>,

        /// Filter by category (comma-separated: DeFi,NFT)
        #[arg(long)]
        category: Option<String>,

        /// Sort by (name, created, updated, relevance)
        #[arg(long)]
        sort: Option<String>,

        /// Maximum results to return
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Results offset
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Compare multiple contracts
    Compare {
        /// Contract IDs to compare (2 to 4 contracts)
        #[arg(required = true, num_args = 2..=4)]
        ids: Vec<String>,

        /// Output detailed comparison as JSON
        #[arg(long)]
        json: bool,

        /// Export comparison report to a file (csv or json)
        #[arg(long)]
        export: Option<String>,

        /// Export format (csv or json). Derived from file extension if not provided.
        #[arg(long)]
        format: Option<String>,

        /// Exit with code 1 when differences are found (0 = identical, 2 = error)
        #[arg(long)]
        exit_code: bool,

        /// Diff output format: none, unified, side-by-side
        #[arg(long, default_value = "none")]
        diff: String,

        /// Limit compared field groups (metadata,verification,deployment,abi,all)
        #[arg(long, value_delimiter = ',')]
        fields: Option<Vec<String>>,
    },

    /// Generate shell completion scripts (#971)
    Completion {
        /// Target shell
        #[arg(value_enum)]
        shell: completion::CompletionShell,
    },

    /// Check CLI version and update availability
    Version {
        /// Check upstream for newer versions
        #[arg(long, default_value_t = true)]
        check_updates: bool,
        /// Print update instructions immediately when newer version exists
        #[arg(long, default_value_t = false)]
        auto_update: bool,
        /// Roll back to a previous version (manual install helper)
        #[arg(long)]
        rollback: Option<String>,
    },

    /// Launch an interactive, real-time terminal dashboard
    Dashboard {
        /// Minimum interval between UI renders (milliseconds)
        #[arg(long, default_value = "100")]
        refresh_rate: u64,
        /// Filter by contract category
        #[arg(long)]
        category: Option<String>,
        /// WebSocket URL (or set SOROBAN_REGISTRY_WS_URL)
        #[arg(long, env = "SOROBAN_REGISTRY_WS_URL")]
        ws_url: Option<String>,
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

    /// Export contract registry data or a contract archive
    Export {
        /// Contract registry ID (UUID or on-chain address). Omit to export a filtered contract list.
        #[arg(long)]
        id: Option<String>,

        /// Output file path. Defaults to contracts-export.<format> or contract-export.tar.gz for archive.
        #[arg(long, short = 'o')]
        output: Option<String>,

        /// Path to contract source directory
        #[arg(long, default_value = ".")]
        contract_dir: String,

        /// Export format: json, csv, markdown, or archive
        #[arg(long, short = 'f')]
        format: Option<String>,

        /// Filter to apply to registry exports, e.g. --filter network=mainnet --filter verified_only=true
        #[arg(long = "filter")]
        filters: Vec<String>,

        /// Number of contracts to fetch per API page for list exports
        #[arg(long, default_value_t = 100)]
        page_size: usize,
    },

    /// Import contract data from a file (JSON, CSV, or Archive)
    Import {
        /// Path to the import file
        file: String,

        /// Format of the file (json | csv | archive). If omitted, inferred from extension.
        #[arg(long)]
        format: Option<String>,

        /// Directory to extract into (only for archive format)
        #[arg(long, default_value = "./imported")]
        output_dir: String,

        /// Validate the data before importing
        #[arg(long)]
        validate: bool,

        /// Perform a dry run without actually importing
        #[arg(long)]
        dry_run: bool,
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

    /// Start an interactive contract deployment workflow
    Deploy {},

    /// Manage contract semantic versions
    #[command(name = "versions")]
    VersionSemver {
        #[command(subcommand)]
        action: VersionCommands,
    },

    /// Perform batch operations on multiple contracts
    Batch {
        /// Operation: tag, categorize, verify, deprecate
        operation: String,
        /// Contract IDs
        contracts: Vec<String>,
        /// Optional file containing contract IDs (one per line)
        #[arg(long)]
        file: Option<String>,
        /// Optional operation value (required for tag/categorize)
        #[arg(long)]
        value: Option<String>,
        /// Roll back already-applied operations when any item fails
        #[arg(long)]
        rollback_on_error: bool,
        /// Recipients file/filter for `batch notify`
        #[arg(long)]
        recipients: Option<String>,
        /// Message type for `batch notify`
        #[arg(long, default_value = "info")]
        message_type: String,
        /// Template file or inline template for `batch notify`
        #[arg(long)]
        template: Option<String>,
        /// Preview notification/migration without sending/writing
        #[arg(long)]
        preview: bool,
        /// RFC3339 schedule for `batch notify`
        #[arg(long)]
        schedule: Option<String>,
        /// Channels for `batch notify`: email,in-app,webhook
        #[arg(long, value_delimiter = ',')]
        channels: Vec<String>,
        /// Filter expression for `batch migrate`
        #[arg(long)]
        filter: Option<String>,
        /// Use atomic/fail-safe migration semantics
        #[arg(long)]
        atomic: bool,
        /// Migration report output path
        #[arg(long)]
        report: Option<String>,

        /// Output JSON summary
        #[arg(long)]
        json: bool,
    },

    /// Manage contract upgrades and rollbacks
    Upgrade {
        #[command(subcommand)]
        action: UpgradeSubcommands,
    },

    /// Launch the interactive setup wizard
    Wizard {},

    /// Enter interactive REPL mode
    #[command(alias = "shell")]
    Repl {
        /// Initial network
        #[arg(long)]
        network: Option<String>,
    },

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

    /// Perf contract execution performance
    #[command(name = "perf")]
    Perf {
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

    /// Manage your user profile and publishing preferences (#841)
    Profile {
        #[command(subcommand)]
        action: ProfileCommands,
    },

    /// Run integration tests
    Test {
        /// Optional path to scenario test file (YAML or JSON)
        ///
        /// If omitted, auto-detects and runs contract project tests.
        test_file: Option<String>,

        /// Path to contract directory or file
        #[arg(long)]
        contract_path: Option<String>,

        /// Custom test command (for auto-detected project tests mode)
        #[arg(long)]
        test_command: Option<String>,

        /// Output JUnit XML report
        #[arg(long)]
        junit: Option<String>,

        /// Show coverage report
        #[arg(long, default_value = "true")]
        coverage: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,

        /// Require coverage data and fail if unavailable
        #[arg(long)]
        require_coverage: bool,

        /// Minimum required coverage percentage (0-100)
        #[arg(long, default_value_t = 0.0)]
        coverage_threshold: f64,

        /// Optional shell command to run before executing tests
        #[arg(long)]
        setup_hook: Option<String>,

        /// Optional shell command to run after executing tests
        #[arg(long)]
        teardown_hook: Option<String>,

        /// Optional JSON or YAML file describing mock services used in the run
        #[arg(long)]
        mock_config: Option<String>,

        /// Optional JSON report output for the full test session
        #[arg(long)]
        report: Option<String>,

        /// Optional JSON profile output for load-test metadata
        #[arg(long)]
        profile_output: Option<String>,

        /// Number of iterations to simulate for load testing
        #[arg(long, default_value_t = 1)]
        load_iterations: u32,
    },

    /// Run a local contract security audit
    Audit {
        /// Path to contract file or project directory
        contract_path: String,

        /// Output format: text, json, markdown
        #[arg(long, default_value = "text")]
        format: String,

        /// Optional report output file
        #[arg(long, short = 'o')]
        output: Option<String>,

        /// Fail the command when findings at or above this severity are present
        #[arg(long)]
        fail_on: Option<String>,
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

    /// Manage authentication sessions and API tokens
    Auth {
        #[command(subcommand)]
        action: AuthCommands,
    },

    /// Manage contract backups and disaster recovery
    Backup {
        #[command(subcommand)]
        action: BackupCommands,
    },

    /// Inspect and modify contract state (dev/test mutation only)
    State {
        #[command(subcommand)]
        action: StateSubcommands,
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
    VerifyPackage {
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

    /// Verify a contract in the registry (check status, submit for audit, or show history)
    Verify {
        /// Contract UUID or on-chain address
        #[arg(required_unless_present_any = ["history", "check"])]
        id: Option<String>,

        /// Submit for verification (requires id or local project)
        #[arg(long, short = 's')]
        submit: bool,

        /// Check current verification status
        #[arg(long, short = 'c')]
        check: bool,

        /// Show verification history
        #[arg(long)]
        history: bool,

        /// Verification level: basic, intermediate, advanced
        #[arg(long, default_value = "basic")]
        level: String,

        /// Output results as JSON
        #[arg(long, short = 'j')]
        json: bool,

        /// Path to contract project directory (defaults to current dir)
        #[arg(long, default_value = ".")]
        path: String,

        /// Optional notes for submission
        #[arg(long)]
        notes: Option<String>,
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

    /// Contract deployment verification and security scan (#522)
    Contract {
        #[command(subcommand)]
        action: ContractCommands,
    },

    /// Manage API keys for programmatic access (#842)
    #[command(name = "api-key")]
    ApiKey {
        #[command(subcommand)]
        action: ApiKeyCommands,
    },

    /// Verify multiple contracts in a bulk batch (#850)
    BatchVerify {
        /// Path to a contract list file (.txt one-ID-per-line, .json, or .yaml)
        #[arg(long)]
        file: Option<String>,

        /// Comma-separated IDs — fallback when --file is absent
        #[arg(long)]
        contracts: Option<String>,

        /// Filter by network when discovering from API (mainnet|testnet|futurenet)
        #[arg(long)]
        network: Option<String>,

        /// Filter by category when discovering from API (e.g. defi, nft)
        #[arg(long)]
        category: Option<String>,

        /// Only include contracts created within this many days
        #[arg(long)]
        age: Option<u32>,

        /// Stellar address or username initiating the batch
        #[arg(long)]
        initiated_by: String,

        /// Verification depth: basic | standard | strict
        #[arg(long, default_value = "standard")]
        level: String,

        /// Export report to file; format inferred from extension (.json or .csv)
        #[arg(long)]
        export: Option<String>,

        /// Save human-readable report to a text file
        #[arg(long)]
        output: Option<String>,

        /// Save cron schedule and print crontab entry
        #[arg(long)]
        schedule: Option<String>,

        /// Output machine-readable JSON
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

    /// CI/CD pipeline integration and automation
    Cicd {
        #[command(subcommand)]
        action: CicdCommands,
    },

    /// Check the status of supported Stellar networks
    Network {
        #[command(subcommand)]
        action: NetworkCommands,
    },

    /// Register multiple contracts from a YAML or JSON manifest file
    BatchRegister {
        /// Path to the manifest file (.yaml, .yml, or .json)
        #[arg(long)]
        manifest: String,

        /// Publisher Stellar address (overrides `publisher` field in the manifest)
        #[arg(long)]
        publisher: Option<String>,

        /// Validate all entries and show what would be registered without submitting
        #[arg(long)]
        dry_run: bool,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Audit multiple contracts in batch for security and best practices
    BatchAudit {
        /// File containing contract paths (one per line) or comma-separated paths
        file: String,
        /// Report format: text, json, markdown
        #[arg(long, default_value = "text")]
        format: String,
        /// Output directory for generated reports
        #[arg(long)]
        output_dir: Option<String>,
        /// Fail on findings at or above this severity
        #[arg(long)]
        fail_on: Option<String>,
        /// Show only high and critical findings
        #[arg(long)]
        high_risk: bool,
        /// Audit profile: basic, standard, comprehensive
        #[arg(long, default_value = "standard")]
        profile: String,
        /// Export audit findings to a file
        #[arg(long)]
        export: Option<String>,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Deploy a contract WASM to multiple networks
    BatchDeploy {
        /// Path to the WASM file
        wasm_file: String,
        /// Comma-separated target networks (mainnet,testnet,futurenet)
        #[arg(long, default_value = "testnet")]
        networks: String,
        /// Signer Stellar address or secret
        #[arg(long)]
        signer: String,
        /// Stop and report failure if any deployment fails (no on-chain rollback)
        #[arg(long)]
        atomic: bool,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Export multiple contracts in bulk
    BatchExport {
        /// Output directory for exported files
        output_dir: String,
        /// Filter query (e.g. network=testnet or category=defi)
        #[arg(long)]
        filter: Option<String>,
        /// Output format: json, csv, archive
        #[arg(long, default_value = "json")]
        format: String,
        /// Organize output by network/category subdirectories
        #[arg(long)]
        organize: bool,
        /// Compress the output directory into a .tar.gz
        #[arg(long)]
        compress: bool,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Import contracts in bulk from a directory
    BatchImport {
        /// Input directory containing contract files to import
        input_dir: String,
        /// Force a specific format (json, csv, archive); auto-detected if omitted
        #[arg(long)]
        format: Option<String>,
        /// How to handle duplicates: skip or fail
        #[arg(long, default_value = "skip")]
        on_duplicate: String,
        /// Preview what would be imported without committing
        #[arg(long)]
        dry_run: bool,
        /// Abort on first error; report atomically
        #[arg(long)]
        atomic: bool,
        /// Output directory for archive imports
        #[arg(long, default_value = "./imported")]
        output_dir: String,
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Update metadata for multiple contracts in bulk (#849)
    BatchUpdate {
        /// Path to a YAML or JSON manifest file describing the updates
        #[arg(long)]
        file: Option<String>,

        /// Filter contracts from the API (e.g. "category=defi" or "network=mainnet")
        #[arg(long)]
        filter: Option<String>,

        /// Show what would change without making any writes
        #[arg(long)]
        preview: bool,

        /// Only update contracts where this field=value condition is true
        #[arg(long, value_name = "CONDITION")]
        r#if: Option<String>,

        /// User ID to attribute the update to
        #[arg(long)]
        user_id: Option<String>,

        /// On partial failure, rollback all successfully applied contracts
        #[arg(long)]
        rollback_on_error: bool,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Run advanced analysis on a deployed contract (#530)
    Analyze {
        /// On-chain contract ID to analyse
        contract_id: String,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "testnet")]
        network: String,

        /// Report format: text (default), json, yaml
        #[arg(long, default_value = "text")]
        report_format: String,

        /// Write the report to a file instead of stdout
        #[arg(long, short = 'o')]
        output: Option<String>,
    },

    /// Track contract deployment status until confirmed or timeout (#524)
    TrackDeployment {
        /// On-chain contract ID
        #[arg(long)]
        contract_id: String,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "testnet")]
        network: String,

        /// Optional transaction hash to track (polls transaction endpoints first)
        #[arg(long)]
        tx_hash: Option<String>,

        /// Maximum wait time in seconds before exiting with code 2
        #[arg(long, default_value_t = 60)]
        wait_timeout: u64,

        /// Output machine-readable JSON status
        #[arg(long)]
        json: bool,
    },

    /// Plugin management (install, configure, run)
    Plugins {
        #[command(subcommand)]
        action: PluginCommands,
    },

    /// Manage local cache of registry API responses (#845)
    Cache {
        #[command(subcommand)]
        action: CacheCommands,
    },
    /// Manage environment variable sets for different deployments (#843)
    Env {
        #[command(subcommand)]
        action: EnvCommands,
    },

    /// External command (may be provided by an installed plugin)
    #[command(external_subcommand)]
    External(Vec<String>),
}

/// Sub-commands for the `network` group
#[derive(Debug, Subcommand)]
pub enum NetworkCommands {
    /// Show status of all supported Stellar networks
    Status {
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
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

/// Sub-commands for the `cicd` group
#[derive(Debug, Subcommand)]
pub enum CicdCommands {
    /// Run a full CI/CD pipeline (validate, scan, build, publish, verify)
    Run {
        /// Path to contract directory
        #[arg(long, default_value = ".")]
        contract_path: String,

        /// Network to target (testnet|mainnet)
        #[arg(long, default_value = "testnet")]
        network: String,

        /// Skip security scans
        #[arg(long)]
        skip_scan: bool,

        /// Auto-register contract if not found in registry
        #[arg(long, default_value_t = true)]
        auto_register: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate the current environment for CI/CD integration
    Validate {
        /// Path to contract directory
        #[arg(long, default_value = ".")]
        contract_path: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommands {
    /// Get a user config value by key
    #[command(name = "get")]
    UserGet { key: String },
    /// Set a user config value by key
    #[command(name = "set")]
    UserSet { key: String, value: String },
    /// List all persisted user config values
    #[command(name = "list")]
    UserList {},
    /// Reset user config to defaults
    #[command(name = "reset")]
    UserReset {},

    /// Get contract environment configuration
    #[command(name = "contract-get")]
    ContractGet {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
    },
    /// Set contract environment configuration
    #[command(name = "contract-set")]
    ContractSet {
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
    /// Show contract config history
    #[command(name = "contract-history")]
    ContractHistory {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        environment: String,
    },
    /// Roll back contract config to a previous version
    #[command(name = "contract-rollback")]
    ContractRollback {
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

#[derive(Debug, Subcommand)]
pub enum AuthCommands {
    /// Sign in with a GitHub account, Stellar wallet, or API key
    Login {
        /// Authentication method to use
        #[arg(long, value_enum)]
        method: Option<crate::auth::AuthMethod>,

        /// Identity to authenticate with
        #[arg(long)]
        identity: Option<String>,

        /// Secret credential or signing seed
        #[arg(long)]
        secret: Option<String>,

        /// Comma-separated token scopes
        #[arg(long, value_delimiter = ',')]
        scopes: Vec<String>,

        /// Token lifetime, e.g. 1h, 30m, 7d, or seconds
        #[arg(long)]
        expires: Option<String>,
    },

    /// Sign out and remove stored credentials
    Logout {},

    /// Show the current authentication state
    Status {},

    /// Print the current API token, refreshing it when possible
    Token {
        /// Comma-separated token scopes
        #[arg(long, value_delimiter = ',')]
        scopes: Vec<String>,

        /// Token lifetime, e.g. 1h, 30m, 7d, or seconds
        #[arg(long)]
        expires: Option<String>,
    },
}

/// Sub-commands for the `backup` group
#[derive(Debug, Subcommand)]
pub enum BackupCommands {
    /// Create a new contract backup
    Create {
        /// Contract ID to back up
        contract_id: String,
        /// Include full contract state in backup
        #[arg(long)]
        include_state: bool,
    },
    /// List recent backups for a contract
    List {
        /// Contract ID
        contract_id: String,
    },
    /// Restore a contract from a specific backup date
    Restore {
        /// Contract ID to restore
        contract_id: String,
        /// Backup date to restore from (YYYY-MM-DD)
        backup_date: String,
    },
    /// Verify integrity of a specific backup
    Verify {
        /// Contract ID
        contract_id: String,
        /// Backup date to verify (YYYY-MM-DD)
        backup_date: String,
    },
    /// Show backup statistics for a contract
    Stats {
        /// Contract ID
        contract_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum StateSubcommands {
    /// Get a single state value by key
    Get {
        /// Contract identifier
        contract_id: String,
        /// State key
        key: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Set a state key/value (testnet and futurenet only)
    Set {
        /// Contract identifier
        contract_id: String,
        /// State key
        key: String,
        /// New value (JSON is parsed, otherwise stored as string)
        value: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Dump full contract state
    Dump {
        /// Contract identifier
        contract_id: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Create a state snapshot
    Snapshot {
        /// Contract identifier
        contract_id: String,
        /// Optional label for the snapshot
        #[arg(long)]
        label: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List saved state snapshots
    Snapshots {
        /// Contract identifier
        contract_id: String,
        /// Maximum number of snapshots to return
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Browse state change history
    History {
        /// Contract identifier
        contract_id: String,
        /// Filter by key
        #[arg(long)]
        key: Option<String>,
        /// Maximum number of entries to return
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Sub-commands for the `plugins` group
#[derive(Debug, Subcommand)]
pub enum PluginCommands {
    /// List installed plugins and their commands
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Browse the registry marketplace
    Marketplace {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Install a plugin from the registry
    Install {
        /// Plugin name
        name: String,
        /// Optional version (defaults to marketplace version)
        #[arg(long)]
        version: Option<String>,
    },

    /// Uninstall an installed plugin
    Uninstall {
        /// Plugin name
        name: String,
        /// Optional version (defaults to removing all versions)
        #[arg(long)]
        version: Option<String>,
    },

    /// Run a plugin-provided command explicitly
    Run {
        /// The plugin command name
        command: String,
        /// Arguments passed to the plugin command
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Enable/disable plugins and set per-plugin configuration
    Config {
        #[command(subcommand)]
        action: PluginConfigCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum PluginConfigCommands {
    /// Get the current JSON config for a plugin
    Get {
        /// Plugin name
        name: String,
    },

    /// Replace the plugin JSON config (must be a JSON object)
    Set {
        /// Plugin name
        name: String,
        /// JSON object
        #[arg(long)]
        json: String,
    },

    /// Disable a plugin (commands won't be discovered)
    Disable {
        /// Plugin name
        name: String,
    },

    /// Enable a plugin (default)
    Enable {
        /// Plugin name
        name: String,
    },
}

/// Sub-commands for the `contracts` group
#[derive(Debug, Subcommand)]
pub enum ContractsCommands {
    /// List contracts with filtering and pagination
    List {
        /// Filter by network (mainnet, testnet, futurenet)
        #[arg(long)]
        network: Option<String>,

        /// Filter by category (e.g., DEX, token, lending, oracle)
        #[arg(long)]
        category: Option<String>,

        /// Maximum number of contracts to return
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Number of contracts to skip (for pagination)
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Sort by field: name, created_at, health_score, network
        #[arg(long, default_value = "created_at")]
        sort_by: String,

        /// Sort order: asc or desc
        #[arg(long, default_value = "desc")]
        sort_order: String,

        /// Output format: table, json, or csv
        #[arg(long, default_value = "table")]
        format: String,

        /// Output results as JSON (shorthand for --format json)
        #[arg(long)]
        json: bool,

        /// Output results as CSV (shorthand for --format csv)
        #[arg(long)]
        csv: bool,
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

/// Sub-commands for the `contract` group (#522)
#[derive(Debug, Subcommand)]
pub enum ContractCommands {

    /// Assess security and operational risks for a contract (#837)
    ///
    /// Usage: soroban-registry contract risk <address> [--network <n>] [--threshold <level>] [--json]
    Risk {
        /// On-chain contract address or registry UUID to assess
        address: String,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "mainnet")]
        network: String,

        /// Exit with code 1 if overall risk level meets or exceeds this threshold
        /// (low | medium | high | critical)
        #[arg(long)]
        threshold: Option<String>,

        /// Output the risk report as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Deploy and register a new contract in the registry
    ///
    /// Usage: soroban-registry contract deploy <WASM_PATH> --name <NAME> --network <NETWORK>
    ///        [--description <DESC>] [--category <CAT>] [--icon <ICON_PATH>]
    ///        [--interactive] [--publisher <ADDRESS>] [--tags <TAGS>]
    Deploy {
        /// Path to the WASM binary file
        wasm_path: String,

        /// Contract name (human-readable)
        #[arg(long)]
        name: Option<String>,

        /// Contract description
        #[arg(long)]
        description: Option<String>,

        /// Contract category (DeFi, Token, Oracle, NFT, Utility, Other)
        #[arg(long)]
        category: Option<String>,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "testnet")]
        network: String,

        /// Path to contract icon file (PNG, JPG, SVG)
        #[arg(long)]
        icon: Option<String>,

        /// Enable interactive mode for guided deployment
        #[arg(long)]
        interactive: bool,

        /// Publisher's Stellar address (if not set, uses default publisher)
        #[arg(long)]
        publisher: Option<String>,

        /// Comma-separated list of tags for the contract
        #[arg(long)]
        tags: Option<String>,

        /// Skip ABI extraction and deployment verification
        #[arg(long)]
        skip_abi: bool,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Register one or more contracts in the registry
    Register {
        /// Path to a YAML or JSON metadata file
        #[arg(long)]
        file: Option<String>,

        /// Enable repeated prompts for multiple contracts
        #[arg(long)]
        batch: bool,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Verify a deployed contract's authenticity against the on-chain registry
    ///
    /// Usage: soroban-registry contract verify <address> --network <network> [--json] [--strict] [--batch] [--no-cache]
    Verify {
        /// On-chain contract address to verify (or comma-separated list for batch verification)
        address: String,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "mainnet")]
        network: String,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,

        /// Strict mode: fail if any warnings or errors are found
        #[arg(long)]
        strict: bool,

        /// Batch mode: verify multiple contracts (comma-separated addresses)
        #[arg(long)]
        batch: bool,

        /// Skip cache and always fetch fresh data from registry
        #[arg(long)]
        no_cache: bool,
    },

    /// Display detailed information about a contract
    ///
    /// Usage: soroban-registry contract details <address> --network <network> [--json]
    Details {
        /// On-chain contract address to inspect
        address: String,

        /// Stellar network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "mainnet")]
        network: String,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Show contract registry statistics and analytics
    ///
    /// Usage: soroban-registry contract stats [--network testnet] [--category defi]
    Stats {
        /// Filter stats by network
        #[arg(long)]
        network: Option<String>,

        /// Filter stats by category
        #[arg(long)]
        category: Option<String>,

        /// Number of popular contracts to display
        #[arg(long, default_value_t = 10)]
        top_n: usize,

        /// Output format: table, json, csv, yaml
        #[arg(long, default_value = "table")]
        format: String,

        /// Export stats to a file
        #[arg(long, short = 'o')]
        output: Option<String>,

        /// Compare against another period, for example 7d or 30d
        #[arg(long)]
        compare: Option<String>,
    },

    /// Export contracts and related registry data for backup or migration
    ///
    /// Usage: soroban-registry contract export [OUTPUT_FILE] --format json
    Export {
        /// Optional output file path
        output_file: Option<String>,

        /// Output file path
        #[arg(long, short = 'o')]
        output: Option<String>,

        /// Export format: json, csv, jsonl, sqlite, markdown, or archive
        #[arg(long, short = 'f', default_value = "json")]
        format: String,

        /// Filter by network
        #[arg(long)]
        network: Option<String>,

        /// Filter by category
        #[arg(long)]
        category: Option<String>,

        /// Export only contracts updated since this date
        #[arg(long)]
        since: Option<String>,

        /// Write a gzip-compressed export file
        #[arg(long)]
        compress: bool,

        /// Include related data such as versions, dependencies, analytics, and reviews
        #[arg(long, default_value_t = true)]
        include_related: bool,

        /// Number of contracts to fetch per API page
        #[arg(long, default_value_t = 100)]
        page_size: usize,
    },
    /// Manage featured (highlighted) contracts (#832)
    ///
    /// Usage: soroban-registry contract highlight [ADDRESS] --action <add|remove|list|check>
    Highlight {
        /// Contract address (required for add/remove/check)
        address: Option<String>,
        /// Action to perform: add | remove | list | check
        #[arg(long, default_value = "list")]
        action: String,
        /// Curator bearer token for mutating actions (add/remove)
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// View a contract's interactions and call patterns (#835)
    Interaction {
        /// On-chain contract address
        address: String,
        /// Max number of recent interactions to display
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        json: bool,
    },

    /// Analyze a contract's dependencies and relationships (#836, #1008)
    ///
    /// Retrieves the full dependency graph: contracts this address depends on,
    /// contracts that depend on it, and a recursive dependency tree.
    ///
    /// Use `--summary` for a compact view when dealing with large graphs.
    /// Use `--format json` to get the raw API response for scripting.
    Dependency {
        /// On-chain contract address
        address: String,
        /// Dependency tree depth (0 = direct dependencies only)
        #[arg(long, default_value_t = 1)]
        depth: u32,
        /// Output format: table, json, csv, yaml
        #[arg(long, default_value = "table")]
        format: String,
        /// Compact summary mode: show aggregate counts without the full tree
        #[arg(long)]
        summary: bool,
    },

    /// Update contract metadata after registration (#828)
    ///
    /// Usage: soroban-registry contract update <ADDRESS> [--name ...] [--dry-run]
    Update {
        /// Contract address, slug, or registry UUID
        address: String,

        /// Updated contract name
        #[arg(long)]
        name: Option<String>,

        /// Updated description
        #[arg(long)]
        description: Option<String>,

        /// Updated category
        #[arg(long)]
        category: Option<String>,

        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,

        /// Path to a new icon image (PNG, JPG, or SVG)
        #[arg(long)]
        icon: Option<String>,

        /// Contract homepage URL (not yet supported by registry API)
        #[arg(long)]
        homepage: Option<String>,

        /// Preview changes without submitting them
        #[arg(long)]
        dry_run: bool,

        /// Skip interactive confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Import contracts into the registry from an external file (#831)
    ///
    /// Supports JSON, JSONL (newline-delimited JSON), CSV, and archive formats.
    ///
    /// Usage: soroban-registry contract import <INPUT_FILE> [OPTIONS]
    Import {
        /// Path to the input file (JSON, JSONL, CSV, or .tar.gz archive)
        input_file: String,

        /// Input format override (json | jsonl | csv | sqlite | archive).
        /// Inferred from the file extension when omitted.
        #[arg(long, short = 'f')]
        format: Option<String>,

        /// How to handle duplicate contracts: skip | update | fail (default: skip)
        #[arg(long, default_value = "skip")]
        on_duplicate: String,

        /// Network alias mappings, e.g. --network-map futurenet=testnet
        /// May be repeated for multiple aliases.
        #[arg(long = "network-map")]
        network_map: Vec<String>,

        /// Preview what would be imported without writing to the registry
        #[arg(long)]
        dry_run: bool,

        /// Validate all records before importing; abort on any error
        #[arg(long)]
        validate: bool,

        /// Roll back all successful imports if any record fails
        #[arg(long)]
        atomic: bool,

        /// Write the JSON import-summary report to this file path
        /// (prints to stdout when omitted)
        #[arg(long, short = 'o')]
        report_output: Option<String>,

        /// Directory for archive extraction (archive format only)
        #[arg(long, default_value = "./imported")]
        output_dir: String,
    },
}

/// Sub-commands for the `api-key` group (#842)
#[derive(Debug, Subcommand)]
pub enum ApiKeyCommands {
    /// Create a new API key
    Create {
        /// Expiry (ISO date or duration, e.g. 2026-12-31 or 30d)
        #[arg(long)]
        expires: Option<String>,
        /// Comma-separated scopes / permissions
        #[arg(long)]
        scopes: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List your API keys
    List {
        #[arg(long)]
        json: bool,
    },
    /// Permanently delete an API key
    Delete {
        /// API key id
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Revoke (disable) an API key without deleting its audit record
    Revoke {
        /// API key id
        id: String,
        #[arg(long)]
        json: bool,
    },
}

/// Sub-commands for the `env` group (#843)
#[derive(Debug, Subcommand)]
pub enum EnvCommands {
    /// Set a variable in an environment
    ///
    /// Usage: soroban-registry env set <NAME> <VALUE> [--env <environment>]
    Set {
        /// Variable name (shell identifier: letters, digits, underscores)
        name: String,
        /// Value to assign
        value: String,
        /// Target environment (defaults to the active environment)
        #[arg(long)]
        env: Option<String>,
        /// Print the full value instead of masking it
        #[arg(long)]
        show_value: bool,
    },

    /// Get a variable's value from an environment
    ///
    /// Usage: soroban-registry env get <NAME> [--env <environment>] [--json]
    Get {
        /// Variable name to look up
        name: String,
        /// Source environment (defaults to the active environment)
        #[arg(long)]
        env: Option<String>,
        /// Output as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// List variables in an environment
    ///
    /// Usage: soroban-registry env list [--env <environment>] [--all] [--merged] [--json]
    List {
        /// Environment to list (defaults to the active environment)
        #[arg(long)]
        env: Option<String>,
        /// List variables in every environment
        #[arg(long)]
        all: bool,
        /// Merge global config defaults into the output
        #[arg(long)]
        merged: bool,
        /// Output as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Copy all variables from one environment to another
    ///
    /// Usage: soroban-registry env copy --from <src> --to <dst>
    Copy {
        /// Source environment name
        #[arg(long)]
        from: String,
        /// Destination environment name
        #[arg(long)]
        to: String,
        /// Overwrite the destination if it already exists
        #[arg(long)]
        overwrite: bool,
    },

    /// Delete a variable from an environment
    ///
    /// Usage: soroban-registry env delete <NAME> [--env <environment>]
    Delete {
        /// Variable name to remove
        name: String,
        /// Source environment (defaults to the active environment)
        #[arg(long)]
        env: Option<String>,
    },

    /// Export environment variables as a shell-sourceable file
    ///
    /// Usage: soroban-registry env export [--env <environment>] [--format shell|json|dotenv]
    Export {
        /// Environment to export (defaults to the active environment)
        #[arg(long)]
        env: Option<String>,
        /// Output format: shell (default), json, dotenv
        #[arg(long, value_enum, default_value_t = EnvExportFormat::Shell)]
        format: EnvExportFormat,
        /// Merge global config defaults into the export
        #[arg(long)]
        merged: bool,
    },

    /// Switch the active environment
    ///
    /// Usage: soroban-registry env switch <ENVIRONMENT>
    Switch {
        /// Environment name to activate
        environment: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum EnvExportFormat {
    Shell,
    Json,
    Dotenv,
}

impl EnvExportFormat {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::Json => "json",
            Self::Dotenv => "dotenv",
        }
    }
}

/// Sub-commands for the `cache` group (#845)
#[derive(Debug, Subcommand)]
pub enum CacheCommands {
    /// Clear cached entries from disk
    ///
    /// Usage: soroban-registry cache clear [--level disk|memory|all] [--key <key>]
    Clear {
        /// Cache level to clear: disk (default), memory, all
        #[arg(long, default_value = "disk")]
        level: String,
        /// Clear only the entry matching this specific cache key
        #[arg(long)]
        key: Option<String>,
    },

    /// Show cache statistics and configuration
    ///
    /// Usage: soroban-registry cache status [--json]
    Status {
        /// Output as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Configure cache settings
    ///
    /// Usage: soroban-registry cache configure [--ttl <secs>] [--max-size <bytes>]
    ///                                         [--compression on|off] [--auto-refresh on|off]
    Configure {
        /// Default TTL for cached entries in seconds
        #[arg(long)]
        ttl: Option<u64>,
        /// Maximum disk cache size in bytes (0 = unlimited)
        #[arg(long)]
        max_size: Option<u64>,
        /// Enable or disable compression for disk entries: on | off
        #[arg(long)]
        compression: Option<String>,
        /// Enable or disable automatic refresh of stale entries: on | off
        #[arg(long)]
        auto_refresh: Option<String>,
        /// Output current (or updated) config as JSON
        #[arg(long)]
        json: bool,
    },

    /// Remove stale entries and enforce disk size limit
    ///
    /// Usage: soroban-registry cache optimize [--json]
    Optimize {
        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Export cache entries for analysis
    ///
    /// Usage: soroban-registry cache export [--format json|csv] [--include-stale]
    Export {
        /// Output format: json (default) or csv
        #[arg(long, default_value = "json")]
        format: String,
        /// Include stale (expired) entries in the export
        #[arg(long)]
        include_stale: bool,
    },
}

/// Sub-commands for the `profile` group (#841)
#[derive(Debug, Subcommand)]
pub enum ProfileCommands {
    /// Display a publisher profile
    ///
    /// Usage: soroban-registry profile view [--address <stellar-address>] [--json]
    View {
        /// Stellar address or publisher UUID to look up (defaults to the address in local config)
        #[arg(long)]
        address: Option<String>,

        /// Output results as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Update profile fields
    ///
    /// Usage: soroban-registry profile edit --name <n> --website <url> ...
    Edit {
        /// Display name
        #[arg(long)]
        name: Option<String>,

        /// Short biography or description
        #[arg(long)]
        bio: Option<String>,

        /// Personal or project website URL
        #[arg(long)]
        website: Option<String>,

        /// Contact email address
        #[arg(long)]
        email: Option<String>,

        /// GitHub profile URL
        #[arg(long)]
        github: Option<String>,

        /// Avatar image URL
        #[arg(long)]
        avatar: Option<String>,
    },

    /// Update a single profile field by key
    ///
    /// Usage: soroban-registry profile update --field <key> --value <val>
    Update {
        /// Field to update (name | bio | website | email | github | avatar)
        #[arg(long)]
        field: String,

        /// New value for the field
        #[arg(long)]
        value: String,
    },

    /// List contracts published by a profile
    ///
    /// Usage: soroban-registry profile list-contracts [--address <addr>] [--limit N]
    #[command(name = "list-contracts")]
    ListContracts {
        /// Stellar address or publisher UUID (defaults to local config)
        #[arg(long)]
        address: Option<String>,

        /// Maximum number of contracts to return
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Output format: table | csv
        #[arg(long, default_value = "table")]
        format: String,

        /// Output as machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Export full profile data to JSON or CSV
    ///
    /// Usage: soroban-registry profile export [--address <addr>] [--format json|csv]
    Export {
        /// Stellar address or publisher UUID (defaults to local config)
        #[arg(long)]
        address: Option<String>,

        /// Export format: json | csv
        #[arg(long, default_value = "json")]
        format: String,
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
    Preview { old_id: String, new_id: String },
    /// Analyze schema differences between versions
    Analyze { old_id: String, new_id: String },
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
    Validate { old_id: String, new_id: String },
    /// Apply migration and record history
    Apply { old_id: String, new_id: String },
    /// Rollback a migration by migration ID
    Rollback { migration_id: String },
    /// Show migration history
    History {
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
pub enum VersionCommands {
    /// List versions for a contract
    List {
        /// Contract identifier
        contract_id: String,
    },
    /// Bump the semantic version
    Bump {
        /// Current version
        current: String,
        /// Bump level: major, minor, or patch
        #[arg(long, default_value = "patch")]
        level: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum UpgradeSubcommands {
    /// Analyze compatibility between two contract versions
    Analyze {
        /// Path to old WASM
        old_wasm: String,
        /// Path to new WASM
        new_wasm: String,
    },
    /// Apply an upgrade to a deployed contract
    Apply {
        /// Contract identifier
        contract_id: String,
        /// Path to new WASM
        new_wasm: String,
    },
    /// Rollback a contract to a previous version
    Rollback {
        /// Contract identifier
        contract_id: String,
        /// Version to rollback to
        version: String,
    },
    /// Generate a migration script template between versions
    Generate {
        /// Old contract identifier
        old_id: String,
        /// New contract identifier
        new_id: String,
        /// Language (rust or js)
        #[arg(long, default_value = "rust")]
        language: String,
        /// Output file path
        #[arg(long, short = 'o')]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();

    if cli.check_updates {
        let update_checks_enabled = user_config::load()
            .map(|cfg| cfg.update_checks_enabled)
            .unwrap_or(true);
        if update_checks_enabled {
            let _ = version::check_version(true, false, None).await;
        }
    }

    let cli_api_base = if cli.api_url.trim().is_empty() {
        None
    } else {
        Some(cli.api_url.clone())
    };
    let runtime = config::resolve_runtime_config(
        cli.network.clone(),
        cli_api_base,
        cli.timeout,
        cli.profile.clone(),
    )?;
    cli.api_url = runtime.api_base;
    cli.network = Some(runtime.network.to_string());
    cli.timeout = Some(runtime.timeout);

    cached_http::init(cached_http::HttpCacheOptions {
        no_cache: cli.no_cache,
        verbose: cli.verbose,
    });

    // ── Initialise logger ─────────────────────────────────────────────────────
    // -v counts; each level raises verbosity by one step.
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    env_logger::Builder::new()
        .parse_filters(log_level)
        .format_timestamp(None) // no timestamps in CLI output
        .format_module_path(cli.verbose > 0) // show module path only when verbose
        .init();

    log::debug!("Verbose mode enabled");
    log::debug!("API URL: {}", cli.api_url);

    handle_command(cli).await
}

pub async fn handle_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Repl {
            network: shell_network,
        } => shell::run(&cli.api_url, shell_network).await,
        _ => {
            // ── Resolve network ───────────────────────────────────────────────────────
            let cfg_network = config::resolve_network(cli.network.clone(), cli.profile.clone())?;
            let mut net_str = cfg_network.to_string();
            if net_str == "auto" {
                net_str = "mainnet".to_string();
            }
            let network: commands::Network = net_str.parse().unwrap();

            dispatch_command(cli, network, cfg_network).await
        }
    }
}

pub async fn dispatch_command(
    cli: Cli,
    network: commands::Network,
    cfg_network: crate::config::Network,
) -> Result<()> {
    log::debug!("Network: {:?}", network);

    match cli.command {
        Commands::Repl { .. } => {
            // Already handled at top level, but for completeness or nested calls:
            // We could call shell::run here again but to break recursion we don't.
            println!("{}", "Warning: REPL already running".yellow());
            return Ok(());
        }
        Commands::TrackDeployment {
            contract_id,
            network,
            tx_hash,
            wait_timeout,
            json,
        } => {
            log::debug!(
                "Command: track-deployment | contract_id={} network={} tx_hash={:?} wait_timeout={} json={}",
                contract_id, network, tx_hash, wait_timeout, json
            );
            track_deployment::run(
                &cli.api_url,
                &contract_id,
                &network,
                tx_hash.as_deref(),
                wait_timeout,
                json,
            )
            .await?;
        }
        Commands::Plugins { action } => match action {
            PluginCommands::List { json } => {
                let installed = plugins::discover_installed()?;
                if json {
                    let out: Vec<serde_json::Value> = installed
                        .into_iter()
                        .map(|p| {
                            serde_json::json!({
                                "manifest": p.manifest,
                                "path": p.manifest_path.to_string_lossy().to_string()
                            })
                        })
                        .collect();
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({ "plugins": out }))?
                    );
                } else {
                    if installed.is_empty() {
                        println!("{}", "No plugins installed.".yellow());
                    } else {
                        println!("\n{}", "Installed Plugins:".bold().cyan());
                        println!("{}", "=".repeat(80).cyan());
                        for p in installed {
                            let desc = p.manifest.description.clone().unwrap_or_default();
                            println!(
                                "  {}@{}  {}",
                                p.manifest.name.bold(),
                                p.manifest.version.bright_blue(),
                                desc.bright_black()
                            );
                            for cmd in &p.manifest.commands {
                                println!(
                                    "    - {}  {}",
                                    cmd.name.bright_green(),
                                    cmd.description.clone().unwrap_or_default().bright_black()
                                );
                            }
                        }
                    }
                }
            }
            PluginCommands::Marketplace { json } => {
                let marketplace = plugins::fetch_marketplace(&cli.api_url).await?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&marketplace)?);
                } else {
                    if marketplace.plugins.is_empty() {
                        println!("{}", "Marketplace returned no plugins.".yellow());
                    } else {
                        println!("\n{}", "Plugin Marketplace:".bold().cyan());
                        println!("{}", "=".repeat(80).cyan());
                        for p in marketplace.plugins {
                            println!(
                                "  {}@{}  {}",
                                p.name.bold(),
                                p.version.bright_blue(),
                                p.description.unwrap_or_default().bright_black()
                            );
                            for cmd in p.commands {
                                println!(
                                    "    - {}  {}",
                                    cmd.name.bright_green(),
                                    cmd.description.unwrap_or_default().bright_black()
                                );
                            }
                        }
                    }
                }
            }
            PluginCommands::Install { name, version } => {
                plugins::install_from_registry(&cli.api_url, &name, version.as_deref()).await?;
            }
            PluginCommands::Uninstall { name, version } => {
                plugins::uninstall(&name, version.as_deref())?;
            }
            PluginCommands::Run { command, args } => {
                let result = plugins::run_installed_command(
                    &cli.api_url,
                    &network.to_string(),
                    &command,
                    args,
                )
                .await?;
                print!("{}", result.stdout);
            }
            PluginCommands::Config { action } => match action {
                PluginConfigCommands::Get { name } => {
                    let cfg = plugins::get_plugin_config(&name)?;
                    println!("{}", serde_json::to_string_pretty(&cfg)?);
                }
                PluginConfigCommands::Set { name, json } => {
                    plugins::set_plugin_config_json(&name, &json)?;
                    println!("{} Updated config for {}", "✓".green(), name.bold());
                }
                PluginConfigCommands::Disable { name } => {
                    plugins::set_plugin_enabled(&name, false)?;
                    println!("{} Disabled {}", "✓".green(), name.bold());
                }
                PluginConfigCommands::Enable { name } => {
                    plugins::set_plugin_enabled(&name, true)?;
                    println!("{} Enabled {}", "✓".green(), name.bold());
                }
            },
        },
        Commands::External(args) => {
            if args.is_empty() {
                anyhow::bail!("No external command provided");
            }
            let cmd = args[0].clone();
            let rest = args.into_iter().skip(1).collect::<Vec<_>>();
            let result =
                plugins::run_installed_command(&cli.api_url, &network.to_string(), &cmd, rest)
                    .await?;
            print!("{}", result.stdout);
        }
        Commands::Search {
            query,
            verified_only,
            networks: filter_networks,
            category,
            sort,
            limit,
            offset,
            json,
        } => {
            let networks_vec: Vec<String> = filter_networks
                .map(|n| n.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            log::debug!(
                "Command: search | query={:?} verified_only={} networks={:?} category={:?} sort={:?}",
                query,
                verified_only,
                networks_vec,
                category,
                sort
            );
            commands::search(
                &cli.api_url,
                &query,
                network,
                verified_only,
                networks_vec,
                category.as_deref(),
                sort.as_deref(),
                limit,
                offset,
                json,
            )
            .await?;
        }
        Commands::Info { id, json, raw } => {
            let use_json = json || raw;
            contracts::info(&cli.api_url, &id, use_json).await?;
        }
        Commands::Compare {
            ids,
            json,
            export,
            format,
            exit_code,
            diff,
            fields,
        } => {
            let diff_format = compare::DiffFormat::parse(&diff)?;
            let field_filter = fields.map(|values| values.join(","));
            let code = compare::run(
                &cli.api_url,
                ids,
                json,
                export.as_deref(),
                format.as_deref(),
                compare::CompareOptions {
                    exit_code,
                    diff_format,
                    fields: field_filter,
                },
            )
            .await?;
            if exit_code && code != compare::EXIT_IDENTICAL {
                std::process::exit(code);
            }
        }
        Commands::Completion { shell } => {
            completion::generate_script(shell);
            eprintln!("\n{}", completion::install_hint(shell));
        }
        Commands::Analytics {
            query,
            period,
            format,
            sort,
            export,
        } => {
            let parsed_query = analytics::AnalyticsQuery::parse(&query)?;
            analytics::run(
                &cli.api_url,
                parsed_query,
                &period,
                &format,
                sort.as_deref(),
                export.as_deref(),
            )
            .await?;
        }
        Commands::Stats {
            timeframe,
            format,
            output,
        } => {
            log::debug!("Command: stats | timeframe={} format={}", timeframe, format);
            commands::stats(&cli.api_url, &timeframe, &format, output.as_deref()).await?;
        }
        Commands::Version {
            check_updates,
            auto_update,
            rollback,
        } => {
            version::check_version(check_updates, auto_update, rollback).await?;
        }
        Commands::Publish {
            contract_id,
            name,
            description,
            network: _publish_network,
            category,
            tags,
            publisher,
            contract_path,
            test_command,
            require_coverage,
            coverage_threshold,
            skip_tests,
        } => {
            let tags_vec = tags
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            log::debug!(
                "Command: publish | contract_id={} name={} tags={:?}",
                contract_id,
                name,
                tags_vec
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
                false,
                &contract_path,
                test_command.as_deref(),
                require_coverage,
                coverage_threshold,
                skip_tests,
            )
            .await?;
        }
        Commands::List {
            limit,
            offset,
            networks: filter_networks,
            category,
            format,
        } => {
            let networks_vec: Vec<String> = filter_networks
                .map(|n| n.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            commands::contract_list(
                &cli.api_url,
                limit,
                offset,
                Some(cfg_network),
                networks_vec,
                category,
                &format,
            )
            .await?;
        }
        Commands::Dashboard {
            refresh_rate,
            category,
            ws_url,
        } => {
            log::debug!(
                "Command: dashboard | refresh_rate={} network={:?} category={:?}",
                refresh_rate,
                cli.network,
                category
            );
            dashboard::run_dashboard(dashboard::DashboardParams {
                refresh_rate_ms: refresh_rate,
                network: cli.network.clone(),
                category,
                ws_url,
            })
            .await?;
        }
        Commands::BreakingChanges {
            old_id,
            new_id,
            json,
        } => {
            log::debug!("Command: breaking-changes | old={} new={}", old_id, new_id);
            commands::breaking_changes(&cli.api_url, &old_id, &new_id, json).await?;
        }
        Commands::UpgradeAnalyze { old, new, json } => {
            log::debug!("Command: upgrade analyze | old={} new={}", old, new);
            commands::upgrade_analyze(&cli.api_url, &old, &new, json).await?;
        }
        Commands::Migrate { action } => match action {
            MigrateCommands::Preview { old_id, new_id } => {
                log::debug!(
                    "Command: migrate preview | old_id={} new_id={}",
                    old_id,
                    new_id
                );
                migration::preview(&old_id, &new_id)?;
            }
            MigrateCommands::Analyze { old_id, new_id } => {
                log::debug!(
                    "Command: migrate analyze | old_id={} new_id={}",
                    old_id,
                    new_id
                );
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
                log::debug!(
                    "Command: migrate validate | old_id={} new_id={}",
                    old_id,
                    new_id
                );
                migration::validate(&old_id, &new_id)?;
            }
            MigrateCommands::Apply { old_id, new_id } => {
                log::debug!(
                    "Command: migrate apply | old_id={} new_id={}",
                    old_id,
                    new_id
                );
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
            format,
            filters,
            page_size,
        } => {
            log::debug!(
                "Command: export | id={:?} output={:?} format={:?}",
                id,
                output,
                format
            );
            commands::export(
                &cli.api_url,
                id.as_deref(),
                output.as_deref(),
                &contract_dir,
                format.as_deref(),
                filters,
                page_size,
            )
            .await?;
        }
        Commands::Import {
            file,
            format,
            output_dir,
            validate,
            dry_run,
        } => {
            let network = cli.network.as_deref();
            log::debug!(
                "Command: import | file={} format={:?} output_dir={} validate={} dry_run={}",
                file,
                format,
                output_dir,
                validate,
                dry_run
            );
            let opts = crate::import::ImportOptions {
                api_url: &cli.api_url,
                file_path: &file,
                format: format.as_deref(),
                network_flag: network,
                output_dir: &output_dir,
                validate,
                dry_run,
                on_duplicate: crate::import::OnDuplicate::Skip,
                network_map: std::collections::HashMap::new(),
                atomic: false,
                report_output: None,
            };
            crate::import::run(opts).await?;
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
        Commands::Deploy {} => {
            log::debug!("Command: deploy");
            deploy::run_interactive().await?;
        }
        Commands::VersionSemver { action } => match action {
            VersionCommands::List { contract_id } => {
                log::debug!("Command: version list | contract_id={}", contract_id);
                upgrade::version::list(&contract_id)?;
            }
            VersionCommands::Bump { current, level } => {
                log::debug!(
                    "Command: version bump | current={} level={}",
                    current,
                    level
                );
                let next = upgrade::version::bump(&current, &level)?;
                println!("Next version: {}", next.green().bold());
            }
        },
        Commands::Upgrade { action } => match action {
            UpgradeSubcommands::Analyze { old_wasm, new_wasm } => {
                log::debug!(
                    "Command: upgrade analyze | old={} new={}",
                    old_wasm,
                    new_wasm
                );
                upgrade::manager::analyze(&old_wasm, &new_wasm).await?;
            }
            UpgradeSubcommands::Apply {
                contract_id,
                new_wasm,
            } => {
                log::debug!(
                    "Command: upgrade apply | contract_id={} new={}",
                    contract_id,
                    new_wasm
                );
                upgrade::manager::apply(&contract_id, &new_wasm).await?;
            }
            UpgradeSubcommands::Rollback {
                contract_id,
                version,
            } => {
                log::debug!(
                    "Command: upgrade rollback | contract_id={} version={}",
                    contract_id,
                    version
                );
                upgrade::manager::rollback(&contract_id, &version).await?;
            }
            UpgradeSubcommands::Generate {
                old_id,
                new_id,
                language,
                output,
            } => {
                log::debug!(
                    "Command: upgrade generate | old={} new={} lang={}",
                    old_id,
                    new_id,
                    language
                );
                crate::migration::generate_template(
                    &old_id,
                    &new_id,
                    &language,
                    output.as_deref(),
                )?;
            }
        },
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
        Commands::Perf {
            contract_path,
            method,
            output,
            flamegraph,
            compare,
            recommendations,
        } => {
            log::debug!(
                "Command: perf | contract_path={} method={:?} output={:?} flamegraph={:?} compare={:?} recommendations={}",
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
        // ── User profile management (#841) ───────────────────────────────────
        Commands::Profile { action } => match action {
            ProfileCommands::View { address, json } => {
                log::debug!(
                    "Command: profile view | address={:?} json={}",
                    address,
                    json
                );
                user_profile::view(&cli.api_url, address.as_deref(), json).await?;
            }
            ProfileCommands::Edit {
                name,
                bio,
                website,
                email,
                github,
                avatar,
            } => {
                log::debug!("Command: profile edit");
                user_profile::edit(
                    &cli.api_url,
                    name.as_deref(),
                    bio.as_deref(),
                    website.as_deref(),
                    email.as_deref(),
                    github.as_deref(),
                    avatar.as_deref(),
                )
                .await?;
            }
            ProfileCommands::Update { field, value } => {
                log::debug!("Command: profile update | field={} value={}", field, value);
                user_profile::update_field(&cli.api_url, &field, &value).await?;
            }
            ProfileCommands::ListContracts {
                address,
                limit,
                format,
                json,
            } => {
                log::debug!(
                    "Command: profile list-contracts | address={:?} limit={} format={}",
                    address,
                    limit,
                    format
                );
                user_profile::list_contracts(
                    &cli.api_url,
                    address.as_deref(),
                    limit,
                    &format,
                    json,
                )
                .await?;
            }
            ProfileCommands::Export { address, format } => {
                log::debug!(
                    "Command: profile export | address={:?} format={}",
                    address,
                    format
                );
                user_profile::export(&cli.api_url, address.as_deref(), &format).await?;
            }
        },
        Commands::Test {
            test_file,
            contract_path,
            test_command,
            junit,
            coverage,
            verbose,
            require_coverage,
            coverage_threshold,
            setup_hook,
            teardown_hook,
            mock_config,
            report,
            profile_output,
            load_iterations,
        } => {
            commands::run_test_suite(commands::TestSuiteOptions {
                test_file: test_file.as_deref(),
                contract_path: contract_path.as_deref().unwrap_or("."),
                test_command: test_command.as_deref(),
                junit_output: junit.as_deref(),
                show_coverage: coverage,
                verbose,
                require_coverage,
                coverage_threshold,
                setup_hook: setup_hook.as_deref(),
                teardown_hook: teardown_hook.as_deref(),
                mock_config: mock_config.as_deref(),
                report_output: report.as_deref(),
                profile_output: profile_output.as_deref(),
                load_iterations,
            })
            .await?;
        }
        Commands::Audit {
            contract_path,
            format,
            output,
            fail_on,
        } => {
            log::debug!(
                "Command: audit | contract_path={} format={} output={:?} fail_on={:?}",
                contract_path,
                format,
                output,
                fail_on
            );
            audit_command::run(
                &contract_path,
                &format,
                output.as_deref(),
                fail_on.as_deref(),
            )?;
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
            ConfigSubcommands::UserGet { key } => {
                user_config::validate_key(&key)?;
                let value = user_config::get_key(&key)?;
                match value {
                    Some(v) => println!("{}", v),
                    None => anyhow::bail!("Key '{}' was not found in user config.", key),
                }
            }
            ConfigSubcommands::UserSet { key, value } => {
                user_config::set_key(&key, &value)?;
                println!("Updated '{}' in user config.", key);
            }
            ConfigSubcommands::UserList {} => {
                let cfg = user_config::list()?;
                println!("{}", serde_json::to_string_pretty(&cfg)?);
            }
            ConfigSubcommands::UserReset {} => {
                let cfg = user_config::reset_to_defaults()?;
                println!("User config reset to defaults:");
                println!("{}", serde_json::to_string_pretty(&cfg)?);
            }
            ConfigSubcommands::ContractGet {
                contract_id,
                environment,
            } => {
                commands::config_get(&cli.api_url, &contract_id, &environment).await?;
            }
            ConfigSubcommands::ContractSet {
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
            ConfigSubcommands::ContractHistory {
                contract_id,
                environment,
            } => {
                commands::config_history(&cli.api_url, &contract_id, &environment).await?;
            }
            ConfigSubcommands::ContractRollback {
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
        Commands::Auth { action } => match action {
            AuthCommands::Login {
                method,
                identity,
                secret,
                scopes,
                expires,
            } => {
                let method = match method {
                    Some(method) => method,
                    None => {
                        let selected = wizard::prompt_with_validation(
                            "Authentication method [github|stellar|api-key]",
                            Some("stellar".to_string()),
                            |value| {
                                matches!(
                                    value.trim().to_ascii_lowercase().as_str(),
                                    "github" | "stellar" | "api-key"
                                )
                            },
                            "Choose github, stellar, or api-key.",
                        )?;
                        match selected.trim().to_ascii_lowercase().as_str() {
                            "github" => crate::auth::AuthMethod::Github,
                            "stellar" => crate::auth::AuthMethod::Stellar,
                            "api-key" => crate::auth::AuthMethod::ApiKey,
                            _ => unreachable!(),
                        }
                    }
                };
                log::debug!(
                    "Command: auth login | method={} identity={:?} scopes={:?} expires={:?}",
                    method,
                    identity,
                    scopes,
                    expires
                );
                auth::login(
                    &cli.api_url,
                    method,
                    identity.as_deref(),
                    secret.as_deref(),
                    scopes,
                    expires.as_deref(),
                )
                .await?;
            }
            AuthCommands::Logout {} => {
                log::debug!("Command: auth logout");
                auth::logout()?;
            }
            AuthCommands::Status {} => {
                log::debug!("Command: auth status");
                auth::status(&cli.api_url).await?;
            }
            AuthCommands::Token { scopes, expires } => {
                log::debug!(
                    "Command: auth token | scopes={:?} expires={:?}",
                    scopes,
                    expires
                );
                auth::token(&cli.api_url, scopes, expires.as_deref()).await?;
            }
        },
        Commands::Backup { action } => match action {
            BackupCommands::Create {
                contract_id,
                include_state,
            } => {
                backup::create_backup(&cli.api_url, &contract_id, include_state).await?;
            }
            BackupCommands::List { contract_id } => {
                backup::list_backups(&cli.api_url, &contract_id).await?;
            }
            BackupCommands::Restore {
                contract_id,
                backup_date,
            } => {
                backup::restore_backup(&cli.api_url, &contract_id, &backup_date).await?;
            }
            BackupCommands::Verify {
                contract_id,
                backup_date,
            } => {
                backup::verify_backup(&cli.api_url, &contract_id, &backup_date).await?;
            }
            BackupCommands::Stats { contract_id } => {
                backup::backup_stats(&cli.api_url, &contract_id).await?;
            }
        },
        Commands::State { action } => match action {
            StateSubcommands::Get {
                contract_id,
                key,
                json,
            } => {
                commands::state_get(&cli.api_url, &contract_id, &key, network, json).await?;
            }
            StateSubcommands::Set {
                contract_id,
                key,
                value,
                json,
            } => {
                commands::state_set(&cli.api_url, &contract_id, &key, &value, network, json)
                    .await?;
            }
            StateSubcommands::Dump { contract_id, json } => {
                commands::state_dump(&contract_id, network, json)?;
            }
            StateSubcommands::Snapshot {
                contract_id,
                label,
                json,
            } => {
                commands::state_snapshot_create(&contract_id, network, label.as_deref(), json)?;
            }
            StateSubcommands::Snapshots {
                contract_id,
                limit,
                json,
            } => {
                commands::state_snapshot_list(&contract_id, network, limit, json)?;
            }
            StateSubcommands::History {
                contract_id,
                key,
                limit,
                json,
            } => {
                commands::state_history(&contract_id, network, key.as_deref(), limit, json)?;
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
        Commands::VerifyPackage {
            package,
            contract_id,
            version,
            signature,
        } => {
            log::debug!(
                "Command: verify-package | package={} contract_id={}",
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
        Commands::Verify {
            id,
            submit,
            check,
            history,
            level,
            json,
            path,
            notes,
        } => {
            log::debug!(
                "Command: verify | id={:?} submit={} check={}",
                id,
                submit,
                check
            );
            verification::run(
                &cli.api_url,
                id,
                submit,
                check,
                history,
                level,
                json,
                &path,
                notes,
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
            file,
            contracts,
            network,
            category,
            age,
            initiated_by,
            level,
            export,
            output,
            schedule,
            json,
        } => {
            log::debug!(
                "Command: batch-verify | contracts={:?} initiated_by={}",
                contracts,
                initiated_by
            );
            batch_verify::run_batch_verify(batch_verify::BatchVerifyArgs {
                api_url: &cli.api_url,
                file: file.as_deref(),
                contracts: contracts.as_deref(),
                network: network.as_deref(),
                category: category.as_deref(),
                age,
                initiated_by: &initiated_by,
                level: &level,
                export: export.as_deref(),
                output: output.as_deref(),
                schedule: schedule.as_deref(),
                json,
            })
            .await?;
        }
        Commands::Webhook { action } => match action {
            WebhookCommands::Create {
                url,
                events,
                secret,
            } => {
                let event_list: Vec<String> =
                    events.split(',').map(|s| s.trim().to_string()).collect();
                log::debug!(
                    "Command: webhook create | url={} events={:?}",
                    url,
                    event_list
                );
                webhook::create_webhook(&cli.api_url, &url, event_list, secret.as_deref()).await?;
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
            WebhookCommands::VerifySig {
                secret,
                payload,
                signature,
            } => {
                log::debug!("Command: webhook verify-sig");
                webhook::verify_signature_cmd(&secret, &payload, &signature)?;
            }
        },
        // ── Contract verify command (#522) ───────────────────────────────────
        Commands::Contract { action } => match action {
            ContractCommands::Register { file, batch, json } => {
                log::debug!(
                    "Command: contract register | file={:?} batch={} json={}",
                    file,
                    batch,
                    json
                );
                contract_register::run(&cli.api_url, cfg_network, file.as_deref(), batch, json)
                    .await?;
            }
            ContractCommands::Verify {
                address,
                network,
                json,
                strict,
                batch,
                no_cache,
            } => {
                log::debug!(
                    "Command: contract verify | address={} network={} json={} strict={} batch={} no_cache={}",
                    address,
                    network,
                    json,
                    strict,
                    batch,
                    no_cache
                );
                contract_verify::run(
                    &cli.api_url,
                    &address,
                    &network,
                    json,
                    strict,
                    batch,
                    no_cache,
                )
                .await?;
            }
            ContractCommands::Details {
                address,
                network,
                json,
            } => {
                log::debug!(
                    "Command: contract details | address={} network={} json={}",
                    address,
                    network,
                    json
                );
                contracts::run_details(&cli.api_url, &address, &network, json).await?;
            }
            ContractCommands::Deploy {
                wasm_path,
                name,
                description,
                category,
                network,
                icon,
                interactive,
                publisher,
                tags,
                skip_abi,
                json,
            } => {
                log::debug!(
                    "Command: contract deploy | wasm_path={} network={} interactive={}",
                    wasm_path,
                    network,
                    interactive
                );
                contract_deploy::run_deploy(
                    &cli.api_url,
                    &wasm_path,
                    name.as_deref(),
                    description.as_deref(),
                    category.as_deref(),
                    &network,
                    icon.as_deref(),
                    interactive,
                    publisher.as_deref(),
                    tags.as_deref(),
                    skip_abi,
                    json,
                )
                .await?;
            }
            ContractCommands::Risk {
                address,
                network,
                threshold,
                json,
            } => {
                log::debug!(
                    "Command: contract risk | address={} network={} threshold={:?} json={}",
                    address,
                    network,
                    threshold,
                    json
                );
                contract_risk::run(&cli.api_url, &address, &network, threshold.as_deref(), json)
                    .await?;
            }
            ContractCommands::Stats {
                network,
                category,
                top_n,
                format,
                output,
                compare,
            } => {
                log::debug!(
                    "Command: contract stats | network={:?} category={:?} format={}",
                    network,
                    category,
                    format
                );
                commands::contract_stats(
                    &cli.api_url,
                    network.as_deref(),
                    category.as_deref(),
                    top_n,
                    &format,
                    output.as_deref(),
                    compare.as_deref(),
                )
                .await?;
            }
            ContractCommands::Export {
                output_file,
                output,
                format,
                network,
                category,
                since,
                compress,
                include_related,
                page_size,
            } => {
                let resolved_output = output.or(output_file);
                log::debug!(
                    "Command: contract export | output={:?} format={} network={:?} category={:?}",
                    resolved_output,
                    format,
                    network,
                    category
                );
                commands::contract_export(
                    &cli.api_url,
                    resolved_output.as_deref(),
                    &format,
                    network.as_deref(),
                    category.as_deref(),
                    since.as_deref(),
                    compress,
                    include_related,
                    page_size,
                )
                .await?;
            }
            ContractCommands::Highlight {
                address,
                action,
                token,
                json,
            } => {
                log::debug!("Command: contract highlight | action={}", action);
                contract_highlight::run(
                    &cli.api_url,
                    address.as_deref(),
                    &action,
                    token.as_deref(),
                    json,
                )
                .await?;
            }
            ContractCommands::Interaction {
                address,
                limit,
                json,
            } => {
                log::debug!("Command: contract interaction | address={}", address);
                contract_interaction::run(&cli.api_url, &address, limit, json).await?;
            }
            ContractCommands::Dependency {
                address,
                depth,
                format,
                summary,
            } => {
                log::debug!("Command: contract dependency | address={} depth={}", address, depth);
                let fmt = crate::output_format::validate_format(&format)
                    .unwrap_or(crate::output_format::OutputFormat::Table);
                contract_dependency::run(&cli.api_url, &address, depth, fmt, summary).await?;
            }
            ContractCommands::Update {
                address,
                name,
                description,
                category,
                tags,
                icon,
                homepage,
                dry_run,
                yes,
                json,
            } => {
                let tags_vec = tags.map(|t| {
                    t.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                });
                contract_update::run(contract_update::UpdateArgs {
                    api_url: &cli.api_url,
                    address: &address,
                    name,
                    description,
                    category,
                    tags: tags_vec,
                    icon,
                    homepage,
                    dry_run,
                    yes,
                    json,
                })
                .await?;
            }
            ContractCommands::Import {
                input_file,
                format,
                on_duplicate,
                network_map,
                dry_run,
                validate,
                atomic,
                report_output,
                output_dir,
            } => {
                log::debug!(
                    "Command: contract import | file={} format={:?} on_duplicate={} dry_run={} validate={} atomic={}",
                    input_file,
                    format,
                    on_duplicate,
                    dry_run,
                    validate,
                    atomic
                );
                let dup_strategy = crate::import::OnDuplicate::parse(&on_duplicate)?;
                let net_map = crate::import::parse_network_map(&network_map)?;
                let opts = crate::import::ImportOptions {
                    api_url: &cli.api_url,
                    file_path: &input_file,
                    format: format.as_deref(),
                    network_flag: cli.network.as_deref(),
                    output_dir: &output_dir,
                    validate,
                    dry_run,
                    on_duplicate: dup_strategy,
                    network_map: net_map,
                    atomic,
                    report_output,
                };
                crate::import::run(opts).await?;
            }
        },
        Commands::ApiKey { action } => match action {
            ApiKeyCommands::Create {
                expires,
                scopes,
                json,
            } => {
                log::debug!("Command: api-key create");
                api_key::create(&cli.api_url, expires.as_deref(), scopes.as_deref(), json).await?;
            }
            ApiKeyCommands::List { json } => {
                log::debug!("Command: api-key list");
                api_key::list(&cli.api_url, json).await?;
            }
            ApiKeyCommands::Delete { id, json } => {
                log::debug!("Command: api-key delete | id={}", id);
                api_key::delete(&cli.api_url, &id, false, json).await?;
            }
            ApiKeyCommands::Revoke { id, json } => {
                log::debug!("Command: api-key revoke | id={}", id);
                api_key::delete(&cli.api_url, &id, true, json).await?;
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
            ReleaseNotesCommands::List { contract_id, json } => {
                log::debug!("Command: release-notes list | contract_id={}", contract_id);
                release_notes::list(&cli.api_url, &contract_id, json).await?;
            }
        },

        Commands::Cicd { action } => match action {
            CicdCommands::Run {
                contract_path,
                network,
                skip_scan,
                auto_register,
                json,
            } => {
                log::debug!(
                    "Command: cicd run | path={} network={}",
                    contract_path,
                    network
                );
                cicd::run_pipeline(
                    &cli.api_url,
                    &contract_path,
                    &network,
                    skip_scan,
                    auto_register,
                    json,
                )
                .await?;
            }
            CicdCommands::Validate { contract_path } => {
                log::debug!("Command: cicd validate | path={}", contract_path);
                cicd::validate_env(&contract_path).await?;
            }
        },

        // ── Network commands (issue #523) ────────────────────────────────────
        Commands::Network { action } => match action {
            NetworkCommands::Status { json } => {
                log::debug!("Command: network status");
                network::status(json).await?;
            }
        },

        // ── Advanced contract analysis (issue #530) ─────────────────────────
        Commands::Analyze {
            contract_id,
            network: net_str,
            report_format,
            output,
        } => {
            log::debug!(
                "Command: analyze | contract_id={} network={} format={}",
                contract_id,
                net_str,
                report_format
            );
            analyze::run(
                &cli.api_url,
                &contract_id,
                &net_str,
                &report_format,
                output.as_deref(),
            )
            .await?;
        }

        // ── Bulk contract registration (issue #525) ──────────────────────────
        Commands::BatchRegister {
            manifest,
            publisher,
            dry_run,
            json,
        } => {
            log::debug!(
                "Command: batch-register | manifest={} dry_run={} publisher={:?}",
                manifest,
                dry_run,
                publisher
            );
            batch_register::run_batch_register(
                &cli.api_url,
                &manifest,
                publisher.as_deref(),
                dry_run,
                json,
                // Documented defaults: None = sequential (1 at a time), no retry pass.
                // `batch-register` exposes no flags for these yet.
                None,
                false,
            )
            .await?;
        }
        Commands::BatchAudit {
            file,
            format,
            output_dir,
            fail_on,
            high_risk,
            profile,
            export,
            json,
        } => {
            log::debug!("Command: batch-audit | file={}", file);
            batch_audit::run_batch_audit(
                &file,
                &format,
                output_dir.as_deref(),
                fail_on.as_deref(),
                high_risk,
                &profile,
                export.as_deref(),
                json,
            )?;
        }
        Commands::BatchDeploy {
            wasm_file,
            networks,
            signer,
            atomic,
            json,
        } => {
            log::debug!("Command: batch-deploy | wasm={}", wasm_file);
            batch_deploy::run_batch_deploy(&wasm_file, &networks, &signer, atomic, json)?;
        }
        Commands::BatchExport {
            output_dir,
            filter,
            format,
            organize,
            compress,
            json,
        } => {
            log::debug!("Command: batch-export | output_dir={}", output_dir);
            batch_export::run_batch_export(
                &cli.api_url,
                &output_dir,
                filter.as_deref(),
                &format,
                organize,
                compress,
                json,
            )
            .await?;
        }
        Commands::BatchUpdate {
            file,
            filter,
            preview,
            r#if: condition,
            user_id,
            rollback_on_error,
            json,
        } => {
            batch_update::run_batch_update(batch_update::BatchUpdateArgs {
                api_url: &cli.api_url,
                file: file.as_deref(),
                filter: filter.as_deref(),
                preview,
                condition: condition.as_deref(),
                user_id: user_id.as_deref(),
                rollback_on_error,
                json,
            })
            .await?;
        }
        Commands::BatchImport {
            input_dir,
            format,
            on_duplicate,
            dry_run,
            atomic,
            output_dir,
            json,
        } => {
            log::debug!("Command: batch-import | input_dir={}", input_dir);
            batch_import::run_batch_import(
                &cli.api_url,
                &input_dir,
                format.as_deref(),
                &on_duplicate,
                dry_run,
                atomic,
                &output_dir,
                json,
            )
            .await?;
        }
        Commands::Batch {
            operation,
            contracts,
            file,
            value,
            rollback_on_error,
            recipients,
            message_type,
            template,
            preview,
            schedule,
            channels,
            filter,
            atomic,
            report,
            json,
        } => {
            if operation == "notify" {
                let recipients = recipients
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("batch notify requires --recipients"))?;
                let message = contracts.join(" ");
                batch_notify::run_batch_notify(
                    &cli.api_url,
                    &message,
                    recipients,
                    &message_type,
                    template.as_deref(),
                    preview,
                    schedule.as_deref(),
                    channels,
                    json,
                )
                .await?;
                return Ok(());
            }
            if operation == "migrate" {
                anyhow::ensure!(
                    contracts.len() >= 2,
                    "batch migrate requires SOURCE and DESTINATION"
                );
                batch_migrate::run_batch_migrate(
                    &contracts[0],
                    &contracts[1],
                    filter.as_deref(),
                    preview,
                    atomic,
                    report.as_deref(),
                    json,
                )
                .await?;
                return Ok(());
            }
            let op = batch_ops::BatchOperation::parse(&operation)?;
            batch_ops::run(
                &cli.api_url,
                op,
                contracts,
                file.as_deref(),
                value.as_deref(),
                rollback_on_error,
                json,
            )
            .await?;
        }
        // ── Local cache management (#845) ────────────────────────────────────
        Commands::Cache { action } => match action {
            CacheCommands::Clear { level, key } => {
                log::debug!("Command: cache clear | level={} key={:?}", level, key);
                cache::clear(&level, key.as_deref())?;
            }
            CacheCommands::Status { json } => {
                log::debug!("Command: cache status | json={}", json);
                cache::status(json)?;
            }
            CacheCommands::Configure {
                ttl,
                max_size,
                compression,
                auto_refresh,
                json,
            } => {
                log::debug!("Command: cache configure");
                cache::configure(
                    ttl,
                    max_size,
                    compression.as_deref(),
                    auto_refresh.as_deref(),
                    json,
                )?;
            }
            CacheCommands::Optimize { json } => {
                log::debug!("Command: cache optimize | json={}", json);
                cache::optimize(json)?;
            }
            CacheCommands::Export {
                format,
                include_stale,
            } => {
                log::debug!(
                    "Command: cache export | format={} include_stale={}",
                    format,
                    include_stale
                );
                cache::export(&format, include_stale)?;
            }
        },
        // ── Environment variable management (#843) ───────────────────────────
        Commands::Env { action } => match action {
            EnvCommands::Set {
                name,
                value,
                env,
                show_value,
            } => {
                log::debug!(
                    "Command: env set | name={} env={:?} show_value={}",
                    name,
                    env,
                    show_value
                );
                env::set_var(&name, &value, env.as_deref(), show_value)?;
            }
            EnvCommands::Get { name, env, json } => {
                log::debug!(
                    "Command: env get | name={} env={:?} json={}",
                    name,
                    env,
                    json
                );
                env::get_var(&name, env.as_deref(), json)?;
            }
            EnvCommands::List {
                env,
                all,
                merged,
                json,
            } => {
                log::debug!(
                    "Command: env list | env={:?} all={} merged={} json={}",
                    env,
                    all,
                    merged,
                    json
                );
                env::list_vars(env.as_deref(), all, merged, json)?;
            }
            EnvCommands::Copy {
                from,
                to,
                overwrite,
            } => {
                log::debug!(
                    "Command: env copy | from={} to={} overwrite={}",
                    from,
                    to,
                    overwrite
                );
                env::copy_env(&from, &to, overwrite)?;
            }
            EnvCommands::Delete { name, env } => {
                log::debug!("Command: env delete | name={} env={:?}", name, env);
                env::delete_var(&name, env.as_deref())?;
            }
            EnvCommands::Export {
                env,
                format,
                merged,
            } => {
                log::debug!(
                    "Command: env export | env={:?} format={:?} merged={}",
                    env,
                    format,
                    merged
                );
                env::export_env(env.as_deref(), format.as_str(), merged)?;
            }
            EnvCommands::Switch { environment } => {
                log::debug!("Command: env switch | environment={}", environment);
                env::switch_env(&environment)?;
            }
        },
    }

    Ok(())
}

#[cfg(test)]
mod verbose_flag_tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("CLI should parse")
    }

    #[test]
    fn no_flag_yields_zero() {
        let cli = parse(&["soroban-registry", "version"]);
        assert_eq!(cli.verbose, 0);
    }

    #[test]
    fn single_short_flag_yields_one() {
        let cli = parse(&["soroban-registry", "-v", "version"]);
        assert_eq!(cli.verbose, 1);
    }

    #[test]
    fn repeated_short_flags_count() {
        let cli = parse(&["soroban-registry", "-v", "-v", "-v", "version"]);
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn stacked_short_flag_counts() {
        let cli = parse(&["soroban-registry", "-vvv", "version"]);
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn long_flag_counts_too() {
        let cli = parse(&["soroban-registry", "--verbose", "--verbose", "version"]);
        assert_eq!(cli.verbose, 2);
    }

    #[test]
    fn verbose_works_after_subcommand_when_global() {
        let cli = parse(&["soroban-registry", "version", "-vv"]);
        assert_eq!(cli.verbose, 2);
    }

    #[test]
    fn env_export_rejects_invalid_format() {
        let err = Cli::try_parse_from(["soroban-registry", "env", "export", "--format", "invalid"])
            .expect_err("CLI should reject invalid export format");

        assert!(
            err.to_string().contains("possible values"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn env_set_parses_show_value_flag() {
        let cli = parse(&[
            "soroban-registry",
            "env",
            "set",
            "API_KEY",
            "secret",
            "--show-value",
        ]);

        match cli.command {
            Commands::Env {
                action: EnvCommands::Set { show_value, .. },
            } => assert!(show_value),
            _ => panic!("expected env set command"),
        }
    }
}
