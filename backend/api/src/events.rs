use crate::state::{AppState, RealtimeEvent};
use chrono::Utc;
use shared::models::Network;

#[allow(dead_code)]
pub fn emit_contract_deployment(
    state: &AppState,
    contract_id: String,
    contract_name: String,
    publisher: String,
    version: String,
    network: Network,
) {
    let event = RealtimeEvent::ContractDeployed {
        contract_id,
        contract_name,
        publisher,
        version,
        timestamp: Utc::now().to_rfc3339(),
        network,
    };

    state.contract_events.publish(event);
}

#[allow(dead_code)]
pub fn emit_contract_update(
    state: &AppState,
    contract_id: String,
    update_type: String,
    details: serde_json::Value,
) {
    let event = RealtimeEvent::ContractUpdated {
        contract_id,
        update_type,
        details,
        timestamp: Utc::now().to_rfc3339(),
    };

    state.contract_events.publish(event);
}

pub fn emit_cicd_pipeline(
    state: &AppState,
    contract_id: String,
    status: String,
    steps_completed: u32,
) {
    let event = RealtimeEvent::CicdPipeline {
        contract_id,
        status,
        steps_completed,
        total_steps: 5,
        timestamp: Utc::now().to_rfc3339(),
    };

    state.contract_events.publish(event);
}
