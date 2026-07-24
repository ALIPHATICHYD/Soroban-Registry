use crate::net::RequestBuilderExt;
use std::fmt;
use std::str::FromStr;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl FromStr for Severity {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Self::Critical),
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            _ => bail!(
                "invalid severity: {} (expected critical|high|medium|low)",
                s
            ),
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Critical => write!(f, "CRITICAL"),
            Self::High => write!(f, "HIGH"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::Low => write!(f, "LOW"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPatch {
    pub id: Uuid,
    pub target_version: String,
    pub severity: Severity,
    pub new_wasm_hash: String,
    pub rollout_percentage: u8,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchAudit {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub patch_id: Uuid,
    pub applied_at: DateTime<Utc>,
}

fn lookup_items(data: &serde_json::Value) -> Vec<serde_json::Value> {
    data.get("items")
        .and_then(serde_json::Value::as_array)
        .or_else(|| data.get("contracts").and_then(serde_json::Value::as_array))
        .cloned()
        .unwrap_or_default()
}

fn lookup_total(data: &serde_json::Value) -> usize {
    data.get("total")
        .and_then(serde_json::Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or_else(|| lookup_items(data).len())
}

fn ensure_single_contract_match(data: &serde_json::Value, wasm_hash: &str) -> Result<usize> {
    let total = lookup_total(data);

    match total {
        0 => bail!(
            "no contract found for patch target wasm hash '{}'; verify the patch target version/hash before applying",
            wasm_hash
        ),
        1 => Ok(total),
        n => bail!(
            "ambiguous contract lookup for patch target wasm hash '{}': {} contracts matched; apply by resolving the target contract before patching",
            wasm_hash,
            n
        ),
    }
}

pub struct PatchManager;

impl PatchManager {
    pub fn check_rollout(applied: usize, total: usize, rollout_pct: u8) -> bool {
        if total == 0 {
            return false;
        }
        let max_allowed = (total as f64 * rollout_pct as f64 / 100.0).ceil() as usize;
        applied < max_allowed
    }

    pub async fn create(
        api_url: &str,
        version: &str,
        hash: &str,
        severity: Severity,
        rollout: u8,
    ) -> Result<SecurityPatch> {
        let client = crate::net::client();
        let payload = serde_json::json!({
            "target_version": version,
            "severity": severity,
            "new_wasm_hash": hash,
            "rollout_percentage": rollout,
        });

        let resp = client
            .post(format!("{}/api/patches", api_url))
            .json(&payload)
            .send_with_retry()
            .await?;

        if !resp.status().is_success() {
            bail!("failed to create patch: {}", resp.text().await?);
        }

        Ok(resp.json().await?)
    }

    pub async fn find_vulnerable(
        api_url: &str,
        patch_id: &str,
    ) -> Result<(SecurityPatch, Vec<serde_json::Value>)> {
        let client = crate::net::client();

        let patch_resp = client
            .get(format!("{}/api/patches/{}", api_url, patch_id))
            .send_with_retry()
            .await?;

        if !patch_resp.status().is_success() {
            bail!("patch not found: {}", patch_id);
        }

        let patch: SecurityPatch = patch_resp.json().await?;

        let contracts_resp = client
            .get(format!(
                "{}/api/contracts?wasm_hash={}",
                api_url, patch.target_version
            ))
            .send_with_retry()
            .await?;

        let data: serde_json::Value = contracts_resp.json().await?;
        let contracts = lookup_items(&data);

        Ok((patch, contracts))
    }

    pub async fn apply(api_url: &str, contract_id: &str, patch_id: &str) -> Result<PatchAudit> {
        let client = crate::net::client();

        let patch_resp = client
            .get(format!("{}/api/patches/{}", api_url, patch_id))
            .send_with_retry()
            .await?;

        if !patch_resp.status().is_success() {
            bail!("patch not found: {}", patch_id);
        }

        let patch: SecurityPatch = patch_resp.json().await?;

        let audits_resp = client
            .get(format!("{}/api/patches/{}/audits", api_url, patch_id))
            .send_with_retry()
            .await?;

        let audits_data: serde_json::Value = audits_resp.json().await?;
        let applied = audits_data["total"].as_u64().unwrap_or(0) as usize;

        let contracts_resp = client
            .get(format!(
                "{}/api/contracts?wasm_hash={}",
                api_url, patch.target_version
            ))
            .send_with_retry()
            .await?;

        let contracts_data: serde_json::Value = contracts_resp.json().await?;
        let total = ensure_single_contract_match(&contracts_data, &patch.target_version)?;

        if !Self::check_rollout(applied, total, patch.rollout_percentage) {
            bail!(
                "rollout quota exceeded: {}/{} ({}% of {} eligible)",
                applied,
                (total as f64 * patch.rollout_percentage as f64 / 100.0).ceil() as usize,
                patch.rollout_percentage,
                total
            );
        }

        let payload = serde_json::json!({
            "contract_id": contract_id,
            "patch_id": patch_id,
        });

        let resp = client
            .post(format!("{}/api/patches/{}/apply", api_url, patch_id))
            .json(&payload)
            .send_with_retry()
            .await?;

        if !resp.status().is_success() {
            bail!("failed to apply patch: {}", resp.text().await?);
        }

        Ok(resp.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_parse_valid() {
        assert_eq!(Severity::from_str("critical").unwrap(), Severity::Critical);
        assert_eq!(Severity::from_str("HIGH").unwrap(), Severity::High);
        assert_eq!(Severity::from_str("Medium").unwrap(), Severity::Medium);
        assert_eq!(Severity::from_str("low").unwrap(), Severity::Low);
    }

    #[test]
    fn severity_parse_invalid() {
        assert!(Severity::from_str("extreme").is_err());
        assert!(Severity::from_str("").is_err());
        assert!(Severity::from_str("CRIT").is_err());
    }

    #[test]
    fn severity_display() {
        assert_eq!(Severity::Critical.to_string(), "CRITICAL");
        assert_eq!(Severity::Low.to_string(), "LOW");
    }

    #[test]
    fn rollout_zero_percent() {
        assert!(!PatchManager::check_rollout(0, 100, 0));
    }

    #[test]
    fn rollout_full() {
        assert!(PatchManager::check_rollout(0, 10, 100));
        assert!(PatchManager::check_rollout(9, 10, 100));
        assert!(!PatchManager::check_rollout(10, 10, 100));
    }

    #[test]
    fn rollout_fifty_percent() {
        assert!(PatchManager::check_rollout(0, 10, 50));
        assert!(PatchManager::check_rollout(4, 10, 50));
        assert!(!PatchManager::check_rollout(5, 10, 50));
    }

    #[test]
    fn rollout_rounds_up() {
        assert!(PatchManager::check_rollout(0, 3, 50));
        assert!(PatchManager::check_rollout(1, 3, 50));
        assert!(!PatchManager::check_rollout(2, 3, 50));
    }

    #[test]
    fn rollout_empty_total() {
        assert!(!PatchManager::check_rollout(0, 0, 100));
    }

    #[test]
    fn rollout_one_contract() {
        assert!(PatchManager::check_rollout(0, 1, 1));
        assert!(!PatchManager::check_rollout(1, 1, 1));
    }

    #[test]
    fn contract_lookup_rejects_empty_matches() {
        let data = serde_json::json!({ "items": [], "total": 0 });
        let err = ensure_single_contract_match(&data, "hash-a")
            .unwrap_err()
            .to_string();

        assert!(err.contains("no contract found"));
        assert!(err.contains("hash-a"));
    }

    #[test]
    fn contract_lookup_rejects_ambiguous_matches() {
        let data = serde_json::json!({
            "items": [{"id": "c1"}, {"id": "c2"}],
            "total": 2
        });
        let err = ensure_single_contract_match(&data, "hash-b")
            .unwrap_err()
            .to_string();

        assert!(err.contains("ambiguous contract lookup"));
        assert!(err.contains("2 contracts matched"));
    }

    #[test]
    fn contract_lookup_accepts_single_match_without_total() {
        let data = serde_json::json!({ "contracts": [{"id": "c1"}] });

        assert_eq!(ensure_single_contract_match(&data, "hash-c").unwrap(), 1);
    }
}
