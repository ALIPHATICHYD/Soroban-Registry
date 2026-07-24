//! Marketplace — paid contract usage and licensing.
//!
//! Phase 1 scope (current):
//!   * `contract_pricing_plans` CRUD per contract
//!   * Ed25519-signed JWT licenses, issuance + offline validation
//!   * Usage metering ledger
//!
//! Out of scope (Phase 2/3, intentionally deferred):
//!   * Stripe checkout + webhook handling
//!   * USDC on-chain payment verification
//!   * Billing aggregation / invoicing
//!
//! Queries in this module use `sqlx::query()` / `query_as()` (non-macro)
//! rather than `sqlx::query!`. This matches `usage_counter.rs` and keeps
//! the module compile-checkable without forcing the new migration to be
//! applied at build time.

pub mod issuance;
pub mod license;
pub mod license_handlers;
pub mod metering;
pub mod models;
pub mod pricing_handlers;
pub mod stripe;
pub mod stripe_handlers;
pub mod usdc;
pub mod usdc_handlers;

pub use license::LicenseSigner;

use crate::error::{ApiError, ApiResult};

/// Lazy-load the Ed25519 signer from env. Returns 503 with a clear
/// message if `MARKETPLACE_LICENSE_SIGNING_KEY` is unset/invalid so the
/// endpoint surfaces the misconfiguration instead of 500ing silently.
pub fn load_signer() -> ApiResult<LicenseSigner> {
    LicenseSigner::from_env().map_err(|e| {
        ApiError::service_unavailable_with(
            "license_signing_unavailable",
            format!(
                "marketplace license signing key is not configured: {e}. \
                 Set MARKETPLACE_LICENSE_SIGNING_KEY (base64-encoded 32-byte Ed25519 seed)."
            ),
        )
    })
}
