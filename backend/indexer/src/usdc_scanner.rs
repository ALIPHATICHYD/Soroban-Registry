//! USDC payment scanner.
//!
//! Polls Horizon's `/accounts/{addr}/payments` endpoint for the
//! configured marketplace receiving address, filters to USDC payments,
//! looks up each transaction's memo, and POSTs to the registry's
//! `/api/marketplace/usdc/confirm` endpoint to trigger license
//! issuance. Runs as a parallel tokio task alongside the existing
//! ledger indexer; the two share nothing but the DB pool.
//!
//! Why a separate poller (not folded into the ledger loop):
//!   * Horizon's paging cursor on `/payments` is independent of the
//!     ledger-height cursor used by the contract-deployment detector.
//!   * The two have different latency requirements — payments should
//!     confirm quickly; contract deployments don't need second-level
//!     freshness.
//!   * Decoupling means a misconfigured marketplace doesn't stop
//!     contract indexing and vice versa.
//!
//! Auth: the confirm endpoint requires `AuthenticatedUser`, so the
//! scanner uses a pre-issued bearer token from
//! `MARKETPLACE_INDEXER_API_TOKEN`. Issuing/rotating that token is an
//! operator concern; the scanner doesn't try to mint one.
//!
//! Idempotency: the API endpoint refuses double-confirm by atomic
//! `tx_hash IS NULL` check (see `marketplace::usdc_handlers`), so even
//! if the scanner reprocesses payments after a crash before its
//! cursor advances, no double licenses get issued. Unknown memos hit
//! 404 and are logged + skipped.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration;
use tracing::{debug, error, info, warn};

const HORIZON_PAGE_LIMIT: usize = 50;
const SCAN_INTERVAL_SECS: u64 = 15;
/// USDC has 7 decimals on Stellar; one stroop is 1e-7 USDC.
const USDC_STROOP_DECIMALS: u32 = 7;

