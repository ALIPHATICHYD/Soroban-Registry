use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{error, info};

use crate::state::RealtimeEvent;
use crate::state_monitor::anomaly_detector::AnomalyDetector;

/// Listens for blockchain events and contract state changes
pub struct EventListener {
    db: PgPool,
    event_broadcaster: broadcast::Sender<RealtimeEvent>,
    anomaly_detector: Arc<AnomalyDetector>,
    monitored_contracts: Arc<tokio::sync::RwLock<HashMap<String, (String, bool)>>>, // contract_id -> (network, active)
    poll_interval: Duration,
}

impl EventListener {
    pub fn new(
        db: PgPool,
        event_broadcaster: broadcast::Sender<RealtimeEvent>,
        anomaly_detector: Arc<AnomalyDetector>,
    ) -> Result<Self> {
        let poll_interval = Duration::from_secs(
            std::env::var("STATE_POLL_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
        );

        Ok(Self {
            db,
            event_broadcaster,
            anomaly_detector,
            monitored_contracts: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            poll_interval,
        })
    }

    /// Subscribe a contract for state change monitoring
    pub async fn subscribe_contract(&self, contract_id: &str, network: &str) -> Result<()> {
        let mut contracts = self.monitored_contracts.write().await;
        contracts.insert(contract_id.to_string(), (network.to_string(), true));

        info!("Contract {} subscribed for state monitoring", contract_id);
        Ok(())
    }

    /// Unsubscribe a contract
    pub async fn unsubscribe_contract(&self, contract_id: &str, _network: &str) -> Result<()> {
        let mut contracts = self.monitored_contracts.write().await;
        contracts.remove(contract_id);

        info!(
            "Contract {} unsubscribed from state monitoring",
            contract_id
        );
        Ok(())
    }

    /// Start the event listener loop (run as background task)
    pub async fn run(self: Arc<Self>) -> Result<()> {
        info!("Starting event listener for contract state changes");

        let mut ticker = interval(self.poll_interval);

        loop {
            ticker.tick().await;

            // Process new state changes
            if let Err(e) = self.poll_state_changes().await {
                error!("Error polling state changes: {}", e);
            }
        }
    }

    /// Poll for new state changes from database
    async fn poll_state_changes(&self) -> Result<()> {
        let contracts = self.monitored_contracts.read().await;
        if contracts.is_empty() {
            return Ok(());
        }

        let contract_ids: Vec<String> = contracts.keys().cloned().collect();

        for contract_id in &contract_ids {
            // Query for state changes since last check
            let changes: Vec<crate::state_monitor::StateChangeEntry> =
                sqlx::query_as::<_, crate::state_monitor::StateChangeEntry>(
                    r#"
                SELECT
                    sh.id,
                    sh.contract_id,
                    sh.state_key,
                    sh.old_value,
                    sh.new_value,
                    sh.value_type,
                    sh.transaction_hash,
                    sh.ledger_index,
                    sh.contract_version,
                    sh.created_at,
                    sh.metadata
                FROM contract_state_history sh
                WHERE sh.contract_id = $1
                  AND sh.created_at > NOW() - INTERVAL '1 minute'
                ORDER BY sh.created_at ASC
                "#,
                )
                .bind(uuid::Uuid::parse_str(contract_id)?)
                .fetch_all(&self.db)
                .await?;

            for change in changes {
                // Convert to RealtimeEvent
                let event = RealtimeEvent::MetadataUpdated {
                    contract_id: contract_id.clone(),
                    timestamp: change.created_at.to_rfc3339(),
                    changes: serde_json::json!({
                        "state_key": change.state_key,
                        "old_value": change.old_value,
                        "new_value": change.new_value,
                        "transaction_hash": change.transaction_hash,
                        "ledger_index": change.ledger_index,
                    }),
                    visibility: crate::state::ContractEventVisibility::Public,
                };

                // Broadcast event
                if let Err(e) = self.event_broadcaster.send(event) {
                    error!("Failed to broadcast state change event: {}", e);
                }

                // Check for anomalies
                if let Err(e) = self.anomaly_detector.analyze_state_change(&change).await {
                    error!("Anomaly detection error: {}", e);
                }
            }
        }

        Ok(())
    }
}
