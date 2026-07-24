#[cfg(feature = "openapi")]
use crate::openapi;
use crate::{
    ab_test_handlers, abi_versioning_handlers,
    ai::handlers as ai_handlers,
    analytics_handlers, archival, auth, auth_handlers, batch_verify_handlers, breaking_changes,
    bulk_operations_handlers, canary_handlers, category_handlers, client_observability_handlers,
    clone_federation_handlers, collaborative_reviews, compatibility_testing_handlers,
    contract_events, contract_stats_handlers, contributor_handlers, custom_metrics_handlers,
    db_pool, dependency_handlers, deprecated_contracts_handlers, deprecation_handlers,
    elasticsearch_handlers, error_logging, feature_flags, formal_verification_handlers,
    formal_verification_integration, gas_estimation_handlers, governance_handlers,
    graph_analysis_handlers, handlers, integrity, interoperability_handlers,
    marketplace::{
        license_handlers as mp_license, metering as mp_metering, pricing_handlers as mp_pricing,
        stripe_handlers as mp_stripe, usdc_handlers as mp_usdc,
    },
    metrics_handler, migration_handlers, mutation_testing_handlers, org_handlers,
    partition_manager, patch_handlers, performance_handlers, plugin_marketplace_handlers,
    publisher_verification_handlers, query_analysis, query_monitor, recommendation_handlers,
    report_handlers, resource_handlers, search_postgres, security_scan_handlers,
    signature_verification, similarity_handlers, simulation_handlers,
    state::AppState,
    state_monitor::handlers as state_monitor_handlers,
    stats, subscription_handlers, v1_contract_handlers, v1_search_handlers, v1_similar_handlers,
    v1_trending_handlers, verification_handlers, websocket, zk_proof_handlers,
};

use axum::{
    middleware,
    routing::{delete, get, patch, post, put},
    Router,
};
#[cfg(feature = "openapi")]
use utoipa::OpenApi;
#[cfg(feature = "openapi")]
use utoipa_swagger_ui::SwaggerUi;

/// Build the application route tree grouped by resource domain.
///
/// `main.rs` owns process setup, middleware, and graceful shutdown; this module
/// owns route registration so resource groups stay discoverable and testable.
pub fn application_routes(_schema: crate::graphql::schema::RegistrySchema) -> Router<AppState> {
    Router::new()
        // Identity and marketplace primitives
        .merge(auth_routes())
        .merge(marketplace_routes())
        .merge(plugin_routes())
        .merge(organization_routes())
        .merge(publisher_routes())
        .merge(contributor_routes())
        // Contracts and lifecycle operations
        .merge(contract_routes())
        .merge(category_routes())
        .merge(network_routes())
        .merge(canary_routes())
        .merge(ab_test_routes())
        .merge(performance_routes())
        .merge(federation_routes())
        .merge(multisig_routes_group())
        .merge(security_scanning_routes())
        .merge(zk_proof_routes())
        .merge(backup_routes())
        .merge(post_incident_routes())
        // Analysis, verification, and collaboration
        .merge(compatibility_dashboard_routes())
        .merge(governance_routes())
        .merge(mutation_testing_routes())
        .merge(collaborative_review_routes())
        .merge(subscription_routes())
        .merge(notification_routes())
        .merge(graph_analysis_routes())
        .merge(formal_verification_routes())
        .merge(verification_status_routes())
        .merge(release_notes_routes())
        // Operations
        .merge(health_routes())
        .merge(health_monitor_routes())
        .merge(admin_routes())
        .merge(migration_routes())
        .merge(crate::incident_routes::incident_routes())
        .merge(observability_routes())
        .merge(websocket_routes())
        .merge(quota_routes())
        .merge(validator_routes())
        .merge(openapi_routes())
        .nest("/api", crate::activity_feed_routes::routes())
        .merge(query_monitor_routes())
        .merge(partition_routes())
        .merge(archival_routes())
        .merge(elasticsearch_search_routes())
        .merge(integrity_routes())
        // Discovery & reporting endpoints (issues #870–#873)
        .merge(discovery_reporting_routes())
        // Contract signature verification system (issue #888)
        .merge(signature_verification_routes())
        // Backend feature flag management (issue #1007)
        .merge(feature_flag_routes())
}

// ── Issue #888: contract signature verification system ───────────────────────

pub fn signature_verification_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/signatures/keys",
            post(signature_verification::register_key),
        )
        .route(
            "/api/signatures/keys/:key_id",
            get(signature_verification::get_key),
        )
        .route(
            "/api/signatures/keys/:key_id/rotate",
            post(signature_verification::rotate_key),
        )
        .route(
            "/api/signatures/keys/:key_id/revoke",
            post(signature_verification::revoke_key),
        )
        .route(
            "/api/signatures/keys/:key_id/verify-chain",
            post(signature_verification::verify_chain),
        )
        .route(
            "/api/signatures/revocations",
            get(signature_verification::list_revocations),
        )
        .route(
            "/api/signatures",
            post(signature_verification::store_signature),
        )
        .route(
            "/api/signatures/verify",
            post(signature_verification::verify),
        )
        .route(
            "/api/contracts/:id/signatures",
            get(signature_verification::list_contract_signatures),
        )
        // Application-side query logging & analysis (issue #887)
        .merge(query_analysis_routes())
}

// ── Issue #887: application-side query logging and analysis ──────────────────

pub fn query_analysis_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/admin/db/queries/stats",
            get(query_analysis::get_query_stats),
        )
        .route(
            "/api/admin/db/queries/frequent",
            get(query_analysis::get_frequent_queries),
        )
        .route(
            "/api/admin/db/queries/slow",
            get(query_analysis::get_slow_queries),
        )
        .route(
            "/api/admin/db/queries/n-plus-one",
            get(query_analysis::get_nplus1),
        )
        .route(
            "/api/admin/db/queries/trends",
            get(query_analysis::get_query_trends),
        )
        .route(
            "/api/admin/db/queries/incidents",
            get(query_analysis::get_nplus1_incidents),
        )
        .route(
            "/api/admin/db/queries/report",
            get(query_analysis::get_query_report),
        )
        .route(
            "/api/admin/db/queries/explain",
            post(query_analysis::explain_query),
        )
        .route(
            "/api/admin/db/queries/reset",
            post(query_analysis::reset_query_stats),
        )
}