#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error("config error: {0}")]
    Config(String),
    #[error("horizon http error: {0}")]
    Horizon(String),
    #[error("registry http error: {0}")]
    Registry(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

pub struct ScannerConfig {
    pub network: String,           // "testnet" | "public"
    pub horizon_url: String,       // e.g. "https://horizon-testnet.stellar.org"
    pub receiving_address: String, // G...
    pub usdc_issuer: String,       // G...
    pub registry_base_url: String, // e.g. "http://localhost:3001"
    pub registry_token: String,    // Bearer token for the confirm endpoint
}

impl ScannerConfig {
    pub fn from_env() -> Result<Option<Self>, ScannerError> {
        // Scanner is opt-in — if the receiving address isn't set, we
        // simply don't start the task. This keeps the indexer working
        // for deployments that don't use the marketplace.
        let receiving_address = match std::env::var("MARKETPLACE_USDC_RECEIVING_ADDRESS") {
            Ok(v) if !v.is_empty() => v,
            _ => return Ok(None),
        };
        let usdc_issuer = std::env::var("MARKETPLACE_USDC_ASSET_ISSUER")
            .map_err(|_| ScannerError::Config("MARKETPLACE_USDC_ASSET_ISSUER not set".into()))?;
        let network =
            std::env::var("MARKETPLACE_USDC_NETWORK").unwrap_or_else(|_| "testnet".into());
        let horizon_url =
            std::env::var("MARKETPLACE_HORIZON_URL").unwrap_or_else(|_| match network.as_str() {
                "public" => "https://horizon.stellar.org".to_string(),
                _ => "https://horizon-testnet.stellar.org".to_string(),
            });
        let registry_base_url = std::env::var("MARKETPLACE_REGISTRY_BASE_URL")
            .map_err(|_| ScannerError::Config("MARKETPLACE_REGISTRY_BASE_URL not set".into()))?;
        let registry_token = std::env::var("MARKETPLACE_INDEXER_API_TOKEN")
            .map_err(|_| ScannerError::Config("MARKETPLACE_INDEXER_API_TOKEN not set".into()))?;

        Ok(Some(ScannerConfig {
            network,
            horizon_url,
            receiving_address,
            usdc_issuer,
            registry_base_url,
            registry_token,
        }))
    }
}

pub struct UsdcScanner {
    cfg: ScannerConfig,
    http: reqwest::Client,
    db: PgPool,
}

impl UsdcScanner {
    pub fn new(cfg: ScannerConfig, db: PgPool) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { cfg, http, db }
    }

    /// Top-level driver — never returns under normal operation. Errors
    /// are logged but don't terminate the task; we sleep + retry.
    pub async fn run(self) {
        info!(
            network = %self.cfg.network,
            receiving_address = %self.cfg.receiving_address,
            "USDC scanner starting"
        );

        // Best-effort ensure row exists. Avoids the first-cycle UPSERT
        // race against a freshly migrated DB.
        if let Err(e) = self.ensure_state_row().await {
            error!(error = %e, "failed to ensure scanner state row; will retry");
        }

        loop {
            match self.poll_once().await {
                Ok(n) if n > 0 => info!(processed = n, "USDC scanner cycle complete"),
                Ok(_) => debug!("USDC scanner cycle complete (no new payments)"),
                Err(e) => warn!(error = %e, "USDC scanner cycle failed"),
            }
            tokio::time::sleep(Duration::from_secs(SCAN_INTERVAL_SECS)).await;
        }
    }

    async fn ensure_state_row(&self) -> Result<(), ScannerError> {
        sqlx::query(
            r#"
            INSERT INTO marketplace_usdc_scanner_state (network, receiving_address)
            VALUES ($1, $2)
            ON CONFLICT (network, receiving_address) DO NOTHING
            "#,
        )
        .bind(&self.cfg.network)
        .bind(&self.cfg.receiving_address)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// One poll cycle. Returns the number of payments processed
    /// (matched USDC payments that we attempted to confirm).
    ///
    /// Cursor advancement: we advance the cursor up to the **last
    /// successfully handled** payment. A handled payment is one we
    /// either (a) decided to skip for non-transient reasons (wrong
    /// asset, no memo, etc.) or (b) successfully POSTed to the
    /// registry. On the first **transient** failure (registry
    /// unreachable, Horizon error fetching memo) we stop advancing
    /// so the next cycle retries from that point. Without this, a
    /// registry outage would silently lose all in-flight confirms.
    async fn poll_once(&self) -> Result<usize, ScannerError> {
        let cursor = self.load_cursor().await?;
        let page = self.fetch_payments(cursor.as_deref()).await?;

        let mut processed = 0usize;
        let mut advance_to_token: Option<String> = None;
        let mut advance_to_tx: Option<String> = None;

        for payment in &page.embedded.records {
            // We only care about credit payments where `to` is our
            // receiving address and the asset matches USDC issuer.
            // Horizon emits `payment` (op type 1) and
            // `path_payment_strict_*` (op types 2, 13). Path payments
            // can also settle to our account; treat them the same.
            let is_payment_like = matches!(
                payment.type_field.as_str(),
                "payment" | "path_payment_strict_receive" | "path_payment_strict_send"
            );
            let is_relevant = is_payment_like
                && payment.to.as_deref() == Some(self.cfg.receiving_address.as_str())
                && payment.asset_code.as_deref() == Some("USDC")
                && payment.asset_issuer.as_deref() == Some(self.cfg.usdc_issuer.as_str());

            if !is_relevant {
                // Non-relevant payments are safe to skip over — no
                // amount of retry will turn them into marketplace
                // confirmations. Advance past them.
                advance_to_token = Some(payment.paging_token.clone());
                continue;
            }

            let stroops = match parse_amount_to_stroops(payment.amount.as_deref().unwrap_or("0")) {
                Ok(s) => s,
                Err(e) => {
                    warn!(
                        tx_hash = payment.transaction_hash.as_deref().unwrap_or("?"),
                        amount = payment.amount.as_deref().unwrap_or("?"),
                        error = %e,
                        "could not parse payment amount; permanent skip"
                    );
                    advance_to_token = Some(payment.paging_token.clone());
                    continue;
                }
            };

            let tx_hash = match payment.transaction_hash.as_deref() {
                Some(h) => h.to_string(),
                None => {
                    warn!("payment missing transaction_hash; permanent skip");
                    advance_to_token = Some(payment.paging_token.clone());
                    continue;
                }
            };

            // Memo fetch can fail transiently (Horizon flake). On
            // transient failure we STOP advancing the cursor so the
            // next cycle retries this payment.
            let memo = match self.fetch_memo(&tx_hash).await {
                Ok(Some(m)) => m,
                Ok(None) => {
                    debug!(tx_hash = %tx_hash, "payment has no memo_text; permanent skip");
                    advance_to_token = Some(payment.paging_token.clone());
                    continue;
                }
                Err(e) => {
                    warn!(
                        tx_hash = %tx_hash,
                        error = %e,
                        "transient memo-fetch failure; will retry next cycle"
                    );
                    break;
                }
            };

            if !memo.starts_with("mp-") {
                debug!(tx_hash = %tx_hash, memo = %memo, "memo not in marketplace prefix; permanent skip");
                advance_to_token = Some(payment.paging_token.clone());
                continue;
            }

            match self.confirm_payment(&memo, &tx_hash, stroops).await {
                ConfirmOutcome::Handled => {
                    advance_to_token = Some(payment.paging_token.clone());
                    advance_to_tx = Some(tx_hash);
                    processed += 1;
                }
                ConfirmOutcome::Transient => {
                    warn!(
                        memo = %memo,
                        tx_hash = %tx_hash,
                        "transient confirm failure; halting cursor at previous payment"
                    );
                    break;
                }
            }
        }

        if let Some(token) = advance_to_token {
            self.save_cursor(&token, advance_to_tx.as_deref()).await?;
        }

        Ok(processed)
    }

    async fn load_cursor(&self) -> Result<Option<String>, ScannerError> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT cursor FROM marketplace_usdc_scanner_state WHERE network = $1 AND receiving_address = $2",
        )
        .bind(&self.cfg.network)
        .bind(&self.cfg.receiving_address)
        .fetch_optional(&self.db)
        .await?;
        Ok(row.and_then(|r| r.0))
    }

    async fn save_cursor(&self, cursor: &str, last_tx: Option<&str>) -> Result<(), ScannerError> {
        sqlx::query(
            r#"
            UPDATE marketplace_usdc_scanner_state
            SET cursor = $3, last_seen_tx = COALESCE($4, last_seen_tx), updated_at = NOW()
            WHERE network = $1 AND receiving_address = $2
            "#,
        )
        .bind(&self.cfg.network)
        .bind(&self.cfg.receiving_address)
        .bind(cursor)
        .bind(last_tx)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn fetch_payments(
        &self,
        cursor: Option<&str>,
    ) -> Result<HorizonPaymentsPage, ScannerError> {
        let mut url = format!(
            "{}/accounts/{}/payments?order=asc&limit={}",
            self.cfg.horizon_url, self.cfg.receiving_address, HORIZON_PAGE_LIMIT
        );
        if let Some(c) = cursor {
            url.push_str("&cursor=");
            url.push_str(c);
        }
        debug!(url = %url, "fetching payments page");

        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ScannerError::Horizon(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(ScannerError::Horizon(format!("HTTP {}", resp.status())));
        }
        resp.json::<HorizonPaymentsPage>()
            .await
            .map_err(|e| ScannerError::Horizon(format!("decode: {e}")))
    }

    async fn fetch_memo(&self, tx_hash: &str) -> Result<Option<String>, ScannerError> {
        let url = format!("{}/transactions/{}", self.cfg.horizon_url, tx_hash);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ScannerError::Horizon(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(ScannerError::Horizon(format!("HTTP {}", resp.status())));
        }
        let tx: HorizonTransaction = resp
            .json()
            .await
            .map_err(|e| ScannerError::Horizon(format!("decode: {e}")))?;
        // We only honour MEMO_TEXT; ID/hash/return memos can't carry
        // our `mp-…` prefix in a human-shareable way.
        if tx.memo_type.as_deref() == Some("text") {
            Ok(tx.memo)
        } else {
            Ok(None)
        }
    }

    /// POST to the registry's confirm endpoint.
    ///
    /// Returns `Handled` when the confirmation reached a settled
    /// state — either we issued a license (2xx), the memo wasn't ours
    /// (404), the intent was already confirmed (409 / 4xx generally),
    /// or the request was malformed (4xx). In all of these cases the
    /// scanner can safely advance the cursor.
    ///
    /// Returns `Transient` for network errors and 5xx responses — the
    /// registry might be down, restarting, or briefly overloaded.
    /// The caller halts cursor advancement so the next cycle retries.
    async fn confirm_payment(
        &self,
        memo: &str,
        tx_hash: &str,
        observed_stroops: i64,
    ) -> ConfirmOutcome {
        let url = format!(
            "{}/api/marketplace/usdc/confirm",
            self.cfg.registry_base_url
        );
        let body = ConfirmRequest {
            memo: memo.to_string(),
            tx_hash: tx_hash.to_string(),
            observed_stroops,
        };
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.cfg.registry_token)
            .json(&body)
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                info!(memo = %memo, tx_hash = %tx_hash, "marketplace confirm OK");
                ConfirmOutcome::Handled
            }
            Ok(r) if r.status().as_u16() == 404 => {
                info!(memo = %memo, tx_hash = %tx_hash, "marketplace confirm 404 — unknown memo, skipping");
                ConfirmOutcome::Handled
            }
            Ok(r) if r.status().as_u16() == 409 => {
                debug!(memo = %memo, tx_hash = %tx_hash, "marketplace confirm 409 — already confirmed");
                ConfirmOutcome::Handled
            }
            Ok(r) if r.status().is_client_error() => {
                // 4xx other than 404/409: malformed request, auth
                // failure, or business-rule rejection (e.g.
                // underpayment). Retrying won't help — same input
                // would fail again — so treat as handled.
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                warn!(
                    memo = %memo,
                    tx_hash = %tx_hash,
                    status = %status,
                    body = %text,
                    "marketplace confirm rejected (4xx); permanent skip"
                );
                ConfirmOutcome::Handled
            }
            Ok(r) => {
                // 5xx: likely transient. Don't advance.
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                warn!(
                    memo = %memo,
                    tx_hash = %tx_hash,
                    status = %status,
                    body = %text,
                    "marketplace confirm 5xx; will retry next cycle"
                );
                ConfirmOutcome::Transient
            }
            Err(e) => {
                warn!(memo = %memo, tx_hash = %tx_hash, error = %e, "marketplace confirm network error; will retry");
                ConfirmOutcome::Transient
            }
        }
    }
}

