//! AES-256-GCM authenticated encryption with a self-describing envelope (#895).
//!
//! Envelope format (ASCII): `enc:v1:<key_id>:<base64(nonce(12) || ciphertext||tag)>`
//!
//! Storing the key id alongside the ciphertext lets us decrypt with the right
//! key after a rotation, and the `enc:v1:` prefix lets callers distinguish
//! already-encrypted values from legacy plaintext during a migration.

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};

use super::key_manager::{EncryptionKey, KeyManager, KEY_BYTES};

/// Marker prefix identifying a value produced by [`encrypt`].
pub const ENVELOPE_PREFIX: &str = "enc:v1:";

const NONCE_BYTES: usize = 12;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("AES-GCM encryption failed")]
    EncryptFailed,
    #[error("AES-GCM decryption failed (wrong key or tampered ciphertext)")]
    DecryptFailed,
    #[error("value is not a recognized encryption envelope")]
    NotAnEnvelope,
    #[error("malformed encryption envelope")]
    MalformedEnvelope,
    #[error("ciphertext is shorter than the nonce")]
    CiphertextTooShort,
    #[error("envelope references unknown key id `{0}`")]
    UnknownKeyId(String),
    #[error("envelope payload is not valid base64")]
    InvalidBase64,
}

/// Encrypt `plaintext` with the key manager's active key, returning an envelope
/// string safe to store in a `TEXT` column.
pub fn encrypt(keys: &KeyManager, plaintext: &[u8]) -> Result<String, CryptoError> {
    let key = keys.active_key();
    let cipher = cipher_for(key);

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| CryptoError::EncryptFailed)?;

    let mut payload = Vec::with_capacity(NONCE_BYTES + ciphertext.len());
    payload.extend_from_slice(nonce.as_slice());
    payload.extend_from_slice(&ciphertext);

    Ok(format!(
        "{ENVELOPE_PREFIX}{}:{}",
        key.id(),
        STANDARD.encode(payload)
    ))
}

/// Decrypt an envelope produced by [`encrypt`], selecting the key referenced in
/// the envelope so rotated/retired keys still work.
pub fn decrypt(keys: &KeyManager, envelope: &str) -> Result<Vec<u8>, CryptoError> {
    let body = envelope
        .strip_prefix(ENVELOPE_PREFIX)
        .ok_or(CryptoError::NotAnEnvelope)?;

    let (key_id, b64) = body.split_once(':').ok_or(CryptoError::MalformedEnvelope)?;
    let key = keys
        .key(key_id)
        .ok_or_else(|| CryptoError::UnknownKeyId(key_id.to_string()))?;

    let payload = STANDARD
        .decode(b64.as_bytes())
        .map_err(|_| CryptoError::InvalidBase64)?;
    if payload.len() <= NONCE_BYTES {
        return Err(CryptoError::CiphertextTooShort);
    }

    let (nonce_bytes, ciphertext) = payload.split_at(NONCE_BYTES);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher_for(key)
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptFailed)
}

fn cipher_for(key: &EncryptionKey) -> Aes256Gcm {
    let key = Key::<Aes256Gcm>::from_slice(key.bytes());
    Aes256Gcm::new(key)
}

// Compile-time check that the constant matches the cipher's key size.
const _: () = assert!(KEY_BYTES == 32);

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD as B64;

    fn key_manager() -> KeyManager {
        let spec = format!("k1:{}", B64.encode([7u8; KEY_BYTES]));
        KeyManager::from_keys_spec(&spec, None).unwrap()
    }

    #[test]
    fn round_trips_plaintext() {
        let km = key_manager();
        let env = encrypt(&km, b"super secret value").unwrap();
        assert!(env.starts_with(ENVELOPE_PREFIX));
        assert!(env.contains(":k1:"));
        let out = decrypt(&km, &env).unwrap();
        assert_eq!(out, b"super secret value");
    }

    #[test]
    fn ciphertext_is_nondeterministic() {
        let km = key_manager();
        let a = encrypt(&km, b"same input").unwrap();
        let b = encrypt(&km, b"same input").unwrap();
        assert_ne!(a, b, "random nonce should make ciphertext differ");
        assert_eq!(decrypt(&km, &a).unwrap(), decrypt(&km, &b).unwrap());
    }

    #[test]
    fn tampering_is_detected() {
        let km = key_manager();
        let env = encrypt(&km, b"integrity matters").unwrap();
        // Flip the final base64 char to corrupt the tag/ciphertext.
        let mut chars: Vec<char> = env.chars().collect();
        let last = chars.len() - 1;
        chars[last] = if chars[last] == 'A' { 'B' } else { 'A' };
        let tampered: String = chars.into_iter().collect();
        assert!(matches!(
            decrypt(&km, &tampered),
            Err(CryptoError::DecryptFailed) | Err(CryptoError::InvalidBase64)
        ));
    }

    #[test]
    fn rejects_non_envelope() {
        let km = key_manager();
        assert!(matches!(
            decrypt(&km, "plain text value"),
            Err(CryptoError::NotAnEnvelope)
        ));
    }

    #[test]
    fn decrypts_with_retired_key_after_rotation() {
        // Encrypt under k1, then rotate so k2 is active but k1 is retained.
        let old = format!("k1:{}", B64.encode([1u8; KEY_BYTES]));
        let old_km = KeyManager::from_keys_spec(&old, None).unwrap();
        let env = encrypt(&old_km, b"written before rotation").unwrap();

        let rotated = format!(
            "k1:{},k2:{}",
            B64.encode([1u8; KEY_BYTES]),
            B64.encode([2u8; KEY_BYTES])
        );
        let new_km = KeyManager::from_keys_spec(&rotated, Some("k2")).unwrap();

        // New writes use k2, but the old k1 ciphertext still decrypts.
        assert_eq!(new_km.active_key_id(), "k2");
        assert_eq!(decrypt(&new_km, &env).unwrap(), b"written before rotation");
    }

    #[test]
    fn unknown_key_id_is_reported() {
        let km = key_manager();
        let env = encrypt(&km, b"x").unwrap().replace(":k1:", ":kX:");
        assert!(matches!(
            decrypt(&km, &env),
            Err(CryptoError::UnknownKeyId(_))
        ));
    }
}
