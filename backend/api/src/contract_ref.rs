//! Resolving a `:id` path parameter that may be a registry UUID or a Stellar
//! contract address (Issue #1147).
//!
//! The dependency routes previously declared `Path<Uuid>` while the CLI sent a
//! `C...` strkey, so axum rejected the request with 400 before any handler ran.
//! Accepting both is the fix, but a bare address is genuinely ambiguous:
//! `contracts` is `UNIQUE(contract_id, network)`, so the same address can name a
//! different contract on mainnet and on testnet.
//!
//! Rather than silently picking one -- which would answer a question about the
//! wrong contract, and do so invisibly -- an ambiguous address without `?network=`
//! returns **409 with the candidates listed**, so the caller can retry with the
//! network they meant.

use crate::error::{ApiError, ApiResult};
use crate::handlers::db_internal_error;
use crate::state::AppState;
use serde_json::json;
use shared::Network;
use uuid::Uuid;

/// A contract resolved from a path parameter, with the network it lives on.
///
/// The network travels with the id because every dependency query is
/// network-scoped; re-deriving it downstream would be an extra round trip and an
/// opportunity to scope a query to the wrong network.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedContract {
    pub uuid: Uuid,
    pub network: Network,
}

#[derive(sqlx::FromRow)]
struct ContractIdentity {
    id: Uuid,
    contract_id: String,
    network: Network,
}

/// Resolve `id` (a UUID or a Stellar contract address) to a contract.
///
/// `network` disambiguates an address that is registered on more than one
/// network. It is ignored for a UUID, which is already unique.
pub async fn resolve(
    state: &AppState,
    id: &str,
    network: Option<Network>,
) -> ApiResult<ResolvedContract> {
    let id = id.trim();

    if let Ok(uuid) = Uuid::parse_str(id) {
        let row: Option<ContractIdentity> =
            sqlx::query_as("SELECT id, contract_id, network FROM contracts WHERE id = $1")
                .bind(uuid)
                .fetch_optional(&state.db)
                .await
                .map_err(|err| db_internal_error("resolve contract by uuid", err))?;

        let row = row.ok_or_else(|| not_found(id))?;

        // A UUID names exactly one row, so a mismatched `?network=` is a
        // contradiction in the request rather than an ambiguity to resolve.
        if let Some(requested) = network {
            if row.network != requested {
                return Err(ApiError::bad_request(
                    "NetworkMismatch",
                    format!(
                        "Contract {id} is registered on {}, but network={requested} was requested",
                        row.network
                    ),
                ));
            }
        }

        return Ok(ResolvedContract {
            uuid: row.id,
            network: row.network,
        });
    }

    // Not a UUID: it must be a Stellar contract address. Validating up front
    // turns a typo into a 400 that says what was wrong, instead of a 404 that
    // implies the contract might exist somewhere.
    crate::validation::validate_contract_id(id).map_err(|reason| {
        ApiError::bad_request(
            "InvalidContractRef",
            format!("'{id}' is neither a contract UUID nor a Stellar contract address: {reason}"),
        )
    })?;

    if let Some(network) = network {
        let row: Option<ContractIdentity> = sqlx::query_as(
            "SELECT id, contract_id, network FROM contracts WHERE contract_id = $1 AND network = $2",
        )
        .bind(id)
        .bind(network)
        .fetch_optional(&state.db)
        .await
        .map_err(|err| db_internal_error("resolve contract by address and network", err))?;

        let row = row.ok_or_else(|| {
            ApiError::not_found(
                "ContractNotFound",
                format!("Contract {id} is not registered on {network}"),
            )
        })?;

        return Ok(ResolvedContract {
            uuid: row.id,
            network: row.network,
        });
    }

    let candidates: Vec<ContractIdentity> = sqlx::query_as(
        "SELECT id, contract_id, network FROM contracts WHERE contract_id = $1 ORDER BY network",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("resolve contract by address", err))?;

    match candidates.len() {
        0 => Err(not_found(id)),
        1 => {
            let row = &candidates[0];
            Ok(ResolvedContract {
                uuid: row.id,
                network: row.network,
            })
        }
        _ => Err(ApiError::conflict(
            "AmbiguousContractRef",
            format!(
                "Contract address {id} is registered on {} networks. Retry with ?network=",
                candidates.len()
            ),
        )
        .with_details(json!({
            "candidates": candidates
                .iter()
                .map(|row| json!({
                    "id": row.id,
                    "contract_id": row.contract_id,
                    "network": row.network.to_string(),
                }))
                .collect::<Vec<_>>()
        }))),
    }
}

fn not_found(id: &str) -> ApiError {
    ApiError::not_found("ContractNotFound", format!("Contract {id} not found"))
}
