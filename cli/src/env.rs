//! env.rs — `soroban-registry env` (#843)
//!
//! Manages environment variable sets for different deployment environments
//! (dev, staging, production, or any custom name). Variables are stored
//! locally in `~/.soroban-registry/environments.json` and can be exported
//! as shell-sourceable files, JSON, or .env format.
//!
//! Environments are isolated from each other. When listing or exporting,
//! the active environment's variables are merged with global registry
//! config values (api_key, default_network) as fallback defaults.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;

// ── Storage types ─────────────────────────────────────────────────────────────

/// A single environment's variable set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Environment {
    /// Ordered map of variable name → value.
    pub vars: BTreeMap<String, String>,
}

/// Root of the environments storage file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environments {
    /// Name of the currently active environment.
    pub active: String,
    /// All named environments.
    pub environments: BTreeMap<String, Environment>,
}

impl Default for Environments {
    fn default() -> Self {
        let mut envs: BTreeMap<String, Environment> = BTreeMap::new();
        // Ship three starter environments.
        envs.insert("dev".to_string(), dev_template());
        envs.insert("staging".to_string(), staging_template());
        envs.insert("production".to_string(), production_template());
        Self {
            active: "dev".to_string(),
            environments: envs,
        }
    }
}

// ── Built-in templates ────────────────────────────────────────────────────────

fn dev_template() -> Environment {
    let mut vars = BTreeMap::new();
    vars.insert(
        "SOROBAN_REGISTRY_API_URL".to_string(),
        "http://localhost:3001".to_string(),
    );
    vars.insert(
        "SOROBAN_REGISTRY_NETWORK".to_string(),
        "testnet".to_string(),
    );
    vars.insert(
        "SOROBAN_REGISTRY_LOG_LEVEL".to_string(),
        "debug".to_string(),
    );
    Environment { vars }
}

fn staging_template() -> Environment {
    let mut vars = BTreeMap::new();
    vars.insert(
        "SOROBAN_REGISTRY_API_URL".to_string(),
        "https://staging-registry.example.com".to_string(),
    );
    vars.insert(
        "SOROBAN_REGISTRY_NETWORK".to_string(),
        "testnet".to_string(),
    );
    vars.insert("SOROBAN_REGISTRY_LOG_LEVEL".to_string(), "info".to_string());
    Environment { vars }
}

fn production_template() -> Environment {
    let mut vars = BTreeMap::new();
    vars.insert(
        "SOROBAN_REGISTRY_API_URL".to_string(),
        "https://registry.example.com".to_string(),
    );
    vars.insert(
        "SOROBAN_REGISTRY_NETWORK".to_string(),
        "mainnet".to_string(),
    );
    vars.insert("SOROBAN_REGISTRY_LOG_LEVEL".to_string(), "warn".to_string());
    Environment { vars }
}

#[allow(dead_code)]
pub fn template_by_name(name: &str) -> Option<Environment> {
    match name {
        "dev" | "development" => Some(dev_template()),
        "staging" | "stage" => Some(staging_template()),
        "production" | "prod" => Some(production_template()),
        _ => None,
    }
}

// ── Storage helpers ───────────────────────────────────────────────────────────

fn storage_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not resolve home directory")?;
    Ok(home.join(".soroban-registry").join("environments.json"))
}

