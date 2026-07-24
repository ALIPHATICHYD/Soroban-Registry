use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

use crate::state_monitor::StateChangeEntry;

/// Detects anomalies in contract state changes
/// Uses statistical analysis and rule-based detection
pub struct AnomalyDetector {
    db: PgPool,
    rules: Vec<Box<dyn AnomalyRule + Send + Sync>>,
}

impl AnomalyDetector {
    pub fn new(db: PgPool) -> Self {
        let mut detector = Self {
            db,
            rules: Vec::new(),
        };

        // Register built-in rules
        detector.register_default_rules();

        detector
    }

    fn register_default_rules(&mut self) {
        self.rules.push(Box::new(SuddenSpikeRule::new()));
        self.rules.push(Box::new(UnusualPatternRule::new()));
        self.rules.push(Box::new(UnexpectedValueChangeRule::new()));
    }

    /// Analyze a state change for anomalies
    pub async fn analyze_state_change(&self, change: &StateChangeEntry) -> Result<()> {
        for rule in &self.rules {
            if let Some(anomaly) = rule.check(change, &self.db).await? {
                let anomaly_log = anomaly.clone();
                // Save anomaly to database
                self.record_anomaly(anomaly).await?;
                info!(
                    "Anomaly detected: {} for contract {} key={}",
                    anomaly_log.anomaly_type,
                    anomaly_log.contract_id,
                    anomaly_log.state_key.as_deref().unwrap_or("N/A")
                );
            }
        }

        Ok(())
    }

    /// Record an anomaly in the database
    async fn record_anomaly(&self, anomaly: AnomalyRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO state_anomalies (
                contract_id, anomaly_type, severity, description,
                state_key, old_value, new_value, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(anomaly.contract_id)
        .bind(anomaly.anomaly_type)
        .bind(anomaly.severity)
        .bind(anomaly.description)
        .bind(anomaly.state_key)
        .bind(anomaly.old_value)
        .bind(anomaly.new_value)
        .bind(anomaly.metadata)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

// Anomaly rule trait
#[async_trait::async_trait]
trait AnomalyRule: Send + Sync {
    fn name(&self) -> &'static str;
    async fn check(&self, change: &StateChangeEntry, db: &PgPool) -> Result<Option<AnomalyRecord>>;
}

// Sudden value spike detector
struct SuddenSpikeRule {
    threshold: f64, // percentage increase that triggers
    min_occurrences: i32,
}

impl SuddenSpikeRule {
    fn new() -> Self {
        Self {
            threshold: 100.0, // 100% increase
            min_occurrences: 3,
        }
    }
}

#[async_trait::async_trait]
impl AnomalyRule for SuddenSpikeRule {
    fn name(&self) -> &'static str {
        "sudden_spike"
    }

    async fn check(&self, change: &StateChangeEntry, db: &PgPool) -> Result<Option<AnomalyRecord>> {
        let (Some(old_val), Some(new_val)) = (&change.old_value, &change.new_value) else {
            return Ok(None);
        };

        // Try to parse as numbers
        if let (Ok(old_num), Ok(new_num)) = (old_val.parse::<f64>(), new_val.parse::<f64>()) {
            if old_num > 0.0 {
                let increase_pct = ((new_num - old_num) / old_num) * 100.0;

                if increase_pct >= self.threshold {
                    // Check frequency of changes in last hour
                    let count: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM contract_state_history 
                         WHERE contract_id = $1 AND state_key = $2 
                         AND created_at > NOW() - INTERVAL '1 hour'",
                    )
                    .bind(change.contract_id)
                    .bind(&change.state_key)
                    .fetch_one(db)
                    .await?;

                    if count >= i64::from(self.min_occurrences) {
                        return Ok(Some(AnomalyRecord {
                            contract_id: change.contract_id,
                            anomaly_type: "sudden_spike".to_string(),
                            severity: "high".to_string(),
                            description: format!(
                                "Value for key '{}' increased by {:.1}% ({} -> {}) with {} changes in past hour",
                                change.state_key, increase_pct, old_val, new_val, count
                            ),
                            state_key: Some(change.state_key.clone()),
                            old_value: Some(old_val.clone()),
                            new_value: Some(new_val.clone()),
                            metadata: serde_json::json!({
                                "threshold": self.threshold,
                                "increase_pct": increase_pct,
                                "change_count": count,
                            }),
                        }));
                    }
                }
            }
        }

        Ok(None)
    }
}

// Unusual pattern detector (multiple changes in short time)
struct UnusualPatternRule {
    max_changes_per_minute: i32,
}

impl UnusualPatternRule {
    fn new() -> Self {
        Self {
            max_changes_per_minute: 10,
        }
    }
}

#[async_trait::async_trait]
impl AnomalyRule for UnusualPatternRule {
    fn name(&self) -> &'static str {
        "unusual_pattern"
    }

