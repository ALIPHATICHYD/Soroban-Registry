use crate::{error::ApiError, state::AppState};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::{
    extract::Request,
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::Engine as _;
use chrono::{Duration, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::{distributions::Alphanumeric, Rng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use stellar_strkey::{ed25519::PublicKey as StellarPublicKey, Strkey};

pub const MIN_JWT_SECRET_LEN: usize = 32;

/// Authenticated user extracted from a valid Bearer JWT.
/// The `sub` claim is expected to be the publisher's Stellar address,
/// and `publisher_id` is derived by looking up the publisher in the DB.
/// For simplicity (matching the existing subscription_handlers pattern),
/// we store the sub as a string and expose a UUID parsed from it when possible,
/// falling back to a nil UUID so callers can handle the error themselves.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// The `sub` claim from the JWT (Stellar address / publisher identifier)
    pub stellar_address: String,
    /// Publisher UUID — parsed from sub if it is a UUID, otherwise nil
    pub publisher_id: uuid::Uuid,
    /// Database user id (publisher primary key)
    pub id: uuid::Uuid,
    /// Full claims for callers that need them
    pub claims: AuthClaims,
}

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let auth_manager =
            AuthManager::from_env().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let claims = auth_manager
            .validate_jwt(auth_header)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        let publisher_id = uuid::Uuid::parse_str(&claims.sub).unwrap_or(uuid::Uuid::nil());

        Ok(AuthenticatedUser {
            stellar_address: claims.sub.clone(),
            publisher_id,
            id: publisher_id,
            claims,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    pub sub: String,
    pub publisher_id: uuid::Uuid,
    pub iat: i64,
    pub exp: i64,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub mfa_verified: bool,
    #[serde(default)]
    pub session_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct ChallengeRecord {
    pub nonce: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub subject: String,
    pub publisher_id: uuid::Uuid,
    pub role: Option<String>,
    pub scopes: Vec<String>,
    pub mfa_verified: bool,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in_seconds: u64,
}

pub struct AuthManager {
    challenges: HashMap<String, ChallengeRecord>,
    sessions: HashMap<uuid::Uuid, SessionRecord>,
    refresh_tokens: HashMap<String, uuid::Uuid>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthConfigError {
    MissingJwtSecret,
    JwtSecretTooShort { min_len: usize, actual_len: usize },
}

impl fmt::Display for AuthConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthConfigError::MissingJwtSecret => write!(f, "JWT_SECRET must be set"),
            AuthConfigError::JwtSecretTooShort {
                min_len,
                actual_len,
            } => write!(
                f,
                "JWT_SECRET must be at least {} characters (got {})",
                min_len, actual_len
            ),
        }
    }
}

impl std::error::Error for AuthConfigError {}

impl AuthManager {
    pub fn new(secret: String) -> Self {
        Self {
            challenges: HashMap::new(),
            sessions: HashMap::new(),
            refresh_tokens: HashMap::new(),
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn from_env() -> Result<Self, AuthConfigError> {
        let secret = std::env::var("JWT_SECRET").map_err(|_| AuthConfigError::MissingJwtSecret)?;
        Self::validate_jwt_secret(&secret)?;
        Ok(Self::new(secret))
    }

    fn validate_jwt_secret(secret: &str) -> Result<(), AuthConfigError> {
        let actual_len = secret.len();
        if actual_len < MIN_JWT_SECRET_LEN {
            return Err(AuthConfigError::JwtSecretTooShort {
                min_len: MIN_JWT_SECRET_LEN,
                actual_len,
            });
        }
        Ok(())
    }

    pub fn create_challenge(&mut self, address: &str) -> String {
        let nonce: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let expires_at = (Utc::now() + Duration::minutes(5)).timestamp();
        self.challenges.insert(
            address.to_string(),
            ChallengeRecord {
                nonce: nonce.clone(),
                expires_at,
            },
        );
        nonce
    }

    pub fn verify_and_issue_jwt(
        &mut self,
        address: &str,
        public_key_hex: &str,
        signature_hex: &str,
        publisher_id: uuid::Uuid,
        scopes: Vec<String>,
        expires_in_seconds: u64,
    ) -> Result<String, &'static str> {
        let challenge = self
            .challenges
            .remove(address)
            .ok_or("challenge_not_found")?;
        if Utc::now().timestamp() > challenge.expires_at {
            return Err("challenge_expired");
        }
        let public_key = decode_hex_32(public_key_hex).ok_or("invalid_public_key_hex")?;
        let address_public_key = decode_stellar_public_key(address).ok_or("invalid_address")?;
        if address_public_key != public_key {
            return Err("address_public_key_mismatch");
        }
        let signature = decode_hex_64(signature_hex).ok_or("invalid_signature_hex")?;
        let vk = VerifyingKey::from_bytes(&public_key).map_err(|_| "invalid_public_key")?;
        let sig = Signature::from_bytes(&signature);
        vk.verify(challenge.nonce.as_bytes(), &sig)
            .map_err(|_| "invalid_signature")?;
        let iat = Utc::now().timestamp();
        let expires_in_seconds = expires_in_seconds.clamp(300, 30 * 24 * 60 * 60);
        let exp = (Utc::now() + Duration::seconds(expires_in_seconds as i64)).timestamp();
        let claims = AuthClaims {
            sub: address.to_string(),
            publisher_id,
            iat,
            exp,
            scopes,
            role: None,
            admin: false,
            mfa_verified: false,
            session_id: None,
        };
        encode(&Header::default(), &claims, &self.encoding_key).map_err(|_| "jwt_encode_failed")
    }

    pub fn verify_and_issue_token_pair(
        &mut self,
        address: &str,
        public_key_hex: &str,
        signature_hex: &str,
        publisher_id: uuid::Uuid,
        scopes: Vec<String>,
        expires_in_seconds: u64,
    ) -> Result<TokenPair, &'static str> {
        let challenge = self
            .challenges
            .remove(address)
            .ok_or("challenge_not_found")?;
        if Utc::now().timestamp() > challenge.expires_at {
            return Err("challenge_expired");
        }
        let public_key = decode_hex_32(public_key_hex).ok_or("invalid_public_key_hex")?;
        let address_public_key = decode_stellar_public_key(address).ok_or("invalid_address")?;
        if address_public_key != public_key {
            return Err("address_public_key_mismatch");
        }
        let signature = decode_hex_64(signature_hex).ok_or("invalid_signature_hex")?;
        let vk = VerifyingKey::from_bytes(&public_key).map_err(|_| "invalid_public_key")?;
        let sig = Signature::from_bytes(&signature);
        vk.verify(challenge.nonce.as_bytes(), &sig)
            .map_err(|_| "invalid_signature")?;

        let session_id = uuid::Uuid::new_v4();
        let expires_in_seconds = expires_in_seconds.clamp(300, 30 * 24 * 60 * 60);
        let refresh_token = generate_refresh_token();
        self.refresh_tokens
            .insert(hash_refresh_token(&refresh_token), session_id);
        self.sessions.insert(
            session_id,
            SessionRecord {
                subject: address.to_string(),
                publisher_id,
                role: None,
                scopes: scopes.clone(),
                mfa_verified: false,
                expires_at: (Utc::now() + Duration::days(30)).timestamp(),
            },
        );

        let access_token = self.issue_access_token_for_session(
            session_id,
            address,
            publisher_id,
            scopes,
            None,
            false,
            expires_in_seconds,
        )?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer",
            expires_in_seconds,
        })
    }

    pub fn refresh_access_token(
        &mut self,
        refresh_token: &str,
        expires_in_seconds: u64,
    ) -> Result<TokenPair, &'static str> {
        let token_hash = hash_refresh_token(refresh_token);
        let session_id = *self
            .refresh_tokens
            .get(&token_hash)
            .ok_or("refresh_token_not_found")?;
        let session = self
            .sessions
            .get(&session_id)
            .cloned()
            .ok_or("session_not_found")?;
        if Utc::now().timestamp() > session.expires_at {
            self.sessions.remove(&session_id);
            self.refresh_tokens.remove(&token_hash);
            return Err("session_expired");
        }

        self.refresh_tokens.remove(&token_hash);
        let next_refresh = generate_refresh_token();
        self.refresh_tokens
            .insert(hash_refresh_token(&next_refresh), session_id);
        let expires_in_seconds = expires_in_seconds.clamp(300, 30 * 24 * 60 * 60);
        let access_token = self.issue_access_token_for_session(
            session_id,
            &session.subject,
            session.publisher_id,
            session.scopes,
            session.role,
            session.mfa_verified,
            expires_in_seconds,
        )?;

        Ok(TokenPair {
            access_token,
            refresh_token: next_refresh,
            token_type: "Bearer",
            expires_in_seconds,
        })
    }

    pub fn revoke_session(&mut self, session_id: uuid::Uuid) -> bool {
        let existed = self.sessions.remove(&session_id).is_some();
        self.refresh_tokens.retain(|_, sid| *sid != session_id);
        existed
    }

    fn issue_access_token_for_session(
        &self,
        session_id: uuid::Uuid,
        subject: &str,
        publisher_id: uuid::Uuid,
        scopes: Vec<String>,
        role: Option<String>,
        mfa_verified: bool,
        expires_in_seconds: u64,
    ) -> Result<String, &'static str> {
        let iat = Utc::now().timestamp();
        let exp = (Utc::now() + Duration::seconds(expires_in_seconds as i64)).timestamp();
        let claims = AuthClaims {
            sub: subject.to_string(),
            publisher_id,
            iat,
            exp,
            scopes,
            role,
            admin: false,
            mfa_verified,
            session_id: Some(session_id),
        };
        encode(&Header::default(), &claims, &self.encoding_key).map_err(|_| "jwt_encode_failed")
    }

    pub fn validate_jwt(&self, token: &str) -> Result<AuthClaims, &'static str> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        decode::<AuthClaims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| "invalid_token")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    ApiKey,
    Jwt,
    OAuth2,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub subject: String,
    pub publisher_id: Option<uuid::Uuid>,
    pub role: String,
    pub permissions: Vec<String>,
    pub method: AuthMethod,
    pub mfa_verified: bool,
}