fn multisig_routes_group() -> Router<AppState> {
    Router::new().merge(crate::multisig_routes::routes())
}

fn backup_routes() -> Router<AppState> {
    crate::backup_routes::backup_routes()
}

fn notification_routes() -> Router<AppState> {
    crate::notification_routes::notification_routes()
}

fn post_incident_routes() -> Router<AppState> {
    crate::post_incident_routes::post_incident_routes()
}

fn release_notes_routes() -> Router<AppState> {
    crate::release_notes_routes::release_notes_routes()
}

pub fn observability_routes() -> Router<AppState> {
    Router::new()
        .route("/metrics", get(metrics_handler::metrics_endpoint))
        .route(
            "/api/observability/client_breaker",
            post(client_observability_handlers::report_client_breaker),
        )
        .route("/api/errors/report", post(error_logging::report_error))
        .route("/api/errors/dashboard", get(error_logging::error_dashboard))
}

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/csrf", get(auth_handlers::get_csrf_token))
        .route("/api/auth/challenge", get(auth_handlers::get_challenge))
        .route("/api/auth/verify", post(auth_handlers::verify_challenge))
        .route("/api/auth/refresh", post(auth_handlers::refresh_token))
}

pub fn validator_routes() -> Router<AppState> {
    Router::new()
}

pub fn quota_routes() -> Router<AppState> {
    Router::new().route("/api/quota", get(crate::quota_handlers::get_quota))
}

pub fn plugin_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/plugins/marketplace",
            get(plugin_marketplace_handlers::get_marketplace),
        )
        .route(
            "/api/plugins/:name/:version",
            get(plugin_marketplace_handlers::get_plugin_manifest),
        )
}

/// Marketplace Phase 1 — paid contract pricing + Ed25519 license issuance,
/// validation, revocation, and usage metering. Payment-provider integration
/// (Stripe, USDC) lives in later phases and will hang off the same routes.
pub fn marketplace_routes() -> Router<AppState> {
    Router::new()
        // Pricing plans per contract
        .route(
            "/api/contracts/:contract_id/pricing-plans",
            get(mp_pricing::list_plans).post(mp_pricing::create_plan),
        )
        .route(
            "/api/contracts/:contract_id/pricing-plans/:plan_id",
            patch(mp_pricing::update_plan),
        )
        // License issuance + lifecycle
        .route(
            "/api/contracts/:contract_id/licenses",
            post(mp_license::issue_license),
        )
        .route(
            "/api/marketplace/licenses",
            get(mp_license::list_my_licenses),
        )
        .route(
            "/api/marketplace/licenses/validate",
            post(mp_license::validate_license),
        )
        .route(
            "/api/marketplace/licenses/:jti/revoke",
            post(mp_license::revoke_license),
        )
        .route(
            "/api/marketplace/license-pubkey",
            get(mp_license::license_pubkey),
        )
        // Usage metering
        .route(
            "/api/marketplace/licenses/:jti/usage",
            get(mp_metering::get_usage).post(mp_metering::record_usage),
        )
        // Phase 2 — Stripe checkout + webhook (idempotent by event id)
        .route(
            "/api/contracts/:contract_id/checkout",
            post(mp_stripe::create_checkout),
        )
        .route("/api/marketplace/stripe/webhook", post(mp_stripe::webhook))
        // Phase 3 — USDC on Stellar: payment intents + confirm
        .route(
            "/api/contracts/:contract_id/usdc-intents",
            post(mp_usdc::create_intent),
        )
        .route(
            "/api/marketplace/usdc/confirm",
            post(mp_usdc::confirm_intent),
        )
        .route(
            "/api/marketplace/usdc-payments/:payment_id",
            get(mp_usdc::get_intent),
        )
}

