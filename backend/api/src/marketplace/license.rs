//! Ed25519-signed JWT licenses.
//!
//! Tokens are standard JOSE-format JWTs with `alg: EdDSA`:
//!
//!     header_b64url . claims_b64url . sig_b64url
//!
//! We sign with `ed25519-dalek` directly rather than going through
//! `jsonwebtoken` so the env-var-provided seed can be used verbatim — no
//! PKCS#8 DER wrapping. The resulting tokens validate against any standard
//! JWT library that understands `EdDSA`.
//!
//! The signing key is loaded from `MARKETPLACE_LICENSE_SIGNING_KEY` as a
//! base64-encoded 32-byte Ed25519 seed. Generate one with:
//!
//!     openssl rand -base64 32
//!
//! Public-key bytes are also exposed at the API so clients can verify
//! tokens offline.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64URL, Engine as _};
use chrono::Utc;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey, SECRET_KEY_LENGTH};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const ENV_VAR: &str = "MARKETPLACE_LICENSE_SIGNING_KEY";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseClaims {
    /// JWT id — matches `contract_licenses.jti`. Used for revocation lookup.
    pub jti: Uuid,
    /// License owner (publisher uuid).
    pub sub: Uuid,
    /// Contract this license is bound to.
    pub aud: Uuid,
    /// Pricing plan id.
    pub plan_id: Uuid,
    /// Human-readable plan name (e.g. "Pro"). Informational only — server
    /// re-checks against the DB on validation.
    pub plan_name: String,
    /// Issued-at, seconds since epoch.
    pub iat: i64,
    /// Expiry, seconds since epoch. None = non-expiring.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
    /// Call quota for the billing period; None = unlimited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota: Option<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    #[error("MARKETPLACE_LICENSE_SIGNING_KEY is not set")]
    SigningKeyMissing,
    #[error("MARKETPLACE_LICENSE_SIGNING_KEY is not valid base64")]
    SigningKeyEncoding,
    #[error("MARKETPLACE_LICENSE_SIGNING_KEY must decode to exactly 32 bytes")]
    SigningKeyLength,
    #[error("malformed token")]
    MalformedToken,
    #[error("unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
    #[error("signature verification failed")]
    BadSignature,
    #[error("token expired")]
    Expired,
}

pub struct LicenseSigner {
    signing_key: SigningKey,
}

impl LicenseSigner {
    pub fn from_env() -> Result<Self, LicenseError> {
        let raw = std::env::var(ENV_VAR).map_err(|_| LicenseError::SigningKeyMissing)?;
        Self::from_b64_seed(&raw)
    }

    pub fn from_b64_seed(b64_seed: &str) -> Result<Self, LicenseError> {
        let seed = B64URL
            .decode(b64_seed.trim())
            .or_else(|_| base64::engine::general_purpose::STANDARD.decode(b64_seed.trim()))
            .map_err(|_| LicenseError::SigningKeyEncoding)?;

        if seed.len() != SECRET_KEY_LENGTH {
            return Err(LicenseError::SigningKeyLength);
        }
        let mut buf = [0u8; SECRET_KEY_LENGTH];
        buf.copy_from_slice(&seed);
        Ok(Self {
            signing_key: SigningKey::from_bytes(&buf),
        })
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn public_key_b64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(self.verifying_key().to_bytes())
    }

    /// Sign a claims set into a compact JWS.
    pub fn sign(&self, claims: &LicenseClaims) -> Result<String, LicenseError> {
        let header = serde_json::json!({"alg": "EdDSA", "typ": "JWT"});
        let header_b64 =
            B64URL.encode(serde_json::to_vec(&header).map_err(|_| LicenseError::MalformedToken)?);
        let claims_b64 =
            B64URL.encode(serde_json::to_vec(claims).map_err(|_| LicenseError::MalformedToken)?);

        let signing_input = format!("{header_b64}.{claims_b64}");
        let sig: Signature = self.signing_key.sign(signing_input.as_bytes());
        let sig_b64 = B64URL.encode(sig.to_bytes());

        Ok(format!("{signing_input}.{sig_b64}"))
    }

