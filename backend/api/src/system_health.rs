use crate::alerting::{Alert, AlertManager, AlertSeverity};
use crate::cache::CacheLayer;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{self, Duration};
use tracing::info;

pub struct SystemHealthMonitor {
    pool: PgPool,
    cache: Arc<CacheLayer>,
    alert_mgr: Arc<AlertManager>,
}

impl SystemHealthMonitor {
    pub fn new(pool: PgPool, cache: Arc<CacheLayer>, alert_mgr: Arc<AlertManager>) -> Self {
        Self {
            pool,
            cache,
            alert_mgr,
        }
    }

    pub async fn run(&self) {
        info!("Starting System Health Monitor background task");
        let mut interval = time::interval(Duration::from_secs(30)); // Continuous monitoring (every 30 seconds)

        loop {
            interval.tick().await;
            self.check_database().await;
            self.check_cache().await;
        }
    }

    async fn check_database(&self) {
        match sqlx::query("SELECT 1").execute(&self.pool).await {
            Ok(_) => {
                // Database is healthy
            }
            Err(e) => {
                let alert = Alert {
                    id: uuid::Uuid::new_v4().to_string(),
                    source: "Database".to_string(),
                    message: format!("Database health check failed: {}", e),
                    severity: AlertSeverity::Critical,
                    timestamp: Utc::now(),
                };
                self.alert_mgr.dispatch_alert(alert).await;
            }
        }
    }

    async fn check_cache(&self) {
        match self.cache.ping().await {
            Ok(_) => {
                // Cache is healthy
            }
            Err(e) => {
                let alert = Alert {
                    id: uuid::Uuid::new_v4().to_string(),
                    source: "Cache".to_string(),
                    message: format!("Cache health check failed: {:?}", e),
                    severity: AlertSeverity::Warning,
                    timestamp: Utc::now(),
                };
                self.alert_mgr.dispatch_alert(alert).await;
            }
        }
    }
}

pub fn spawn_system_health_monitor(
    pool: PgPool,
    cache: Arc<CacheLayer>,
    alert_mgr: Arc<AlertManager>,
) {
    let monitor = SystemHealthMonitor::new(pool, cache, alert_mgr);
    tokio::spawn(async move {
        monitor.run().await;
    });
}