fn load_store() -> Result<Environments> {
    let path = storage_path()?;
    if !path.exists() {
        return Ok(Environments::default());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read environments file: {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse environments file: {}", path.display()))
}

fn save_store(store: &Environments) -> Result<()> {
    let path = storage_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config dir: {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(store)?;
    #[cfg(unix)]
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .with_context(|| format!("Failed to open environments file: {}", path.display()))?;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        file.write_all(content.as_bytes())
            .with_context(|| format!("Failed to write environments file: {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&path, content)
            .with_context(|| format!("Failed to write environments file: {}", path.display()))?;
    }
    Ok(())
}

/// Resolve the environment name: use the provided name or fall back to active.
fn resolve_env_name<'a>(store: &'a Environments, env: Option<&'a str>) -> &'a str {
    env.unwrap_or(&store.active)
}

// ── Validation ────────────────────────────────────────────────────────────────

/// Validate that a variable name is a legal shell identifier.
fn validate_var_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Variable name cannot be empty.");
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        anyhow::bail!(
            "Invalid variable name '{}': must start with a letter or underscore.",
            name
        );
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        anyhow::bail!(
            "Invalid variable name '{}': only letters, digits, and underscores are allowed.",
            name
        );
    }
    Ok(())
}

/// Validate that an environment name is a safe identifier.
fn validate_env_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Environment name cannot be empty.");
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphanumeric() && first != '_' {
        anyhow::bail!(
            "Invalid environment name '{}': must start with a letter, digit, or underscore.",
            name
        );
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        anyhow::bail!(
            "Invalid environment name '{}': only letters, digits, hyphens, and underscores are allowed.",
            name
        );
    }
    Ok(())
}

/// Escape a value for safe inclusion in a shell `export` statement.
fn shell_escape(value: &str) -> String {
    // Wrap in single quotes; escape any embedded single quotes.
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Escape a value for safe inclusion in dotenv `KEY=VALUE` format.
fn dotenv_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for c in value.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(c),
        }
    }
    escaped.push('"');
    escaped
}

fn masked_value(value: &str) -> String {
    format!("[hidden] ({} chars)", value.chars().count())
}

// ── Global config merge ───────────────────────────────────────────────────────

/// Build a merged variable map: global registry config values as defaults,
/// overridden by the named environment's own vars.
fn merged_vars(env: &Environment) -> BTreeMap<String, String> {
    let global = crate::user_config::load().unwrap_or_default();

    let mut merged: BTreeMap<String, String> = BTreeMap::new();

    // Inject global config as low-priority fallbacks.
    if let Some(key) = global.api_key {
        merged.insert("SOROBAN_REGISTRY_API_KEY".to_string(), key);
    }
    merged.insert(
        "SOROBAN_REGISTRY_NETWORK".to_string(),
        global.default_network,
    );

    // Environment-specific vars take precedence.
    merged.extend(env.vars.clone());
    merged
}

// ── Public entry points ───────────────────────────────────────────────────────

/// `soroban-registry env set <NAME> <VALUE> [--env <env>] [--show-value]`
pub fn set_var(name: &str, value: &str, env: Option<&str>, show_value: bool) -> Result<()> {
    validate_var_name(name)?;

    let mut store = load_store()?;
    let env_name = resolve_env_name(&store, env).to_string();

    if let Some(e) = env.filter(|e| !store.environments.contains_key(*e)) {
        validate_env_name(e)?;
    }

    let environment = store.environments.entry(env_name.clone()).or_default();

    let overwriting = environment.vars.contains_key(name);
    environment.vars.insert(name.to_string(), value.to_string());
    save_store(&store)?;

    println!();
    if overwriting {
        println!(
            "  {} {} updated in {}",
            "↺".cyan().bold(),
            name.bold(),
            env_name.bright_blue()
        );
    } else {
        println!(
            "  {} {} set in {}",
            "✔".green().bold(),
            name.bold(),
            env_name.bright_blue()
        );
    }
    if show_value {
        println!("     {} {}", "Value:".bold(), value.dimmed());
    } else {
        println!(
            "     {} {}",
            "Value (masked):".bold(),
            masked_value(value).dimmed()
        );
    }
    println!();
    Ok(())
}

/// `soroban-registry env get <NAME> [--env <env>] [--json]`
pub fn get_var(name: &str, env: Option<&str>, json: bool) -> Result<()> {
    validate_var_name(name)?;

    let store = load_store()?;
    let env_name = resolve_env_name(&store, env);

    let environment = store.environments.get(env_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Environment '{}' does not exist. Create it with: soroban-registry env set <NAME> <VALUE> --env {}",
            env_name,
            env_name
        )
    })?;

    let merged = merged_vars(environment);

    match merged.get(name) {
        Some(value) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "environment": env_name,
                        "name": name,
                        "value": value,
                        "source": if environment.vars.contains_key(name) { "environment" } else { "global" }
                    }))?
                );
            } else {
                println!("{}", value);
            }
        }
        None => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "environment": env_name,
                        "name": name,
                        "value": null
                    }))?
                );
            } else {
                anyhow::bail!("Variable '{}' not set in environment '{}'.", name, env_name);
            }
        }
    }

    Ok(())
}