pub fn contract_routes() -> Router<AppState> {
    Router::new()
        // Core contract operations
        .route(
            "/api/contracts",
            get(handlers::list_contracts).post(handlers::publish_contract),
        )
        .route("/api/contracts/tags", get(handlers::list_tags))
        .route(
            "/api/contracts/export",
            get(bulk_operations_handlers::get_export_contracts)
                .post(handlers::export_contract_metadata),
        )
        .route(
            "/contracts/export",
            get(bulk_operations_handlers::get_export_contracts)
                .post(handlers::export_contract_metadata),
        )
        .route(
            "/api/contracts/import",
            post(bulk_operations_handlers::import_contracts),
        )
        .route(
            "/contracts/import",
            post(bulk_operations_handlers::import_contracts),
        )
        .route(
            "/api/contracts/import/:job_id",
            get(bulk_operations_handlers::get_import_status),
        )
        .route(
            "/contracts/import/:job_id",
            get(bulk_operations_handlers::get_import_status),
        )
        .route(
            "/api/contracts/export/:job_id",
            get(handlers::get_contract_export_status),
        )
        .route(
            "/contracts/export/:job_id",
            get(handlers::get_contract_export_status),
        )
        .route(
            "/api/contracts/export/:job_id/download",
            get(handlers::download_contract_export),
        )
        .route(
            "/contracts/export/:job_id/download",
            get(handlers::download_contract_export),
        )
        .route(
            "/api/contracts/suggestions",
            get(handlers::get_contract_search_suggestions),
        )
        .route(
            "/api/contracts/trending",
            get(contract_stats_handlers::get_trending_contracts),
        )
        .route(
            "/contracts/trending",
            get(contract_stats_handlers::get_trending_contracts),
        )
        .route("/api/contracts/batch", post(handlers::get_contracts_batch))
        .route("/contracts/batch", post(handlers::get_contracts_batch))
        .route("/api/contracts/graph", get(handlers::get_contract_graph))
        .route("/api/contracts/:id", get(handlers::get_contract))
        .route(
            "/api/contracts/:id/metadata",
            patch(handlers::update_contract_metadata),
        )
        .route(
            "/api/v1/contracts/:id/metadata",
            get(v1_contract_handlers::get_contract_metadata_v1),
        )
        .route(
            "/api/v1/contracts/:id/vulnerability-assessment",
            get(v1_contract_handlers::vulnerability_assessment_v1),
        )
        .route(
            "/api/contracts/:id/publisher",
            patch(handlers::change_contract_publisher),
        )
        .route(
            "/api/contracts/:id/status",
            patch(handlers::update_contract_status),
        )
        .route(
            "/api/contracts/:id/audit-log",
            get(handlers::get_contract_audit_log),
        )
        .route(
            "/api/v1/contracts/:id/audits",
            get(handlers::get_contract_audits),
        )
        .route(
            "/api/contracts/:id/abi",
            get(handlers::get_contract_abi).post(abi_versioning_handlers::publish_abi),
        )
        .route(
            "/api/v1/contracts/:id/abi",
            get(handlers::get_contract_abi_v1),
        )
        .route(
            "/api/contracts/:id/abi/:version",
            get(abi_versioning_handlers::get_abi_version),
        )
        .route(
            "/api/contracts/:id/check-compatibility",
            post(abi_versioning_handlers::check_compatibility),
        )
        .route(
            "/api/contracts/:id/openapi.yaml",
            get(handlers::get_contract_openapi_yaml),
        )
        .route(
            "/api/contracts/:id/openapi.json",
            get(handlers::get_contract_openapi_json),
        )
        .route(
            "/api/contracts/:id/versions",
            get(handlers::get_contract_versions).post(handlers::create_contract_version),
        )
        .route(
            "/api/contracts/:id/versions/compare",
            get(handlers::compare_contract_versions),
        )
        .route(
            "/api/contracts/:id/versions/:version",
            get(handlers::get_specific_contract_version),
        )
        .route(
            "/api/contracts/:id/changelog",
            get(handlers::get_contract_changelog),
        )
        .route(
            "/contracts/:id/changelog",
            get(handlers::get_contract_changelog),
        )
        .route(
            "/api/contracts/breaking-changes",
            get(breaking_changes::get_breaking_changes),
        )
        .route(
            "/api/contracts/:id/patches",
            get(patch_handlers::list_contract_patches),
        )
        .route(
            "/api/contracts/:id/patches/:from_version/:to_version",
            get(patch_handlers::get_patch_between_versions),
        )
        .route(
            "/api/contracts/:id/patches/reconstruct",
            post(patch_handlers::reconstruct_contract_version),
        )
        .route(
            "/api/contracts/patches/bulk-apply",
            post(patch_handlers::bulk_apply_patches),
        )
        .route(
            "/api/contracts/:id/versions/:version/source",
            get(handlers::get_contract_source).post(handlers::upload_contract_source),
        )
        .route(
            "/api/contracts/:id/versions/:version/source/diff",
            get(handlers::get_contract_source_diff),
        )
        .route(
            "/api/contracts/:id/interactions",
            get(handlers::get_contract_interactions).post(handlers::post_contract_interaction),
        )
        .route(
            "/api/contracts/:id/interactions/batch",
            post(handlers::post_contract_interactions_batch),
        )
        .route(
            "/api/contracts/:id/deprecation-info",
            get(deprecation_handlers::get_deprecation_info),
        )
        .route(
            "/api/contracts/:id/deprecate",
            post(deprecation_handlers::deprecate_contract),
        )
        // AI-Powered Contract Code Assistant
        .route(
            "/api/contracts/:id/ai/chat",
            get(ai_handlers::ai_chat_handler).post(ai_handlers::ai_chat_handler),
        )
        .route(
            "/api/contracts/:id/ai/analyze",
            get(ai_handlers::analyze_contract_handler),
        )
        .route(
            "/api/contracts/:id/ai/vulnerabilities",
            get(ai_handlers::check_vulnerabilities_handler),
        )
        .route(
            "/api/contracts/:id/ai/explain",
            get(ai_handlers::explain_contract_handler),
        )
        .route(
            "/api/contracts/:id/ai/suggest",
            post(ai_handlers::suggest_code_handler),
        )
        .route(
            "/api/ai/chat",
            get(ai_handlers::ai_chat_handler).post(ai_handlers::ai_chat_handler),
        )
        .route(
            "/api/ai/sessions",
            get(ai_handlers::get_chat_sessions_handler),
        )
        .route(
            "/api/ai/sessions/:session_id",
            get(ai_handlers::get_chat_session_handler),
        )
        // Real-Time Contract State Monitor
        .route(
            "/api/contracts/:id/state/history",
            get(state_monitor_handlers::get_state_history_handler),
        )
        // Point-in-time + diff state queries (derived from the change log)
        .route(
            "/api/contracts/:id/state-at",
            get(state_monitor_handlers::get_state_at_handler),
        )
        .route(
            "/api/contracts/:id/state-diff",
            get(state_monitor_handlers::get_state_diff_handler),
        )
        .route(
            "/api/contracts/:id/anomalies",
            get(state_monitor_handlers::get_contract_anomalies_handler),
        )
        .route(
            "/api/anomalies",
            get(state_monitor_handlers::get_anomalies_handler),
        )
        .route(
            "/api/anomalies/:anomaly_id/resolve",
            post(state_monitor_handlers::resolve_anomaly_handler),
        )
        // PostgreSQL Full-Text Search
        .route("/api/search", get(search_postgres::fulltext_search_handler))
        // State get/update (existing)
        .route(
            "/api/contracts/:id/state/:key",
            get(handlers::get_contract_state)
                .put(handlers::update_contract_state)
                .post(handlers::update_contract_state),
        )
        .route(
            "/api/contracts/:id/analytics",
            get(analytics_handlers::get_contract_analytics),
        )
        .route(
            "/api/contracts/:id/stats",
            get(handlers::get_contract_stats),
        )
        .route(
            "/api/analytics/dashboard",
            get(analytics_handlers::get_analytics_dashboard),
        )
        .route(
            "/api/contracts/:id/dependencies",
            get(crate::dependency_handlers::get_contract_dependencies)
                .post(dependency_handlers::declare_contract_dependencies),
        )
        .route(
            "/api/contracts/:id/graph",
            get(handlers::get_contract_local_graph),
        )
        .route(
            "/api/contracts/:id/trust-score",
            get(handlers::get_trust_score),
        )
        .route(
            "/api/contracts/:id/dependents",
            get(handlers::get_contract_dependents),
        )
        .route(
            "/api/contracts/:id/impact",
            get(handlers::get_impact_analysis),
        )
        .route(
            "/api/contracts/:id/similar",
            get(similarity_handlers::get_similar_contracts),
        )
        .route(
            "/api/contracts/:id/recommendations",
            get(recommendation_handlers::get_contract_recommendations),
        )
        .route(
            "/api/contracts/:id/related",
            get(recommendation_handlers::get_contract_recommendations),
        )
        .route(
            "/contracts/:id/recommendations",
            get(recommendation_handlers::get_contract_recommendations),
        )
        .route(
            "/contracts/:id/related",
            get(recommendation_handlers::get_contract_recommendations),
        )
        .route(
            "/contracts/:id/similar",
            get(similarity_handlers::get_similar_contracts),
        )
        .route("/api/contracts/verify", post(handlers::verify_contract))
        .route(
            "/api/contracts/batch-verify",
            post(batch_verify_handlers::batch_verify_contracts),
        )
        .route(
            "/api/v1/contracts/bulk-verify",
            post(v1_contract_handlers::bulk_verify_v1),
        )
        .route(
            "/api/v1/contracts/bulk-verify/jobs/:job_id",
            get(v1_contract_handlers::get_bulk_verify_job_v1),
        )
        // Async batch verification job endpoints
        .route(
            "/api/contracts/batch-verify/jobs",
            post(batch_verify_handlers::submit_batch_verify_job),
        )
        .route(
            "/api/contracts/batch-verify/jobs/:job_id",
            get(batch_verify_handlers::get_batch_verify_job),
        )
        .route(
            "/api/contracts/similarity/analyze",
            post(similarity_handlers::analyze_contract_similarity_batch),
        )
        .route(
            "/api/contracts/status/bulk",
            post(handlers::bulk_update_contract_status),
        )
        // Batch metadata update (#849) — static route must precede parameterised :id routes
        .route(
            "/api/contracts/metadata/batch",
            post(handlers::batch_update_contract_metadata),
        )
        // Metadata history & rollback (#729, wired here)
        .route(
            "/api/contracts/:id/metadata/versions",
            get(handlers::contract_metadata::get_metadata_versions),
        )
        .route(
            "/api/contracts/:id/metadata/versions/:version_id",
            get(handlers::contract_metadata::get_metadata_version),
        )
        .route(
            "/api/contracts/:id/metadata/rollback/:version_id",
            post(handlers::contract_metadata::rollback_metadata),
        )
        .route(
            "/api/contracts/:id/performance",
            get(performance_handlers::get_contract_performance_overview),
        )
        .route(
            "/api/contracts/:id/metrics",
            get(custom_metrics_handlers::get_contract_metrics)
                .post(custom_metrics_handlers::record_contract_metric),
        )
        .route(
            "/api/contracts/:id/resources",
            get(resource_handlers::get_contract_resources),
        )
        .route(
            "/api/contracts/:id/metrics/batch",
            post(custom_metrics_handlers::record_metrics_batch),
        )
        .route(
            "/api/contracts/:id/metrics/catalog",
            get(custom_metrics_handlers::get_metric_catalog),
        )
        .route(
            "/api/contracts/:id/compatibility",
            get(handlers::compatibility::get_contract_compatibility)
                .post(handlers::compatibility::add_contract_compatibility),
        )
        .route(
            "/api/contracts/:id/compatibility/export",
            get(handlers::compatibility::export_contract_compatibility),
        )
        .route(
            "/api/contracts/:id/interoperability",
            get(interoperability_handlers::get_contract_interoperability),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix",
            get(compatibility_testing_handlers::get_compatibility_matrix),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/test",
            post(compatibility_testing_handlers::run_compatibility_test),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/history",
            get(compatibility_testing_handlers::get_compatibility_history),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/notifications",
            get(compatibility_testing_handlers::get_compatibility_notifications),
        )
        .route(
            "/api/contracts/:id/compatibility-matrix/notifications/read",
            post(compatibility_testing_handlers::mark_notifications_read),
        )
        .route(
            "/api/contracts/:id/deployments",
            get(handlers::get_contract_deployments),
        )
        .route(
            "/api/contracts/:id/deployments/status",
            get(handlers::get_deployment_status),
        )
        .route(
            "/api/contracts/:id/deployment-status",
            get(handlers::get_deployment_status),
        )
        .route("/api/deployments/green", post(handlers::deploy_green))
        .route(
            "/api/contracts/:id/deploy-green",
            post(handlers::deploy_green),
        )
        .route(
            "/contracts/simulate-deploy",
            post(simulation_handlers::simulate_deploy),
        )
        // Gas usage estimation
        .route(
            "/api/contracts/:id/methods/gas-estimate/batch",
            post(gas_estimation_handlers::batch_gas_estimate),
        )
        .route(
            "/api/contracts/:id/methods/:method/gas-estimate",
            get(gas_estimation_handlers::get_method_gas_estimate),
        )
        // Review system
        .route(
            "/api/contracts/:id/reviews",
            get(handlers::reviews::get_reviews).post(handlers::reviews::create_review),
        )
        .route(
            "/api/contracts/:id/reviews/:review_id/vote",
            post(handlers::reviews::vote_review),
        )
        .route(
            "/api/contracts/:id/reviews/:review_id/flag",
            post(handlers::reviews::flag_review),
        )
        .route(
            "/api/contracts/:id/reviews/:review_id/moderate",
            post(handlers::reviews::moderate_review),
        )
        .route(
            "/api/contracts/:id/rating-stats",
            get(handlers::reviews::get_rating_stats),
        )
        // Contract clone endpoints (#487)
        .route(
            "/api/contracts/:id/clone",
            post(clone_federation_handlers::clone_contract),
        )
        .route(
            "/api/contracts/:id/clones",
            get(clone_federation_handlers::get_contract_clones),
        )
        .merge(favorite_routes())
}

