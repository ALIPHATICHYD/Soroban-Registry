//! Feature flag system for the registry API (#1007)
//!
//! Flags can be loaded from configuration (FEATURE_FLAGS_JSON env var) or
//! managed at runtime through the admin API. Each flag supports:
//! - Global on/off toggle
//! - Gradual rollout percentage
//! - Per-user targeting
//! - Evaluation metrics

use crate::config::FeatureFlagEntry;
use crate::state::AppState;
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct FeatureFlag {
    pub key: String,
    pub is_enabled: bool,
    pub rollout_percentage: u8,
    pub targeted_users: Vec<Uuid>,
}

#[derive(Default)]
pub struct FeatureFlagMetrics {
    pub evaluations: AtomicU64,
    pub hits: AtomicU64,
}

#[derive(Serialize)]
pub struct FlagStatus {
    pub key: String,
    pub is_enabled: bool,
    pub rollout_percentage: u8,
    pub evaluations: u64,
    pub hits: u64,
}

pub struct FeatureFlagManager {
    flags: RwLock<HashMap<String, FeatureFlag>>,
    metrics: RwLock<HashMap<String, Arc<FeatureFlagMetrics>>>,
}

impl FeatureFlagManager {
    pub fn new() -> Self {
        Self::from_config(&[])
    }

    /// Create a manager initialized from configuration entries.
    /// Falls back to safe defaults for any flags not present in config.
    pub fn from_config(entries: &[FeatureFlagEntry]) -> Self {
        let mut flags = HashMap::new();

        // Apply config entries
        for entry in entries {
            flags.insert(
                entry.key.clone(),
                FeatureFlag {
                    key: entry.key.clone(),
                    is_enabled: entry.is_enabled,
                    rollout_percentage: entry.rollout_percentage,
                    targeted_users: vec![],
                },
            );
        }

        // Ensure safe defaults for known flags if not in config
        flags
            .entry("new_dashboard".to_string())
            .or_insert(FeatureFlag {
                key: "new_dashboard".to_string(),
                is_enabled: false,
                rollout_percentage: 0,
                targeted_users: vec![],
            });

        Self {
            flags: RwLock::new(flags),
            metrics: RwLock::new(HashMap::new()),
        }
    }

    pub async fn is_enabled(&self, key: &str, user_id: Option<Uuid>) -> bool {
        let metrics_arc = {
            let mut metrics_guard = self.metrics.write().await;
            if !metrics_guard.contains_key(key) {
                metrics_guard.insert(key.to_string(), Arc::new(FeatureFlagMetrics::default()));
            }
            metrics_guard.get(key).cloned().unwrap()
        };

        metrics_arc.evaluations.fetch_add(1, Ordering::Relaxed);

        let flags = self.flags.read().await;
        if let Some(flag) = flags.get(key) {
            if !flag.is_enabled {
                return false;
            }

            if let Some(uid) = user_id {
                if flag.targeted_users.contains(&uid) {
                    metrics_arc.hits.fetch_add(1, Ordering::Relaxed);
                    return true;
                }
            }

            let enabled = if flag.rollout_percentage >= 100 {
                true
            } else if flag.rollout_percentage == 0 {
                false
            } else if let Some(uid) = user_id {
                let hash = self.hash_uuid(uid);
                (hash % 100) < (flag.rollout_percentage as u64)
            } else {
                rand::random::<u8>() % 100 < flag.rollout_percentage
            };

            if enabled {
                metrics_arc.hits.fetch_add(1, Ordering::Relaxed);
            }

            return enabled;
        }

        false
    }

    pub async fn update_flag(&self, flag: FeatureFlag) {
        info!("Updating feature flag: {}", flag.key);
        let mut flags = self.flags.write().await;
        flags.insert(flag.key.clone(), flag);
    }

    pub async fn get_metrics(&self) -> HashMap<String, (u64, u64)> {
        let metrics = self.metrics.read().await;
        let mut result = HashMap::new();
        for (key, m) in metrics.iter() {
            result.insert(
                key.clone(),
                (
                    m.evaluations.load(Ordering::Relaxed),
                    m.hits.load(Ordering::Relaxed),
                ),
            );
        }
        result
    }

    /// Return all registered flags with their current status and evaluation metrics
    pub async fn get_all_status(&self) -> Vec<FlagStatus> {
        let flags = self.flags.read().await;
        let metrics = self.metrics.read().await;
        let mut result = Vec::with_capacity(flags.len());
        for (key, flag) in flags.iter() {
            let (evaluations, hits) = metrics
                .get(key)
                .map(|m| {
                    (
                        m.evaluations.load(Ordering::Relaxed),
                        m.hits.load(Ordering::Relaxed),
                    )
                })
                .unwrap_or((0, 0));
            result.push(FlagStatus {
                key: key.clone(),
                is_enabled: flag.is_enabled,
                rollout_percentage: flag.rollout_percentage,
                evaluations,
                hits,
            });
        }
        result.sort_by(|a, b| a.key.cmp(&b.key));
        result
    }

    fn hash_uuid(&self, uid: Uuid) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        uid.hash(&mut hasher);
        hasher.finish()
    }
}

/// Axum middleware that rejects a request when the named feature flag is disabled.
/// Use for routes that should only be accessible when a feature flag is active.
pub async fn require_feature_flag(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // Extract the feature flag key from an extension set by the router
    let flag_key = match req.extensions().get::<String>() {
        Some(key) => key.clone(),
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "No feature flag key set").into_response();
        }
    };

    if state.feature_flags.is_enabled(&flag_key, None).await {
        next.run(req).await
    } else {
        (
            StatusCode::NOT_FOUND,
            format!(
                "This endpoint requires the '{}' feature flag to be enabled",
                flag_key
            ),
        )
            .into_response()
    }
}

/// GET /api/feature-flags — return status of all registered feature flags
pub async fn get_flag_status_handler(State(state): State<AppState>) -> Json<Vec<FlagStatus>> {
    Json(state.feature_flags.get_all_status().await)
}
