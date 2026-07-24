use axum::{
    extract::{Path, State},
    Json,
};
use shared::ContractInteroperabilityResponse;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    interoperability,
    state::AppState,
};

/// GET /api/contracts/:id/interoperability
///
/// Returns protocol compliance, interoperable contract suggestions, and graph relationships
/// for a single contract.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/interoperability",
    params(("id" = String, Path, description = "Contract UUID")),
    responses(
        (status = 200, description = "Interoperability analysis", body = ContractInteroperabilityResponse),
        (status = 404, description = "Contract not found")
    ),
    tag = "Analysis"
)]
pub async fn get_contract_interoperability(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ContractInteroperabilityResponse>> {
    let contract_id = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request_with("InvalidContractId", format!("Invalid ID: {id}"))
    })?;

    interoperability::analyze_contract_interoperability(&state.db, contract_id)
        .await
        .map(Json)
}
