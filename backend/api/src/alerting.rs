use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub id: String,
    pub source: String,
    pub message: String,
    pub severity: AlertSeverity,
    pub timestamp: DateTime<Utc>,
}

pub struct AlertManager {
    client: Client,
    slack_webhook_url: Option<String>,
    pagerduty_routing_key: Option<String>,
    last_alerts: RwLock<HashMap<String, DateTime<Utc>>>,
    dedup_interval_secs: u64,
}

impl AlertManager {
    pub fn new() -> Self {
        let slack_webhook_url = std::env::var("SLACK_WEBHOOK_URL").ok();
        let pagerduty_routing_key = std::env::var("PAGERDUTY_ROUTING_KEY").ok();

        Self {
            client: Client::new(),
            slack_webhook_url,
            pagerduty_routing_key,
            last_alerts: RwLock::new(HashMap::new()),
            dedup_interval_secs: 300, // 5 minutes deduplication
        }
    }

    pub async fn dispatch_alert(&self, alert: Alert) {
        let dedup_key = format!("{}_{}", alert.source, alert.message);

        // Deduplication check
        {
            let mut last_alerts = self.last_alerts.write().await;
            if let Some(last_time) = last_alerts.get(&dedup_key) {
                if Utc::now().signed_duration_since(*last_time).num_seconds()
                    < self.dedup_interval_secs as i64
                {
                    // Suppress duplicate alert
                    return;
                }
            }
            last_alerts.insert(dedup_key.clone(), Utc::now());
        }

        // Log the alert
        match alert.severity {
            AlertSeverity::Info => info!("ALERT: [{}] {}", alert.source, alert.message),
            AlertSeverity::Warning => warn!("ALERT: [{}] {}", alert.source, alert.message),
            AlertSeverity::Critical => error!("ALERT: [{}] {}", alert.source, alert.message),
        }

        // Send to configured channels
        if alert.severity == AlertSeverity::Critical || alert.severity == AlertSeverity::Warning {
            if let Some(url) = &self.slack_webhook_url {
                self.send_to_slack(url, &alert).await;
            }
        }

        if alert.severity == AlertSeverity::Critical {
            if let Some(key) = &self.pagerduty_routing_key {
                self.send_to_pagerduty(key, &alert).await;
            }
        }
    }

    async fn send_to_slack(&self, url: &str, alert: &Alert) {
        #[derive(Serialize)]
        struct SlackPayload {
            text: String,
        }

        let emoji = match alert.severity {
            AlertSeverity::Critical => "🚨",
            AlertSeverity::Warning => "⚠️",
            _ => "ℹ️",
        };

        let text = format!(
            "{} *[{:?} - {}]*\n{}",
            emoji, alert.severity, alert.source, alert.message
        );

        let payload = SlackPayload { text };

        match self.client.post(url).json(&payload).send().await {
            Ok(res) => {
                if !res.status().is_success() {
                    error!("Failed to send Slack alert. Status: {}", res.status());
                }
            }
            Err(e) => error!("Failed to send Slack alert: {}", e),
        }
    }

    async fn send_to_pagerduty(&self, routing_key: &str, alert: &Alert) {
        #[derive(Serialize)]
        struct PagerDutyPayload {
            routing_key: String,
            event_action: String,
            payload: PagerDutyEventPayload,
        }

        #[derive(Serialize)]
        struct PagerDutyEventPayload {
            summary: String,
            source: String,
            severity: String,
        }

        let payload = PagerDutyPayload {
            routing_key: routing_key.to_string(),
            event_action: "trigger".to_string(),
            payload: PagerDutyEventPayload {
                summary: alert.message.clone(),
                source: alert.source.clone(),
                severity: "critical".to_string(),
            },
        };

        let url = "https://events.pagerduty.com/v2/enqueue";

        match self.client.post(url).json(&payload).send().await {
            Ok(res) => {
                if !res.status().is_success() {
                    error!("Failed to send PagerDuty alert. Status: {}", res.status());
                }
            }
            Err(e) => error!("Failed to send PagerDuty alert: {}", e),
        }
    }
}
