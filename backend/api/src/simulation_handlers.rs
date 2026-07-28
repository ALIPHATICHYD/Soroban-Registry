use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use base64::Engine;
use shared::models::{
    ContractFunctionInfo, GasEstimate, PerformanceMetrics, SimulateDeployRequest, SimulationError,
    SimulationResult, SimulationWarning,
};
use std::time::Instant;

use crate::{error::ApiResult, simulation, state::AppState, validation::validate_contract_id};

pub async fn simulate_deploy(
    State(_state): State<AppState>,
    ValidatedJson(req): ValidatedJson<SimulateDeployRequest>,
) -> ApiResult<impl IntoResponse> {
    let start_time = Instant::now();

    let wasm_binary = match base64::engine::general_purpose::STANDARD.decode(&req.wasm_binary) {
        Ok(bytes) => bytes,
        Err(e) => {
            return Ok(Json(SimulationResult {
                valid: false,
                errors: vec![SimulationError {
                    code: "InvalidBase64".to_string(),
                    message: format!("Failed to decode base64 WASM binary: {}", e),
                    field: Some("wasm_binary".to_string()),
                }],
                warnings: vec![],
                gas_estimate: GasEstimate {
                    total_cost_stroops: 0,
                    total_cost_xlm: 0.0,
                    wasm_size_kb: 0.0,
                    complexity_factor: 0.0,
                    deployment_cost_stroops: 0,
                    storage_cost_stroops: 0,
                },
                performance_metrics: PerformanceMetrics {
                    estimated_execution_time_ms: 0,
                    memory_estimate_kb: 0,
                    function_count: 0,
                    table_size_bytes: 0,
                    data_section_bytes: 0,
                    warnings: vec![],
                },
                abi_preview: None,
                contract_functions: None,
            }));
        }
    };

    let wasm_bytes = wasm_binary.as_slice();
    let _wasm_size_kb = wasm_bytes.len() as f64 / 1024.0;

    if wasm_bytes.is_empty() {
        return Ok(Json(SimulationResult {
            valid: false,
            errors: vec![SimulationError {
                code: "EmptyWasm".to_string(),
                message: "WASM binary is empty".to_string(),
                field: Some("wasm_binary".to_string()),
            }],
            warnings: vec![],
            gas_estimate: GasEstimate {
                total_cost_stroops: 0,
                total_cost_xlm: 0.0,
                wasm_size_kb: 0.0,
                complexity_factor: 0.0,
                deployment_cost_stroops: 0,
                storage_cost_stroops: 0,
            },
            performance_metrics: PerformanceMetrics {
                estimated_execution_time_ms: 0,
                memory_estimate_kb: 0,
                function_count: 0,
                table_size_bytes: 0,
                data_section_bytes: 0,
                warnings: vec![],
            },
            abi_preview: None,
            contract_functions: None,
        }));
    }

    // Validate contract_id
    if let Err(e) = validate_contract_id(&req.contract_id) {
        return Ok(Json(SimulationResult {
            valid: false,
            errors: vec![SimulationError {
                code: "InvalidContractId".to_string(),
                message: e,
                field: Some("contract_id".to_string()),
            }],
            warnings: vec![],
            gas_estimate: GasEstimate {
                total_cost_stroops: 0,
                total_cost_xlm: 0.0,
                wasm_size_kb: 0.0,
                complexity_factor: 0.0,
                deployment_cost_stroops: 0,
                storage_cost_stroops: 0,
            },
            performance_metrics: PerformanceMetrics {
                estimated_execution_time_ms: 0,
                memory_estimate_kb: 0,
                function_count: 0,
                table_size_bytes: 0,
                data_section_bytes: 0,
                warnings: vec![],
            },
            abi_preview: None,
            contract_functions: None,
        }));
    }

    // Validate name
    if req.name.is_empty() {
        return Ok(Json(SimulationResult {
            valid: false,
            errors: vec![SimulationError {
                code: "InvalidName".to_string(),
                message: "Contract name cannot be empty".to_string(),
                field: Some("name".to_string()),
            }],
            warnings: vec![],
            gas_estimate: GasEstimate {
                total_cost_stroops: 0,
                total_cost_xlm: 0.0,
                wasm_size_kb: 0.0,
                complexity_factor: 0.0,
                deployment_cost_stroops: 0,
                storage_cost_stroops: 0,
            },
            performance_metrics: PerformanceMetrics {
                estimated_execution_time_ms: 0,
                memory_estimate_kb: 0,
                function_count: 0,
                table_size_bytes: 0,
                data_section_bytes: 0,
                warnings: vec![],
            },
            abi_preview: None,
            contract_functions: None,
        }));
    }

    // Run WASM validation
    let validation_result = simulation::validate_wasm(wasm_bytes);

    if !validation_result.valid {
        let errors: Vec<SimulationError> = validation_result
            .errors
            .iter()
            .map(|e| SimulationError {
                code: "WasmValidationError".to_string(),
                message: e.clone(),
                field: Some("wasm_binary".to_string()),
            })
            .collect();

        return Ok(Json(SimulationResult {
            valid: false,
            errors,
            warnings: vec![],
            gas_estimate: GasEstimate {
                total_cost_stroops: 0,
                total_cost_xlm: 0.0,
                wasm_size_kb: 0.0,
                complexity_factor: 0.0,
                deployment_cost_stroops: 0,
                storage_cost_stroops: 0,
            },
            performance_metrics: PerformanceMetrics {
                estimated_execution_time_ms: 0,
                memory_estimate_kb: 0,
                function_count: 0,
                table_size_bytes: 0,
                data_section_bytes: 0,
                warnings: vec![],
            },
            abi_preview: None,
            contract_functions: None,
        }));
    }

    // Extract ABI
    let abi_result = simulation::extract_abi(wasm_bytes);

    // Estimate gas
    let gas_result = simulation::estimate_gas(wasm_bytes, &validation_result);

    // Analyze performance
    let performance_result =
        simulation::analyze_performance(wasm_bytes, &validation_result, &abi_result);

    // Convert warnings
    let warnings: Vec<SimulationWarning> = validation_result
        .warnings
        .iter()
        .map(|w| SimulationWarning {
            code: "WasmWarning".to_string(),
            message: w.clone(),
            severity: Some("low".to_string()),
        })
        .chain(
            performance_result
                .warnings
                .iter()
                .map(|w| SimulationWarning {
                    code: w.code.clone(),
                    message: w.message.clone(),
                    severity: Some(w.severity.clone()),
                }),
        )
        .collect();

    // Build contract functions info
    let contract_functions: Vec<ContractFunctionInfo> = abi_result
        .functions
        .iter()
        .map(|f| ContractFunctionInfo {
            name: f.name.clone(),
            param_count: f.param_count,
            return_type: f.return_type.clone(),
            is_view: f.is_view,
        })
        .collect();

    // Calculate elapsed time
    let elapsed_ms = start_time.elapsed().as_millis() as u64;

    // Add timeout warning if near limit
    let mut final_warnings = warnings;
    if elapsed_ms > 4000 {
        final_warnings.push(SimulationWarning {
            code: "SlowSimulation".to_string(),
            message: format!("Simulation took {}ms - approaching 5s limit", elapsed_ms),
            severity: Some("medium".to_string()),
        });
    }

    Ok(Json(SimulationResult {
        valid: true,
        errors: vec![],
        warnings: final_warnings,
        gas_estimate: GasEstimate {
            total_cost_stroops: gas_result.total_cost_stroops,
            total_cost_xlm: gas_result.total_cost_xlm,
            wasm_size_kb: gas_result.wasm_size_kb,
            complexity_factor: gas_result.complexity_factor,
            deployment_cost_stroops: gas_result.deployment_cost_stroops,
            storage_cost_stroops: gas_result.storage_cost_stroops,
        },
        performance_metrics: PerformanceMetrics {
            estimated_execution_time_ms: performance_result.estimated_execution_time_ms,
            memory_estimate_kb: performance_result.memory_estimate_kb,
            function_count: validation_result.function_count,
            table_size_bytes: validation_result.table_count * 8,
            data_section_bytes: validation_result.data_section_size,
            warnings: vec![],
        },
        abi_preview: if !abi_result.types.is_empty() {
            Some(serde_json::json!({
                "function_count": abi_result.functions.len(),
                "type_count": abi_result.types.len(),
            }))
        } else {
            None
        },
        contract_functions: if contract_functions.is_empty() {
            None
        } else {
            Some(contract_functions)
        },
    }))
}
