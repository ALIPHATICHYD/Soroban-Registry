use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::ApiError,
    models::{
        FormalVerificationProperty, FormalVerificationPropertyResult, FormalVerificationReport,
        FormalVerificationResult, FormalVerificationSession, RunVerificationRequest,
        VerificationStatus,
    },
    state::AppState,
};

/// POST /api/contracts/:id/formal-verification
/// For simplicity, the engine logic lives heavily in the CLI. 
/// This endpoint accepts the *result* (or simulates it) and persists it.
pub async fn run_formal_verification(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Json(payload): Json<FormalVerificationReport>, // Expecting the full report from CLI
) -> Result<Json<FormalVerificationReport>, ApiError> {
    let mut tx = state.pool.begin().await.map_err(|e| {
        log::error!("Database transaction failed: {}", e);
        ApiError::InternalServerError
    })?;

    let session = payload.session.clone();
    
    // Insert session
    sqlx::query!(
        r#"
        INSERT INTO formal_verification_sessions (id, contract_id, version, verifier_version, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        session.id,
        contract_id,
        session.version,
        session.verifier_version,
        session.created_at,
        session.updated_at
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        log::error!("Failed to insert formal verification session: {}", e);
        ApiError::InternalServerError
    })?;

    // Insert properties and results
    for prop_out in payload.properties.iter() {
        let prop = &prop_out.property;
        sqlx::query!(
            r#"
            INSERT INTO formal_verification_properties (id, session_id, property_id, description, invariant, severity)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            prop.id,
            session.id,
            prop.property_id,
            prop.description,
            prop.invariant,
            prop.severity
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            log::error!("Failed to insert formal verification property: {}", e);
            ApiError::InternalServerError
        })?;

        let res = &prop_out.result;
        sqlx::query!(
            r#"
            INSERT INTO formal_verification_results (id, property_id, status, counterexample, details)
            VALUES ($1, $2, $3::formal_verification_status, $4, $5)
            "#,
            res.id,
            prop.id,
            res.status as VerificationStatus,
            res.counterexample,
            res.details
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            log::error!("Failed to insert formal verification result: {}", e);
            ApiError::InternalServerError
        })?;
    }

    tx.commit().await.map_err(|e| {
        log::error!("Failed to commit transaction: {}", e);
        ApiError::InternalServerError
    })?;

    Ok(Json(payload))
}

/// GET /api/contracts/:id/formal-verification
pub async fn get_formal_verification_history(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> Result<Json<Vec<FormalVerificationReport>>, ApiError> {
    let sessions = sqlx::query_as!(
        FormalVerificationSession,
        r#"
        SELECT id, contract_id, version, verifier_version, created_at, updated_at
        FROM formal_verification_sessions
        WHERE contract_id = $1
        ORDER BY created_at DESC
        "#,
        contract_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::InternalServerError)?;

    let mut reports = Vec::new();

    for session in sessions {
        let properties = sqlx::query_as!(
            FormalVerificationProperty,
            r#"
            SELECT id, session_id, property_id, description, invariant, severity
            FROM formal_verification_properties
            WHERE session_id = $1
            "#,
            session.id
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|_| ApiError::InternalServerError)?;

        let mut prop_results = Vec::new();
        for prop in properties.into_iter() {
            let result = sqlx::query_as!(
                FormalVerificationResult,
                r#"
                SELECT id, property_id, status as "status: VerificationStatus", counterexample, details
                FROM formal_verification_results
                WHERE property_id = $1
                "#,
                prop.id
            )
            .fetch_one(&state.pool)
            .await
            .map_err(|_| ApiError::InternalServerError)?;

            prop_results.push(FormalVerificationPropertyResult {
                property: prop,
                result,
            });
        }

        reports.push(FormalVerificationReport {
            session,
            properties: prop_results,
        });
    }

    Ok(Json(reports))
}
