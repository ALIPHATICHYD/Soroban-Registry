use crate::ai::{
    context_manager::ContextManager,
    models::{ChatMessage as DbChatMessage, ChatSession},
    prompt_builder::PromptBuilder,
    service::{ChatMessage, ContractContext},
};
use crate::error::ApiError;
use crate::state::AppState;
use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

// Request/Response types for AI endpoints

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub session_id: Uuid,
    pub message_id: Uuid,
    pub response: String,
    pub model_used: String,
    pub response_time_ms: u64,
    pub token_count: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeResponse {
    pub contract_id: Uuid,
    pub analysis: String,
    pub model_used: String,
    pub response_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VulnerabilityResponse {
    pub contract_id: Uuid,
    pub vulnerabilities: String,
    pub model_used: String,
    pub response_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExplainResponse {
    pub contract_id: Uuid,
    pub explanation: String,
    pub model_used: String,
    pub response_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestResponse {
    pub session_id: Option<Uuid>,
    pub suggestion: String,
    pub model_used: String,
    pub response_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionResponse {
    pub session: ChatSession,
    pub messages: Vec<DbChatMessage>,
}

/// AI Chat handler - general Q&A
pub async fn ai_chat_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    // Validate AI is configured
    let ai_service = state.ai_service.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "AI_SERVICE_NOT_CONFIGURED",
            "AI service is not configured. Set OPENAI_API_KEY or ANTHROPIC_API_KEY",
        )
    })?;

    // Get or create session
    let context_manager = ContextManager::new(state.db.clone());
    let session_id = payload.session_id.unwrap_or_else(Uuid::new_v4);

    // Get contract context if provided
    let contract_context = if let Some(contract_id) = payload.contract_id {
        let ctx = context_manager
            .get_contract_context(contract_id)
            .await
            .map_err(|e| {
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CONTEXT_ERROR",
                    e.to_string(),
                )
            })?;
        ctx
    } else {
        None
    };

    // Build messages with context
    let messages = PromptBuilder::build_chat_prompt(&payload.messages, contract_context.as_ref());

    // Call AI
    let ai_request = crate::ai::service::AIRequest {
        messages,
        model: payload.model,
        temperature: Some(0.7),
        max_tokens: Some(4096),
        stream: Some(false),
        contract_context: None, // Already embedded in system prompt
    };

    let start = std::time::Instant::now();
    let ai_response = ai_service.chat(ai_request).await.map_err(|e| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "AI_API_ERROR",
            e.to_string(),
        )
    })?;

    let response_time = start.elapsed().as_millis() as u64;

    // Save user message
    if let Err(e) = context_manager
        .add_message(
            session_id,
            "user",
            &payload
                .messages
                .last()
                .map(|m| m.content.as_str())
                .unwrap_or(""),
            None,
            None,
            None,
            None,
            None,
        )
        .await
    {
        warn!("Failed to save user message: {}", e);
    }

    // Save assistant message
    if let Err(e) = context_manager
        .add_message(
            session_id,
            "assistant",
            &ai_response.content,
            None,
            ai_response.token_count.map(|c| c as i32),
            Some(&ai_response.model_used),
            Some(response_time as i32),
            Some(serde_json::json!({"tokens": ai_response.token_count})),
        )
        .await
    {
        warn!("Failed to save assistant message: {}", e);
    }

    Ok(Json(ChatResponse {
        session_id,
        message_id: Uuid::new_v4(), // Would fetch from DB
        response: ai_response.content,
        model_used: ai_response.model_used,
        response_time_ms: response_time,
        token_count: ai_response.token_count,
    }))
}

/// Real-time AI chat via WebSocket
pub async fn ai_chat_ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(|socket| handle_ai_chat_ws(socket, state))
}

