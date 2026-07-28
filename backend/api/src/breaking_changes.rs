use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use contract_abi::{
    self, parser::parse_json_spec, ContractABI, ContractFunction, EnumVariant, SorobanType,
    StructField,
};

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSeverity {
    Breaking,
    NonBreaking,
}

#[derive(Debug, Serialize, Clone, utoipa::ToSchema)]
pub struct BreakingChange {
    pub severity: ChangeSeverity,
    pub category: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
}

#[derive(Debug, Serialize, Clone, utoipa::ToSchema)]
pub struct BreakingChangeReport {
    pub old_id: String,
    pub new_id: String,
    pub breaking: bool,
    pub breaking_count: usize,
    pub non_breaking_count: usize,
    pub changes: Vec<BreakingChange>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct BreakingChangeQuery {
    pub old_id: String,
    pub new_id: String,
    pub bypass_cache: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/api/contracts/breaking-changes",
    params(
        ("old_id" = String, Query, description = "Old contract@version selector"),
        ("new_id" = String, Query, description = "New contract@version selector")
    ),
    responses(
        (status = 200, description = "Breaking change report", body = BreakingChangeReport),
        (status = 400, description = "Invalid ABI or selector")
    ),
    tag = "Analysis"
)]
pub async fn get_breaking_changes(
    Query(query): Query<BreakingChangeQuery>,
    State(state): State<AppState>,
) -> ApiResult<Json<BreakingChangeReport>> {
    let bypass = query.bypass_cache.unwrap_or(false);
    let old_abi = resolve_abi(&state, &query.old_id, bypass).await?;
    let new_abi = resolve_abi(&state, &query.new_id, bypass).await?;

    let old_spec = parse_json_spec(&old_abi, &query.old_id).map_err(|e| {
        ApiError::bad_request_with("InvalidABI", format!("Failed to parse old ABI: {}", e))
    })?;
    let new_spec = parse_json_spec(&new_abi, &query.new_id).map_err(|e| {
        ApiError::bad_request_with("InvalidABI", format!("Failed to parse new ABI: {}", e))
    })?;

    let changes = diff_abi(&old_spec, &new_spec);
    let breaking_count = changes
        .iter()
        .filter(|c| c.severity == ChangeSeverity::Breaking)
        .count();
    let non_breaking_count = changes.len() - breaking_count;

    Ok(Json(BreakingChangeReport {
        old_id: query.old_id,
        new_id: query.new_id,
        breaking: breaking_count > 0,
        breaking_count,
        non_breaking_count,
        changes,
    }))
}

pub fn diff_abi(old: &ContractABI, new: &ContractABI) -> Vec<BreakingChange> {
    let mut changes = Vec::new();

    let old_funcs: HashMap<&str, &ContractFunction> =
        old.functions.iter().map(|f| (f.name.as_str(), f)).collect();
    let new_funcs: HashMap<&str, &ContractFunction> =
        new.functions.iter().map(|f| (f.name.as_str(), f)).collect();

    for name in old_funcs.keys() {
        if !new_funcs.contains_key(name) {
            changes.push(BreakingChange {
                severity: ChangeSeverity::Breaking,
                category: "function_removed".to_string(),
                message: format!("Function '{}' was removed", name),
                function: Some((*name).to_string()),
                type_name: None,
            });
        }
    }

    for name in new_funcs.keys() {
        if !old_funcs.contains_key(name) {
            changes.push(BreakingChange {
                severity: ChangeSeverity::NonBreaking,
                category: "function_added".to_string(),
                message: format!("Function '{}' was added", name),
                function: Some((*name).to_string()),
                type_name: None,
            });
        }
    }

    for (name, old_func) in &old_funcs {
        if let Some(new_func) = new_funcs.get(name) {
            diff_function(&mut changes, old_func, new_func);
        }
    }

    diff_types(&mut changes, &old.types, &new.types);

    changes
}

