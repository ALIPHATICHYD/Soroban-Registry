#![allow(dead_code, unused)]

/// Stellar Blockchain Indexer Service
/// Continuously monitors Stellar network for contract deployments and syncs to registry database
///
/// This service:
/// - Polls Stellar RPC endpoint on 30-second intervals (configurable)
/// - Detects createContract operations in new ledgers
/// - Extracts contract metadata (ID, deployer, network)
/// - Writes unverified contract records to database
/// - Tracks last indexed ledger for safe resume after restarts
/// - Handles RPC failures with exponential backoff
/// - Detects and recovers from ledger reorgs
/// - Provides structured logging for observability
mod backoff;
mod config;
mod db;
mod detector;
mod reorg;
mod rpc;
mod state;
mod telemetry;
mod usdc_scanner;

use anyhow::Result;
use config::{DatabaseConfig, ServiceConfig};
use db::DatabaseWriter;
use reorg::ReorgHandler;
use rpc::StellarRpcClient;
use sqlx::ConnectOptions;
use state::{IndexerState, StateManager};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info, warn};

struct IndexerService {
    config: ServiceConfig,
    rpc_client: StellarRpcClient,
    db_writer: DatabaseWriter,
    state_manager: StateManager,
    reorg_handler: ReorgHandler,
    backoff: backoff::ExponentialBackoff,
    current_state: Arc<Mutex<Option<IndexerState>>>,
}

impl IndexerService {
    /// Initialize the indexer service
    async fn new(config: ServiceConfig) -> Result<Self> {
        // Initialize database connection pool (Issue #876)
        let connect_options = config
            .database
            .connection_string
            .parse::<sqlx::postgres::PgConnectOptions>()?
            .log_slow_statements(
                log::LevelFilter::Warn,
                std::time::Duration::from_millis(config.database.slow_query_threshold_ms as u64),
            );

        let db_pool = sqlx::postgres::PgPoolOptions::new()
            .min_connections(config.database.min_connections)
            .max_connections(config.database.max_connections)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(std::time::Duration::from_secs(
                config.database.idle_timeout_secs,
            ))
            .max_lifetime(std::time::Duration::from_secs(
                config.database.max_lifetime_secs,
            ))
            .connect_with(connect_options)
            .await?;

        let rpc_client = StellarRpcClient::new(config.network.rpc_endpoint.clone());
        let db_writer = DatabaseWriter::new(db_pool.clone());
        let state_manager = StateManager::new(db_pool);
        let reorg_handler = ReorgHandler::new(config.reorg_checkpoint_depth);
        let backoff = backoff::ExponentialBackoff::new(
            config.backoff_base_interval_secs,
            config.backoff_max_interval_secs,
        );

        Ok(IndexerService {
            config,
            rpc_client,
            db_writer,
            state_manager,
            reorg_handler,
            backoff,
            current_state: Arc::new(Mutex::new(None)),
        })
    }

    /// Flush the current state to the database before shutdown
    async fn flush_state(&self) -> Result<()> {
        let state_clone = self
            .current_state
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        if let Some(state) = state_clone {
            info!(
                "Flushing indexer state to database before shutdown: ledger={}",
                state.last_indexed_ledger_height
            );
            self.state_manager.update_state(&state).await?;
        }
        Ok(())
    }

