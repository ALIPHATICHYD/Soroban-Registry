// Issue #1049: monitoring and alerting for registry uptime and error rates.
//
// These tests assert the observability wiring stays in place — that the
// alerting rules, routing config, and documentation described in
// docs/ALERTING.md actually exist and reference the metrics/routes the code
// emits. They mirror the pattern in database_replication_ha_tests.rs.
// Degraded-condition classification logic (healthy/degraded/unhealthy
// thresholds) is unit-tested in-crate in src/service_health.rs.

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {}: {}", path, e))
}

#[test]
fn test_alerting_documentation_exists() {
    let doc_path = "../../docs/ALERTING.md";
    assert!(
        std::path::Path::new(doc_path).exists(),
        "Alerting/escalation documentation should exist at {}",
        doc_path
    );
    let doc = read(doc_path);
    assert!(
        doc.contains("Escalation") || doc.contains("escalation"),
        "ALERTING.md should document escalation routing"
    );
    assert!(
        doc.contains("/health/services"),
        "ALERTING.md should document the per-service health endpoint"
    );
}

#[test]
fn test_health_services_endpoint_is_registered() {
    let routes = read("../../backend/api/src/routes.rs");
    assert!(
        routes.contains("/health/services"),
        "routes.rs should register GET /health/services"
    );
    assert!(
        routes.contains("service_health::health_check_services"),
        "routes.rs should wire /health/services to service_health::health_check_services"
    );
}

#[test]
fn test_alert_rules_cover_uptime_error_rate_and_latency() {
    let alerts = read("../../observability/prometheus/alert_rules.yml");

    // Uptime
    assert!(
        alerts.contains("RegistryAPIDown"),
        "alert_rules.yml should alert when the API is unscrapeable (uptime)"
    );
    assert!(
        alerts.contains(r#"up{job="soroban-registry-api"}"#),
        "uptime alerts should key off the API's Prometheus scrape target"
    );

    // Error rate (overall + per core service)
    for alert in [
        "RegistryHighErrorRate",
        "RegistryCriticalErrorRate",
        "PublishServiceErrorRateHigh",
        "SearchServiceErrorRateHigh",
    ] {
        assert!(
            alerts.contains(alert),
            "alert_rules.yml should define {}",
            alert
        );
    }
    assert!(
        alerts.contains("http_requests_total"),
        "error-rate alerts should be built on the http_requests_total metric"
    );

    // Latency
    for alert in ["RegistryHighLatencyP95", "RegistryHighLatencyP99"] {
        assert!(
            alerts.contains(alert),
            "alert_rules.yml should define {}",
            alert
        );
    }
    assert!(
        alerts.contains("http_request_duration_seconds_bucket"),
        "latency alerts should be built on the http_request_duration_seconds histogram"
    );

    // Core service degradation (verification, search)
    for alert in [
        "VerificationFailureRateHigh",
        "VerificationQueueBacklog",
        "SearchSlowQueriesHigh",
    ] {
        assert!(
            alerts.contains(alert),
            "alert_rules.yml should define {}",
            alert
        );
    }

    // Every alert must carry a severity label so Alertmanager can route it.
    let severity_labels = alerts.matches("severity:").count();
    let alert_defs = alerts.matches("- alert:").count();
    assert!(alert_defs >= 10, "expected a substantial alert catalog");
    assert_eq!(
        severity_labels, alert_defs,
        "every alert rule must declare a severity label for routing"
    );
}

#[test]
fn test_alertmanager_routes_by_severity_with_receivers() {
    let am = read("../../observability/alertmanager/alertmanager.yml");

    assert!(
        am.contains(r#"severity = "critical""#),
        "alertmanager.yml should route on severity=critical"
    );
    assert!(
        am.contains(r#"severity = "warning""#),
        "alertmanager.yml should route on severity=warning"
    );
    assert!(
        am.contains("pagerduty_configs"),
        "critical alerts should be configured to page via PagerDuty"
    );
    assert!(
        am.contains("slack_configs"),
        "alerts should be configured to notify Slack"
    );
    let receiver_count = am.matches("- name:").count();
    assert!(
        receiver_count >= 2,
        "alertmanager.yml should define distinct receivers for critical vs warning severity, found {}",
        receiver_count
    );
}

#[test]
fn test_alertmanager_secrets_are_provisioned_from_env_at_container_start() {
    let compose = read("../../docker-compose.yml");
    assert!(
        compose.contains("SLACK_WEBHOOK_URL"),
        "docker-compose.yml should pass SLACK_WEBHOOK_URL to alertmanager"
    );
    assert!(
        compose.contains("PAGERDUTY_SERVICE_KEY"),
        "docker-compose.yml should pass PAGERDUTY_SERVICE_KEY to alertmanager"
    );
    assert!(
        compose.contains("/alertmanager/secrets/slack_webhook_url"),
        "docker-compose.yml should materialize the Slack secret file alertmanager.yml expects"
    );
}

#[test]
fn test_verification_and_search_metrics_are_actually_recorded() {
    // Guards against the alert rules referencing metrics that are declared
    // but never incremented anywhere (which would mean the alert can never fire).
    let handlers = read("../../backend/api/src/handlers.rs");
    assert!(
        handlers.contains("observe_verification_latency"),
        "verify_contract should record verification outcomes so \
         VerificationFailureRateHigh has real data to alert on"
    );

    let search = read("../../backend/api/src/search_postgres.rs");
    assert!(
        search.contains("SEARCH_QUERY_DURATION") && search.contains("SEARCH_SLOW_QUERIES"),
        "the search handler should record query duration/slow-query metrics so \
         SearchSlowQueriesHigh has real data to alert on"
    );
}
