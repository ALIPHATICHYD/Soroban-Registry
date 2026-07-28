use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::metrics;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ClientBreakerReport {
    pub endpoint: String,
    pub state: String,
    pub failures: Option<u32>,
    pub opened_at: Option<i64>,
}

pub async fn report_client_breaker(
    State(_state): State<AppState>,
    Json(payload): Json<ClientBreakerReport>,
) -> impl IntoResponse {
    let endpoint_label = payload.endpoint.as_str();
    let value = if payload.state.to_lowercase() == "open" {
        1
    } else {
        0
    };
    metrics::CLIENT_BREAKER_OPEN
        .with_label_values(&[endpoint_label])
        .set(value);

    (StatusCode::OK, Json(serde_json::json!({"ok": true})))
}
