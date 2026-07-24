use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiResult},
    security::{generate_csrf_token, WebSecurityConfig, CSRF_HEADER_NAME},
    state::AppState,
    validation::extractors::{FieldError, Validatable},
};

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ChallengeQuery {
    /// Stellar wallet address to authenticate
    pub address: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ChallengeResponse {
    /// Same address passed in query
    pub address: String,
    /// Random nonce to be signed by the wallet
    pub nonce: String,
    /// How long before this challenge expires
    pub expires_in_seconds: u64,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = AuthVerifyRequest)]
pub struct VerifyRequest {
    /// Stellar wallet address being authenticated
    pub address: String,
    /// Ed25519 public key in hex
    pub public_key: String,
    /// Signed nonce in hex
    pub signature: String,
    /// Fine-grained scopes to embed in the JWT
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Custom token lifetime in seconds
    #[serde(default)]
    pub expires_in_seconds: Option<u64>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VerifyResponse {
    /// JSON Web Token for authentication
    pub token: String,
    /// Opaque refresh token for rotating access tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Always "Bearer"
    pub token_type: &'static str,
    /// Seconds until token expiration
    pub expires_in_seconds: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CsrfTokenResponse {
    pub token: String,
    pub header_name: &'static str,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
    #[serde(default)]
    pub expires_in_seconds: Option<u64>,
}

impl Validatable for RefreshTokenRequest {
    fn sanitize(&mut self) {
        self.refresh_token = self.refresh_token.trim().to_string();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        if self.refresh_token.is_empty() {
            Err(vec![FieldError::new(
                "refresh_token",
                "refresh_token is required",
            )])
        } else {
            Ok(())
        }
    }
}

/// Generate a CSRF token and same-site cookie for browser clients.
///
/// GET /api/auth/csrf
pub async fn get_csrf_token() -> ApiResult<(HeaderMap, Json<CsrfTokenResponse>)> {
    let config = WebSecurityConfig::from_env();
    let token = generate_csrf_token();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&config.csrf_cookie(&token))
            .map_err(|_| ApiError::internal("Failed to build CSRF cookie"))?,
    );
    headers.insert(
        CSRF_HEADER_NAME,
        HeaderValue::from_str(&token)
            .map_err(|_| ApiError::internal("Failed to build CSRF token header"))?,
    );

    Ok((
        headers,
        Json(CsrfTokenResponse {
            token,
            header_name: CSRF_HEADER_NAME,
            expires_in_seconds: 7_200,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/auth/challenge",
    params(ChallengeQuery),
    responses(
        (status = 200, description = "Challenge created", body = ChallengeResponse),
        (status = 400, description = "Invalid address provided")
    ),
    tag = "Authentication"
)]
pub async fn get_challenge(
    State(state): State<AppState>,
    Query(query): Query<ChallengeQuery>,
) -> ApiResult<Json<ChallengeResponse>> {
    if query.address.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidAddress",
            "address is required",
        ));
    }
    let mut mgr = state.auth_mgr.write().unwrap();
    let nonce = mgr.create_challenge(&query.address);
    Ok(Json(ChallengeResponse {
        address: query.address,
        nonce,
        expires_in_seconds: 300,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/verify",
    request_body = VerifyRequest,
    responses(
        (status = 200, description = "Authentication successful", body = VerifyResponse),
        (status = 401, description = "Authentication failed"),
        (status = 400, description = "Invalid payload")
    ),
    tag = "Authentication"
)]
pub async fn verify_challenge(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<VerifyRequest>,
) -> Result<(StatusCode, Json<VerifyResponse>), ApiError> {
    if payload.address.trim().is_empty()
        || payload.public_key.trim().is_empty()
        || payload.signature.trim().is_empty()
    {
        return Err(ApiError::bad_request(
            "InvalidPayload",
            "address, public_key and signature are required",
        ));
    }
    // Fetch publisher_id from address
    let publisher_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM publishers WHERE stellar_address = $1")
            .bind(&payload.address)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::NOT_FOUND,
                    "PublisherNotFound",
                    "Publisher not registered",
                )
            })?;

    let expires_in_seconds = payload
        .expires_in_seconds
        .unwrap_or(86_400)
        .clamp(300, 30 * 24 * 60 * 60);
    let mut mgr = state.auth_mgr.write().unwrap();
    let pair = mgr
        .verify_and_issue_token_pair(
            &payload.address,
            &payload.public_key,
            &payload.signature,
            publisher_id,
            payload.scopes,
            expires_in_seconds,
        )
        .map_err(|_| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                "AuthFailed",
                "invalid challenge response",
            )
        })?;
    Ok((
        StatusCode::OK,
        Json(VerifyResponse {
            token: pair.access_token,
            refresh_token: Some(pair.refresh_token),
            token_type: "Bearer",
            expires_in_seconds: pair.expires_in_seconds,
        }),
    ))
}