pub fn organization_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/organizations",
            post(org_handlers::create_organization),
        )
        .route(
            "/api/organizations/:id",
            get(org_handlers::get_organization).patch(org_handlers::update_organization),
        )
        .route(
            "/api/organizations/:id/members",
            get(org_handlers::list_org_members),
        )
        .route(
            "/api/organizations/:id/invitations",
            post(org_handlers::invite_member),
        )
        .route(
            "/api/organizations/invitations/:token/accept",
            post(org_handlers::accept_invitation),
        )
}

#[cfg(not(feature = "openapi"))]
pub fn openapi_routes() -> Router<AppState> {
    Router::new()
}

#[cfg(feature = "openapi")]
pub fn openapi_routes() -> Router<AppState> {
    Router::new().merge(SwaggerUi::new("/docs").url("/openapi.json", openapi::ApiDoc::openapi()))
}

pub fn publisher_routes() -> Router<AppState> {
    Router::new()
        .route("/api/publishers", post(handlers::create_publisher))
        .route("/api/publishers/:id", get(handlers::get_publisher))
        .route(
            "/api/publishers/:id/contracts",
            get(handlers::get_publisher_contracts),
        )
        // Issue #603: publisher verification badge endpoint
        .route(
            "/api/publishers/:id/verify",
            post(publisher_verification_handlers::verify_publisher),
        )
}