fn diff_function(
    changes: &mut Vec<BreakingChange>,
    old_func: &ContractFunction,
    new_func: &ContractFunction,
) {
    if old_func.params.len() != new_func.params.len() {
        changes.push(BreakingChange {
            severity: ChangeSeverity::Breaking,
            category: "function_params_changed".to_string(),
            message: format!(
                "Function '{}' parameter count changed from {} to {}",
                old_func.name,
                old_func.params.len(),
                new_func.params.len()
            ),
            function: Some(old_func.name.clone()),
            type_name: None,
        });
    }

    let min_len = std::cmp::min(old_func.params.len(), new_func.params.len());
    for idx in 0..min_len {
        let old_param = &old_func.params[idx];
        let new_param = &new_func.params[idx];

        if old_param.param_type != new_param.param_type {
            changes.push(BreakingChange {
                severity: ChangeSeverity::Breaking,
                category: "param_type_changed".to_string(),
                message: format!(
                    "Function '{}' param '{}' type changed from '{}' to '{}'",
                    old_func.name,
                    old_param.name,
                    old_param.param_type.display_name(),
                    new_param.param_type.display_name()
                ),
                function: Some(old_func.name.clone()),
                type_name: None,
            });
        } else if old_param.name != new_param.name {
            changes.push(BreakingChange {
                severity: ChangeSeverity::NonBreaking,
                category: "param_name_changed".to_string(),
                message: format!(
                    "Function '{}' param name changed from '{}' to '{}'",
                    old_func.name, old_param.name, new_param.name
                ),
                function: Some(old_func.name.clone()),
                type_name: None,
            });
        }
    }

    if old_func.return_type != new_func.return_type {
        changes.push(BreakingChange {
            severity: ChangeSeverity::Breaking,
            category: "return_type_changed".to_string(),
            message: format!(
                "Function '{}' return type changed from '{}' to '{}'",
                old_func.name,
                old_func.return_type.display_name(),
                new_func.return_type.display_name()
            ),
            function: Some(old_func.name.clone()),
            type_name: None,
        });
    }
}

fn diff_types(
    changes: &mut Vec<BreakingChange>,
    old_types: &HashMap<String, SorobanType>,
    new_types: &HashMap<String, SorobanType>,
) {
    for (name, old_type) in old_types {
        if let Some(new_type) = new_types.get(name) {
            if old_type != new_type {
                diff_type_definition(changes, name, old_type, new_type);
            }
        } else {
            changes.push(BreakingChange {
                severity: ChangeSeverity::Breaking,
                category: "type_removed".to_string(),
                message: format!("Type '{}' was removed", name),
                function: None,
                type_name: Some(name.clone()),
            });
        }
    }

    for name in new_types.keys() {
        if !old_types.contains_key(name) {
            changes.push(BreakingChange {
                severity: ChangeSeverity::NonBreaking,
                category: "type_added".to_string(),
                message: format!("Type '{}' was added", name),
                function: None,
                type_name: Some(name.clone()),
            });
        }
    }
}

fn diff_type_definition(
    changes: &mut Vec<BreakingChange>,
    name: &str,
    old_type: &SorobanType,
    new_type: &SorobanType,
) {
    match (old_type, new_type) {
        (
            SorobanType::Struct {
                fields: old_fields, ..
            },
            SorobanType::Struct {
                fields: new_fields, ..
            },
        ) => {
            diff_struct_fields(changes, name, old_fields, new_fields);
        }
        (
            SorobanType::Enum {
                variants: old_variants,
                ..
            },
            SorobanType::Enum {
                variants: new_variants,
                ..
            },
        ) => {
            diff_enum_variants(changes, name, old_variants, new_variants);
        }
        _ => {
            changes.push(BreakingChange {
                severity: ChangeSeverity::Breaking,
                category: "type_changed".to_string(),
                message: format!("Type '{}' changed definition", name),
                function: None,
                type_name: Some(name.to_string()),
            });
        }
    }
}

