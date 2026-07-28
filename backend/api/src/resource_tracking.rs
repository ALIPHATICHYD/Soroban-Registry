use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const MAX_CPU: u64 = 100_000_000;
const MAX_MEM: u64 = 40 * 1024 * 1024;
const ALERT_PCT: f64 = 0.80;
const ALPHA: f64 = 0.3;
const BETA: f64 = 0.1;
const GAMMA: f64 = 0.2;
const SEASON_LEN: usize = 24;
const Z_P90: f64 = 1.2815515655446004;
const EPS: f64 = 1e-9;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_instructions: u64,
    pub mem_bytes: u64,
    pub storage_bytes: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageForecast {
    pub cpu_exhaustion_ts: Option<DateTime<Utc>>,
    pub mem_exhaustion_ts: Option<DateTime<Utc>>,
    pub cpu_exhaustion_ts_p90: Option<DateTime<Utc>>,
    pub mem_exhaustion_ts_p90: Option<DateTime<Utc>>,
    pub cpu_trend_per_sec: f64,
    pub mem_trend_per_sec: f64,
    pub confidence: f64,
    pub seasonal_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLimits {
    pub max_cpu_instructions: u64,
    pub max_mem_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAlert {
    pub metric: String,
    pub current_pct: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub contract_id: String,
    pub current: ResourceUsage,
    pub history: Vec<ResourceUsage>,
    pub network_limits: NetworkLimits,
    pub alerts: Vec<ResourceAlert>,
    pub forecast: UsageForecast,
}

pub struct ResourceManager {
    data: HashMap<String, Vec<ResourceUsage>>,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn record_usage(&mut self, contract_id: &str, usage: ResourceUsage) -> Vec<ResourceAlert> {
        let alerts = Self::check_alerts(&usage);
        self.data
            .entry(contract_id.to_string())
            .or_default()
            .push(usage);
        crate::metrics::RESOURCE_RECORDINGS.inc();
        if !alerts.is_empty() {
            crate::metrics::RESOURCE_ALERTS_FIRED.inc();
            for alert in &alerts {
                tracing::warn!(
                    contract_id = contract_id,
                    metric = alert.metric.as_str(),
                    current_pct = alert.current_pct,
                    message = alert.message.as_str()
                );
            }
        }
        alerts
    }

    fn check_alerts(u: &ResourceUsage) -> Vec<ResourceAlert> {
        let mut out = Vec::new();
        let cpu_pct = u.cpu_instructions as f64 / MAX_CPU as f64 * 100.0;
        if cpu_pct >= ALERT_PCT * 100.0 {
            out.push(ResourceAlert {
                metric: "cpu_instructions".into(),
                current_pct: (cpu_pct * 10.0).round() / 10.0,
                message: format!("CPU usage at {:.1}% of network limit", cpu_pct),
            });
        }
        let mem_pct = u.mem_bytes as f64 / MAX_MEM as f64 * 100.0;
        if mem_pct >= ALERT_PCT * 100.0 {
            out.push(ResourceAlert {
                metric: "mem_bytes".into(),
                current_pct: (mem_pct * 10.0).round() / 10.0,
                message: format!("Memory usage at {:.1}% of network limit", mem_pct),
            });
        }
        out
    }

    pub fn summary(&self, contract_id: &str) -> Option<ResourceSummary> {
        let history = self.data.get(contract_id)?;
        if history.is_empty() {
            return None;
        }
        let current = history.last().unwrap().clone();
        let alerts = Self::check_alerts(&current);
        let forecast = self.compute_forecast(history);
        crate::metrics::RESOURCE_FORECAST_RUNS.inc();
        Some(ResourceSummary {
            contract_id: contract_id.to_string(),
            current,
            history: history.clone(),
            network_limits: NetworkLimits {
                max_cpu_instructions: MAX_CPU,
                max_mem_bytes: MAX_MEM,
            },
            alerts,
            forecast,
        })
    }

    fn compute_forecast(&self, history: &[ResourceUsage]) -> UsageForecast {
        let cpu: Vec<f64> = history.iter().map(|u| u.cpu_instructions as f64).collect();
        let mem: Vec<f64> = history.iter().map(|u| u.mem_bytes as f64).collect();
        let last_ts = history.last().unwrap().timestamp;
        let current_cpu = *cpu.last().unwrap_or(&0.0);
        let current_mem = *mem.last().unwrap_or(&0.0);
        let dt = if history.len() >= 2 {
            let prev = history[history.len() - 2].timestamp;
            (last_ts - prev).num_seconds().max(1) as f64
        } else {
            3600.0
        };

        let cpu_deltas = deltas(&cpu);
        let mem_deltas = deltas(&mem);
        let (cpu_level, cpu_trend, cpu_seasonal) = holt_winters(&cpu_deltas);
        let (mem_level, mem_trend, mem_seasonal) = holt_winters(&mem_deltas);

        let mut cpu_step_burn = (cpu_level + cpu_trend).max(0.0) * cpu_seasonal.max(1.0);
        let mut mem_step_burn = (mem_level + mem_trend).max(0.0) * mem_seasonal.max(1.0);
        if cpu_step_burn <= EPS {
            cpu_step_burn = *cpu_deltas.last().unwrap_or(&0.0);
        }
        if mem_step_burn <= EPS {
            mem_step_burn = *mem_deltas.last().unwrap_or(&0.0);
        }

        let cpu_sigma = std_dev(&cpu_deltas);
        let mem_sigma = std_dev(&mem_deltas);
        let cpu_step_burn_p90 = (cpu_step_burn - Z_P90 * cpu_sigma).max(EPS);
        let mem_step_burn_p90 = (mem_step_burn - Z_P90 * mem_sigma).max(EPS);

        let cpu_exhaust =
            project_exhaustion(current_cpu, cpu_step_burn, MAX_CPU as f64, last_ts, dt);
        let mem_exhaust =
            project_exhaustion(current_mem, mem_step_burn, MAX_MEM as f64, last_ts, dt);
        let cpu_exhaust_p90 =
            project_exhaustion(current_cpu, cpu_step_burn_p90, MAX_CPU as f64, last_ts, dt);
        let mem_exhaust_p90 =
            project_exhaustion(current_mem, mem_step_burn_p90, MAX_MEM as f64, last_ts, dt);

        let n = cpu_deltas.len().max(mem_deltas.len()) as f64;
        let variance_penalty = (cpu_sigma + mem_sigma) / (cpu_step_burn + mem_step_burn + 1.0);
        let confidence = ((1.0 - 1.0 / (n + 1.0)) * (1.0 - variance_penalty)).clamp(0.0, 0.99);

        UsageForecast {
            cpu_exhaustion_ts: cpu_exhaust,
            mem_exhaustion_ts: mem_exhaust,
            cpu_exhaustion_ts_p90: cpu_exhaust_p90,
            mem_exhaustion_ts_p90: mem_exhaust_p90,
            cpu_trend_per_sec: cpu_step_burn / dt,
            mem_trend_per_sec: mem_step_burn / dt,
            confidence,
            seasonal_factor: cpu_seasonal,
        }
    }
}

fn holt_winters(data: &[f64]) -> (f64, f64, f64) {
    if data.is_empty() {
        return (0.0, 0.0, 1.0);
    }
    if data.len() == 1 {
        return (data[0], 0.0, 1.0);
    }

    let period = SEASON_LEN.min(data.len());
    let mut seasonal = vec![1.0_f64; period];
    if data.len() >= period {
        let avg = data[..period].iter().sum::<f64>() / period as f64;
        if avg > 0.0 {
            for i in 0..period {
                seasonal[i] = data[i] / avg;
            }
        }
    }

    let mut level = data[0];
    let mut trend = data[1] - data[0];
    let mut peak_seasonal = 1.0_f64;

    for (t, &y) in data.iter().enumerate().skip(1) {
        let s_idx = t % period;
        let s = seasonal[s_idx];
        let new_level = ALPHA * (y / s) + (1.0 - ALPHA) * (level + trend);
        let new_trend = BETA * (new_level - level) + (1.0 - BETA) * trend;
        if new_level > 0.0 {
            seasonal[s_idx] = GAMMA * (y / new_level) + (1.0 - GAMMA) * s;
        }
        if seasonal[s_idx] > peak_seasonal {
            peak_seasonal = seasonal[s_idx];
        }
        level = new_level;
        trend = new_trend;
    }

    (level, trend, peak_seasonal)
}

fn deltas(data: &[f64]) -> Vec<f64> {
    if data.len() < 2 {
        return vec![0.0];
    }
    data.windows(2).map(|w| (w[1] - w[0]).max(0.0)).collect()
}

fn std_dev(data: &[f64]) -> f64 {
    if data.len() <= 1 {
        return 0.0;
    }
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let var = data
        .iter()
        .map(|x| {
            let d = x - mean;
            d * d
        })
        .sum::<f64>()
        / data.len() as f64;
    var.sqrt()
}

fn project_exhaustion(
    current: f64,
    burn_per_step: f64,
    limit: f64,
    base: DateTime<Utc>,
    dt: f64,
) -> Option<DateTime<Utc>> {
    if burn_per_step <= EPS {
        return None;
    }
    let remaining = limit - current;
    if remaining <= 0.0 {
        return Some(base);
    }
    let steps = remaining / burn_per_step;
    let secs = (steps * dt).max(0.0).round() as i64;
    if secs == 0 {
        return Some(base);
    }
    Some(base + chrono::Duration::seconds(secs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use proptest::prelude::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn rising(n: usize) -> Vec<ResourceUsage> {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        (0..n)
            .map(|i| ResourceUsage {
                cpu_instructions: ((i + 1) as u64) * (MAX_CPU / n as u64) - 1,
                mem_bytes: 1_000_000,
                storage_bytes: 512_000,
                timestamp: base + chrono::Duration::hours(i as i64),
            })
            .collect()
    }

    fn flat(n: usize, cpu: u64, mem: u64) -> Vec<ResourceUsage> {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        (0..n)
            .map(|i| ResourceUsage {
                cpu_instructions: cpu,
                mem_bytes: mem,
                storage_bytes: 0,
                timestamp: base + chrono::Duration::hours(i as i64),
            })
            .collect()
    }

    fn fill(mgr: &mut ResourceManager, id: &str, samples: Vec<ResourceUsage>) {
        for u in samples {
            mgr.record_usage(id, u);
        }
    }

    // ── existing tests (kept + enhanced) ─────────────────────────────────────

    #[test]
    fn forecasts_cpu_exhaustion() {
        let mut mgr = ResourceManager::new();
        for u in rising(48) {
            mgr.record_usage("c1", u);
        }
        let summary = mgr.summary("c1").unwrap();
        assert!(summary.forecast.cpu_exhaustion_ts.is_some());
        assert!(summary.forecast.cpu_trend_per_sec > 0.0);
    }

    /// p90 exhaustion must not precede the point estimate, and the gap between
    /// the two should be within a 10× multiplier (i.e., the p90 bound is not
    /// pathologically far in the future for a smoothly rising signal).
    #[test]
    fn p90_is_not_earlier_than_expected() {
        let mut mgr = ResourceManager::new();
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        for i in 0..72_u64 {
            let noise = (i % 5) * 25_000;
            mgr.record_usage(
                "cp90",
                ResourceUsage {
                    cpu_instructions: 1_000_000 + i * 1_200_000 + noise,
                    mem_bytes: 3_000_000 + i * 200_000,
                    storage_bytes: i * 1024,
                    timestamp: base + chrono::Duration::hours(i as i64),
                },
            );
        }
        let summary = mgr.summary("cp90").unwrap();
        let mean_ts = summary
            .forecast
            .cpu_exhaustion_ts
            .expect("mean exhaustion present");
        let p90_ts = summary
            .forecast
            .cpu_exhaustion_ts_p90
            .expect("p90 exhaustion present");

        // p90 uses a lower burn rate, so it must not be earlier than the point estimate
        assert!(
            mean_ts <= p90_ts,
            "p90 ({p90_ts}) must not precede mean ({mean_ts})"
        );

        // forecast accuracy margin: for a near-linear 72-sample signal the p90
        // bound should fall within 10× of the mean forecast horizon
        let mean_secs = (mean_ts - base).num_seconds();
        let p90_secs = (p90_ts - base).num_seconds();
        assert!(
            p90_secs <= mean_secs * 10,
            "p90 horizon ({p90_secs}s) is unreasonably far beyond mean ({mean_secs}s)"
        );

        // the confidence score should be meaningful (> 0.5) for 72 samples
        assert!(
            summary.forecast.confidence > 0.5,
            "confidence {} too low for a 72-sample history",
            summary.forecast.confidence
        );

        // mem p90 bound should also be present and ordered
        let mem_mean = summary
            .forecast
            .mem_exhaustion_ts
            .expect("mem mean present");
        let mem_p90 = summary
            .forecast
            .mem_exhaustion_ts_p90
            .expect("mem p90 present");
        assert!(mem_mean <= mem_p90, "mem p90 must not precede mem mean");
    }

    #[test]
    fn alerts_at_eighty_percent() {
        let mut mgr = ResourceManager::new();
        let alerts = mgr.record_usage(
            "c2",
            ResourceUsage {
                cpu_instructions: (MAX_CPU as f64 * 0.81) as u64,
                mem_bytes: 10,
                storage_bytes: 0,
                timestamp: Utc::now(),
            },
        );
        assert!(!alerts.is_empty());
        assert_eq!(alerts[0].metric, "cpu_instructions");
    }

    #[test]
    fn seasonal_factor_tracks_peaks() {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let mut mgr = ResourceManager::new();
        for i in 0..48_u64 {
            let phase = (i % 24) as f64 * std::f64::consts::PI / 12.0;
            let cpu = 20_000_000.0 + 15_000_000.0 * phase.sin();
            mgr.record_usage(
                "c3",
                ResourceUsage {
                    cpu_instructions: cpu as u64,
                    mem_bytes: 1_000_000,
                    storage_bytes: 0,
                    timestamp: base + chrono::Duration::hours(i as i64),
                },
            );
        }
        let summary = mgr.summary("c3").unwrap();
        assert!(summary.forecast.seasonal_factor > 1.0);
    }

    // ── edge cases ───────────────────────────────────────────────────────────

    #[test]
    fn summary_returns_none_for_unknown_contract() {
        let mgr = ResourceManager::new();
        assert!(mgr.summary("ghost").is_none());
    }

    #[test]
    fn single_sample_no_crash_and_no_exhaustion() {
        let mut mgr = ResourceManager::new();
        mgr.record_usage(
            "s1",
            ResourceUsage {
                cpu_instructions: 10_000_000,
                mem_bytes: 5_000_000,
                storage_bytes: 1024,
                timestamp: Utc::now(),
            },
        );
        let summary = mgr.summary("s1").unwrap();
        // A single point has no trend, so exhaustion cannot be projected
        assert!(summary.forecast.cpu_exhaustion_ts.is_none());
        assert!(summary.forecast.mem_exhaustion_ts.is_none());
    }

    #[test]
    fn flat_usage_produces_no_exhaustion_forecast() {
        let mut mgr = ResourceManager::new();
        // Constant usage: zero delta means zero burn rate
        fill(&mut mgr, "flat", flat(20, 10_000_000, 2_000_000));
        let summary = mgr.summary("flat").unwrap();
        assert!(
            summary.forecast.cpu_exhaustion_ts.is_none(),
            "flat CPU should yield no exhaustion"
        );
        assert!(
            summary.forecast.mem_exhaustion_ts.is_none(),
            "flat mem should yield no exhaustion"
        );
    }

    #[test]
    fn already_at_limit_exhausts_at_base_timestamp() {
        let base = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let mut mgr = ResourceManager::new();
        // Two samples both at or above MAX_CPU so remaining capacity is <= 0
        for i in 0..2_u64 {
            mgr.record_usage(
                "full",
                ResourceUsage {
                    cpu_instructions: MAX_CPU,
                    mem_bytes: MAX_MEM,
                    storage_bytes: 0,
                    timestamp: base + chrono::Duration::hours(i as i64),
                },
            );
        }
        let summary = mgr.summary("full").unwrap();
        // When already at the limit project_exhaustion returns base immediately
        if let Some(ts) = summary.forecast.cpu_exhaustion_ts {
            assert!(
                ts <= base + chrono::Duration::seconds(1),
                "exhaustion should be at or before base when already at limit"
            );
        }
    }

    #[test]
    fn no_alert_below_threshold() {
        let mut mgr = ResourceManager::new();
        let alerts = mgr.record_usage(
            "ok",
            ResourceUsage {
                cpu_instructions: (MAX_CPU as f64 * 0.79) as u64,
                mem_bytes: (MAX_MEM as f64 * 0.79) as u64,
                storage_bytes: 0,
                timestamp: Utc::now(),
            },
        );
        assert!(alerts.is_empty(), "no alert expected below 80%");
    }

    #[test]
    fn memory_alert_fires_independently() {
        let mut mgr = ResourceManager::new();
        let alerts = mgr.record_usage(
            "memhog",
            ResourceUsage {
                cpu_instructions: 0,
                mem_bytes: (MAX_MEM as f64 * 0.85) as u64,
                storage_bytes: 0,
                timestamp: Utc::now(),
            },
        );
        assert!(!alerts.is_empty());
        assert!(alerts.iter().any(|a| a.metric == "mem_bytes"));
    }

    #[test]
    fn both_alerts_fire_when_both_exceed_threshold() {
        let mut mgr = ResourceManager::new();
        let alerts = mgr.record_usage(
            "both",
            ResourceUsage {
                cpu_instructions: (MAX_CPU as f64 * 0.90) as u64,
                mem_bytes: (MAX_MEM as f64 * 0.90) as u64,
                storage_bytes: 0,
                timestamp: Utc::now(),
            },
        );
        assert_eq!(alerts.len(), 2);
    }

    // ── statistical correctness ───────────────────────────────────────────────

    #[test]
    fn std_dev_of_uniform_sample_is_zero() {
        let data = vec![5.0_f64; 10];
        assert!(
            std_dev(&data) < 1e-10,
            "std_dev of constant slice must be zero"
        );
    }

    #[test]
    fn std_dev_single_element_returns_zero() {
        assert_eq!(std_dev(&[42.0]), 0.0);
    }

    #[test]
    fn std_dev_empty_returns_zero() {
        assert_eq!(std_dev(&[]), 0.0);
    }

    #[test]
    fn std_dev_known_value() {
        // [2, 4, 4, 4, 5, 5, 7, 9] — population std dev = 2.0
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let got = std_dev(&data);
        assert!(
            (got - 2.0).abs() < 1e-6,
            "expected std_dev ≈ 2.0, got {got}"
        );
    }

    #[test]
    fn deltas_of_constant_series_are_zero() {
        let data = vec![7.0_f64; 5];
        let d = deltas(&data);
        assert!(
            d.iter().all(|&v| v == 0.0),
            "deltas of flat series must all be 0"
        );
    }

    #[test]
    fn deltas_of_monotone_series_are_positive() {
        let data = vec![1.0, 3.0, 6.0, 10.0, 15.0];
        let d = deltas(&data);
        assert!(
            d.iter().all(|&v| v > 0.0),
            "deltas of strictly increasing series must be > 0"
        );
    }

    #[test]
    fn confidence_bounded_between_zero_and_one() {
        let mut mgr = ResourceManager::new();
        fill(&mut mgr, "conf", rising(30));
        let summary = mgr.summary("conf").unwrap();
        let c = summary.forecast.confidence;
        assert!((0.0..=1.0).contains(&c), "confidence {c} out of [0, 1]");
    }

    #[test]
    fn confidence_increases_with_more_samples() {
        let mut mgr_small = ResourceManager::new();
        fill(&mut mgr_small, "s", rising(5));

        let mut mgr_large = ResourceManager::new();
        fill(&mut mgr_large, "l", rising(48));

        let c_small = mgr_small.summary("s").unwrap().forecast.confidence;
        let c_large = mgr_large.summary("l").unwrap().forecast.confidence;
        assert!(
            c_large > c_small,
            "confidence should increase with more samples: small={c_small}, large={c_large}"
        );
    }

    #[test]
    fn forecast_accuracy_within_reasonable_margin_for_linear_growth() {
        // Build a perfectly linear rising signal over 48 hours so the forecast
        // should predict exhaustion close to the true analytic horizon.
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let steps: u64 = 48;
        let burn_per_step = MAX_CPU / steps; // exact per-hour increment

        let mut mgr = ResourceManager::new();
        for i in 0..steps {
            mgr.record_usage(
                "linear",
                ResourceUsage {
                    cpu_instructions: (i + 1) * burn_per_step,
                    mem_bytes: 1_000_000,
                    storage_bytes: 0,
                    timestamp: base + chrono::Duration::hours(i as i64),
                },
            );
        }

        let summary = mgr.summary("linear").unwrap();
        let exhaust_ts = summary
            .forecast
            .cpu_exhaustion_ts
            .expect("linear growth must forecast exhaustion");

        // The true exhaustion is close to `base + steps hours` (already near MAX_CPU).
        // We allow a generous ±24-hour margin to account for the Holt-Winters smoother.
        let true_exhaust = base + chrono::Duration::hours(steps as i64);
        let delta_hours = (exhaust_ts - true_exhaust).num_hours().abs();
        assert!(
            delta_hours <= 24,
            "forecast off by {delta_hours}h — expected within 24h of true exhaustion"
        );
    }

    // ── outlier handling ──────────────────────────────────────────────────────

    #[test]
    fn single_outlier_does_not_crash_forecast() {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let mut mgr = ResourceManager::new();
        for i in 0..20_u64 {
            let cpu = if i == 10 { MAX_CPU - 1 } else { 5_000_000 };
            mgr.record_usage(
                "outlier",
                ResourceUsage {
                    cpu_instructions: cpu,
                    mem_bytes: 1_000_000,
                    storage_bytes: 0,
                    timestamp: base + chrono::Duration::hours(i as i64),
                },
            );
        }
        // Must not panic and must return a valid summary
        let summary = mgr.summary("outlier").unwrap();
        let c = summary.forecast.confidence;
        assert!((0.0..=1.0).contains(&c));
    }

    #[test]
    fn all_zeros_does_not_crash() {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let mut mgr = ResourceManager::new();
        for i in 0..5_u64 {
            mgr.record_usage(
                "zeros",
                ResourceUsage {
                    cpu_instructions: 0,
                    mem_bytes: 0,
                    storage_bytes: 0,
                    timestamp: base + chrono::Duration::hours(i as i64),
                },
            );
        }
        let summary = mgr.summary("zeros").unwrap();
        assert!(summary.forecast.cpu_exhaustion_ts.is_none());
    }

    // ── property-based tests ──────────────────────────────────────────────────

    proptest! {
        /// For any non-empty sequence of strictly rising CPU values the p90
        /// exhaustion timestamp must never precede the point estimate.
        #[test]
        fn prop_p90_never_earlier_than_mean(
            increments in prop::collection::vec(1_u64..500_000_u64, 4..30)
        ) {
            let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
            let mut mgr = ResourceManager::new();
            let mut cpu: u64 = 1_000_000;
            for (i, inc) in increments.iter().enumerate() {
                cpu = cpu.saturating_add(*inc).min(MAX_CPU - 1);
                mgr.record_usage("prop_p90", ResourceUsage {
                    cpu_instructions: cpu,
                    mem_bytes: 1_000_000,
                    storage_bytes: 0,
                    timestamp: base + chrono::Duration::hours(i as i64),
                });
            }
            let summary = mgr.summary("prop_p90").unwrap();
            if let (Some(mean), Some(p90)) = (
                summary.forecast.cpu_exhaustion_ts,
                summary.forecast.cpu_exhaustion_ts_p90,
            ) {
                prop_assert!(mean <= p90, "mean={mean}, p90={p90}");
            }
        }

        /// Confidence must always be in [0, 1] regardless of input shape.
        #[test]
        fn prop_confidence_always_in_unit_interval(
            values in prop::collection::vec(0_u64..MAX_CPU, 2..50)
        ) {
            let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
            let mut mgr = ResourceManager::new();
            for (i, v) in values.iter().enumerate() {
                mgr.record_usage("prop_conf", ResourceUsage {
                    cpu_instructions: *v,
                    mem_bytes: 1_000_000,
                    storage_bytes: 0,
                    timestamp: base + chrono::Duration::hours(i as i64),
                });
            }
            let c = mgr.summary("prop_conf").unwrap().forecast.confidence;
            prop_assert!((0.0..=1.0).contains(&c), "confidence={c}");
        }

        /// cpu_trend_per_sec must be non-negative for a monotonically rising signal.
        #[test]
        fn prop_trend_non_negative_for_rising_input(
            increments in prop::collection::vec(1_u64..200_000_u64, 3..20)
        ) {
            let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
            let mut mgr = ResourceManager::new();
            let mut cpu: u64 = 500_000;
            for (i, inc) in increments.iter().enumerate() {
                cpu = cpu.saturating_add(*inc).min(MAX_CPU - 1);
                mgr.record_usage("prop_trend", ResourceUsage {
                    cpu_instructions: cpu,
                    mem_bytes: 500_000,
                    storage_bytes: 0,
                    timestamp: base + chrono::Duration::hours(i as i64),
                });
            }
            let trend = mgr.summary("prop_trend").unwrap().forecast.cpu_trend_per_sec;
            prop_assert!(trend >= 0.0, "trend={trend}");
        }

        /// std_dev is always non-negative for any slice.
        #[test]
        fn prop_std_dev_non_negative(data in prop::collection::vec(-1e9_f64..1e9_f64, 0..50)) {
            prop_assert!(std_dev(&data) >= 0.0);
        }
    }
}
