//! Publisher environment diagnostics (`soroban-registry publisher doctor`).
//!
//! Reports whether the local machine is ready to publish: a config file, a
//! stored session, a signing secret, and a reachable registry. Every check is
//! read-only — the command must never repair, refresh, or overwrite state,
//! because an operator running it is trying to see what is actually there.

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

/// Connectivity probe timeout. Shorter than [`crate::net`]'s 30s default: a
/// doctor command reports a down registry, it does not wait one out.
const PROBE_TIMEOUT_SECS: u64 = 5;

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub config_file_exists: bool,
    pub session_present: bool,
    pub session_valid: bool,
    pub signing_key_present: bool,
    pub registry_reachable: bool,
    pub overall_status: bool,
    pub errors: Vec<String>,
}

pub async fn doctor(api_url: &str, json: bool) -> Result<()> {
    let mut report = DoctorReport {
        config_file_exists: false,
        session_present: false,
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

    check_config_file(&mut report, json);
    check_session(&mut report, json).await;
    check_registry(&mut report, api_url, json).await;

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
            "\n{} All checks passed. The publisher environment is healthy.",
            "[OK]".green().bold()
        );
    } else {
        println!(
            "\n{} Environment has issues. See remediation steps below:\n",
            "[ERR]".red().bold()
        );
        for err in &report.errors {
            println!("  {} {}", "-".yellow(), err);
        }
    }

    Ok(())
}

fn check_config_file(report: &mut DoctorReport, json: bool) {
    match crate::config::config_file_path() {
        Some(path) if path.exists() => {
            report.config_file_exists = true;
            if !json {
                println!("  {} Config file found: {}", "[OK]".green(), path.display());
            }
        }
        Some(path) => {
            report.errors.push(format!(
                "Config file not found at {}. Run 'soroban-registry wizard' or 'soroban-registry config edit'.",
                path.display()
            ));
            if !json {
                println!(
                    "  {} Config file missing: {}",
                    "[ERR]".red(),
                    path.display()
                );
            }
        }
        None => {
            report
                .errors
                .push("Could not determine the config path (no home directory).".into());
            if !json {
                println!("  {} Could not determine config path", "[ERR]".red());
            }
        }
    }
}

async fn check_session(report: &mut DoctorReport, json: bool) {
    let session = match crate::auth::session_diagnostics().await {
        Ok(session) => session,
        Err(err) => {
            report
                .errors
                .push(format!("Could not read the stored session: {err}"));
            if !json {
                println!("  {} Could not read stored session: {}", "[ERR]".red(), err);
            }
            return;
        }
    };

    let Some(session) = session else {
        report
            .errors
            .push("No session found. Run 'soroban-registry auth login' before publishing.".into());
        if !json {
            println!("  {} No session found", "[ERR]".red());
        }
        return;
    };

    report.session_present = true;

    if !session.token_present {
        report.errors.push(
            "Stored session has an empty access token. Run 'soroban-registry auth login' again."
                .into(),
        );
        if !json {
            println!("  {} Stored access token is empty", "[ERR]".red());
        }
    } else if session.expired {
        report.errors.push(
            "Access token has expired. Run 'soroban-registry auth login' to re-authenticate."
                .into(),
        );
        if !json {
            println!("  {} Access token has expired", "[ERR]".red());
        }
    } else {
        report.session_valid = true;
        if !json {
            println!(
                "  {} Session valid ({} as {})",
                "[OK]".green(),
                session.method,
                session.identity
            );
        }
    }

    if session.signing_secret_present {
        report.signing_key_present = true;
        if !json {
            println!(
                "  {} Signing key available for this session",
                "[OK]".green()
            );
        }
    } else {
        report.errors.push(
            "No signing key stored for this session. Log in with 'soroban-registry auth login --method stellar' so publishes can be signed.".into(),
        );
        if !json {
            println!("  {} No signing key stored for this session", "[ERR]".red());
        }
    }

    if session.legacy_storage && !json {
        println!(
            "  {} Credentials are in the legacy config file, not the OS keychain",
            "[WARN]".yellow()
        );
    }
}

async fn check_registry(report: &mut DoctorReport, api_url: &str, json: bool) {
    let base = api_url.trim_end_matches('/');
    let url = format!("{base}/health");

    if !json {
        print!(
            "  {} Checking registry connectivity... ",
            "[~]".bright_black()
        );
    }

    // Plain send, not `send_with_retry`: retries would triple the wait on a
    // registry that is simply down, and would attach an auth header the
    // connectivity probe does not need.
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(PROBE_TIMEOUT_SECS))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            report
                .errors
                .push(format!("Could not build an HTTP client: {err}"));
            if !json {
                println!("\r  {} Could not build HTTP client: {}", "[ERR]".red(), err);
            }
            return;
        }
    };

    match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            report.registry_reachable = true;
            if !json {
                println!("\r  {} Registry reachable at {}", "[OK]".green(), base);
            }
        }
        Ok(response) => {
            // A non-2xx from /health still proves the host answered, but it is
            // not a healthy registry, so it is reported as a failure with the
            // status attached rather than being smoothed over.
            report.errors.push(format!(
                "Registry at {base} answered {} on /health. It is reachable but not healthy.",
                response.status()
            ));
            if !json {
                println!(
                    "\r  {} Registry at {} returned {} on /health",
                    "[ERR]".red(),
                    base,
                    response.status()
                );
            }
        }
        Err(err) => {
            report
                .errors
                .push(format!("Registry connection to {base} failed: {err}"));
            if !json {
                println!("\r  {} Registry connection failed: {}", "[ERR]".red(), err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> DoctorReport {
        DoctorReport {
            config_file_exists: true,
            session_present: true,
            session_valid: true,
            signing_key_present: true,
            registry_reachable: true,
            overall_status: false,
            errors: vec![],
        }
    }

    #[test]
    fn report_serializes_every_check() {
        let value = serde_json::to_value(report()).unwrap();
        for key in [
            "config_file_exists",
            "session_present",
            "session_valid",
            "signing_key_present",
            "registry_reachable",
            "overall_status",
            "errors",
        ] {
            assert!(value.get(key).is_some(), "missing field {key}");
        }
    }
}
