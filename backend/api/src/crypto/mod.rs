//! Application-level encryption (#895).
//!
//! Provides authenticated symmetric encryption for sensitive data at rest and a
//! versioned key-management layer that supports zero-downtime key rotation.
//!
//! ## Algorithm choices
//! - **Cipher:** AES-256-GCM (AEAD). 256-bit keys, 96-bit random nonces, 128-bit
//!   authentication tags. AES-GCM is FIPS-approved, hardware-accelerated on
//!   modern CPUs (AES-NI), and provides confidentiality + integrity in one pass.
//! - **Nonce:** 96 bits drawn from the OS CSPRNG (`OsRng`) for every encryption.
//!   A fresh random nonce per message keeps the (key, nonce) pair unique, which
//!   is the safety requirement for GCM.
//! - **Envelope:** ciphertext is stored as a self-describing string so values can
//!   be decrypted with the correct key after rotation without external metadata.
//!
//! ## Key management
//! Keys never live in config files. They are loaded from the process environment
//! (or, in production, injected from a secrets manager into the environment) and
//! held only in memory. Multiple keys can be registered at once: one *active* key
//! used for new encryptions, plus any number of retired keys retained so that
//! previously-encrypted data stays readable until it is re-encrypted.
//!
//! See `docs/ENCRYPTION.md` for the operational runbook.

mod cipher;
mod key_manager;
mod service;

pub use cipher::{decrypt, encrypt, CryptoError, ENVELOPE_PREFIX};
pub use key_manager::{EncryptionKey, KeyManager, KeyManagerError, KEY_BYTES};
pub use service::EncryptionService;