    /// Run the main indexing loop
    async fn run(&mut self) -> Result<()> {
        info!(
            "Starting indexer service for network: {}",
            self.config.network.network_name()
        );

        // Load initial state
        let mut state = match self
            .state_manager
            .load_state(&self.config.network.network)
            .await
        {
            Ok(s) => {
                info!(
                    "Loaded indexer state: last_indexed_ledger={}",
                    s.last_indexed_ledger_height
                );
                s
            }
            Err(e) => {
                error!(
                    "Failed to load indexer state: {}, initializing with defaults",
                    e
                );
                IndexerState {
                    network: self.config.network.network.clone(),
                    last_indexed_ledger_height: 0,
                    last_indexed_ledger_hash: None,
                    last_checkpoint_ledger_height: 0,
                    consecutive_failures: 0,
                }
            }
        };

        // Store initial state for graceful shutdown
        if let Ok(mut state_guard) = self.current_state.lock() {
            *state_guard = Some(state.clone());
        }

        // Health check before starting
        match self.rpc_client.health_check().await {
            Ok(_) => info!("RPC endpoint health check passed"),
            Err(e) => warn!("Initial RPC health check failed: {}, will retry", e),
        }

        // Main polling loop
        loop {
            let poll_duration = Duration::from_secs(self.config.network.poll_interval_secs);

            match self.poll_and_index(&mut state).await {
                Ok(_) => {
                    self.backoff.on_success();
                    // Store current state for graceful shutdown
                    if let Ok(mut state_guard) = self.current_state.lock() {
                        *state_guard = Some(state.clone());
                    }
                }
                Err(e) => {
                    error!("Error during polling cycle: {}", e);
                    state.record_failure();
                    // Store state even on error for graceful shutdown
                    if let Ok(mut state_guard) = self.current_state.lock() {
                        *state_guard = Some(state.clone());
                    }

                    let backoff_duration = self.backoff.on_failure(&e.to_string());
                    let backoff_secs = backoff_duration.as_secs();

                    // Record error in state manager
                    let _ = self
                        .state_manager
                        .record_error(&self.config.network.network, &e.to_string())
                        .await;

                    warn!(
                        attempt = self.backoff.attempts(),
                        backoff_secs = backoff_secs,
                        "Backing off before retry"
                    );

                    tokio::time::sleep(backoff_duration).await;
                    continue;
                }
            }

            // Wait for next poll cycle
            tokio::time::sleep(poll_duration).await;
        }
    }

