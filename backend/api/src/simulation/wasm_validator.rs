//! WASM structural validation.
//!
//! The implementation now lives in `shared::wasm` so the CLI's local
//! `contract verify` command and the backend apply identical checks. This
//! module re-exports it to preserve the existing `simulation::wasm_validator`
//! call sites.

pub use shared::wasm::{validate_wasm, WasmValidationResult};
