#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionProfile {
    pub name: String,
    #[serde(with = "duration_nanos")]
    pub total_time: Duration,
    pub call_count: u64,
    #[serde(with = "duration_nanos")]
    pub avg_time: Duration,
    #[serde(with = "duration_nanos")]
    pub min_time: Duration,
    #[serde(with = "duration_nanos")]
    pub max_time: Duration,
    pub children: Vec<String>,
}

mod duration_nanos {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_nanos() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let nanos = u64::deserialize(deserializer)?;
        Ok(Duration::from_nanos(nanos))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileData {
    pub contract_path: String,
    pub method: Option<String>,
    pub timestamp: String,
    #[serde(with = "duration_nanos")]
    pub total_duration: Duration,
    pub functions: HashMap<String, FunctionProfile>,
    pub call_stack: Vec<CallFrame>,
    pub overhead_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallFrame {
    pub function: String,
    pub start_time: u64,
    pub end_time: u64,
    pub children: Vec<CallFrame>,
}

pub struct Profiler {
    start_time: Instant,
    call_stack: Vec<(String, Instant)>,
    function_stats: HashMap<String, Vec<Duration>>,
    call_graph: HashMap<String, Vec<String>>,
    overhead_start: Instant,
    overhead_total: Duration,
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            call_stack: Vec::new(),
            function_stats: HashMap::new(),
            call_graph: HashMap::new(),
            overhead_start: Instant::now(),
            overhead_total: Duration::ZERO,
        }
    }

    pub fn enter_function(&mut self, name: &str) {
        let overhead = self.overhead_start.elapsed();
        self.overhead_total += overhead;
        self.overhead_start = Instant::now();

        self.call_stack.push((name.to_string(), Instant::now()));

        if let Some(parent) = self.call_stack.get(self.call_stack.len().saturating_sub(2)) {
            self.call_graph
                .entry(parent.0.clone())
                .or_default()
                .push(name.to_string());
        }
    }

    pub fn exit_function(&mut self, name: &str, duration: Duration) {
        let overhead = self.overhead_start.elapsed();
        self.overhead_total += overhead;
        self.overhead_start = Instant::now();

        self.call_stack.pop();
        self.function_stats
            .entry(name.to_string())
            .or_default()
            .push(duration);
    }

    pub fn finish(self, contract_path: String, method: Option<String>) -> ProfileData {
        let total_duration = self.start_time.elapsed();
        let overhead_percent = if total_duration.as_nanos() > 0 {
            (self.overhead_total.as_nanos() as f64 / total_duration.as_nanos() as f64) * 100.0
        } else {
            0.0
        };

        let functions: HashMap<String, FunctionProfile> = self
            .function_stats
            .into_iter()
            .map(|(name, durations)| {
                let total: Duration = durations.iter().sum();
                let count = durations.len() as u64;
                let avg = if count > 0 {
                    Duration::from_nanos(total.as_nanos() as u64 / count)
                } else {
                    Duration::ZERO
                };
                let min = durations.iter().min().copied().unwrap_or(Duration::ZERO);
                let max = durations.iter().max().copied().unwrap_or(Duration::ZERO);
                let children = self.call_graph.get(&name).cloned().unwrap_or_default();

                (
                    name.clone(),
                    FunctionProfile {
                        name,
                        total_time: total,
                        call_count: count,
                        avg_time: avg,
                        min_time: min,
                        max_time: max,
                        children,
                    },
                )
            })
            .collect();

        ProfileData {
            contract_path,
            method,
            timestamp: chrono::Utc::now().to_rfc3339(),
            total_duration,
            functions,
            call_stack: vec![],
            overhead_percent,
        }
    }
}

