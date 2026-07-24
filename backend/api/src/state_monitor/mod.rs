// State Monitor module - Real-Time Contract State Monitor
// Tracks state changes, detects anomalies, broadcasts via WebSocket

pub mod anomaly_detector;
pub mod event_listener;
pub mod handlers;
pub mod point_in_time;

use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::state_monitor::{anomaly_detector::AnomalyDetector, event_listener::EventListener};

use anyhow::Result;
use uuid::Uuid;

/// Service that manages contract state monitoring
pub struct StateMonitorService {
    db: PgPool,
    event_broadcaster: broadcast::Sender<crate::state::RealtimeEvent>,
    event_listener: Arc<EventListener>,
    anomaly_detector: Arc<AnomalyDetector>,
}

impl StateMonitorService {
    pub fn new(
        db: PgPool,
        event_broadcaster: broadcast::Sender<crate::state::RealtimeEvent>,
    ) -> Result<Self, anyhow::Error> {
        let anomaly_detector = Arc::new(AnomalyDetector::new(db.clone()));
        let event_listener = EventListener::new(
            db.clone(),
            event_broadcaster.clone(),
            anomaly_detector.clone(),
        )?;

        Ok(Self {
            db,
            event_broadcaster,
            event_listener: Arc::new(event_listener),
            anomaly_detector,
        })
    }

    /// Start the monitoring background task
    pub async fn run(self: Arc<Self>) -> Result<()> {
        tracing::info!("Starting state monitor service");
        self.event_listener.clone().run().await?;
        Ok(())
    }

    /// Start monitoring state changes for a specific contract
    pub async fn start_contract_monitoring(
        &self,
        contract_id: &str,
        network: &str,
    ) -> Result<(), anyhow::Error> {
        self.event_listener
            .subscribe_contract(contract_id, network)
            .await
    }

    /// Stop monitoring a contract
    pub async fn stop_contract_monitoring(
        &self,
        contract_id: &str,
        network: &str,
    ) -> Result<(), anyhow::Error> {
        self.event_listener
            .unsubscribe_contract(contract_id, network)
            .await
    }

    /// Get recent state changes for a contract
    pub async fn get_state_history(
        &self,
        contract_id: &str,
        limit: i32,
    ) -> Result<Vec<StateChangeEntry>, anyhow::Error> {
        let uuid = Uuid::parse_str(contract_id)
            .map_err(|_| anyhow::anyhow!("Invalid contract ID format"))?;

        let changes = sqlx::query_as::<_, StateChangeEntry>(
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
            ORDER BY sh.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(uuid)
        .bind(limit as i64)
        .fetch_all(&self.db)
        .await?;

        Ok(changes)
    }

    /// Get anomalies — optionally filtered by contract_id and/or severity
    pub async fn get_anomalies(
        &self,
        contract_id: Option<&str>,
        severity: Option<&str>,
        limit: i32,
    ) -> Result<Vec<AnomalyInfo>, anyhow::Error> {
        // Four explicit branches avoids dynamic PgArguments accumulation,
        // which sqlx doesn't support as a .bind() argument.
        let anomalies = match (contract_id, severity) {
            (Some(cid_str), Some(sev)) => {
                let cid = Uuid::parse_str(cid_str)
                    .map_err(|_| anyhow::anyhow!("Invalid contract ID format"))?;
                sqlx::query_as::<_, AnomalyInfo>(
                    r#"
                    SELECT id, contract_id, anomaly_type, severity, description,
                           state_key, old_value, new_value, detected_at,
                           is_resolved, resolution_notes, metadata
                    FROM state_anomalies
                    WHERE is_resolved = FALSE
                      AND contract_id = $1
                      AND severity = $2
                    ORDER BY detected_at DESC
                    LIMIT $3
                    "#,
                )
                .bind(cid)
                .bind(sev)
                .bind(limit as i64)
                .fetch_all(&self.db)
                .await?
            }
            (Some(cid_str), None) => {
                let cid = Uuid::parse_str(cid_str)
                    .map_err(|_| anyhow::anyhow!("Invalid contract ID format"))?;
                sqlx::query_as::<_, AnomalyInfo>(
                    r#"
                    SELECT id, contract_id, anomaly_type, severity, description,
                           state_key, old_value, new_value, detected_at,
                           is_resolved, resolution_notes, metadata
                    FROM state_anomalies
                    WHERE is_resolved = FALSE
                      AND contract_id = $1
                    ORDER BY detected_at DESC
                    LIMIT $2
                    "#,
                )
                .bind(cid)
                .bind(limit as i64)
                .fetch_all(&self.db)
                .await?
            }
            (None, Some(sev)) => {
                sqlx::query_as::<_, AnomalyInfo>(
                    r#"
                    SELECT id, contract_id, anomaly_type, severity, description,
                           state_key, old_value, new_value, detected_at,
                           is_resolved, resolution_notes, metadata
                    FROM state_anomalies
                    WHERE is_resolved = FALSE
                      AND severity = $1
                    ORDER BY detected_at DESC
                    LIMIT $2
                    "#,
                )
                .bind(sev)
                .bind(limit as i64)
                .fetch_all(&self.db)
                .await?
            }
            (None, None) => {
                sqlx::query_as::<_, AnomalyInfo>(
                    r#"
                    SELECT id, contract_id, anomaly_type, severity, description,
                           state_key, old_value, new_value, detected_at,
                           is_resolved, resolution_notes, metadata
                    FROM state_anomalies
                    WHERE is_resolved = FALSE
                    ORDER BY detected_at DESC
                    LIMIT $1
                    "#,
                )
                .bind(limit as i64)
                .fetch_all(&self.db)
                .await?
            }
        };

        Ok(anomalies)
    }

    /// Resolve an anomaly
    pub async fn resolve_anomaly(
        &self,
        anomaly_id: &str,
        resolution_notes: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let uuid = Uuid::parse_str(anomaly_id)
            .map_err(|_| anyhow::anyhow!("Invalid anomaly ID format"))?;

        sqlx::query(
            r#"
            UPDATE state_anomalies
            SET is_resolved = TRUE,
                resolved_at = NOW(),
                resolution_notes = $1
            WHERE id = $2
            "#,
        )
        .bind(resolution_notes)
        .bind(uuid)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

// Helper models
#[derive(sqlx::FromRow, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateChangeEntry {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub state_key: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub value_type: Option<String>,
    pub transaction_hash: Option<String>,
    pub ledger_index: Option<i64>,
    pub contract_version: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(sqlx::FromRow, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnomalyInfo {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub anomaly_type: String,
    pub severity: String,
    pub description: String,
    pub state_key: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub detected_at: chrono::DateTime<chrono::Utc>,
    pub is_resolved: bool,
    pub resolution_notes: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
