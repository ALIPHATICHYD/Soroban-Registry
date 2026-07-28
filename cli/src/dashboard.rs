use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use tokio::process::Command;

pub struct DashboardParams {
    pub refresh_rate_ms: u64,
    pub network: Option<String>,
    pub category: Option<String>,
    pub ws_url: Option<String>,
}

pub async fn run_dashboard(params: DashboardParams) -> Result<()> {
    let entry = dashboard_entrypoint().context("Unable to locate dashboard Node entrypoint")?;

    let mut cmd = Command::new("node");
    cmd.current_dir(cli_dir());
    cmd.arg(entry);
    cmd.arg("dashboard");
    cmd.arg("--refresh-rate");
    cmd.arg(params.refresh_rate_ms.to_string());

    if let Some(network) = params.network.as_deref() {
        cmd.arg("--network");
        cmd.arg(network);
    }

    if let Some(category) = params.category.as_deref() {
        cmd.arg("--category");
        cmd.arg(category);
    }

    if let Some(ws_url) = params.ws_url.as_deref() {
        cmd.env("SOROBAN_REGISTRY_WS_URL", ws_url);
    }

    cmd.stdin(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    let status = cmd
        .status()
        .await
        .context("Failed to execute Node dashboard process")?;
    if !status.success() {
        return Err(anyhow!("Dashboard exited with status: {status}"));
    }

    Ok(())
}

fn cli_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn dashboard_entrypoint() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("SOROBAN_REGISTRY_DASHBOARD_ENTRY") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Ok(path);
        }
        return Err(anyhow!(
            "SOROBAN_REGISTRY_DASHBOARD_ENTRY points to a missing path: {}",
            path.display()
        ));
    }

    let path = cli_dir().join("dashboard").join("dist").join("index.js");
    if path.exists() {
        return Ok(path);
    }

    Err(anyhow!(
        "Dashboard entrypoint not found. Build it first: cd cli/dashboard && npm install && npm run build"
    ))
}
