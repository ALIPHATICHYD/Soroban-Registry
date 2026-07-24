/// GET /api/quota — returns the caller's current rate-limit quota usage (issue #727).
///
/// The response includes hourly limit, consumed requests, remaining requests, and
/// burst limit for the 1-minute sliding window.  Tier is resolved from the
/// `X-Api-Plan: free | pro | enterprise` request header (defaults to `free`).
use axum::{
    extract::{Request, State},
    http::StatusCode,
    Json,
};

use crate::{
    error::{ApiError, ApiResult},
    rate_limit::{ApiTier, QuotaSnapshot},
    state::AppState,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct QuotaResponse {
    pub client_key: String,
    pub quota: QuotaSnapshot,
}

pub async fn get_quota(
    State(state): State<AppState>,
    request: Request,
) -> ApiResult<Json<QuotaResponse>> {
    // Resolve tier from header
    let tier = request
        .headers()
        .get("x-api-plan")
        .and_then(|v| v.to_str().ok())
        .map(ApiTier::from_header)
        .unwrap_or(ApiTier::Free);

    // Build the same bucket key the middleware uses
    let client_key = if let Some(auth) = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
    {
        format!("auth:{auth}")
    } else {
        let ip = extract_ip(&request);
        format!("anon:{ip}")
    };

    let quota = state
        .rate_limit_state
        .quota_snapshot(&client_key, &tier)
        .await;

    Ok(Json(QuotaResponse { client_key, quota }))
}

fn extract_ip(request: &Request) -> String {
    if let Some(ip) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| {
            raw.split(',')
                .map(str::trim)
                .find_map(|s| s.parse::<std::net::IpAddr>().ok())
        })
    {
        return ip.to_string();
    }

    if let Some(ip) = request
        .headers()
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<std::net::IpAddr>().ok())
    {
        return ip.to_string();
    }

    if let Some(info) = request
        .extensions()
        .get::<axum::extract::connect_info::ConnectInfo<std::net::SocketAddr>>()
    {
        return info.0.ip().to_string();
    }

    "unknown".to_string()
}

#[allow(dead_code)]
fn _assert_status_unused() {
    let _ = StatusCode::OK;
    let _ = ApiError::not_found("", "");

    pub fn quota_routes() -> axum::Router<AppState> {
        use axum::routing::get;

        axum::Router::new().route("/api/quota", get(get_quota))
    }
}
