//! Persisting a contract's interface fingerprint at publish time (Issue #1147).
//!
//! The derivation itself is pure and lives in
//! [`shared::interface_fingerprint::fingerprint_wasm`] — in `shared` rather
//! than here so the CLI, the API, and any future verifier all compute the same
//! id from the same bytes, and so it is covered by `cargo test -p shared`.
//!
//! This module is only the persistence adapter: it flattens the result into the
//! two nullable `contracts` columns and records *why* an id was not produced.
//! Failure is deliberately never fatal to a publish — a contract with no
//! `contractspecv0` section is a legitimate publish that simply has no
//! interface id, exactly like a publish with no artifact at all. Rejecting it
//! would turn an additive feature into a breaking change to the publish
//! contract.

use shared::interface_fingerprint::{self, FingerprintWasmError};

/// Derive `(interface_id, interface_algorithm)` for the `contracts` row.
///
/// `(None, None)` means "unknown interface", which the dependency graph treats
/// as "cannot assess compatibility" rather than "interfaces differ".
pub fn derive_columns(
    wasm_bytes: &[u8],
    contract_id: &str,
) -> (Option<String>, Option<&'static str>) {
    match interface_fingerprint::fingerprint_wasm(wasm_bytes) {
        Ok(fingerprint) => (
            Some(fingerprint.interface_id),
            Some(interface_fingerprint::ALGORITHM),
        ),
        Err(FingerprintWasmError::NoSpecSection) => {
            // Common and unremarkable: plenty of modules ship without a spec.
            tracing::debug!(
                contract_id,
                "published artifact embeds no contract spec section; no interface id recorded"
            );
            (None, None)
        }
        Err(err) => {
            // A present-but-broken section is worth a louder log: it usually
            // means a build-toolchain problem on the publisher's side.
            tracing::warn!(
                contract_id,
                error = %err,
                "could not derive an interface fingerprint for published artifact"
            );
            (None, None)
        }
    }
}
