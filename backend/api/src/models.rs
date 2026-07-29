// src/models.rs
// Shared data types for the Soroban Security Audit system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────
// Checklist definition types (static / compile-time)
// ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum CheckCategory {
    InputValidation,
    StateManagement,
    AccessControl,
    Reentrancy,
    NumericalSafety,
    AuthenticationAuthorization,
    DataSerialization,
    ErrorHandling,
    StoragePatterns,
    TokenSafety,
    EventLogging,
    Upgradeability,
    CrossContractCalls,
    ResourceLimits,
}

impl std::fmt::Display for CheckCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CheckCategory::InputValidation => "Input Validation",
            CheckCategory::StateManagement => "State Management",
            CheckCategory::AccessControl => "Access Control",
            CheckCategory::Reentrancy => "Reentrancy",
            CheckCategory::NumericalSafety => "Numerical Safety",
            CheckCategory::AuthenticationAuthorization => "Authentication & Authorization",
            CheckCategory::DataSerialization => "Data Serialization",
            CheckCategory::ErrorHandling => "Error Handling",
            CheckCategory::StoragePatterns => "Storage Patterns",
            CheckCategory::TokenSafety => "Token Safety",
            CheckCategory::EventLogging => "Event Logging",
            CheckCategory::Upgradeability => "Upgradeability",
            CheckCategory::CrossContractCalls => "Cross-Contract Calls",
            CheckCategory::ResourceLimits => "Resource Limits",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DetectionMethod {
    Automatic { patterns: Vec<String> },
    Manual,
    SemiAutomatic { patterns: Vec<String> },
}

/// A static checklist item definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: &'static str,
    pub category: CheckCategory,
    pub title: &'static str,
    pub description: &'static str,
    pub severity: Severity,
    pub detection: DetectionMethod,
    pub remediation: &'static str,
    pub references: Vec<&'static str>,
}

// ─────────────────────────────────────────────────────────
// Runtime / database types
// ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum CheckStatus {
    Passed,
    Failed,
    NotApplicable,
    Pending,
}

impl Default for CheckStatus {
    fn default() -> Self {
        CheckStatus::Pending
    }
}