/// Classification of a confirm attempt, used to decide whether the
/// cursor can advance past this payment.
enum ConfirmOutcome {
    /// Done — advance the cursor.
    Handled,
    /// Transient failure (network, 5xx) — halt the cursor at the
    /// previous payment so we retry.
    Transient,
}

// ── Horizon DTOs (minimal projections) ──────────────────────────────

#[derive(Debug, Deserialize)]
struct HorizonPaymentsPage {
    #[serde(rename = "_embedded")]
    embedded: HorizonPaymentsEmbedded,
}

#[derive(Debug, Deserialize)]
struct HorizonPaymentsEmbedded {
    records: Vec<HorizonPayment>,
}

#[derive(Debug, Deserialize)]
struct HorizonPayment {
    paging_token: String,
    #[serde(rename = "type")]
    type_field: String,
    transaction_hash: Option<String>,
    to: Option<String>,
    amount: Option<String>,
    asset_code: Option<String>,
    asset_issuer: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HorizonTransaction {
    memo: Option<String>,
    memo_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct ConfirmRequest {
    memo: String,
    tx_hash: String,
    observed_stroops: i64,
}

/// Horizon returns amounts as decimal strings with up to 7 dp. Convert
/// to integer stroops (×10^7), rejecting anything that has more
/// fractional digits than USDC supports.
fn parse_amount_to_stroops(amount: &str) -> Result<i64, String> {
    let mut parts = amount.splitn(2, '.');
    let int_part = parts.next().ok_or("empty amount")?;
    let frac_part = parts.next().unwrap_or("");
    if frac_part.len() > USDC_STROOP_DECIMALS as usize {
        return Err(format!(
            "amount has more than {USDC_STROOP_DECIMALS} fractional digits"
        ));
    }
    let int: i64 = int_part.parse().map_err(|e| format!("bad integer: {e}"))?;
    let frac: i64 = if frac_part.is_empty() {
        0
    } else {
        let padded = format!("{:0<7}", frac_part); // right-pad with zeros to 7
        padded.parse().map_err(|e| format!("bad fraction: {e}"))?
    };
    int.checked_mul(10_000_000)
        .and_then(|v| v.checked_add(frac))
        .ok_or_else(|| "amount overflow".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amount_parsing() {
        assert_eq!(parse_amount_to_stroops("1").unwrap(), 10_000_000);
        assert_eq!(parse_amount_to_stroops("1.0").unwrap(), 10_000_000);
        assert_eq!(parse_amount_to_stroops("1.0000001").unwrap(), 10_000_001);
        assert_eq!(parse_amount_to_stroops("29.99").unwrap(), 299_900_000);
        assert_eq!(parse_amount_to_stroops("0.0000001").unwrap(), 1);
        assert!(parse_amount_to_stroops("1.00000001").is_err()); // too precise
        assert!(parse_amount_to_stroops("not-a-number").is_err());
    }

    /// Drives the cursor-advancement logic from `poll_once` against a
    /// canned sequence of confirm outcomes. The bug this guards
    /// against: cursor advancing past a payment whose confirm
    /// returned Transient, which on registry recovery would silently
    /// drop those confirms.
    fn simulate_cursor_advance(outcomes: &[(&str, ConfirmOutcome)]) -> Option<String> {
        let mut advance_to: Option<String> = None;
        for (token, outcome) in outcomes {
            match outcome {
                ConfirmOutcome::Handled => advance_to = Some((*token).to_string()),
                ConfirmOutcome::Transient => break,
            }
        }
        advance_to
    }

    #[test]
    fn cursor_halts_on_first_transient_failure() {
        let advance = simulate_cursor_advance(&[
            ("token-1", ConfirmOutcome::Handled),
            ("token-2", ConfirmOutcome::Handled),
            ("token-3", ConfirmOutcome::Transient), // registry down
            ("token-4", ConfirmOutcome::Handled),   // would be lost without halt
        ]);
        assert_eq!(advance.as_deref(), Some("token-2"));
    }

    #[test]
    fn cursor_advances_when_all_handled() {
        let advance = simulate_cursor_advance(&[
            ("token-1", ConfirmOutcome::Handled),
            ("token-2", ConfirmOutcome::Handled),
            ("token-3", ConfirmOutcome::Handled),
        ]);
        assert_eq!(advance.as_deref(), Some("token-3"));
    }

    #[test]
    fn cursor_unchanged_when_first_is_transient() {
        let advance = simulate_cursor_advance(&[
            ("token-1", ConfirmOutcome::Transient),
            ("token-2", ConfirmOutcome::Handled),
        ]);
        assert_eq!(advance, None);
    }
}