async fn handle_ai_chat_ws(socket: axum::extract::ws::WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let ai_service = match state.ai_service.as_ref() {
        Some(s) => s.clone(),
        None => {
            let _ = sender
                .send(axum::extract::ws::Message::Text(
                    serde_json::json!({"error": "AI service not configured"}).to_string(),
                ))
                .await;
            return;
        }
    };

    let context_manager = ContextManager::new(state.db.clone());

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                let request: Result<ChatRequest, _> = serde_json::from_str(&text);
                match request {
                    Ok(payload) => {
                        let session_id = payload.session_id.unwrap_or_else(Uuid::new_v4);

                        // Get contract context if needed
                        let contract_context = if let Some(contract_id) = payload.contract_id {
                            context_manager
                                .get_contract_context(contract_id)
                                .await
                                .ok()
                                .flatten()
                        } else {
                            None
                        };

                        // Build messages
                        let messages = PromptBuilder::build_chat_prompt(
                            &payload.messages,
                            contract_context.as_ref(),
                        );

                        // Create AI request
                        let ai_request = crate::ai::service::AIRequest {
                            messages,
                            model: payload.model,
                            temperature: Some(0.7),
                            max_tokens: Some(4096),
                            stream: Some(true),
                            contract_context: None,
                        };

                        // Call AI with streaming
                        match ai_service.chat(ai_request).await {
                            Ok(response) => {
                                let resp_msg = serde_json::json!({
                                    "type": "response",
                                    "session_id": session_id,
                                    "content": response.content,
                                    "model": response.model_used,
                                    "response_time_ms": response.response_time_ms,
                                });
                                let _ = sender
                                    .send(axum::extract::ws::Message::Text(resp_msg.to_string()))
                                    .await;
                            }
                            Err(e) => {
                                let err_msg = serde_json::json!({
                                    "type": "error",
                                    "error": e.to_string(),
                                });
                                let _ = sender
                                    .send(axum::extract::ws::Message::Text(err_msg.to_string()))
                                    .await;
                            }
                        }
                    }
                    Err(e) => {
                        let err_msg = serde_json::json!({
                            "type": "error",
                            "error": format!("Invalid request: {}", e),
                        });
                        let _ = sender
                            .send(axum::extract::ws::Message::Text(err_msg.to_string()))
                            .await;
                    }
                }
            }
            axum::extract::ws::Message::Close(_) => break,
            _ => {}
        }
    }
}

/// Analyze contract specifically
pub async fn analyze_contract_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<AnalyzeRequest>,
) -> Result<Json<AnalyzeResponse>, ApiError> {
    let ai_service = state.ai_service.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "AI_SERVICE_NOT_CONFIGURED",
            "AI service not configured",
        )
    })?;

    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request_with("INVALID_CONTRACT_ID", "Invalid contract ID format")
    })?;

    // Fetch contract code from source storage
    let contract_code = sqlx::query_scalar::<_, Option<String>>(
        "SELECT source_code FROM verifications
         WHERE contract_id = $1 AND status = 'verified'
         ORDER BY verified_at DESC LIMIT 1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", e.to_string()))?
    .flatten()
    .ok_or_else(|| {
        ApiError::not_found("CONTRACT_NOT_FOUND", "Contract or source code not found")
    })?;

    let contract: shared::models::Contract =
        sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
            .bind(contract_uuid)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", e.to_string())
            })?
            .ok_or_else(|| ApiError::not_found("CONTRACT_NOT_FOUND", "Contract not found"))?;

    let ctx = ContractContext {
        contract_id: contract_uuid.to_string(),
        contract_name: contract.name.clone(),
        contract_code,
        description: contract.description,
        category: contract.category,
        tags: contract.tags.iter().map(|t| t.name.clone()).collect(),
    };

    let prompt = PromptBuilder::build_analysis_prompt(
        &ctx.contract_code,
        &ctx.contract_name,
        ctx.description.as_deref(),
        ctx.category.as_deref(),
        &ctx.tags,
    );

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    let ai_req = crate::ai::service::AIRequest {
        messages,
        model: params.model,
        temperature: Some(0.7),
        max_tokens: Some(4096),
        stream: Some(false),
        contract_context: Some(ctx),
    };

    let start = std::time::Instant::now();
    let response = ai_service
        .chat(ai_req)
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "AI_ERROR", e.to_string()))?;

    Ok(Json(AnalyzeResponse {
        contract_id: contract_uuid,
        analysis: response.content,
        model_used: response.model_used,
        response_time_ms: response.response_time_ms,
    }))
}

/// Check for vulnerabilities
pub async fn check_vulnerabilities_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<VulnerabilityRequest>,
) -> Result<Json<VulnerabilityResponse>, ApiError> {
    let ai_service = state.ai_service.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "AI_SERVICE_NOT_CONFIGURED",
            "AI service not configured",
        )
    })?;

    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request_with("INVALID_CONTRACT_ID", "Invalid contract ID format")
    })?;

    let contract_code = sqlx::query_scalar::<_, Option<String>>(
        "SELECT source_code FROM verifications
         WHERE contract_id = $1 AND status = 'verified'
         ORDER BY verified_at DESC LIMIT 1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", e.to_string()))?
    .flatten()
    .ok_or_else(|| {
        ApiError::not_found("CONTRACT_NOT_FOUND", "Contract or source code not found")
    })?;

    let prompt = PromptBuilder::build_vulnerability_prompt(&contract_code);
    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    let ai_req = crate::ai::service::AIRequest {
        messages,
        model: params.model,
        temperature: Some(0.3), // Lower temperature for security analysis
        max_tokens: Some(4096),
        stream: Some(false),
        contract_context: None,
    };

    let start = std::time::Instant::now();
    let response = ai_service
        .chat(ai_req)
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "AI_ERROR", e.to_string()))?;

    Ok(Json(VulnerabilityResponse {
        contract_id: contract_uuid,
        vulnerabilities: response.content,
        model_used: response.model_used,
        response_time_ms: response.response_time_ms,
    }))
}