pub fn contributor_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contributors",
            get(contributor_handlers::list_contributors)
                .post(contributor_handlers::create_contributor),
        )
        .route(
            "/api/contributors/:id",
            get(contributor_handlers::get_contributor)
                .put(contributor_handlers::update_contributor),
        )
        .route(
            "/api/contributors/:id/contracts",
            get(contributor_handlers::get_contributor_contracts),
        )
}

pub fn favorite_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/favorites/search",
            get(handlers::list_favorite_searches).post(handlers::save_favorite_search),
        )
        .route(
            "/api/favorites/search/:id",
            delete(handlers::delete_favorite_search),
        )
}

pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/health/live", get(handlers::health_check_live))
        .route("/health/ready", get(handlers::health_check_ready))
        .route("/health/detailed", get(handlers::health_check_detailed))
        .route(
            "/health/services",
            get(crate::service_health::health_check_services),
        )
        .route("/api/v1/health", get(v1_contract_handlers::health_v1))
        .route("/api/stats", get(stats::get_stats_handler))
        .route(
            "/api/v1/analytics/contracts",
            get(crate::contract_analytics_handlers::get_contract_analytics),
        )
        .route(
            "/api/analytics/contracts",
            get(crate::contract_analytics_handlers::get_contract_analytics),
        )
        // Registry-wide analytics summary (issue #415)
        .route(
            "/api/analytics/summary",
            get(analytics_handlers::get_analytics_summary),
        )
        .route(
            "/api/analytics/timeseries",
            get(analytics_handlers::get_analytics_timeseries),
        )
}

pub fn governance_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/governance/proposals",
            post(governance_handlers::create_proposal).get(governance_handlers::list_proposals),
        )
        .route(
            "/api/governance/proposals/:id",
            get(governance_handlers::get_proposal),
        )
        .route(
            "/api/governance/proposals/:id/votes",
            post(governance_handlers::cast_vote).get(governance_handlers::get_vote_tally),
        )
        .route(
            "/api/governance/proposals/:id/execute",
            post(governance_handlers::execute_proposal),
        )
        .route(
            "/api/governance/contracts/:id/voting-rights",
            get(governance_handlers::list_voting_rights)
                .post(governance_handlers::upsert_voting_rights),
        )
}

pub fn category_routes() -> Router<AppState> {
    Router::new()
        .route("/api/categories", get(category_handlers::list_categories))
        .route("/api/categories/:id", get(category_handlers::get_category))
}

pub fn network_routes() -> Router<AppState> {
    Router::new()
        .route("/networks", get(handlers::list_networks))
        .route("/api/networks", get(handlers::list_networks))
        .route("/api/v1/networks", get(handlers::list_networks_v1))
        .route("/api/networks/health", get(handlers::get_network_health))
}

pub fn health_monitor_routes() -> Router<AppState> {
    Router::new().route(
        "/api/health-monitor/status",
        get(crate::health_monitor::get_health_monitor_status),
    )
}

pub fn migration_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/admin/migrations/status",
            get(migration_handlers::get_migration_status),
        )
        .route(
            "/api/admin/migrations/register",
            post(migration_handlers::register_migration),
        )
        .route(
            "/api/admin/migrations/validate",
            get(migration_handlers::validate_migrations),
        )
        .route(
            "/api/admin/migrations/lock",
            get(migration_handlers::get_lock_status),
        )
        .route(
            "/api/admin/migrations/:version",
            get(migration_handlers::get_migration_version),
        )
        .route(
            "/api/admin/migrations/:version/rollback",
            post(migration_handlers::rollback_migration),
        )
        .route(
            "/api/admin/migrations/apply",
            post(migration_handlers::apply_migration),
        )
        .route(
            "/api/admin/migrations/audit",
            get(migration_handlers::get_migration_audit),
        )
}

pub fn compatibility_dashboard_routes() -> Router<AppState> {
    Router::new().route(
        "/api/compatibility-dashboard",
        get(compatibility_testing_handlers::get_compatibility_dashboard),
    )
}

/// Issue #619 — mutation testing routes.
pub fn mutation_testing_routes() -> Router<AppState> {
    Router::new()
        // Trigger a new mutation test run
        .route(
            "/api/contracts/:id/mutations",
            post(mutation_testing_handlers::run_mutation_tests)
                .get(mutation_testing_handlers::list_mutation_runs),
        )
}

pub fn canary_routes() -> Router<AppState> {
    Router::new()
        // Contract-scoped canary endpoints
        .route(
            "/api/contracts/:id/canary",
            get(canary_handlers::list_canaries).post(canary_handlers::create_canary),
        )
        // Canary-specific endpoints
        .route("/api/canary/:canary_id", get(canary_handlers::get_canary))
        .route(
            "/api/canary/:canary_id/advance",
            post(canary_handlers::advance_canary),
        )
        .route(
            "/api/canary/:canary_id/rollback",
            post(canary_handlers::rollback_canary),
        )
        .route(
            "/api/canary/:canary_id/complete",
            post(canary_handlers::complete_canary),
        )
        .route(
            "/api/canary/:canary_id/metrics",
            get(canary_handlers::list_canary_metrics).post(canary_handlers::record_canary_metric),
        )
}

