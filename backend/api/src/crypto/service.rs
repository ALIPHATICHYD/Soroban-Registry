//! High-level encryption service shared via `AppState` (#895).
//!
//! This is the type handlers interact with. It hides key management behind two
//! simple operations (`encrypt_str` / `decrypt_str`) so encryption stays
//! transparent to application code.
//!
//! Decryption is migration-friendly: a value that is not an encryption envelope
//! is treated as legacy plaintext and returned unchanged. This lets a column be
//! switched to encrypted storage without a synchronous backfill — old rows keep
//! reading while new writes are encrypted.

use super::cipher::{self, CryptoError, ENVELOPE_PREFIX};
use super::key_manager::{KeyManager, KeyManagerError};

/// Wraps the key manager, or runs in a clearly-logged pass-through mode when no
/// keys are configured (local development only).
pub struct EncryptionService {
    keys: Option<KeyManager>,
}

impl EncryptionService {
    /// Initialize from the environment. If `ENCRYPTION_KEYS` is unset the
    /// service runs in pass-through mode and emits a loud warning, so local dev
    /// works without keys while production misconfiguration is obvious in logs.
    pub fn from_env() -> Self {
        match KeyManager::from_env() {
            Ok(keys) => {
                tracing::info!(
                    active_key_id = keys.active_key_id(),
                    configured_keys = keys.key_ids().count(),
                    "Field encryption enabled (AES-256-GCM)"
                );
                Self { keys: Some(keys) }
            }
            Err(KeyManagerError::Missing) => {
                tracing::warn!(
                    "ENCRYPTION_KEYS is not set: field encryption is DISABLED (pass-through). \
                     Set ENCRYPTION_KEYS in any non-development environment."
                );
                Self { keys: None }
            }
            Err(e) => {
                // A present-but-invalid key config is a hard misconfiguration;
                // fail fast rather than silently storing plaintext.
                panic!("Invalid ENCRYPTION_KEYS configuration: {e}");
            }
        }
    }

    /// Construct an explicitly-configured service (used by callers/tests).
    pub fn with_keys(keys: KeyManager) -> Self {
        Self { keys: Some(keys) }
    }

    /// Construct a disabled (pass-through) service.
    pub fn disabled() -> Self {
        Self { keys: None }
    }

    /// Whether encryption keys are configured.
    pub fn is_enabled(&self) -> bool {
        self.keys.is_some()
    }

    /// Encrypt a UTF-8 string into a storable envelope. In pass-through mode the
    /// input is returned unchanged.
    pub fn encrypt_str(&self, plaintext: &str) -> Result<String, CryptoError> {
        match &self.keys {
            Some(keys) => cipher::encrypt(keys, plaintext.as_bytes()),
            None => Ok(plaintext.to_string()),
        }
    }

    /// Decrypt an envelope back to a UTF-8 string. Non-envelope (legacy
    /// plaintext) values are returned unchanged.
    pub fn decrypt_str(&self, stored: &str) -> Result<String, CryptoError> {
        if !is_envelope(stored) {
            return Ok(stored.to_string());
        }
        match &self.keys {
            Some(keys) => {
                let bytes = cipher::decrypt(keys, stored)?;
                String::from_utf8(bytes).map_err(|_| CryptoError::DecryptFailed)
            }
            // Envelope present but no keys configured: cannot recover.
            None => Err(CryptoError::DecryptFailed),
        }
    }

    /// Encrypt arbitrary JSON, serializing it first. Useful for blob columns
    /// such as backup metadata and state snapshots.
    pub fn encrypt_json(&self, value: &serde_json::Value) -> Result<String, CryptoError> {
        let serialized = serde_json::to_string(value).map_err(|_| CryptoError::EncryptFailed)?;
        self.encrypt_str(&serialized)
    }

    /// Decrypt an envelope (or legacy JSON text) back into a JSON value.
    pub fn decrypt_json(&self, stored: &str) -> Result<serde_json::Value, CryptoError> {
        let plaintext = self.decrypt_str(stored)?;
        serde_json::from_str(&plaintext).map_err(|_| CryptoError::DecryptFailed)
    }
}

/// Whether a stored value looks like an encryption envelope.
pub fn is_envelope(value: &str) -> bool {
    value.starts_with(ENVELOPE_PREFIX)
}

impl std::fmt::Debug for EncryptionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionService")
            .field("enabled", &self.is_enabled())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KEY_BYTES;
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    fn enabled_service() -> EncryptionService {
        let spec = format!("k1:{}", STANDARD.encode([9u8; KEY_BYTES]));
        EncryptionService::with_keys(KeyManager::from_keys_spec(&spec, None).unwrap())
    }

    #[test]
    fn enabled_service_round_trips() {
        let svc = enabled_service();
        let env = svc.encrypt_str("hello@example.com").unwrap();
        assert!(is_envelope(&env));
        assert_eq!(svc.decrypt_str(&env).unwrap(), "hello@example.com");
    }

    #[test]
    fn decrypt_passes_through_legacy_plaintext() {
        let svc = enabled_service();
        // A pre-encryption row stored raw text; reading it must not fail.
        assert_eq!(
            svc.decrypt_str("legacy plaintext").unwrap(),
            "legacy plaintext"
        );
    }

    #[test]
    fn disabled_service_is_pass_through() {
        let svc = EncryptionService::disabled();
        assert!(!svc.is_enabled());
        let stored = svc.encrypt_str("value").unwrap();
        assert_eq!(stored, "value");
        assert_eq!(svc.decrypt_str(&stored).unwrap(), "value");
    }

    #[test]
    fn json_round_trips() {
        let svc = enabled_service();
        let value = serde_json::json!({"name": "alice", "secret": [1, 2, 3]});
        let env = svc.encrypt_json(&value).unwrap();
        assert!(is_envelope(&env));
        assert_eq!(svc.decrypt_json(&env).unwrap(), value);
    }
}
