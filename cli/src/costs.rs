use crate::net::RequestBuilderExt;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct CostEstimateRequest {
    method_name: String,
    invocations: Option<i64>,
    storage_growth_kb: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CostEstimate {
    method_name: String,
    gas_cost: i64,
    storage_cost: i64,
    bandwidth_cost: i64,
    total_stroops: i64,
    total_xlm: f64,
    invocations: i64,
}

#[derive(Debug, Deserialize)]
struct CostOptimization {
    current_cost: i64,
    optimized_cost: i64,
    savings_percent: f64,
    suggestions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CostForecast {
    daily_cost_xlm: f64,
    monthly_cost_xlm: f64,
    yearly_cost_xlm: f64,
    usage_pattern: String,
}

pub async fn estimate_costs(
    api_url: &str,
    contract_id: &str,
    method: &str,
    invocations: Option<i64>,
    storage_kb: Option<i64>,
    optimize: bool,
    forecast: bool,
) -> Result<()> {
    let client = crate::net::client();

    let request = CostEstimateRequest {
        method_name: method.to_string(),
        invocations,
        storage_growth_kb: storage_kb,
    };

    // Get base estimate
    let estimate: CostEstimate = client
        .post(format!("{}/api/contracts/{}/cost-estimate", api_url, contract_id))
        .json(&request)
        .send_with_retry().await?
        .json()
        .await?;

    println!("╔═══════════════════════════════════════════════════════╗");
    println!("║           CONTRACT COST ESTIMATION                   ║");
    println!("╚═══════════════════════════════════════════════════════╝");
    println!();
    println!("Method: {}", estimate.method_name);
    println!("Invocations: {}", estimate.invocations);
    println!();
    println!("Cost Breakdown:");
    println!("  Gas Cost:       {:>12} stroops", estimate.gas_cost);
    println!("  Storage Cost:   {:>12} stroops", estimate.storage_cost);
    println!("  Bandwidth Cost: {:>12} stroops", estimate.bandwidth_cost);
    println!("  ─────────────────────────────────────");
    println!("  Total:          {:>12} stroops", estimate.total_stroops);
    println!("  Total:          {:>12.6} XLM", estimate.total_xlm);
    println!();

    // Show optimization suggestions
    if optimize {
        let optimization: CostOptimization = client
            .post(format!("{}/api/contracts/{}/cost-estimate/optimize", api_url, contract_id))
            .json(&estimate)
            .send_with_retry().await?
            .json()
            .await?;

        println!("╔═══════════════════════════════════════════════════════╗");
        println!("║           OPTIMIZATION SUGGESTIONS                   ║");
        println!("╚═══════════════════════════════════════════════════════╝");
        println!();
        println!("Current Cost:   {} stroops", optimization.current_cost);
        println!("Optimized Cost: {} stroops", optimization.optimized_cost);
        println!("Savings:        {:.1}%", optimization.savings_percent);
        println!();
        println!("Suggestions:");
        for (i, suggestion) in optimization.suggestions.iter().enumerate() {
            println!("  {}. {}", i + 1, suggestion);
        }
        println!();
    }

    // Show forecast
    if forecast {
        let forecast_data: CostForecast = client
            .post(format!("{}/api/contracts/{}/cost-estimate/forecast", api_url, contract_id))
            .json(&request)
            .send_with_retry().await?
            .json()
            .await?;

        println!("╔═══════════════════════════════════════════════════════╗");
        println!("║           COST FORECAST                              ║");
        println!("╚═══════════════════════════════════════════════════════╝");
        println!();
        println!("Usage Pattern: {}", forecast_data.usage_pattern);
        println!();
        println!("Projected Costs:");
        println!("  Daily:   {:>10.6} XLM", forecast_data.daily_cost_xlm);
        println!("  Monthly: {:>10.6} XLM", forecast_data.monthly_cost_xlm);
        println!("  Yearly:  {:>10.6} XLM", forecast_data.yearly_cost_xlm);
        println!();
    }

    Ok(())
}