pub fn ab_test_routes() -> Router<AppState> {
    Router::new()
        // Contract-scoped A/B test endpoints
        .route(
            "/api/contracts/:id/ab-tests",
            get(ab_test_handlers::list_ab_tests).post(ab_test_handlers::create_ab_test),
        )
        // A/B test-specific endpoints
        .route("/api/ab-tests/:test_id", get(ab_test_handlers::get_ab_test))
        .route(
            "/api/ab-tests/:test_id/start",
            post(ab_test_handlers::start_ab_test),
        )
        .route(
            "/api/ab-tests/:test_id/stop",
            post(ab_test_handlers::stop_ab_test),
        )
        .route(
            "/api/ab-tests/:test_id/cancel",
            post(ab_test_handlers::cancel_ab_test),
        )
        .route(
            "/api/ab-tests/:test_id/metrics",
            post(ab_test_handlers::record_ab_test_metric),
        )
        .route(
            "/api/ab-tests/:test_id/results",
            get(ab_test_handlers::get_ab_test_results),
        )
}

pub fn performance_routes() -> Router<AppState> {
    Router::new()
        // Contract-scoped performance endpoints
        .route(
            "/api/contracts/:id/perf/benchmarks",
            get(performance_handlers::list_benchmarks).post(performance_handlers::record_benchmark),
        )
        .route(
            "/api/contracts/:id/perf/metrics",
            get(performance_handlers::list_metrics).post(performance_handlers::record_metric),
        )
        .route(
            "/api/contracts/:id/perf/comparison",
            get(performance_handlers::get_performance_comparison),
        )
        .route(
            "/api/contracts/:id/perf/anomalies",
            get(performance_handlers::list_anomalies),
        )
        .route(
            "/api/contracts/:id/perf/alerts",
            get(performance_handlers::list_alerts),
        )
        .route(
            "/api/contracts/:id/perf/alert-configs",
            get(performance_handlers::list_alert_configs)
                .post(performance_handlers::create_alert_config),
        )
        .route(
            "/api/contracts/:id/perf/trends",
            get(performance_handlers::list_trends),
        )
        .route(
            "/api/contracts/:id/perf/summary",
            get(performance_handlers::get_performance_summary),
        )
        // Alert-specific action endpoints
        .route(
            "/api/perf/alerts/:alert_id/acknowledge",
            post(performance_handlers::acknowledge_alert),
        )
        .route(
            "/api/perf/alerts/:alert_id/resolve",
            post(performance_handlers::resolve_alert),
        )
}

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/audit-logs", get(handlers::get_all_audit_logs))
        .route(
            "/api/admin/audit-logs/export",
            get(handlers::handle_export_audit),
        )
        .route(
            "/api/admin/audit-logs/cleanup",
            post(handlers::handle_retention_cleanup),
        )
        // Category management (issue #414) – admin-only write endpoints
        .route(
            "/api/admin/categories",
            post(category_handlers::create_category),
        )
        .route(
            "/api/admin/categories/:id",
            put(category_handlers::update_category).delete(category_handlers::delete_category),
        )
        // Version revert (issue #486) – admin-only
        .route(
            "/api/admin/contracts/:id/versions/:version/revert",
            post(handlers::revert_contract_version),
        )
        .route_layer(middleware::from_fn(auth::require_admin))
}

pub fn federation_routes() -> Router<AppState> {
    Router::new()
        // Federated registry management (#499)
        .route(
            "/api/federation/registries",
            get(clone_federation_handlers::list_federated_registries)
                .post(clone_federation_handlers::register_federated_registry),
        )
        .route(
            "/api/federation/registries/:id",
            get(clone_federation_handlers::get_federated_registry),
        )
        // Sync operations
        .route(
            "/api/federation/sync",
            post(clone_federation_handlers::sync_from_federated_registry),
        )
        .route(
            "/api/federation/sync/:job_id",
            get(clone_federation_handlers::get_sync_job_status),
        )
        .route(
            "/api/federation/sync-history",
            get(clone_federation_handlers::get_federation_sync_history),
        )
        // Discovery
        .route(
            "/api/federation/discover",
            get(clone_federation_handlers::discover_federated_registries),
        )
        // Configuration
        .route(
            "/api/federation/config",
            get(clone_federation_handlers::get_federation_config),
        )
        // Contract federation attribution
        .route(
            "/api/contracts/:id/federation",
            get(clone_federation_handlers::get_contract_federation_attribution)
                .patch(clone_federation_handlers::update_contract_federation_settings),
        )
}

pub fn websocket_routes() -> Router<AppState> {
    // /ws/contracts is registered in contract_routes via contract_events::contracts_websocket.
    // This function is retained so main.rs can call it without a merge conflict.
    Router::new()
}

// ═══════════════════════════════════════════════════════════════════════════
// SECURITY SCANNING ROUTES (#498)
// ═══════════════════════════════════════════════════════════════════════════

pub fn security_scanning_routes() -> Router<AppState> {
    Router::new()
        // Security scanner management
        .route(
            "/api/security/scanners",
            get(security_scan_handlers::list_security_scanners)
                .post(security_scan_handlers::create_security_scanner),
        )
        // Contract security endpoints
        .route(
            "/api/contracts/:id/scans",
            get(security_scan_handlers::list_security_scans)
                .post(security_scan_handlers::trigger_security_scan),
        )
        .route(
            "/api/contracts/:id/scans/:scan_id",
            get(security_scan_handlers::get_security_scan),
        )
        .route(
            "/api/contracts/:id/security",
            get(security_scan_handlers::get_contract_security_summary),
        )
        .route(
            "/api/contracts/:id/security/score-history",
            get(security_scan_handlers::get_security_score_history),
        )
        .route(
            "/api/contracts/:id/issues",
            get(security_scan_handlers::list_security_issues),
        )
        .route(
            "/api/contracts/:id/issues/:issue_id",
            patch(security_scan_handlers::update_security_issue),
        )
}

