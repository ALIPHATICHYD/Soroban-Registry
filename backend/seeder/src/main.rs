#![allow(dead_code, unused)]

mod data;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use shared::logging::{init_logging, LogConfig};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::fs;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "seeder")]
#[command(about = "Database seeding utility for Soroban Registry")]
struct Args {
    #[arg(long, default_value = "50")]
    count: usize,

    #[arg(long)]
    seed: Option<u64>,

    #[arg(long)]
    data_file: Option<String>,

    #[arg(long, default_value = "postgresql://localhost/soroban_registry")]
    database_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env if present
    dotenv::dotenv().ok();

    // Initialize structured logging
    init_logging(LogConfig::from_env_with_service("soroban-registry-seeder"));

    let mut args = Args::parse();

    // Prioritize DATABASE_URL from environment if not provided via CLI
    if args.database_url == "postgresql://localhost/soroban_registry" {
        if let Ok(env_url) = std::env::var("DATABASE_URL") {
            args.database_url = env_url;
        }
    }

    println!("{}", "=".repeat(80).cyan());
    println!("{}", "Soroban Registry Database Seeder".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&args.database_url)
        .await
        .context("Failed to connect to database")?;

    let skip_migrations = std::env::var("SKIP_MIGRATIONS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    if !skip_migrations {
        sqlx::migrate!("../../database/migrations")
            .run(&pool)
            .await
            .context("Failed to run migrations")?;
    } else {
        println!("{} Skipping migrations (SKIP_MIGRATIONS=true)", "ℹ".blue());
    }

    let mut rng: rand::rngs::StdRng = if let Some(seed) = args.seed {
        println!("{} Using seed: {}", "ℹ".blue(), seed);
        rand::SeedableRng::seed_from_u64(seed)
    } else {
        rand::SeedableRng::from_entropy()
    };

    let start_time = Instant::now();

    let custom_data = if let Some(ref file_path) = args.data_file {
        println!("{} Loading custom data from: {}", "ℹ".blue(), file_path);
        Some(load_custom_data(file_path)?)
    } else {
        None
    };

    let publisher_count = (args.count as f64 * 0.2).ceil() as usize;
    let publishers =
        data::create_publishers(&pool, publisher_count, &mut rng, custom_data.as_ref()).await?;
    println!("{} Created {} publishers", "✓".green(), publishers.len());

    let contracts = data::create_contracts(
        &pool,
        args.count,
        &publishers,
        &mut rng,
        custom_data.as_ref(),
    )
    .await?;
    println!("{} Created {} contracts", "✓".green(), contracts.len());

    let versions = data::create_versions(&pool, &contracts, &mut rng).await?;
    println!("{} Created {} contract versions", "✓".green(), versions);

    let verifications = data::create_verifications(&pool, &contracts, &mut rng).await?;
    println!("{} Created {} verifications", "✓".green(), verifications);

    let elapsed = start_time.elapsed();
    println!();
    println!("{}", "=".repeat(80).cyan());
    println!(
        "{} Seeding completed in {:.2}s",
        "✓".green().bold(),
        elapsed.as_secs_f64()
    );
    println!("{}", "=".repeat(80).cyan());

    Ok(())
}

fn load_custom_data(file_path: &str) -> Result<HashMap<String, serde_json::Value>> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read data file: {}", file_path))?;
    let data: HashMap<String, serde_json::Value> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON: {}", file_path))?;
    Ok(data)
}
