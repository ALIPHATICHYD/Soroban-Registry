#[test]
fn test_database_ha_documentation_exists() {
    let doc_path = "../../docs/database-high-availability.md";
    assert!(
        std::path::Path::new(doc_path).exists(),
        "HA documentation should exist at {}",
        doc_path
    );
}

#[test]
fn test_docker_compose_defines_replication_topology() {
    let compose_path = "../../docker-compose.yml";
    let content = std::fs::read_to_string(compose_path).expect("Should read docker-compose.yml");

    assert!(
        content.contains("postgres-primary"),
        "Compose should define a primary database node"
    );
    assert!(
        content.contains("postgres-replica"),
        "Compose should define a replica database node"
    );
    assert!(
        content.contains("pgpool"),
        "Compose should define a pgpool load balancer"
    );
    assert!(
        content.contains("DATABASE_URL: postgresql://${POSTGRES_USER:-postgres}:${POSTGRES_PASSWORD:?err}@pgpool:5432/${POSTGRES_DB:-soroban_registry}"),
        "Application services should connect through pgpool"
    );
}

#[test]
fn test_replication_monitoring_and_alerts_are_configured() {
    let compose_path = "../../docker-compose.yml";
    let compose = std::fs::read_to_string(compose_path).expect("Should read docker-compose.yml");
    let metrics_path = "../../backend/api/src/metrics.rs";
    let metrics = std::fs::read_to_string(metrics_path).expect("Should read metrics.rs");
    let alerts_path = "../../observability/prometheus/alert_rules.yml";
    let alerts = std::fs::read_to_string(alerts_path).expect("Should read alert rules");

    assert!(
        compose.contains("DATABASE_PRIMARY_URL"),
        "API should receive a primary URL for replication checks"
    );
    assert!(
        compose.contains("DATABASE_REPLICA_URL"),
        "API should receive a replica URL for replication checks"
    );
    assert!(
        metrics.contains("db_replication_lag_ms"),
        "Replication lag metric should be exported"
    );
    assert!(
        metrics.contains("db_replication_health"),
        "Replication health metric should be exported"
    );
    assert!(
        alerts.contains("DatabaseReplicationLagHigh"),
        "Prometheus should alert on excessive replication lag"
    );
}

#[test]
fn test_manual_failover_runbook_is_documented() {
    let doc_path = "../../docs/database-high-availability.md";
    let content = std::fs::read_to_string(doc_path).expect("Should read HA documentation");

    assert!(
        content.contains("docker compose exec postgres-replica repmgr standby promote"),
        "Manual promotion steps should be documented"
    );
    assert!(
        content.contains("docker compose restart pgpool"),
        "Manual failover should include restarting pgpool"
    );
    assert!(
        content.contains("100 ms"),
        "Lag target should be documented"
    );
}