fn diff_struct_fields(
    changes: &mut Vec<BreakingChange>,
    type_name: &str,
    old_fields: &[StructField],
    new_fields: &[StructField],
) {
    let old_map: HashMap<&str, &StructField> =
        old_fields.iter().map(|f| (f.name.as_str(), f)).collect();
    let new_map: HashMap<&str, &StructField> =
        new_fields.iter().map(|f| (f.name.as_str(), f)).collect();

    for (name, field) in &old_map {
        if let Some(new_field) = new_map.get(name) {
            if field.field_type != new_field.field_type {
                changes.push(BreakingChange {
                    severity: ChangeSeverity::Breaking,
                    category: "type_field_changed".to_string(),
                    message: format!(
                        "Type '{}' field '{}' changed from '{}' to '{}'",
                        type_name,
                        name,
                        field.field_type.display_name(),
                        new_field.field_type.display_name()
                    ),
                    function: None,
                    type_name: Some(type_name.to_string()),
                });
            }
        } else {
            changes.push(BreakingChange {
                severity: ChangeSeverity::Breaking,
                category: "type_field_removed".to_string(),
                message: format!("Type '{}' field '{}' was removed", type_name, name),
                function: None,
                type_name: Some(type_name.to_string()),
            });
        }
    }

    for name in new_map.keys() {
        if !old_map.contains_key(name) {
            changes.push(BreakingChange {
                severity: ChangeSeverity::Breaking,
                category: "type_field_added".to_string(),
                message: format!("Type '{}' field '{}' was added", type_name, name),
                function: None,
                type_name: Some(type_name.to_string()),
            });
        }
    }
}

fn diff_enum_variants(
    changes: &mut Vec<BreakingChange>,
    type_name: &str,
    old_variants: &[EnumVariant],
    new_variants: &[EnumVariant],
) {
    let old_map: HashMap<&str, &EnumVariant> =
        old_variants.iter().map(|v| (v.name.as_str(), v)).collect();
    let new_map: HashMap<&str, &EnumVariant> =
        new_variants.iter().map(|v| (v.name.as_str(), v)).collect();

    for (name, old_variant) in &old_map {
        if let Some(new_variant) = new_map.get(name) {
            if old_variant.fields != new_variant.fields {
                changes.push(BreakingChange {
                    severity: ChangeSeverity::Breaking,
                    category: "enum_variant_changed".to_string(),
                    message: format!("Enum '{}' variant '{}' changed", type_name, name),
                    function: None,
                    type_name: Some(type_name.to_string()),
                });
            }
        } else {
            changes.push(BreakingChange {
                severity: ChangeSeverity::Breaking,
                category: "enum_variant_removed".to_string(),
                message: format!("Enum '{}' variant '{}' was removed", type_name, name),
                function: None,
                type_name: Some(type_name.to_string()),
            });
        }
    }

    for name in new_map.keys() {
        if !old_map.contains_key(name) {
            changes.push(BreakingChange {
                severity: ChangeSeverity::NonBreaking,
                category: "enum_variant_added".to_string(),
                message: format!("Enum '{}' variant '{}' was added", type_name, name),
                function: None,
                type_name: Some(type_name.to_string()),
            });
        }
    }
}

pub(crate) async fn resolve_abi(
    state: &AppState,
    selector: &str,
    bypass_cache: bool,
) -> ApiResult<String> {
    if let Some(cached) = state.cache.get_abi(selector, bypass_cache).await {
        return Ok(cached);
    }

    let abi_result = if let Some((contract_id, version)) = selector.split_once('@') {
        fetch_abi_by_contract_and_version(state, contract_id, version).await
    } else if let Ok(version_id) = Uuid::parse_str(selector) {
        if let Some((contract_id, version)) = fetch_contract_version(state, version_id).await? {
            fetch_abi_by_contract_uuid_and_version(state, contract_id, &version).await
        } else {
            fetch_latest_abi_for_contract(state, selector).await
        }
    } else {
        fetch_latest_abi_for_contract(state, selector).await
    };

    if let Ok(abi) = &abi_result {
        state.cache.put_abi(selector, abi.clone()).await;
    }

    abi_result
}

async fn fetch_contract_version(
    state: &AppState,
    version_id: Uuid,
) -> ApiResult<Option<(Uuid, String)>> {
    let row = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT contract_id, version FROM contract_versions WHERE id = $1",
    )
    .bind(version_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(row)
}