// ═══════════════════════════════════════════════════════════════════════════
// SUBSCRIPTION & NOTIFICATION ROUTES (#493)
// ═══════════════════════════════════════════════════════════════════════════

pub fn subscription_routes() -> Router<AppState> {
    Router::new()
        // User subscriptions
        .route(
            "/api/me/subscriptions",
            get(subscription_handlers::list_user_subscriptions),
        )
        .route(
            "/api/contracts/:id/subscribe",
            post(subscription_handlers::subscribe_to_contract)
                .delete(subscription_handlers::unsubscribe_from_contract),
        )
        .route(
            "/api/subscriptions/:id",
            patch(subscription_handlers::update_subscription),
        )
        // Notification preferences
        .route(
            "/api/notifications/preferences",
            get(subscription_handlers::get_notification_preferences)
                .patch(subscription_handlers::update_notification_preferences),
        )
        // Notifications
        .route(
            "/api/notifications",
            get(subscription_handlers::list_notifications),
        )
        .route(
            "/api/notifications/:id/read",
            post(subscription_handlers::mark_notification_read),
        )
        .route(
            "/api/notifications/read-all",
            post(subscription_handlers::mark_all_notifications_read),
        )
        .route(
            "/api/notifications/statistics",
            get(subscription_handlers::get_notification_statistics),
        )
        // Webhooks
        .route(
            "/api/webhooks",
            get(subscription_handlers::list_webhooks).post(subscription_handlers::create_webhook),
        )
        .route(
            "/api/webhooks/:id",
            delete(subscription_handlers::delete_webhook),
        )
        .route(
            "/api/webhooks/:id/deliveries",
            get(subscription_handlers::get_webhook_deliveries),
        )
        .route(
            "/api/webhooks/:id/test",
            post(subscription_handlers::test_webhook),
        )
        .route(
            "/api/webhook-deliveries/:id/retry",
            post(subscription_handlers::retry_webhook_delivery),
        )
}

// ═══════════════════════════════════════════════════════════════════════════
// FORMAL VERIFICATION ROUTES
// ═══════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT INTERACTION GRAPH ANALYSIS ROUTES
// ═══════════════════════════════════════════════════════════════════════════

pub fn graph_analysis_routes() -> Router<AppState> {
    Router::new()
        // Full analysis report: clusters + critical contracts + cycles
        .route(
            "/api/contracts/graph/analysis",
            get(graph_analysis_handlers::get_graph_analysis),
        )
        // Sub-network / community list
        .route(
            "/api/contracts/graph/clusters",
            get(graph_analysis_handlers::get_graph_clusters),
        )
        // Sub-network detail by cluster ID
        .route(
            "/api/contracts/graph/subnetwork/:cluster_id",
            get(graph_analysis_handlers::get_subnetwork),
        )
        // Critical contract ranking
        .route(
            "/api/contracts/graph/critical",
            get(graph_analysis_handlers::get_critical_contracts),
        )
        // Vulnerability propagation from a specific contract
        .route(
            "/api/contracts/:id/vulnerability-propagation",
            get(graph_analysis_handlers::get_vulnerability_propagation),
        )
}

pub fn formal_verification_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts/:id/formal-verification",
            post(formal_verification_handlers::trigger_formal_verification)
                .get(formal_verification_handlers::list_formal_verification_sessions),
        )
        .route(
            "/api/contracts/:id/formal-verification/:session_id",
            get(formal_verification_handlers::get_formal_verification_session),
        )
        .merge(formal_verification_integration_routes())
}

/// Issue #889 — formal verification integration: pluggable backends, property
/// config, optional/mandatory policy, timeout-aware runs, caching, reports.
pub fn formal_verification_integration_routes() -> Router<AppState> {
    Router::new()
        // Per-contract run + profile integration.
        .route(
            "/api/contracts/:id/formal-verification/run",
            post(formal_verification_integration::run_verification),
        )
        .route(
            "/api/contracts/:id/formal-verification/runs",
            get(formal_verification_integration::list_runs),
        )
        .route(
            "/api/contracts/:id/formal-verification/runs/:run_id/report",
            get(formal_verification_integration::get_report),
        )
        .route(
            "/api/contracts/:id/formal-verification/summary",
            get(formal_verification_integration::get_summary),
        )
        .route(
            "/api/contracts/:id/formal-verification/requirement",
            get(formal_verification_integration::get_requirement),
        )
        // Property configuration + per-category policy.
        .route(
            "/api/formal-verification/properties",
            get(formal_verification_integration::list_properties)
                .post(formal_verification_integration::upsert_property),
        )
        .route(
            "/api/formal-verification/policies",
            get(formal_verification_integration::list_policies),
        )
        .route(
            "/api/formal-verification/policies/:category",
            put(formal_verification_integration::set_policy),
        )
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT VERIFICATION STATUS ROUTES (issue #724)
// ═══════════════════════════════════════════════════════════════════════════

pub fn verification_status_routes() -> Router<AppState> {
    Router::new()
        // Submit a contract for verification by ID (complements the global /verify endpoint)
        .route(
            "/api/contracts/:id/verify",
            post(verification_handlers::submit_contract_verification),
        )
        // Get current verification status (cached 1 hour)
        .route(
            "/api/contracts/:id/verification-status",
            get(verification_handlers::get_contract_verification_status),
        )
        // Get chronological audit trail of verification status changes
        .route(
            "/api/contracts/:id/verification-history",
            get(verification_handlers::get_contract_verification_history),
        )
}

// ═══════════════════════════════════════════════════════════════════════════
// ZERO-KNOWLEDGE PROOF VALIDATION ROUTES (#624)
// ═══════════════════════════════════════════════════════════════════════════

pub fn zk_proof_routes() -> Router<AppState> {
    Router::new()
        // ── Circuit management ─────────────────────────────────────────
        .route(
            "/api/contracts/:id/zk/circuits",
            post(zk_proof_handlers::register_circuit).get(zk_proof_handlers::list_circuits),
        )
        .route(
            "/api/contracts/:id/zk/circuits/:circuit_id",
            get(zk_proof_handlers::get_circuit),
        )
        // ── Proof submission & validation ──────────────────────────────
        .route(
            "/api/contracts/:id/zk/proofs",
            post(zk_proof_handlers::submit_proof).get(zk_proof_handlers::list_proofs),
        )
        .route(
            "/api/contracts/:id/zk/proofs/:proof_id",
            get(zk_proof_handlers::get_proof),
        )
        // ── Privacy-preserving analytics ───────────────────────────────
        .route(
            "/api/contracts/:id/zk/analytics",
            get(zk_proof_handlers::get_zk_analytics),
        )
}

pub fn collaborative_review_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/reviews/collaborative",
            post(collaborative_reviews::create_collaborative_review),
        )
        .route(
            "/api/reviews/collaborative/:id",
            get(collaborative_reviews::get_collaborative_review),
        )
        .route(
            "/api/reviews/collaborative/:id/comment",
            post(collaborative_reviews::add_collaborative_comment),
        )
        .route(
            "/api/reviews/collaborative/:id/status",
            patch(collaborative_reviews::update_reviewer_status),
        )
}