/// `soroban-registry env list [--env <env>] [--all] [--merged] [--json]`
pub fn list_vars(env: Option<&str>, all: bool, merged: bool, json: bool) -> Result<()> {
    let store = load_store()?;

    if all {
        // Print every environment.
        if json {
            println!("{}", serde_json::to_string_pretty(&store.environments)?);
        } else {
            print_all_envs(&store);
        }
        return Ok(());
    }

    let env_name = resolve_env_name(&store, env);
    let environment = store
        .environments
        .get(env_name)
        .ok_or_else(|| anyhow::anyhow!("Environment '{}' does not exist.", env_name))?;

    let vars = if merged {
        merged_vars(environment)
    } else {
        environment.vars.clone()
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "environment": env_name,
                "active": store.active == env_name,
                "vars": vars
            }))?
        );
    } else {
        print_env_table(env_name, &vars, store.active == env_name, merged);
    }

    Ok(())
}

/// `soroban-registry env copy --from <src> --to <dst> [--overwrite]`
pub fn copy_env(from: &str, to: &str, overwrite: bool) -> Result<()> {
    validate_env_name(from)?;
    validate_env_name(to)?;

    let mut store = load_store()?;

    let source = store
        .environments
        .get(from)
        .ok_or_else(|| anyhow::anyhow!("Source environment '{}' does not exist.", from))?
        .clone();

    if store.environments.contains_key(to) && !overwrite {
        anyhow::bail!(
            "Environment '{}' already exists. Use --overwrite to replace it.",
            to
        );
    }

    let count = source.vars.len();
    store.environments.insert(to.to_string(), source);
    save_store(&store)?;

    println!();
    println!(
        "  {} Copied {} variable{} from {} to {}",
        "✔".green().bold(),
        count,
        if count == 1 { "" } else { "s" },
        from.bright_blue(),
        to.bright_blue()
    );
    println!();
    Ok(())
}

/// `soroban-registry env delete <NAME> [--env <env>]`
pub fn delete_var(name: &str, env: Option<&str>) -> Result<()> {
    validate_var_name(name)?;

    let mut store = load_store()?;
    let env_name = resolve_env_name(&store, env).to_string();

    let environment = store
        .environments
        .get_mut(&env_name)
        .ok_or_else(|| anyhow::anyhow!("Environment '{}' does not exist.", env_name))?;

    if environment.vars.remove(name).is_none() {
        anyhow::bail!("Variable '{}' not set in environment '{}'.", name, env_name);
    }

    save_store(&store)?;

    println!();
    println!(
        "  {} {} removed from {}",
        "✔".green().bold(),
        name.bold(),
        env_name.bright_blue()
    );
    println!();
    Ok(())
}

/// `soroban-registry env export [--env <env>] [--format shell|json|dotenv] [--merged]`
pub fn export_env(env: Option<&str>, format: &str, merged: bool) -> Result<()> {
    let store = load_store()?;
    let env_name = resolve_env_name(&store, env);

    let environment = store
        .environments
        .get(env_name)
        .ok_or_else(|| anyhow::anyhow!("Environment '{}' does not exist.", env_name))?;

    let vars = if merged {
        merged_vars(environment)
    } else {
        environment.vars.clone()
    };

    match format {
        "json" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "environment": env_name,
                    "vars": vars
                }))?
            );
        }
        "dotenv" | ".env" => {
            println!("# soroban-registry env: {}", env_name);
            for (k, v) in &vars {
                // .env format: KEY="escaped VALUE"
                println!("{}={}", k, dotenv_escape(v));
            }
        }
        _ => {
            // Default: shell (bash/zsh sourceable)
            println!(
                "# soroban-registry env: {} — source this file to activate",
                env_name
            );
            println!("# Generated: {}", chrono::Utc::now().to_rfc3339());
            println!();
            for (k, v) in &vars {
                println!("export {}={}", k, shell_escape(v));
            }
        }
    }

    Ok(())
}