pub fn profile_contract(contract_path: &str, method: Option<&str>) -> Result<ProfileData> {
    let path = Path::new(contract_path);
    let functions = parse_contract_functions(path)?;

    let start = Instant::now();
    let mut function_profiles = HashMap::new();

    for func in &functions {
        if let Some(m) = method {
            if func != m {
                continue;
            }
        }

        let func_start = Instant::now();
        // Simulate function execution
        let mut dummy_profiler = Profiler::new();
        simulate_execution(path, Some(func), &mut dummy_profiler)?;
        let func_duration = func_start.elapsed();

        function_profiles.insert(
            func.clone(),
            FunctionProfile {
                name: func.clone(),
                total_time: func_duration,
                call_count: 1,
                avg_time: func_duration,
                min_time: func_duration,
                max_time: func_duration,
                children: vec![],
            },
        );
    }

    let total_duration = start.elapsed();

    Ok(ProfileData {
        contract_path: contract_path.to_string(),
        method: method.map(|s| s.to_string()),
        timestamp: chrono::Utc::now().to_rfc3339(),
        total_duration,
        functions: function_profiles,
        call_stack: vec![],
        overhead_percent: 0.0,
    })
}

pub fn load_baseline(baseline_path: &str) -> Result<ProfileData> {
    let content = fs::read_to_string(baseline_path)
        .with_context(|| format!("Failed to read baseline file: {}", baseline_path))?;
    serde_json::from_str(&content).with_context(|| "Failed to parse baseline profile data")
}

pub fn parse_contract_functions(contract_path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(contract_path)
        .with_context(|| format!("Failed to read contract: {}", contract_path.display()))?;

    let mut functions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for line in lines.iter() {
        if line.trim().starts_with("pub fn ") || line.trim().starts_with("fn ") {
            if let Some(name_start) = line.find("fn ") {
                let after_fn = &line[name_start + 3..];
                if let Some(name_end) = after_fn.find('(') {
                    let func_name = after_fn[..name_end].trim();
                    if !func_name.is_empty() {
                        functions.push(func_name.to_string());
                    }
                }
            }
        }
    }

    Ok(functions)
}

pub fn simulate_execution(
    contract_path: &Path,
    method: Option<&str>,
    profiler: &mut Profiler,
) -> Result<()> {
    let functions = parse_contract_functions(contract_path)?;

    let target_method =
        method.unwrap_or_else(|| functions.first().map(|s| s.as_str()).unwrap_or("main"));

    if !functions.contains(&target_method.to_string()) {
        anyhow::bail!("Method '{}' not found in contract", target_method);
    }

    profiler.enter_function(target_method);
    let method_start = Instant::now();

    for func in &functions {
        if func == target_method {
            continue;
        }

        profiler.enter_function(func);
        let func_start = Instant::now();

        std::thread::sleep(Duration::from_micros(100));

        let func_duration = func_start.elapsed();
        profiler.exit_function(func, func_duration);
    }

    let method_duration = method_start.elapsed();
    profiler.exit_function(target_method, method_duration);

    Ok(())
}

