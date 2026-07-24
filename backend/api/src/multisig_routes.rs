use axum::{
    routing::{get, post},
    Router,
};

use crate::{multisig_handlers, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/multisig/policies",
            post(multisig_handlers::create_policy),
        )
        .route(
            "/api/multisig/publishers/:id/keys",
            get(multisig_handlers::list_publisher_keys)
                .post(multisig_handlers::create_publisher_key),
        )
        .route(
            "/api/multisig/keys/:id/deactivate",
            post(multisig_handlers::deactivate_publisher_key),
        )
        .route(
            "/api/multisig/proposals",
            get(multisig_handlers::list_proposals),
        )
        .route(
            "/api/contracts/deploy-proposal",
            post(multisig_handlers::create_deploy_proposal),
        )
        .route(
            "/api/contracts/:id/sign",
            post(multisig_handlers::sign_proposal),
        )
        .route(
            "/api/contracts/:id/execute",
            post(multisig_handlers::execute_proposal),
        )
        .route(
            "/api/contracts/:id/proposal",
            get(multisig_handlers::proposal_info),
        )
}