/// `soroban-registry env switch <env>`
pub fn switch_env(env_name: &str) -> Result<()> {
    validate_env_name(env_name)?;

    let mut store = load_store()?;

    if !store.environments.contains_key(env_name) {
        anyhow::bail!(
            "Environment '{}' does not exist. Create it first with:\n\
             soroban-registry env set <NAME> <VALUE> --env {}",
            env_name,
            env_name
        );
    }

    let previous = store.active.clone();
    store.active = env_name.to_string();
    save_store(&store)?;

    println!();
    println!(
        "  {} Switched from {} to {}",
        "✔".green().bold(),
        previous.bright_black(),
        env_name.bright_blue().bold()
    );

    let env = store.environments.get(env_name).unwrap();
    if env.vars.is_empty() {
        println!("  {} This environment has no variables set.", "·".dimmed());
    } else {
        println!(
            "  {} {} variable{} active.",
            "·".dimmed(),
            env.vars.len(),
            if env.vars.len() == 1 { "" } else { "s" }
        );
    }
    println!();
    Ok(())
}

// ── Formatting ────────────────────────────────────────────────────────────────

fn print_env_table(
    env_name: &str,
    vars: &BTreeMap<String, String>,
    is_active: bool,
    is_merged: bool,
) {
    println!();
    let active_tag = if is_active {
        " (active)".green().bold().to_string()
    } else {
        String::new()
    };
    let merged_tag = if is_merged { " [merged]" } else { "" };
    println!(
        "{}  {}{}{}",
        "Environment:".bold().cyan(),
        env_name.bright_blue().bold(),
        active_tag,
        merged_tag.dimmed()
    );
    println!("{}", "═".repeat(60).cyan());

    if vars.is_empty() {
        println!("  {}", "No variables set.".dimmed());
    } else {
        println!("  {:<40} {}", "Variable".bold(), "Value".bold());
        println!("  {}", "─".repeat(56).dimmed());
        for (k, v) in vars {
            let display_value = if v.chars().count() > 50 {
                format!("{}…", v.chars().take(47).collect::<String>())
            } else {
                v.clone()
            };
            println!("  {:<40} {}", k.cyan(), display_value.dimmed());
        }
    }

    println!("{}", "═".repeat(60).cyan());
    println!();
}

fn print_all_envs(store: &Environments) {
    println!();
    println!("{}", "Environments".bold().cyan());
    println!("{}", "═".repeat(60).cyan());

    if store.environments.is_empty() {
        println!("  {}", "No environments configured.".dimmed());
    } else {
        for (name, env) in &store.environments {
            let active_marker = if *name == store.active {
                " ◀ active".green().bold().to_string()
            } else {
                String::new()
            };
            println!(
                "  {} {}  ({} var{})",
                "·".dimmed(),
                name.bright_blue().bold(),
                env.vars.len(),
                if env.vars.len() == 1 { "" } else { "s" }
            );
            print!("    {}", active_marker);
            println!();
        }
    }

    println!("{}", "═".repeat(60).cyan());
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_env_name_rejects_leading_hyphen() {
        let err = validate_env_name("-prod").expect_err("leading hyphen should fail");
        assert!(
            err.to_string().contains("must start"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn dotenv_escape_quotes_special_characters() {
        let escaped = dotenv_escape("a value \"quoted\"\nnext\\line");
        assert_eq!(escaped, "\"a value \\\"quoted\\\"\\nnext\\\\line\"");
    }

    #[test]
    fn masked_value_hides_original_content() {
        let original = "super-secret-token";
        let masked = masked_value(original);
        assert!(!masked.contains(original));
        assert!(masked.contains("18 chars"));
    }
}
