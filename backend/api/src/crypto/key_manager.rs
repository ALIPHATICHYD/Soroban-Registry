//! Versioned key management for at-rest encryption (#895).

use std::collections::HashMap;

use base64::{engine::general_purpose::STANDARD, Engine as _};

/// Length of an AES-256 key in bytes.
pub const KEY_BYTES: usize = 32;

const ENV_KEYS: &str = "ENCRYPTION_KEYS";
const ENV_ACTIVE_KEY: &str = "ENCRYPTION_ACTIVE_KEY_ID";

#[derive(Debug, thiserror::Error)]
pub enum KeyManagerError {
    #[error("ENCRYPTION_KEYS is not set; configure at least one encryption key")]
    Missing,
    #[error("malformed entry in ENCRYPTION_KEYS: expected `id:base64key`")]
    MalformedEntry,
    #[error("encryption key id must not be empty")]
    EmptyId,
    #[error("key `{0}` is not valid base64")]
    InvalidBase64(String),
    #[error("key `{0}` must decode to exactly 32 bytes (AES-256)")]
    WrongLength(String),
    #[error("duplicate key id `{0}`")]
    DuplicateId(String),
    #[error("ENCRYPTION_ACTIVE_KEY_ID (`{0}`) does not match any configured key id")]
    UnknownActiveKey(String),
    #[error("no keys were configured")]
    NoKeys,
}

/// A single named AES-256 key held in memory.
#[derive(Clone)]
pub struct EncryptionKey {
    id: String,
    bytes: [u8; KEY_BYTES],
}

impl EncryptionKey {
    pub fn new(id: impl Into<String>, bytes: [u8; KEY_BYTES]) -> Self {
        Self {
            id: id.into(),
            bytes,
        }
    }

    /// Decode a base64-encoded 32-byte key.
    pub fn from_base64(id: impl Into<String>, b64: &str) -> Result<Self, KeyManagerError> {
        let id = id.into();
        let raw = STANDARD
            .decode(b64.trim())
            .map_err(|_| KeyManagerError::InvalidBase64(id.clone()))?;
        let bytes: [u8; KEY_BYTES] = raw
            .try_into()
            .map_err(|_| KeyManagerError::WrongLength(id.clone()))?;
        Ok(Self { id, bytes })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn bytes(&self) -> &[u8; KEY_BYTES] {
        &self.bytes
    }
}

// Never leak key material through debug formatting.
impl std::fmt::Debug for EncryptionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionKey")
            .field("id", &self.id)
            .field("bytes", &"<redacted>")
            .finish()
    }
}

/// Zero key material on drop so it does not linger in freed memory.
impl Drop for EncryptionKey {
    fn drop(&mut self) {
        for byte in self.bytes.iter_mut() {
            // `write_volatile` is not elided by the optimizer.
            unsafe {
                std::ptr::write_volatile(byte, 0);
            }
        }
    }
}

/// Holds the active key (used for new encryptions) plus retired keys retained
/// for decrypting previously-written data. This is what enables rotation without
/// downtime or a bulk re-encryption pass.
#[derive(Debug)]
pub struct KeyManager {
    keys: HashMap<String, EncryptionKey>,
    active_key_id: String,
}

impl KeyManager {
    /// Build a key manager from explicit keys and an active key id.
    pub fn new(
        keys: Vec<EncryptionKey>,
        active_key_id: impl Into<String>,
    ) -> Result<Self, KeyManagerError> {
        let active_key_id = active_key_id.into();
        if keys.is_empty() {
            return Err(KeyManagerError::NoKeys);
        }

        let mut map = HashMap::with_capacity(keys.len());
        for key in keys {
            if map.contains_key(key.id()) {
                return Err(KeyManagerError::DuplicateId(key.id().to_string()));
            }
            map.insert(key.id().to_string(), key);
        }

        if !map.contains_key(&active_key_id) {
            return Err(KeyManagerError::UnknownActiveKey(active_key_id));
        }

        Ok(Self {
            keys: map,
            active_key_id,
        })
    }

