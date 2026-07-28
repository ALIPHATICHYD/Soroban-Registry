#![allow(dead_code, unused)]

// Library exports for indexer module
pub mod backoff;
pub mod config;
pub mod db;
pub mod detector;
pub mod reorg;
pub mod rpc;
pub mod state;
pub mod telemetry;
pub mod usdc_scanner;

pub use backoff::ExponentialBackoff;
pub use config::{DatabaseConfig, NetworkConfig, ServiceConfig};
pub use db::DatabaseWriter;
pub use detector::detect_contract_deployments;
pub use reorg::ReorgHandler;
pub use rpc::{ContractDeployment, Ledger, Operation, StellarRpcClient};
pub use state::{IndexerState, StateManager};
pub use telemetry::inject_current_trace_context;