async fn fetch_contract_uuid(state: &AppState, contract_id: &str) -> ApiResult<Uuid> {
    if let Ok(uuid) = Uuid::parse_str(contract_id) {
        return Ok(uuid);
    }

    let uuid = sqlx::query_scalar::<_, Uuid>("SELECT id FROM contracts WHERE contract_id = $1")
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| {
            ApiError::not_found(
                "ContractNotFound",
                format!("Contract '{}' not found", contract_id),
            )
        })?;

    Ok(uuid)
}

async fn fetch_latest_abi_for_contract(state: &AppState, contract_id: &str) -> ApiResult<String> {
    let uuid = fetch_contract_uuid(state, contract_id).await?;

    if let Some(abi) = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT abi FROM contract_abis WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    {
        return Ok(abi.to_string());
    }

    let abi = sqlx::query_scalar::<_, serde_json::Value>("SELECT abi FROM contracts WHERE id = $1")
        .bind(uuid)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| {
            ApiError::not_found(
                "AbiNotFound",
                format!("No ABI available for contract '{}'", contract_id),
            )
        })?;

    Ok(abi.to_string())
}

async fn fetch_abi_by_contract_and_version(
    state: &AppState,
    contract_id: &str,
    version: &str,
) -> ApiResult<String> {
    let uuid = fetch_contract_uuid(state, contract_id).await?;
    fetch_abi_by_contract_uuid_and_version(state, uuid, version).await
}

async fn fetch_abi_by_contract_uuid_and_version(
    state: &AppState,
    contract_id: Uuid,
    version: &str,
) -> ApiResult<String> {
    if let Some(abi) = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT abi FROM contract_abis WHERE contract_id = $1 AND version = $2",
    )
    .bind(contract_id)
    .bind(version)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    {
        return Ok(abi.to_string());
    }

    Err(ApiError::not_found(
        "AbiNotFound",
        format!("No ABI available for contract version '{}'", version),
    ))
}

pub fn has_breaking_changes(changes: &[BreakingChange]) -> bool {
    changes
        .iter()
        .any(|c| c.severity == ChangeSeverity::Breaking)
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract_abi::{ContractABI, ContractFunction, FunctionParam, FunctionVisibility};

    fn func(name: &str, params: Vec<FunctionParam>, return_type: SorobanType) -> ContractFunction {
        ContractFunction {
            name: name.to_string(),
            visibility: FunctionVisibility::Public,
            params,
            return_type,
            doc: None,
            is_mutable: true,
        }
    }

    fn param(name: &str, ty: SorobanType) -> FunctionParam {
        FunctionParam {
            name: name.to_string(),
            param_type: ty,
            doc: None,
        }
    }

    #[test]
    fn detects_function_removal_as_breaking() {
        let mut old = ContractABI::new("Old".to_string());
        old.functions.push(func(
            "transfer",
            vec![
                param("from", SorobanType::Address),
                param("to", SorobanType::Address),
                param("amount", SorobanType::U64),
            ],
            SorobanType::Void,
        ));

        let new = ContractABI::new("New".to_string());
        let changes = diff_abi(&old, &new);

        assert!(changes
            .iter()
            .any(|c| c.category == "function_removed" && c.severity == ChangeSeverity::Breaking));
    }

    #[test]
    fn detects_param_type_change_as_breaking() {
        let mut old = ContractABI::new("Old".to_string());
        old.functions.push(func(
            "set_value",
            vec![param("value", SorobanType::U64)],
            SorobanType::Void,
        ));

        let mut new = ContractABI::new("New".to_string());
        new.functions.push(func(
            "set_value",
            vec![param("value", SorobanType::U128)],
            SorobanType::Void,
        ));

        let changes = diff_abi(&old, &new);
        assert!(changes
            .iter()
            .any(|c| c.category == "param_type_changed" && c.severity == ChangeSeverity::Breaking));
    }

    #[test]
    fn detects_function_addition_as_non_breaking() {
        let old = ContractABI::new("Old".to_string());

        let mut new = ContractABI::new("New".to_string());
        new.functions.push(func("ping", vec![], SorobanType::Void));

        let changes = diff_abi(&old, &new);
        assert!(changes
            .iter()
            .any(|c| c.category == "function_added" && c.severity == ChangeSeverity::NonBreaking));
    }
}
