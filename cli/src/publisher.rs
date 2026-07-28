use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct DoctorReport {
    pub config_file_exists: bool,
    pub session_valid: bool,
    pub signing_key_present: bool,
    pub registry_reachable: bool,
    pub overall_status: bool,
    pub errors: Vec<String>,
}

pub async fn doctor(api_url: &str, json: bool) -> Result<()> {
    let mut report = DoctorReport {
        config_file_exists: false,
        session_valid: false,
        signing_key_present: false,
        registry_reachable: false,
        overall_status: false,
        errors: vec![],
    };

    if !json {
        println!("\n{}", "Publisher Doctor".bold().cyan());
        println!("{}", "=".repeat(80).cyan());
        println!("Diagnosing local configuration...\n");
    }

    // 1. Check local config file existence
    let config_path = crate::config::config_file_path();
    if let Some(path) = &config_path {
        if path.exists() {
            report.config_file_exists = true;
            if !json {
                println!("  {} Config file found: {}", "✓".green(), path.display());
            }
        } else {
            report.errors.push("Config file not found. Run 'soroban-registry config set' or 'soroban-registry wizard'".into());
            if !json {
                println!("  {} Config file missing: {}", "✗".red(), path.display());
            }
        }
    } else {
        report.errors.push("Could not determine config path".into());
        if !json {
            println!("  {} Could not determine config path", "✗".red());
        }
    }

    // 2. Check Auth / Session state
    let auth = crate::config::load_auth_section().unwrap_or_default();
    if let Some(token) = &auth.session_token {
        if !token.trim().is_empty() {
            report.session_valid = true;
            if !json {
                println!("  {} Session token is present", "✓".green());
            }
        } else {
            report
                .errors
                .push("Session token is empty. Try logging in again.".into());
            if !json {
                println!("  {} Session token is empty", "✗".red());
            }
        }
    } else {
        report
            .errors
            .push("No session token found. Publisher may not be able to authenticate.".into());
        if !json {
            println!("  {} No session token found", "✗".red());
        }
    }

    // 3. Check Signing Key presence
    if let Some(key_path) = &auth.signing_key_path {
        if Path::new(key_path).exists() {
            report.signing_key_present = true;
            if !json {
                println!("  {} Signing key found at: {}", "✓".green(), key_path);
            }
        } else {
            report
                .errors
                .push(format!("Signing key file not found at: {}", key_path));
            if !json {
                println!("  {} Signing key file missing at: {}", "✗".red(), key_path);
            }
        }
    } else {
        report.errors.push(
            "No signing key configured. Please set 'signing_key_path' in your config.".into(),
        );
        if !json {
            println!("  {} No signing key configured", "✗".red());
        }
    }

    // 4. Check Registry Connectivity
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    // We check /health if available, else /api/health-monitor/status or fallback to /api/contracts
    let url = format!("{}/health", api_url);
    if !json {
        print!(
            "  {} Checking registry connectivity... ",
            "○".bright_black()
        );
    }

    match client.get(&url).send().await {
        Ok(res) if res.status().is_success() => {
            report.registry_reachable = true;
            if !json {
                println!("\r  {} Registry reachable at {}", "✓".green(), api_url);
            }
        }
        Ok(res) => {
            // Fallback for if /health is 404 or something, test base api_url
            match client.get(api_url).send().await {
                Ok(_) => {
                    // even if not 200, if we can connect, it's reachable
                    report.registry_reachable = true;
                    if !json {
                        println!(
                            "\r  {} Registry reachable at {} (health endpoint returned {})",
                            "✓".green(),
                            api_url,
                            res.status()
                        );
                    }
                }
                Err(e) => {
                    report
                        .errors
                        .push(format!("Registry connection failed: {}", e));
                    if !json {
                        println!("\r  {} Registry connection failed: {}", "✗".red(), e);
                    }
                }
            }
        }
        Err(e) => {
            report
                .errors
                .push(format!("Registry connection failed: {}", e));
            if !json {
                println!("\r  {} Registry connection failed: {}", "✗".red(), e);
            }
        }
    }

    // Determine overall status
    report.overall_status = report.config_file_exists
        && report.session_valid
        && report.signing_key_present
        && report.registry_reachable;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("\n{}", "=".repeat(80).cyan());

    if report.overall_status {
        println!(
            "\n{} All checks passed! The publisher environment is healthy.",
            "✓".green().bold()
        );
    } else {
        println!(
            "\n{} Environment has issues. See remediation steps below:\n",
            "✗".red().bold()
        );
        for err in &report.errors {
            println!("  {} {}", "•".yellow(), err);
        }
    }

    Ok(())
}