/// Explain contract code
pub async fn explain_contract_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<ExplainRequest>,
) -> Result<Json<ExplainResponse>, ApiError> {
    let ai_service = state.ai_service.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "AI_SERVICE_NOT_CONFIGURED",
            "AI service not configured",
        )
    })?;

    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request_with("INVALID_CONTRACT_ID", "Invalid contract ID format")
    })?;

    let contract_code = sqlx::query_scalar::<_, Option<String>>(
        "SELECT source_code FROM verifications
         WHERE contract_id = $1 AND status = 'verified'
         ORDER BY verified_at DESC LIMIT 1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", e.to_string()))?
    .flatten()
    .ok_or_else(|| {
        ApiError::not_found("CONTRACT_NOT_FOUND", "Contract or source code not found")
    })?;

    let prompt = PromptBuilder::build_explanation_prompt(&contract_code, params.focus.as_deref());
    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    let ai_req = crate::ai::service::AIRequest {
        messages,
        model: params.model,
        temperature: Some(0.5),
        max_tokens: Some(4096),
        stream: Some(false),
        contract_context: None,
    };

    let start = std::time::Instant::now();
    let response = ai_service
        .chat(ai_req)
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "AI_ERROR", e.to_string()))?;

    Ok(Json(ExplainResponse {
        contract_id: contract_uuid,
        explanation: response.content,
        model_used: response.model_used,
        response_time_ms: response.response_time_ms,
    }))
}

/// Get code suggestions
pub async fn suggest_code_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    ValidatedJson(payload): ValidatedJson<SuggestRequest>,
) -> Result<Json<SuggestResponse>, ApiError> {
    let ai_service = state.ai_service.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "AI_SERVICE_NOT_CONFIGURED",
            "AI service not configured",
        )
    })?;

    let contract_uuid = Uuid::parse_str(&contract_id).map_err(|_| {
        ApiError::bad_request_with("INVALID_CONTRACT_ID", "Invalid contract ID format")
    })?;

    let contract_code = sqlx::query_scalar::<_, Option<String>>(
        "SELECT source_code FROM verifications
         WHERE contract_id = $1 AND status = 'verified'
         ORDER BY verified_at DESC LIMIT 1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", e.to_string()))?
    .flatten()
    .ok_or_else(|| {
        ApiError::not_found("CONTRACT_NOT_FOUND", "Contract or source code not found")
    })?;

    let prompt = PromptBuilder::build_suggestion_prompt(
        &contract_code,
        &payload.request,
        payload.context.as_deref(),
    );

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    let ai_req = crate::ai::service::AIRequest {
        messages,
        model: payload.model,
        temperature: Some(0.7),
        max_tokens: Some(4096),
        stream: Some(false),
        contract_context: None,
    };

    let start = std::time::Instant::now();
    let response = ai_service
        .chat(ai_req)
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "AI_ERROR", e.to_string()))?;

    Ok(Json(SuggestResponse {
        session_id: None,
        suggestion: response.content,
        model_used: response.model_used,
        response_time_ms: response.response_time_ms,
    }))
}

/// Get chat sessions for a user
pub async fn get_chat_sessions_handler(
    State(state): State<AppState>,
    Query(params): Query<GetSessionsQuery>,
) -> Result<Json<Vec<ChatSession>>, ApiError> {
    let context_manager = ContextManager::new(state.db.clone());
    let user_id = params
        .user_id
        .ok_or_else(|| ApiError::bad_request_with("MISSING_USER_ID", "user_id is required"))?;

    let sessions = context_manager
        .get_user_sessions(user_id, params.limit.unwrap_or(20))
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", e.to_string()))?;

    Ok(Json(sessions))
}

/// Get specific chat session with messages
pub async fn get_chat_session_handler(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SessionResponse>, ApiError> {
    let context_manager = ContextManager::new(state.db.clone());

    let (session, messages) = context_manager
        .get_session_with_messages(session_id)
        .await
        .map_err(|e| ApiError::not_found("SESSION_NOT_FOUND", e.to_string()))?;

    Ok(Json(SessionResponse { session, messages }))
}

// Request query types
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub session_id: Option<Uuid>,
    pub contract_id: Option<Uuid>,
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    #[serde(rename = "model")]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VulnerabilityRequest {
    #[serde(rename = "model")]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExplainRequest {
    #[serde(rename = "model")]
    pub model: Option<String>,
    pub focus: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SuggestRequest {
    pub request: String,
    pub context: Option<String>,
    #[serde(rename = "model")]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetSessionsQuery {
    pub user_id: Option<Uuid>,
    pub limit: Option<i32>,
}