    /// Single polling and indexing cycle
    async fn poll_and_index(&mut self, state: &mut IndexerState) -> Result<()> {
        let network_name = self.config.network.network_name();

        // Get latest ledger
        let latest_ledger = self.rpc_client.get_latest_ledger().await?;
        let next_ledger = state.next_ledger_to_process();

        // Calculate lag for observability
        let indexer_lag = latest_ledger.sequence.saturating_sub(next_ledger);

        info!(
            network = network_name,
            latest_ledger = latest_ledger.sequence,
            next_ledger = next_ledger,
            indexer_lag = indexer_lag,
            "Poll cycle started"
        );

        // FIX(#335): Early return when caught up — avoids fetching a non-existent
        // future ledger, which would trigger an RPC error and unnecessary backoff.
        if next_ledger > latest_ledger.sequence {
            tracing::debug!(
                network = network_name,
                latest_ledger = latest_ledger.sequence,
                next_ledger = next_ledger,
                "Indexer caught up, nothing to process"
            );
            return Ok(());
        }

        // Check for reorg
        if self
            .reorg_handler
            .check_for_reorg(&self.rpc_client, state)
            .await?
        {
            warn!(
                network = network_name,
                "Reorg detected, recovering to checkpoint"
            );
            self.reorg_handler
                .recover_from_reorg(state, &self.state_manager)
                .await?;
            return Ok(());
        }

        // Process ledgers up to latest (but limit to prevent long processing cycles)
        let max_ledgers_per_cycle = 10;
        // Safe: next_ledger <= latest_ledger.sequence (guaranteed by the guard above)
        let ledgers_to_process = std::cmp::min(
            latest_ledger.sequence - next_ledger + 1,
            max_ledgers_per_cycle,
        );

        let mut total_contracts = 0;

        for i in 0..ledgers_to_process {
            let ledger_height = next_ledger + i;

            // Fetch ledger details to get the hash
            let ledger = self
                .rpc_client
                .get_ledger(ledger_height)
                .await
                .map_err(|e| {
                    error!(
                        network = network_name,
                        ledger = ledger_height,
                        error = %e,
                        "Failed to fetch ledger details"
                    );
                    e
                })?;

            // Fetch ledger operations
            match self.rpc_client.get_ledger_operations(ledger_height).await {
                Ok(operations) => {
                    info!(
                        network = network_name,
                        ledger = ledger_height,
                        operations = operations.len(),
                        "Fetched ledger operations"
                    );

                    // Detect contract deployments, then immediately drop `operations` so
                    // the Vec of serde_json::Value bodies does not linger for the rest of
                    // the ledger-processing block.
                    let deployments = detector::detect_contract_deployments(
                        &operations,
                        ledger_height,
                        &ledger.timestamp,
                    );
                    drop(operations);

                    if !deployments.is_empty() {
                        info!(
                            network = network_name,
                            ledger = ledger_height,
                            contracts = deployments.len(),
                            "Found contract deployments"
                        );

                        // Write to database
                        match self
                            .db_writer
                            .write_contracts_batch(&deployments, &self.config.network.network)
                            .await
                        {
                            Ok((new_count, duplicate_count)) => {
                                info!(
                                    network = network_name,
                                    ledger = ledger_height,
                                    new = new_count,
                                    duplicates = duplicate_count,
                                    "Contracts written to database"
                                );
                                total_contracts += new_count;
                            }
                            Err(e) => {
                                error!(
                                    network = network_name,
                                    ledger = ledger_height,
                                    error = %e,
                                    "Failed to write contracts"
                                );
                                return Err(e.into());
                            }
                        }
                    }

                    // Update state
                    state.last_indexed_ledger_height = ledger_height;
                    state.last_indexed_ledger_hash = Some(ledger.hash);
                    state.clear_failures();

                    // Check if we should update checkpoint
                    if self.reorg_handler.should_update_checkpoint(
                        ledger_height,
                        state.last_checkpoint_ledger_height,
                    ) {
                        state.update_checkpoint(ledger_height);
                        self.state_manager
                            .update_checkpoint(&self.config.network.network, ledger_height)
                            .await?;
                    }
                }
                Err(e) => {
                    error!(
                        network = network_name,
                        ledger = ledger_height,
                        error = %e,
                        "Failed to fetch ledger operations"
                    );
                    return Err(e.into());
                }
            }
        }

        // Persist state after successful cycle
        self.state_manager.update_state(state).await?;

        info!(
            network = network_name,
            processed = ledgers_to_process,
            new_contracts = total_contracts,
            indexer_lag = indexer_lag.saturating_sub(ledgers_to_process),
            "Poll cycle completed successfully"
        );

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing/logging with optional OTLP export.
    telemetry::init_tracing("soroban-registry-indexer");

    info!("Stellar Blockchain Indexer Service starting...");

    // Load configuration
    let config = ServiceConfig::from_env()?;

    // Spawn the marketplace USDC scanner as a parallel task if the
    // marketplace receiving address is configured. This is opt-in;
    // deployments without the marketplace simply don't get the task.
    // The scanner uses its own DB pool (small, since it only reads/
    // writes the single cursor row) and its own HTTP client. Logged
    // errors don't terminate the task — it self-heals on the next
    // poll cycle. See usdc_scanner.rs for the protocol.
    match usdc_scanner::ScannerConfig::from_env() {
        Ok(Some(scanner_cfg)) => {
            let scanner_pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(3)
                .connect(&config.database.connection_string)
                .await?;
            let scanner = usdc_scanner::UsdcScanner::new(scanner_cfg, scanner_pool);
            tokio::spawn(scanner.run());
            info!("USDC marketplace scanner spawned");
        }
        Ok(None) => {
            info!("MARKETPLACE_USDC_RECEIVING_ADDRESS not set; USDC scanner not started");
        }
        Err(e) => {
            warn!(error = %e, "USDC scanner config invalid; continuing without it");
        }
    }

    // Initialize service
    let mut service = IndexerService::new(config).await?;

    // Setup graceful shutdown signal handler
    let shutdown_signal = signal_support::create_shutdown_signal();

    // Run service
    tokio::select! {
        result = service.run() => {
            match result {
                Ok(_) => {
                    info!("Indexer service completed normally");
                    Ok(())
                }
                Err(e) => {
                    error!("Indexer service encountered fatal error: {}", e);
                    Err(e)
                }
            }
        }
        _ = shutdown_signal => {
            info!("Received shutdown signal, flushing state to database...");
            service.flush_state().await?;
            info!("Indexer state flushed successfully, exiting gracefully");
            Ok(())
        }
    }
}

/// Signal handling support
mod signal_support {
    pub async fn create_shutdown_signal() {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};

            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM");
                }
                _ = sigint.recv() => {
                    tracing::info!("Received SIGINT");
                }
            }
        }

        #[cfg(windows)]
        {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for Ctrl+C");
            tracing::info!("Received Ctrl+C");
        }
    }
}
