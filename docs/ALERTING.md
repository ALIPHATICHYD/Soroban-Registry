# Registry Monitoring & Alerting

This documents the alerting configured for registry uptime, error rates, and
latency (issue #1049), how alerts are routed, and how to respond to them.

## Stack

```
API (/metrics) --> Prometheus (scrape + alert_rules.yml) --> Alertmanager --> Slack / PagerDuty
```

- `backend/api/src/metrics.rs` — Prometheus metric definitions, exported at `GET /metrics`.
- `observability/prometheus/prometheus.yml` — scrape config; scrapes the API job `soroban-registry-api` every 15s.
- `observability/prometheus/alert_rules.yml` — alerting rules (thresholds, `for` durations, severity labels).
- `observability/alertmanager/alertmanager.yml` — routing tree and receivers.
- `GET /health`, `/health/live`, `/health/ready`, `/health/detailed`, `/health/services` — liveness/readiness probes (`backend/api/src/handlers.rs`, `backend/api/src/service_health.rs`).

## Health check endpoints

| Endpoint | Purpose |
|---|---|
| `GET /health` | Basic liveness; 503 while shutting down. |
| `GET /health/live` | Kubernetes-style liveness probe (status code only). |
| `GET /health/ready` | Readiness — 503 if the database is unreachable. |
| `GET /health/detailed` | Liveness + database + cache dependency status. |
| `GET /health/services` | Per-core-service status: **publish**, **search**, **verification** — each independently classified `healthy` / `degraded` / `unhealthy` so an operator can tell which request path is affected without reading raw metrics. |

`/health/services` is the one to alert/page on for "is the registry actually
usable" — the others answer narrower questions (process up? DB reachable?).

## Alert catalog

All rules live in `observability/prometheus/alert_rules.yml`.

| Alert | Severity | Condition | Fires after |
|---|---|---|---|
| `RegistryAPIDown` | critical | Prometheus can't scrape the API | 1m |
| `RegistryAvailabilitySLOBreach` | critical | 30-day scrape uptime < 99.5% | 5m |
| `RegistryHighErrorRate` | warning | Overall 5xx rate > 2% | 5m |
| `RegistryCriticalErrorRate` | critical | Overall 5xx rate > 5% | 5m |
| `RegistryHighLatencyP95` | warning | Overall p95 latency > 1s | 5m |
| `RegistryHighLatencyP99` | critical | Overall p99 latency > 2.5s | 5m |
| `PublishServiceErrorRateHigh` | critical | `POST /api/contracts` 5xx rate > 5% | 10m |
| `SearchServiceErrorRateHigh` | critical | `GET /api/search` 5xx rate > 5% | 10m |
| `SearchServiceLatencyHigh` | warning | `GET /api/search` p95 latency > 1.5s | 10m |
| `VerificationFailureRateHigh` | warning | Verification failure rate > 10% | 10m |
| `VerificationQueueBacklog` | warning | Pending verification queue depth > 50 | 10m |
| `SearchSlowQueriesHigh` | warning | Slow (>500ms) search queries > 1/s | 10m |
| `DatabaseReplicationLagHigh` | warning | Replica lag > 100ms | 1m |
| `DatabaseReplicationHealthDegraded` | critical | Replica reports unhealthy | 1m |

Thresholds are deliberately conservative starting points — tune them against
real traffic once there's a baseline, rather than treating them as final.

## Escalation / routing

Configured in `observability/alertmanager/alertmanager.yml`:

- **critical** → `pagerduty-critical` receiver: pages on-call via PagerDuty **and** posts to the `#registry-alerts` Slack channel. Re-notifies every hour until acknowledged or resolved (`repeat_interval: 1h`, `group_wait: 10s` — pages fast).
- **warning** → `slack-warnings` receiver: posts to `#registry-alerts` only, no page. Re-notifies every 4 hours (`repeat_interval: 4h`).

Secrets (`SLACK_WEBHOOK_URL`, `PAGERDUTY_SERVICE_KEY`) are supplied as
container env vars in `docker-compose.yml` and written to files the
Alertmanager config reads via `slack_api_url_file` / `routing_key_file` —
Alertmanager's config file has no native env-var expansion, so this avoids
committing secrets into `alertmanager.yml`. Set them in `.env` (see
`.env.example`) before running `docker compose up alertmanager`; without
them, alerts still fire and are visible in Alertmanager/Prometheus, they just
won't reach Slack/PagerDuty.

### On-call response

1. **Critical page fires** — check `/health/services` first to identify which
   core service (publish/search/verification) is affected, then
   `/health/detailed` for the underlying dependency (database/cache).
2. **Acknowledge in PagerDuty** to stop re-paging while investigating.
3. Check the Grafana dashboard (`observability/grafana/dashboards/soroban-registry.json`) for the affected metric's recent trend.
4. If it's a database issue, see `docs/database-high-availability.md`.
5. Resolve in PagerDuty once the alert clears in Alertmanager (or it will auto-resolve when Prometheus stops firing it).

Warning-level Slack alerts don't require immediate action but should be
triaged during business hours — they're often an early signal of something
that becomes a critical page later (e.g. `RegistryHighErrorRate` trending
toward `RegistryCriticalErrorRate`).

## Simulating degraded conditions (for testing)

- `backend/api/src/service_health.rs` unit tests exercise the classification
  logic (healthy/degraded/unhealthy thresholds) for each core service without
  needing a live database.
- To see a real alert fire locally: run the stack via `docker-compose.yml`,
  then either stop the `api` container (triggers `RegistryAPIDown`) or use a
  load-testing tool against `POST /api/contracts` with invalid payloads to
  drive up the error rate (triggers `PublishServiceErrorRateHigh` /
  `RegistryHighErrorRate` once sustained past the `for:` duration).