    async fn check(&self, change: &StateChangeEntry, db: &PgPool) -> Result<Option<AnomalyRecord>> {
        // Count changes to this contract's state in the last minute
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM contract_state_history 
             WHERE contract_id = $1 
             AND created_at > NOW() - INTERVAL '1 minute'",
        )
        .bind(change.contract_id)
        .fetch_one(db)
        .await?;

        if count > i64::from(self.max_changes_per_minute) {
            return Ok(Some(AnomalyRecord {
                contract_id: change.contract_id,
                anomaly_type: "high_frequency_changes".to_string(),
                severity: if count > 50 {
                    "critical".to_string()
                } else {
                    "medium".to_string()
                },
                description: format!(
                    "Contract state changed {} times in the last minute (threshold: {})",
                    count, self.max_changes_per_minute
                ),
                state_key: None,
                old_value: None,
                new_value: None,
                metadata: serde_json::json!({
                    "change_count": count,
                    "threshold": self.max_changes_per_minute,
                }),
            }));
        }

        Ok(None)
    }
}

// Unexpected value change rule
struct UnexpectedValueChangeRule;

impl UnexpectedValueChangeRule {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl AnomalyRule for UnexpectedValueChangeRule {
    fn name(&self) -> &'static str {
        "unexpected_value_change"
    }

    async fn check(&self, change: &StateChangeEntry, db: &PgPool) -> Result<Option<AnomalyRecord>> {
        // Detect when a storage value is set to zero or empty unexpectedly
        let new_val = change.new_value.as_ref();
        if let Some(val) = new_val {
            if val.is_empty() || val == "0" || val == "0x0" {
                // Check if this value typically has non-zero data
                let avg_value: Option<String> = sqlx::query_scalar(
                    "SELECT AVG(CAST(new_value AS INTEGER)) 
                     FROM contract_state_history 
                     WHERE contract_id = $1 AND state_key = $2 
                     AND created_at > NOW() - INTERVAL '24 hours'
                     AND new_value ~ '^[0-9]+$'",
                )
                .bind(change.contract_id)
                .bind(&change.state_key)
                .fetch_optional(db)
                .await?;

                if let Some(avg_str) = avg_value {
                    if let Ok(avg) = avg_str.parse::<f64>() {
                        if avg > 10.0 {
                            return Ok(Some(AnomalyRecord {
                                contract_id: change.contract_id,
                                anomaly_type: "unexpected_zero_value".to_string(),
                                severity: "medium".to_string(),
                                description: format!(
                                    "State key '{}' changed to '{}', but average value over 24h is {:.2}",
                                    change.state_key, val, avg
                                ),
                                state_key: Some(change.state_key.clone()),
                                old_value: change.old_value.clone(),
                                new_value: Some(val.clone()),
                                metadata: serde_json::json!({
                                    "average_24h": avg,
                                }),
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

/// Record representing a detected anomaly
#[derive(Debug, Clone)]
struct AnomalyRecord {
    contract_id: uuid::Uuid,
    anomaly_type: String,
    severity: String,
    description: String,
    state_key: Option<String>,
    old_value: Option<String>,
    new_value: Option<String>,
    metadata: serde_json::Value,
}