// ── Issue #878: Database query monitoring ────────────────────────────────────

pub fn query_monitor_routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/db/pool-stats", get(db_pool::get_pool_stats))
        .route(
            "/api/admin/db/slow-queries",
            get(query_monitor::get_slow_queries),
        )
        .route(
            "/api/admin/db/index-stats",
            get(query_monitor::get_index_stats),
        )
        .route(
            "/api/admin/db/lock-monitor",
            get(query_monitor::get_lock_monitor),
        )
        .route(
            "/api/admin/db/performance-trends",
            get(query_monitor::get_performance_trends),
        )
        .route(
            "/api/admin/db/performance-report",
            get(query_monitor::get_performance_report),
        )
        .route(
            "/api/admin/db/performance-report/export",
            post(query_monitor::export_performance_report),
        )
}

// ── Issue #879: Data partitioning ────────────────────────────────────────────

pub fn partition_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/admin/partitions",
            get(partition_manager::list_partitions),
        )
        .route(
            "/api/admin/partitions/status",
            get(partition_manager::get_partition_status),
        )
        .route(
            "/api/admin/partitions/create",
            post(partition_manager::create_partition),
        )
        .route(
            "/api/admin/partitions/:name",
            delete(partition_manager::archive_partition),
        )
}

// ── Issue #881: Data archival ────────────────────────────────────────────────

pub fn archival_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/admin/archival/status",
            get(archival::get_archival_status),
        )
        .route(
            "/api/admin/archival/policies",
            get(archival::get_archival_policies),
        )
        .route(
            "/api/admin/archival/policies/:data_type",
            patch(archival::update_archival_policy),
        )
        .route("/api/admin/archival/run", post(archival::trigger_archival))
        .route(
            "/api/admin/archival/audit-trail",
            get(archival::get_archival_audit_trail),
        )
        .route(
            "/api/admin/archival/restore",
            post(archival::restore_archived_record),
        )
}

// ── Issue #886: Data integrity verification and checksums ────────────────────

pub fn integrity_routes() -> Router<AppState> {
    Router::new()
        // Per-contract checksum + verification endpoints.
        .route(
            "/api/contracts/:id/integrity/checksums",
            post(integrity::compute_checksums_handler),
        )
        .route(
            "/api/contracts/:id/integrity",
            get(integrity::get_checksums_handler),
        )
        .route(
            "/api/contracts/:id/integrity/verify",
            post(integrity::verify_contract_handler),
        )
        .route(
            "/api/contracts/:id/integrity/access-check",
            get(integrity::access_check_handler),
        )
        .route(
            "/api/contracts/:id/integrity/repair",
            post(integrity::repair_contract_handler),
        )
        // Admin / system-wide integrity endpoints.
        .route(
            "/api/admin/integrity/verify",
            post(integrity::trigger_full_verification_handler),
        )
        .route(
            "/api/admin/integrity/status",
            get(integrity::get_integrity_status_handler),
        )
        .route(
            "/api/admin/integrity/runs",
            get(integrity::list_runs_handler),
        )
        .route(
            "/api/admin/integrity/issues",
            get(integrity::list_issues_handler),
        )
}

// ── Issue #880: Elasticsearch / full-text search ─────────────────────────────

pub fn elasticsearch_search_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/search/elasticsearch",
            get(elasticsearch_handlers::elasticsearch_search),
        )
        .route(
            "/api/search/analytics",
            get(elasticsearch_handlers::get_search_analytics),
        )
        .route(
            "/api/search/trending",
            get(elasticsearch_handlers::get_trending_searches),
        )
        .route(
            "/api/admin/search/reindex",
            post(elasticsearch_handlers::reindex_contracts),
        )
        .route(
            "/api/admin/search/synonyms",
            get(elasticsearch_handlers::get_synonyms).put(elasticsearch_handlers::upsert_synonym),
        )
}

// ── Discovery & Reporting routes (issues #870–#873) ───────────────────────────

/// Routes for the v1 discovery and reporting endpoints.
pub fn discovery_reporting_routes() -> Router<AppState> {
    Router::new()
        // Issue: Advanced search endpoint
        .route(
            "/api/v1/contracts/search",
            get(v1_search_handlers::advanced_search),
        )
        // Issue #873: Contract issue reporting
        .route(
            "/api/v1/contracts/:id/report",
            post(report_handlers::report_contract),
        )
        .route(
            "/api/v1/contracts/:id/report/:report_id/status",
            get(report_handlers::get_report_status),
        )
        // Issue #872: List deprecated contracts
        .route(
            "/api/v1/contracts/deprecated",
            get(deprecated_contracts_handlers::list_deprecated_contracts),
        )
        // Issue #871: Similar contracts (v1, with type param + 6h cache)
        .route(
            "/api/v1/contracts/:id/similar",
            get(v1_similar_handlers::get_similar_contracts_v1),
        )
        // Issue #870: Trending endpoint (v1, with window param)
        .route(
            "/api/v1/trending",
            get(v1_trending_handlers::get_trending_v1),
        )
}

// ── Issue #1007: Backend feature flag management ───────────────────────────
pub fn feature_flag_routes() -> Router<AppState> {
    Router::new().route(
        "/api/feature-flags",
        get(crate::feature_flags::get_flag_status_handler),
    )
}