// original/formatting-heavy implementation (kept for benchmarking)
pub fn generate_flame_graph_old(profile: &ProfileData, output_path: &Path) -> Result<()> {
    let mut svg = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" version="1.1" width="1200" height="800">
<style>
.frame { font-family: monospace; font-size: 12px; }
.frame rect { stroke: #000; stroke-width: 1px; }
.hot { fill: #ff6b6b; }
.warm { fill: #ffa500; }
.cool { fill: #4ecdc4; }
</style>
"#,
    );

    let max_time = profile
        .functions
        .values()
        .map(|f| f.total_time.as_nanos())
        .max()
        .unwrap_or(1) as f64;

    let mut y = 20.0;
    let bar_height = 20.0;
    let width = 1200.0;

    let mut sorted_functions: Vec<_> = profile.functions.values().collect();
    sorted_functions.sort_by(|a, b| b.total_time.cmp(&a.total_time));

    for func in sorted_functions.iter().take(30) {
        let time_ratio = func.total_time.as_nanos() as f64 / max_time;
        let bar_width = width * time_ratio.min(1.0);

        let color_class = if time_ratio > 0.7 {
            "hot"
        } else if time_ratio > 0.3 {
            "warm"
        } else {
            "cool"
        };

        svg.push_str(&format!(
            "<g class=\"frame\">\n<rect x=\"0\" y=\"{}\" width=\"{}\" height=\"{}\" class=\"{}\"/>\n<text x=\"5\" y=\"{}\" fill=\"white\">{}</text>\n<text x=\"{}\" y=\"{}\" fill=\"white\" text-anchor=\"end\">{:.2}ms</text>\n</g>\n",
            y,
            bar_width,
            bar_height,
            color_class,
            y + 15.0,
            func.name,
            bar_width - 5.0,
            y + 15.0,
            func.total_time.as_secs_f64() * 1000.0
        ));

        y += bar_height + 2.0;
    }

    svg.push_str("</svg>");

    fs::write(output_path, svg)
        .with_context(|| format!("Failed to write flame graph: {}", output_path.display()))?;

    Ok(())
}

// new builder/template implementation (efficient string building)
pub fn generate_flame_graph(profile: &ProfileData, output_path: &Path) -> Result<()> {
    let mut svg = String::with_capacity(16 * 1024);

    svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" width=\"1200\" height=\"800\">\n");
    svg.push_str("<style>\n");
    svg.push_str(".frame { font-family: monospace; font-size: 12px; }\n");
    svg.push_str(".frame rect { stroke: #000; stroke-width: 1px; }\n");
    svg.push_str(".hot { fill: #ff6b6b; }\n");
    svg.push_str(".warm { fill: #ffa500; }\n");
    svg.push_str(".cool { fill: #4ecdc4; }\n");
    svg.push_str("</style>\n");

    let max_time = profile
        .functions
        .values()
        .map(|f| f.total_time.as_nanos())
        .max()
        .unwrap_or(1) as f64;

    let mut y = 20.0f64;
    let bar_height = 20.0f64;
    let width = 1200.0f64;

    let mut sorted_functions: Vec<_> = profile.functions.values().collect();
    sorted_functions.sort_by(|a, b| b.total_time.cmp(&a.total_time));

    for func in sorted_functions.iter().take(30) {
        let time_ratio = func.total_time.as_nanos() as f64 / max_time;
        let bar_width = width * time_ratio.min(1.0);

        let color_class = if time_ratio > 0.7 {
            "hot"
        } else if time_ratio > 0.3 {
            "warm"
        } else {
            "cool"
        };

        svg.push_str("<g class=\"frame\">\n");
        svg.push_str("<rect x=\"0\" y=\"");
        svg.push_str(&format_float(y));
        svg.push_str("\" width=\"");
        svg.push_str(&format_float(bar_width));
        svg.push_str("\" height=\"");
        svg.push_str(&format_float(bar_height));
        svg.push_str("\" class=\"");
        svg.push_str(color_class);
        svg.push_str("\"/>\n");

        svg.push_str("<text x=\"5\" y=\"");
        svg.push_str(&format_float(y + 15.0));
        svg.push_str("\" fill=\"white\">");
        svg.push_str(&func.name);
        svg.push_str("</text>\n");

        svg.push_str("<text x=\"");
        svg.push_str(&format_float(bar_width - 5.0));
        svg.push_str("\" y=\"");
        svg.push_str(&format_float(y + 15.0));
        svg.push_str("\" fill=\"white\" text-anchor=\"end\">");
        svg.push_str(&format_float(func.total_time.as_secs_f64() * 1000.0));
        svg.push_str("ms</text>\n");

        svg.push_str("</g>\n");

        y += bar_height + 2.0;
    }

    svg.push_str("</svg>");

    fs::write(output_path, svg)
        .with_context(|| format!("Failed to write flame graph: {}", output_path.display()))?;

    Ok(())
}

// helper used by builder impl
fn format_float(v: f64) -> String {
    let mut s = String::new();
    s.reserve(16);
    let _ = write!(&mut s, "{:.2}", v);
    s
}

pub fn compare_profiles(profile1: &ProfileData, profile2: &ProfileData) -> Vec<ComparisonResult> {
    let mut results = Vec::new();

    let all_functions: std::collections::HashSet<_> = profile1
        .functions
        .keys()
        .chain(profile2.functions.keys())
        .collect();

    for func_name in all_functions {
        let func1 = profile1.functions.get(func_name);
        let func2 = profile2.functions.get(func_name);

        match (func1, func2) {
            (Some(f1), Some(f2)) => {
                let time_diff = f2.total_time.as_nanos() as i64 - f1.total_time.as_nanos() as i64;
                let time_diff_percent = if f1.total_time.as_nanos() > 0 {
                    (time_diff as f64 / f1.total_time.as_nanos() as f64) * 100.0
                } else {
                    0.0
                };

                results.push(ComparisonResult {
                    function: func_name.clone(),
                    status: if time_diff > 0 {
                        "slower"
                    } else if time_diff < 0 {
                        "faster"
                    } else {
                        "unchanged"
                    }
                    .to_string(),
                    time_diff_ns: time_diff,
                    time_diff_percent,
                    baseline_time: f1.total_time,
                    current_time: f2.total_time,
                });
            }
            (Some(f1), None) => {
                results.push(ComparisonResult {
                    function: func_name.clone(),
                    status: "removed".to_string(),
                    time_diff_ns: -(f1.total_time.as_nanos() as i64),
                    time_diff_percent: -100.0,
                    baseline_time: f1.total_time,
                    current_time: Duration::ZERO,
                });
            }
            (None, Some(f2)) => {
                results.push(ComparisonResult {
                    function: func_name.clone(),
                    status: "added".to_string(),
                    time_diff_ns: f2.total_time.as_nanos() as i64,
                    time_diff_percent: 100.0,
                    baseline_time: Duration::ZERO,
                    current_time: f2.total_time,
                });
            }
            (None, None) => {}
        }
    }

    results.sort_by(|a, b| b.time_diff_ns.abs().cmp(&a.time_diff_ns.abs()));
    results
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub function: String,
    pub status: String,
    pub time_diff_ns: i64,
    pub time_diff_percent: f64,
    #[serde(with = "duration_nanos")]
    pub baseline_time: Duration,
    #[serde(with = "duration_nanos")]
    pub current_time: Duration,
}

pub fn generate_recommendations(profile: &ProfileData) -> Vec<String> {
    let mut recommendations = Vec::new();

    let hot_functions: Vec<_> = profile
        .functions
        .values()
        .filter(|f| f.total_time.as_nanos() as f64 > profile.total_duration.as_nanos() as f64 * 0.1)
        .collect();

    if !hot_functions.is_empty() {
        recommendations.push(format!(
            "Optimize hot functions: {}",
            hot_functions
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let high_call_count: Vec<_> = profile
        .functions
        .values()
        .filter(|f| f.call_count > 1000)
        .collect();

    if !high_call_count.is_empty() {
        recommendations.push(format!(
            "Consider caching for frequently called functions: {}",
            high_call_count
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let high_variance: Vec<_> = profile
        .functions
        .values()
        .filter(|f| {
            let variance = (f.max_time.as_nanos() as f64 - f.min_time.as_nanos() as f64)
                / f.avg_time.as_nanos().max(1) as f64;
            variance > 2.0
        })
        .collect();

    if !high_variance.is_empty() {
        recommendations.push(format!(
            "Investigate high variance in execution time: {}",
            high_variance
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if profile.overhead_percent > 5.0 {
        recommendations.push(format!(
            "Profiling overhead ({:.2}%) exceeds 5% threshold. Consider reducing instrumentation.",
            profile.overhead_percent
        ));
    }

    if recommendations.is_empty() {
        recommendations.push("No optimization recommendations at this time.".to_string());
    }

    recommendations
}