impl AuthContext {
    pub fn has_permission(&self, permission: &str) -> bool {
        self.role.eq_ignore_ascii_case("admin")
            || self.permissions.iter().any(|scope| {
                scope == "*"
                    || scope == permission
                    || permission
                        .split_once(':')
                        .is_some_and(|(prefix, _)| scope == &format!("{prefix}:*"))
            })
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthContext {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        authenticate_headers(&parts.headers, state)
    }
}

pub fn authenticate_headers(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<AuthContext, ApiError> {
    if let Some(api_key) = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return authenticate_api_key(api_key);
    }

    let Some(auth_header) = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    else {
        audit_auth_attempt("missing", false, "authorization header missing");
        return Err(ApiError::unauthorized("Authentication is required"));
    };

    if let Some(token) = auth_header.strip_prefix("ApiKey ").map(str::trim) {
        return authenticate_api_key(token);
    }

    let (method, token) = if let Some(token) = auth_header.strip_prefix("Bearer ") {
        (AuthMethod::Jwt, token.trim())
    } else if let Some(token) = auth_header.strip_prefix("OAuth2 ") {
        (AuthMethod::OAuth2, token.trim())
    } else {
        audit_auth_attempt("unknown", false, "unsupported authorization scheme");
        return Err(ApiError::unauthorized("Unsupported authorization scheme"));
    };

    let mgr = state
        .auth_mgr
        .read()
        .map_err(|_| ApiError::internal("Authentication state unavailable"))?;
    let claims = mgr
        .validate_jwt(token)
        .map_err(|_| ApiError::unauthorized("Invalid or expired token"))?;

    let role = claims.role.clone().unwrap_or_else(|| {
        if claims.admin {
            "admin".to_string()
        } else {
            "user".to_string()
        }
    });

    audit_auth_attempt(&claims.sub, true, "token accepted");
    Ok(AuthContext {
        subject: claims.sub,
        publisher_id: Some(claims.publisher_id),
        role,
        permissions: claims.scopes,
        method,
        mfa_verified: claims.mfa_verified,
    })
}

pub async fn require_security_write(req: Request, next: Next) -> Result<Response, ApiError> {
    let Some(state) = req.extensions().get::<AppState>().cloned() else {
        return Err(ApiError::internal("Application state unavailable"));
    };
    let context = authenticate_headers(req.headers(), &state)?;
    if !context.has_permission("security:write") {
        return Err(ApiError::forbidden(
            "security:write permission is required for this endpoint",
        ));
    }
    Ok(next.run(req).await)
}

fn authenticate_api_key(api_key: &str) -> Result<AuthContext, ApiError> {
    for entry in configured_api_keys() {
        if entry.key == api_key {
            audit_auth_attempt(&entry.subject, true, "api key accepted");
            return Ok(AuthContext {
                subject: entry.subject,
                publisher_id: None,
                role: entry.role,
                permissions: entry.permissions,
                method: AuthMethod::ApiKey,
                mfa_verified: entry.mfa_verified,
            });
        }
    }

    audit_auth_attempt("api_key", false, "api key rejected");
    Err(ApiError::unauthorized("Invalid API key"))
}

struct ApiKeyEntry {
    key: String,
    subject: String,
    role: String,
    permissions: Vec<String>,
    mfa_verified: bool,
}

fn configured_api_keys() -> Vec<ApiKeyEntry> {
    std::env::var("AUTH_API_KEYS")
        .or_else(|_| std::env::var("API_KEYS"))
        .unwrap_or_default()
        .split(',')
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split(':').collect();
            let key = parts.first()?.trim();
            if key.is_empty() {
                return None;
            }
            Some(ApiKeyEntry {
                key: key.to_string(),
                subject: parts
                    .get(1)
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .unwrap_or("api-key")
                    .to_string(),
                role: parts
                    .get(2)
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .unwrap_or("service")
                    .to_string(),
                permissions: parts
                    .get(3)
                    .map(|v| {
                        v.split('|')
                            .map(str::trim)
                            .filter(|v| !v.is_empty())
                            .map(ToOwned::to_owned)
                            .collect()
                    })
                    .unwrap_or_else(|| vec!["read:*".to_string()]),
                mfa_verified: parts
                    .get(4)
                    .map(|v| matches!(v.trim(), "mfa" | "true" | "1"))
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn audit_auth_attempt(subject: &str, success: bool, reason: &str) {
    tracing::info!(
        target = "auth_audit",
        subject,
        success,
        reason,
        "authentication attempt"
    );
}

fn generate_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn hash_refresh_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthClaims {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &AppState) -> Result<Self, ApiError> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("Missing authorization header"))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(ApiError::unauthorized(
                "Invalid authorization header format",
            ));
        }

        let token = &auth_header[7..];
        let auth_manager = AuthManager::from_env()
            .map_err(|_| ApiError::internal("Authentication configuration error"))?;

        let claims = auth_manager
            .validate_jwt(token)
            .map_err(|_| ApiError::unauthorized("Invalid or expired token"))?;

        Ok(claims)
    }
}