    /// Verify signature, expiry, and decode claims. Does NOT consult the
    /// DB — callers must cross-check `jti` against `contract_licenses` to
    /// catch revocation.
    pub fn verify(&self, token: &str) -> Result<LicenseClaims, LicenseError> {
        let mut parts = token.split('.');
        let header_b64 = parts.next().ok_or(LicenseError::MalformedToken)?;
        let claims_b64 = parts.next().ok_or(LicenseError::MalformedToken)?;
        let sig_b64 = parts.next().ok_or(LicenseError::MalformedToken)?;
        if parts.next().is_some() {
            return Err(LicenseError::MalformedToken);
        }

        // Algorithm check
        let header_bytes = B64URL
            .decode(header_b64)
            .map_err(|_| LicenseError::MalformedToken)?;
        let header: serde_json::Value =
            serde_json::from_slice(&header_bytes).map_err(|_| LicenseError::MalformedToken)?;
        match header.get("alg").and_then(|v| v.as_str()) {
            Some("EdDSA") => {}
            Some(other) => return Err(LicenseError::UnsupportedAlgorithm(other.to_string())),
            None => return Err(LicenseError::MalformedToken),
        }

        // Signature
        let sig_bytes = B64URL
            .decode(sig_b64)
            .map_err(|_| LicenseError::MalformedToken)?;
        let sig = Signature::from_slice(&sig_bytes).map_err(|_| LicenseError::BadSignature)?;
        let signing_input = format!("{header_b64}.{claims_b64}");
        self.verifying_key()
            .verify(signing_input.as_bytes(), &sig)
            .map_err(|_| LicenseError::BadSignature)?;

        // Claims + expiry
        let claims_bytes = B64URL
            .decode(claims_b64)
            .map_err(|_| LicenseError::MalformedToken)?;
        let claims: LicenseClaims =
            serde_json::from_slice(&claims_bytes).map_err(|_| LicenseError::MalformedToken)?;
        if let Some(exp) = claims.exp {
            if Utc::now().timestamp() >= exp {
                return Err(LicenseError::Expired);
            }
        }
        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signer() -> LicenseSigner {
        // Deterministic 32-byte seed for tests
        let seed = base64::engine::general_purpose::STANDARD.encode([7u8; 32]);
        LicenseSigner::from_b64_seed(&seed).expect("test signer")
    }

    fn sample_claims() -> LicenseClaims {
        LicenseClaims {
            jti: Uuid::nil(),
            sub: Uuid::nil(),
            aud: Uuid::nil(),
            plan_id: Uuid::nil(),
            plan_name: "Pro".into(),
            iat: 1_700_000_000,
            exp: Some(Utc::now().timestamp() + 3600),
            quota: Some(1000),
        }
    }

    #[test]
    fn round_trip_signs_and_verifies() {
        let s = signer();
        let claims = sample_claims();
        let token = s.sign(&claims).unwrap();
        let decoded = s.verify(&token).unwrap();
        assert_eq!(decoded.plan_name, "Pro");
    }

    #[test]
    fn tampered_token_rejected() {
        let s = signer();
        let token = s.sign(&sample_claims()).unwrap();
        let mut parts: Vec<&str> = token.split('.').collect();
        // Flip last byte of signature
        let mut sig = parts[2].to_string();
        sig.pop();
        sig.push('A');
        parts[2] = &sig;
        let tampered = parts.join(".");
        assert!(matches!(
            s.verify(&tampered),
            Err(LicenseError::BadSignature) | Err(LicenseError::MalformedToken)
        ));
    }

    #[test]
    fn expired_token_rejected() {
        let s = signer();
        let mut c = sample_claims();
        c.exp = Some(1);
        let token = s.sign(&c).unwrap();
        assert!(matches!(s.verify(&token), Err(LicenseError::Expired)));
    }

    #[test]
    fn invalid_seed_length_rejected() {
        let seed = base64::engine::general_purpose::STANDARD.encode([1u8; 16]);
        assert!(matches!(
            LicenseSigner::from_b64_seed(&seed),
            Err(LicenseError::SigningKeyLength)
        ));
    }
}
