// incident_routes.rs
// Route definitions for the security incident tracking system (Issue #504).

use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{incident_handlers, state::AppState};

pub fn incident_routes() -> Router<AppState> {
    Router::new()
        // Incidents
        .route(
            "/api/security/incidents",
            get(incident_handlers::list_incidents).post(incident_handlers::report_incident),
        )
        .route(
            "/api/security/incidents/:id",
            get(incident_handlers::get_incident),
        )
        .route(
            "/api/security/incidents/:id/status",
            patch(incident_handlers::update_incident_status),
        )
        .route(
            "/api/security/incidents/:id/updates",
            post(incident_handlers::add_incident_update),
        )
        .route(
            "/api/security/incidents/:id/contracts",
            post(incident_handlers::add_affected_contract),
        )
        .route(
            "/api/security/incidents/:id/notify",
            post(incident_handlers::notify_affected_users),
        )
        // Advisories
        .route(
            "/api/security/advisories",
            get(incident_handlers::list_advisories).post(incident_handlers::publish_advisory),
        )
        .route(
            "/api/security/advisories/:id",
            get(incident_handlers::get_advisory),
        )
        // Report
        .route(
            "/api/security/report",
            get(incident_handlers::get_security_report),
        )
        // Contract-scoped shortcut
        .route(
            "/api/contracts/:id/security-incidents",
            get(incident_handlers::get_contract_incidents),
        )
}