fn extract_bearer_token(req: &Request) -> Option<&str> {
    req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn is_admin(claims: &AuthClaims) -> bool {
    claims.admin || matches!(claims.role.as_deref(), Some("admin" | "ADMIN" | "Admin"))
}

pub async fn require_admin(req: Request, next: Next) -> Result<Response, ApiError> {
    let Some(token) = extract_bearer_token(&req) else {
        return Err(ApiError::unauthorized(
            "Authorization header with Bearer token is required",
        ));
    };

    let auth = AuthManager::from_env()
        .map_err(|_| ApiError::internal("Authentication configuration error"))?;
    let claims = auth
        .validate_jwt(token)
        .map_err(|_| ApiError::unauthorized("Invalid or expired authentication token"))?;

    if !is_admin(&claims) {
        return Err(ApiError::forbidden(
            "Administrative privileges are required for this endpoint",
        ));
    }

    Ok(next.run(req).await)
}

fn decode_hex_32(value: &str) -> Option<[u8; 32]> {
    let bytes = decode_hex(value)?;
    let mut out = [0u8; 32];
    if bytes.len() != out.len() {
        return None;
    }
    out.copy_from_slice(&bytes);
    Some(out)
}

fn decode_hex_64(value: &str) -> Option<[u8; 64]> {
    let bytes = decode_hex(value)?;
    let mut out = [0u8; 64];
    if bytes.len() != out.len() {
        return None;
    }
    out.copy_from_slice(&bytes);
    Some(out)
}

fn decode_stellar_public_key(value: &str) -> Option<[u8; 32]> {
    match Strkey::from_string(value).ok()? {
        Strkey::PublicKeyEd25519(StellarPublicKey(bytes)) => Some(bytes),
        _ => None,
    }
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use stellar_strkey::ed25519::PublicKey as StellarPublicKey;

    fn hex_encode(data: &[u8]) -> String {
        data.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    #[test]
    fn challenge_verify_and_jwt_works() {
        let mut auth = AuthManager::new("test-secret".to_string());
        let seed = [7u8; 32];
        let sk = SigningKey::from_bytes(&seed);
        let address = StellarPublicKey(*sk.verifying_key().as_bytes()).to_string();
        let vk_hex = hex_encode(sk.verifying_key().as_bytes());
        let nonce = auth.create_challenge(&address);
        let sig = sk.sign(nonce.as_bytes());
        let token = auth
            .verify_and_issue_jwt(
                &address,
                &vk_hex,
                &hex_encode(&sig.to_bytes()),
                uuid::Uuid::nil(),
                vec!["read".to_string()],
                3_600,
            )
            .expect("jwt must be issued");
        let claims = auth.validate_jwt(&token).expect("token must be valid");
        assert_eq!(claims.sub, address);
        assert_eq!(claims.scopes, vec!["read".to_string()]);
    }

    #[test]
    fn nonce_is_single_use() {
        let mut auth = AuthManager::new("test-secret".to_string());
        let seed = [9u8; 32];
        let sk = SigningKey::from_bytes(&seed);
        let address = StellarPublicKey(*sk.verifying_key().as_bytes()).to_string();
        let vk_hex = hex_encode(sk.verifying_key().as_bytes());
        let nonce = auth.create_challenge(&address);
        let sig = sk.sign(nonce.as_bytes());
        let sig_hex = hex_encode(&sig.to_bytes());
        let first = auth.verify_and_issue_jwt(
            &address,
            &vk_hex,
            &sig_hex,
            uuid::Uuid::nil(),
            vec![],
            3_600,
        );
        assert!(first.is_ok());
        let second = auth.verify_and_issue_jwt(
            &address,
            &vk_hex,
            &sig_hex,
            uuid::Uuid::nil(),
            vec![],
            3_600,
        );
        assert!(second.is_err());
    }

    #[test]
    fn jwt_secret_length_is_enforced() {
        let too_short = "a".repeat(MIN_JWT_SECRET_LEN - 1);
        let result = AuthManager::validate_jwt_secret(&too_short);
        assert!(matches!(
            result,
            Err(AuthConfigError::JwtSecretTooShort {
                min_len: MIN_JWT_SECRET_LEN,
                actual_len: _
            })
        ));

        let valid = "a".repeat(MIN_JWT_SECRET_LEN);
        assert!(AuthManager::validate_jwt_secret(&valid).is_ok());
    }
}