pub async fn refresh_token(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<RefreshTokenRequest>,
) -> Result<(StatusCode, Json<VerifyResponse>), ApiError> {
    let expires_in_seconds = payload
        .expires_in_seconds
        .unwrap_or(86_400)
        .clamp(300, 30 * 24 * 60 * 60);
    let pair = {
        let mut mgr = state
            .auth_mgr
            .write()
            .map_err(|_| ApiError::internal("Authentication state unavailable"))?;
        mgr.refresh_access_token(&payload.refresh_token, expires_in_seconds)
            .map_err(|_| ApiError::unauthorized("Invalid or expired refresh token"))?
    };

    Ok((
        StatusCode::OK,
        Json(VerifyResponse {
            token: pair.access_token,
            refresh_token: Some(pair.refresh_token),
            token_type: pair.token_type,
            expires_in_seconds: pair.expires_in_seconds,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthManager;
    use crate::cache::{CacheConfig, CacheLayer};
    use crate::contract_events::ContractEventHub;
    use crate::health_monitor::HealthMonitorStatus;
    use crate::rate_limit::RateLimitState;
    use crate::resource_tracking::ResourceManager;
    use crate::search_client::SearchClient;
    use crate::search_postgres::PostgresSearchService;
    use axum::extract::Query;
    use ed25519_dalek::{Signer, SigningKey};
    use prometheus::Registry;
    use std::sync::{Arc, RwLock};
    use std::time::Instant;
    use stellar_strkey::ed25519::PublicKey as StellarPublicKey;

    async fn test_app_state() -> AppState {
        let db = sqlx::pool::PoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost/test")
            .expect("lazy pool");
        let registry = Registry::new();
        let auth_mgr = Arc::new(RwLock::new(AuthManager::new(
            "a".repeat(32), // MIN_JWT_SECRET_LEN
        )));
        let resource_mgr = Arc::new(RwLock::new(ResourceManager::new()));
        let (job_engine, _rx) = soroban_batch::engine::JobEngine::new();
        let (event_broadcaster, _) = tokio::sync::broadcast::channel(100);
        AppState {
            db: db.clone(),
            started_at: Instant::now(),
            cache: Arc::new(CacheLayer::new(CacheConfig::default()).await),
            registry,
            job_engine: Arc::new(job_engine),
            is_shutting_down: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            health_monitor_status: HealthMonitorStatus::default(),
            auth_mgr,
            resource_mgr,
            contract_events: Arc::new(ContractEventHub::from_env()),
            source_storage: Arc::new(shared::source_storage::SourceStorage::new().await.unwrap()),
            event_broadcaster,
            search: Arc::new(SearchClient::new("http://localhost:9200").unwrap()),
            pg_search: Arc::new(PostgresSearchService::new(db)),
            ai_service: None,
            state_monitor: None,
            rate_limit_state: Arc::new(RateLimitState::from_env()),
        }
    }

    #[tokio::test]
    async fn challenge_returns_nonce_for_address() {
        let state = test_app_state().await;
        let key = SigningKey::from_bytes(&[1u8; 32]);
        let address = StellarPublicKey(*key.verifying_key().as_bytes()).to_string();
        let query = ChallengeQuery { address };
        let result = get_challenge(State(state.clone()), Query(query)).await;
        assert!(result.is_ok());
        let Json(resp) = result.unwrap();
        assert_eq!(resp.address.len(), 56);
        assert!(!resp.nonce.is_empty());
        assert_eq!(resp.expires_in_seconds, 300);
    }

    #[tokio::test]
    async fn challenge_rejects_empty_address() {
        let state = test_app_state().await;
        let query = ChallengeQuery {
            address: "   ".to_string(),
        };
        let result = get_challenge(State(state), Query(query)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn verify_issues_jwt_when_signature_valid() {
        let state = test_app_state().await;
        let key = SigningKey::from_bytes(&[1u8; 32]);
        let address = StellarPublicKey(*key.verifying_key().as_bytes()).to_string();
        let public_key_hex = hex::encode(key.verifying_key().as_bytes());

        let query = ChallengeQuery {
            address: address.clone(),
        };
        let challenge_result = get_challenge(State(state.clone()), Query(query)).await;
        assert!(challenge_result.is_ok());
        let Json(challenge_resp) = challenge_result.unwrap();
        let nonce = challenge_resp.nonce;

        let sig = key.sign(nonce.as_bytes());
        let signature_hex = hex::encode(sig.to_bytes());

        let payload = VerifyRequest {
            address: address.clone(),
            public_key: public_key_hex,
            signature: signature_hex,
            scopes: vec!["read".to_string()],
            expires_in_seconds: None,
        };
        let result = verify_challenge(State(state.clone()), Json(payload)).await;
        assert!(result.is_ok(), "{:?}", result.err());
        let (status, Json(resp)) = result.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(resp.token_type, "Bearer");
        assert!(!resp.token.is_empty());

        let mgr = state.auth_mgr.read().unwrap();
        let claims = mgr.validate_jwt(&resp.token).expect("valid JWT");
        assert_eq!(claims.sub, address);
    }
}
// TODO: structured audit logging