/// One row in `audit_checks` — per-check status for an audit
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditCheckRow {
    pub id: Uuid,
    pub audit_id: Uuid,
    pub check_id: String,
    pub status: CheckStatus,
    pub notes: Option<String>,
    pub auto_detected: bool,
    pub evidence: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// One row in `security_audits`
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditRecord {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub contract_source: Option<String>, // raw source for auto-detection
    pub auditor: String,
    pub audit_date: DateTime<Utc>,
    pub overall_score: f64,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────
// API request / response types
// ─────────────────────────────────────────────────────────

/// Body for POST /contracts/:id/security-audit
#[derive(Debug, Deserialize)]
pub struct CreateAuditRequest {
    /// Auditor name or Stellar address
    pub auditor: String,
    /// Optional: paste the contract source for auto-detection
    pub source_code: Option<String>,
}

/// Body for PATCH /contracts/:id/security-audit/:audit_id/checks/:check_id
#[derive(Debug, Deserialize)]
pub struct UpdateCheckRequest {
    pub status: CheckStatus,
    pub notes: Option<String>,
}

/// Full audit response sent to client — includes static metadata + live status
#[derive(Debug, Serialize)]
pub struct AuditResponse {
    pub audit: AuditRecord,
    pub checks: Vec<CheckWithStatus>,
    pub category_scores: Vec<CategoryScore>,
    pub auto_detected_count: usize,
}

/// A checklist item merged with its current audit status
#[derive(Debug, Serialize)]
pub struct CheckWithStatus {
    // ── static metadata ──
    pub id: &'static str,
    pub category: String,
    pub title: &'static str,
    pub description: &'static str,
    pub severity: String,
    pub detection_type: &'static str,
    pub auto_patterns: Vec<String>,
    pub remediation: &'static str,
    pub references: Vec<&'static str>,
    // ── live audit status ──
    pub status: CheckStatus,
    pub notes: Option<String>,
    pub auto_detected: bool,
    pub evidence: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CategoryScore {
    pub category: String,
    pub score: f64,
    pub passed: usize,
    pub total: usize,
    pub failed_critical: usize,
    pub failed_high: usize,
}

/// Minimal score info embedded on contract cards
#[derive(Debug, Serialize, FromRow)]
pub struct ContractSecuritySummary {
    pub audit_id: Uuid,
    pub audit_date: DateTime<Utc>,
    pub auditor: String,
    pub overall_score: f64,
    pub score_badge: String,
}

/// Markdown export request
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    /// Include full check descriptions in export
    #[serde(default = "default_true")]
    pub include_descriptions: bool,
    /// Include only failed/pending checks
    #[serde(default)]
    pub failures_only: bool,
}

fn default_true() -> bool {
    true
}

// ─────────────────────────────────────────────────────────
// Compatibility matrix types
// ─────────────────────────────────────────────────────────

/// Raw database row for a compatibility entry (used by sqlx query_as!)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompatibilityRow {
    pub id: Uuid,
    pub source_contract_id: Uuid,
    pub source_version: String,
    pub target_contract_id: Uuid,
    pub target_contract_stellar_id: String,
    pub target_contract_name: String,
    pub target_version: String,
    pub stellar_version: Option<String>,
    pub is_compatible: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single entry in the grouped compatibility matrix response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityEntry {
    pub target_contract_id: Uuid,
    pub target_contract_stellar_id: String,
    pub target_contract_name: String,
    pub target_version: String,
    pub stellar_version: Option<String>,
    pub is_compatible: bool,
}

/// Full API response for GET /contracts/:id/compatibility
#[derive(Debug, Serialize)]
pub struct CompatibilityMatrixResponse {
    pub contract_id: Uuid,
    /// Keyed by source_version; each value is the list of targets
    pub versions: std::collections::BTreeMap<String, Vec<CompatibilityEntry>>,
    /// Human-readable warnings for incompatible pairs
    pub warnings: Vec<String>,
    pub total_entries: usize,
}

/// Request body for POST /contracts/:id/compatibility
#[derive(Debug, Deserialize)]
pub struct AddCompatibilityRequest {
    pub source_version: String,
    pub target_contract_id: Uuid,
    pub target_version: String,
    pub stellar_version: Option<String>,
    #[serde(default = "default_true")]
    pub is_compatible: bool,
}

/// Flat row for CSV / JSON export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompatibilityExportRow {
    pub source_version: String,
    pub target_contract_stellar_id: String,
    pub target_contract_name: String,
    pub target_version: String,
    pub stellar_version: Option<String>,
    pub is_compatible: bool,
}

// ─────────────────────────────────────────────────────────
// Benchmark types
// ─────────────────────────────────────────────────────────

/// Body for POST /contracts/:id/benchmarks
#[derive(Debug, Deserialize)]
pub struct RunBenchmarkRequest {
    pub method: String,
    pub iterations: i32,
    pub version: Option<String>,
    pub args_json: Option<serde_json::Value>,
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold_pct: f64,
}

fn default_alert_threshold() -> f64 {
    10.0
}


// --- Reviews & Ratings Models ---
#[derive(Debug, Deserialize)]
pub struct CreateReviewRequest {
    pub version: String,
    pub rating: f32,
    pub review_text: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ReviewResponse {
    pub id: i32,
    pub contract_id: Uuid,
    pub user_id: Uuid,
    pub version: String,
    pub rating: f32,
    pub review_text: Option<String>,
    pub helpful_count: i32,
    pub is_flagged: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ContractBenchmarkSummary {
    pub contract_id: Uuid,
    pub total_benchmarks: i64,
    pub methods_benchmarked: Vec<String>,
    pub latest_benchmarks: Vec<BenchmarkRecord>,
    pub active_alerts: Vec<PerformanceAlert>,
}

// ─────────────────────────────────────────────────────────
// Network Routing Types
// ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RoutingStrategy {
    Auto,
    Manual(String),
}

impl std::fmt::Display for RoutingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingStrategy::Auto => write!(f, "auto"),
            RoutingStrategy::Manual(s) => write!(f, "{}", s),
        }
    }
}

/// Metadata for network selection criteria
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkCriteria {
    pub id: String,
    pub stability: f32, // 0.0 - 1.0
    pub cost_multiplier: f32, 
    pub latency_ms: u32,
}

// ─────────────────────────────────────────────────────────
// Formal Verification Types
// ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "formal_verification_status", rename_all = "PascalCase")]
pub enum VerificationStatus {
    Proved,
    Violated,
    Unknown,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FormalVerificationSession {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub verifier_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FormalVerificationProperty {
    pub id: Uuid,
    pub session_id: Uuid,
    pub property_id: String,
    pub description: Option<String>,
    pub invariant: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FormalVerificationResult {
    pub id: Uuid,
    pub property_id: Uuid,
    pub status: VerificationStatus,
    pub counterexample: Option<String>,
    pub details: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RunVerificationRequest {
    pub properties_file: String, // could be raw TOML string or base64
    pub verifier_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FormalVerificationPropertyResult {
    pub property: FormalVerificationProperty,
    pub result: FormalVerificationResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FormalVerificationReport {
    pub session: FormalVerificationSession,
    pub properties: Vec<FormalVerificationPropertyResult>,
}