    /// Load keys from the environment.
    ///
    /// `ENCRYPTION_KEYS` is a comma-separated list of `id:base64key` entries.
    /// `ENCRYPTION_ACTIVE_KEY_ID` selects which id signs new ciphertext; when
    /// unset and exactly one key is configured, that key is used.
    pub fn from_env() -> Result<Self, KeyManagerError> {
        let raw = std::env::var(ENV_KEYS).map_err(|_| KeyManagerError::Missing)?;
        Self::from_keys_spec(&raw, std::env::var(ENV_ACTIVE_KEY).ok().as_deref())
    }

    /// Parse the `ENCRYPTION_KEYS` specification. Exposed for testing.
    pub fn from_keys_spec(
        spec: &str,
        active_key_id: Option<&str>,
    ) -> Result<Self, KeyManagerError> {
        let mut keys = Vec::new();
        for entry in spec.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            let (id, b64) = entry
                .split_once(':')
                .ok_or(KeyManagerError::MalformedEntry)?;
            let id = id.trim();
            if id.is_empty() {
                return Err(KeyManagerError::EmptyId);
            }
            keys.push(EncryptionKey::from_base64(id, b64)?);
        }

        if keys.is_empty() {
            return Err(KeyManagerError::NoKeys);
        }

        let active = match active_key_id.map(str::trim).filter(|s| !s.is_empty()) {
            Some(id) => id.to_string(),
            None if keys.len() == 1 => keys[0].id().to_string(),
            None => {
                return Err(KeyManagerError::UnknownActiveKey(
                    "<unset, multiple keys configured>".to_string(),
                ))
            }
        };

        Self::new(keys, active)
    }

    /// The key used to encrypt new values.
    pub fn active_key(&self) -> &EncryptionKey {
        // Existence guaranteed by the constructor invariant.
        &self.keys[&self.active_key_id]
    }

    pub fn active_key_id(&self) -> &str {
        &self.active_key_id
    }

    /// Look up a key by id for decryption.
    pub fn key(&self, id: &str) -> Option<&EncryptionKey> {
        self.keys.get(id)
    }

    pub fn key_ids(&self) -> impl Iterator<Item = &str> {
        self.keys.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b64_key(byte: u8) -> String {
        STANDARD.encode([byte; KEY_BYTES])
    }

    #[test]
    fn parses_single_key_and_defaults_active() {
        let spec = format!("k1:{}", b64_key(1));
        let km = KeyManager::from_keys_spec(&spec, None).unwrap();
        assert_eq!(km.active_key_id(), "k1");
        assert_eq!(km.active_key().bytes(), &[1u8; KEY_BYTES]);
    }

    #[test]
    fn parses_multiple_keys_with_explicit_active() {
        let spec = format!("k1:{},k2:{}", b64_key(1), b64_key(2));
        let km = KeyManager::from_keys_spec(&spec, Some("k2")).unwrap();
        assert_eq!(km.active_key_id(), "k2");
        assert!(km.key("k1").is_some());
        assert!(km.key("k2").is_some());
        assert!(km.key("k3").is_none());
    }

    #[test]
    fn multiple_keys_without_active_id_is_rejected() {
        let spec = format!("k1:{},k2:{}", b64_key(1), b64_key(2));
        assert!(matches!(
            KeyManager::from_keys_spec(&spec, None),
            Err(KeyManagerError::UnknownActiveKey(_))
        ));
    }

    #[test]
    fn rejects_wrong_length_key() {
        let short = STANDARD.encode([0u8; 16]);
        let spec = format!("k1:{short}");
        assert!(matches!(
            KeyManager::from_keys_spec(&spec, None),
            Err(KeyManagerError::WrongLength(_))
        ));
    }

    #[test]
    fn rejects_malformed_entry() {
        assert!(matches!(
            KeyManager::from_keys_spec("no-colon-here", None),
            Err(KeyManagerError::MalformedEntry)
        ));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let spec = format!("k1:{},k1:{}", b64_key(1), b64_key(2));
        assert!(matches!(
            KeyManager::from_keys_spec(&spec, Some("k1")),
            Err(KeyManagerError::DuplicateId(_))
        ));
    }

    #[test]
    fn unknown_active_key_is_rejected() {
        let spec = format!("k1:{}", b64_key(1));
        assert!(matches!(
            KeyManager::from_keys_spec(&spec, Some("missing")),
            Err(KeyManagerError::UnknownActiveKey(_))
        ));
    }
}
