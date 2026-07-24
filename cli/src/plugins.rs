use crate::net::RequestBuilderExt;
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

const ENV_HOME_OVERRIDE: &str = "SOROBAN_REGISTRY_HOME";
const HOME_DIR_NAME: &str = ".soroban-registry";
const PLUGINS_DIR_NAME: &str = "plugins";
const INSTALLED_DIR_NAME: &str = "installed";
const MANIFEST_FILE_NAME: &str = "plugin.json";
const PLUGINS_CONFIG_FILE_NAME: &str = "plugins.config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub steps: Vec<PluginStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginStep {
    Print {
        text: String,
    },
    HttpGet {
        /// Path relative to the registry API base, e.g. "/api/contracts/trending".
        /// Absolute URLs are rejected to prevent SSRF.
        path: String,
        /// Optional JSON pointer (RFC 6901) to select from the response.
        #[serde(default)]
        json_pointer: Option<String>,
        /// Store extracted value into a variable for later steps.
        #[serde(default)]
        save_as: Option<String>,
        /// Pretty-print JSON output (defaults to true when JSON is printed).
        #[serde(default)]
        pretty: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PluginsConfigFile {
    #[serde(default)]
    plugins: BTreeMap<String, PluginConfigEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PluginConfigEntry {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    config: Value,
}

#[derive(Debug, Clone)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PluginRunResult {
    pub stdout: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub commands: Vec<MarketplaceCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceCommand {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceIndexResponse {
    #[serde(default)]
    pub plugins: Vec<MarketplaceEntry>,
}

fn home_dir() -> Result<PathBuf> {
    if let Ok(override_dir) = std::env::var(ENV_HOME_OVERRIDE) {
        return Ok(PathBuf::from(override_dir));
    }
    dirs::home_dir().context("Could not determine home directory")
}

fn plugins_root() -> Result<PathBuf> {
    Ok(home_dir()?.join(HOME_DIR_NAME).join(PLUGINS_DIR_NAME))
}

fn installed_root() -> Result<PathBuf> {
    Ok(plugins_root()?.join(INSTALLED_DIR_NAME))
}

fn plugins_config_path() -> Result<PathBuf> {
    Ok(plugins_root()?.join(PLUGINS_CONFIG_FILE_NAME))
}

fn plugin_install_dir(name: &str, version: &str) -> Result<PathBuf> {
    Ok(installed_root()?.join(name).join(version))
}

fn plugin_manifest_path(name: &str, version: &str) -> Result<PathBuf> {
    Ok(plugin_install_dir(name, version)?.join(MANIFEST_FILE_NAME))
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(value)?;
    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))
}

pub fn discover_installed() -> Result<Vec<InstalledPlugin>> {
    let root = installed_root()?;
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut plugins = Vec::new();
    for name_entry in
        fs::read_dir(&root).with_context(|| format!("Failed to read {}", root.display()))?
    {
        let name_entry = name_entry?;
        if !name_entry.file_type()?.is_dir() {
            continue;
        }
        for version_entry in fs::read_dir(name_entry.path())? {
            let version_entry = version_entry?;
            if !version_entry.file_type()?.is_dir() {
                continue;
            }
            let manifest_path = version_entry.path().join(MANIFEST_FILE_NAME);
            if !manifest_path.exists() {
                continue;
            }
            let manifest: PluginManifest = read_json_file(&manifest_path)?;
            plugins.push(InstalledPlugin {
                manifest,
                manifest_path,
            });
        }
    }
    Ok(plugins)
}

fn load_plugins_config() -> Result<PluginsConfigFile> {
    let path = plugins_config_path()?;
    if !path.exists() {
        return Ok(PluginsConfigFile::default());
    }
    read_json_file(&path)
}

fn save_plugins_config(cfg: &PluginsConfigFile) -> Result<()> {
    let path = plugins_config_path()?;
    write_json_file(&path, cfg)
}

pub fn set_plugin_enabled(name: &str, enabled: bool) -> Result<()> {
    let mut cfg = load_plugins_config()?;
    let entry = cfg.plugins.entry(name.to_string()).or_default();
    entry.enabled = enabled;
    if entry.config.is_null() {
        entry.config = Value::Object(Default::default());
    }
    save_plugins_config(&cfg)
}

pub fn set_plugin_config_json(name: &str, json: &str) -> Result<()> {
    let new_config: Value = serde_json::from_str(json).context("Invalid JSON passed to --json")?;
    if !new_config.is_object() {
        anyhow::bail!("Plugin config must be a JSON object");
    }

    let mut cfg = load_plugins_config()?;
    let entry = cfg.plugins.entry(name.to_string()).or_default();
    entry.enabled = true;
    entry.config = new_config;
    save_plugins_config(&cfg)
}

pub fn get_plugin_config(name: &str) -> Result<Value> {
    let cfg = load_plugins_config()?;
    Ok(cfg
        .plugins
        .get(name)
        .map(|e| e.config.clone())
        .unwrap_or(Value::Object(Default::default())))
}

fn is_plugin_enabled(name: &str) -> Result<bool> {
    let cfg = load_plugins_config()?;
    Ok(cfg.plugins.get(name).map(|e| e.enabled).unwrap_or(true))
}

fn build_command_index(
    installed: &[InstalledPlugin],
) -> Result<HashMap<String, (PluginManifest, PluginCommand)>> {
    let mut index = HashMap::new();
    for plugin in installed {
        if !is_plugin_enabled(&plugin.manifest.name)? {
            continue;
        }
        for cmd in &plugin.manifest.commands {
            index.insert(cmd.name.clone(), (plugin.manifest.clone(), cmd.clone()));
        }
    }
    Ok(index)
}

#[derive(Debug, Clone)]
struct TemplateContext {
    api_url: String,
    network: String,
    args: Vec<String>,
    plugin_config: Value,
    vars: HashMap<String, Value>,
}

fn render_template(input: &str, ctx: &TemplateContext) -> Result<String> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("{{") {
        let (before, after_start) = rest.split_at(start);
        out.push_str(before);
        let Some(end) = after_start.find("}}") else {
            anyhow::bail!("Unclosed template expression in: {}", input);
        };
        let expr = after_start[2..end].trim();
        let value = resolve_template_expr(expr, ctx)
            .ok_or_else(|| anyhow!("Unknown template expression: {}", expr))?;
        out.push_str(&value);
        rest = &after_start[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

fn resolve_template_expr(expr: &str, ctx: &TemplateContext) -> Option<String> {
    if expr == "api_url" {
        return Some(ctx.api_url.clone());
    }
    if expr == "network" {
        return Some(ctx.network.clone());
    }

    if let Some(idx) = expr.strip_prefix("args.") {
        if let Ok(i) = idx.parse::<usize>() {
            return ctx.args.get(i).cloned();
        }
    }

    if let Some(key) = expr.strip_prefix("vars.") {
        return ctx.vars.get(key).and_then(stringify_value);
    }

    if let Some(path) = expr.strip_prefix("config.") {
        return lookup_json_path(&ctx.plugin_config, path).and_then(stringify_value);
    }

    None
}

fn lookup_json_path<'a>(value: &'a Value, dotted: &str) -> Option<&'a Value> {
    let mut cur = value;
    for part in dotted.split('.') {
        if part.is_empty() {
            return None;
        }
        cur = cur.get(part)?;
    }
    Some(cur)
}

fn stringify_value(v: &Value) -> Option<String> {
    match v {
        Value::Null => Some(String::new()),
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        other => serde_json::to_string(other).ok(),
    }
}

pub async fn install_from_registry(api_url: &str, name: &str, version: Option<&str>) -> Result<()> {
    let resolved_version = match version {
        Some(v) => v.to_string(),
        None => {
            let marketplace = fetch_marketplace(api_url).await?;
            let entry = marketplace
                .plugins
                .into_iter()
                .find(|p| p.name == name)
                .ok_or_else(|| anyhow!("Plugin `{}` not found in marketplace", name))?;
            entry.version
        }
    };

    let client = crate::net::client();
    let resp = client
        .get(format!(
            "{}/api/plugins/{}/{}",
            api_url.trim_end_matches('/'),
            name,
            resolved_version
        ))
        .send_with_retry()
        .await
        .context("Failed to fetch plugin manifest")?;

    if !resp.status().is_success() {
        anyhow::bail!(
            "Registry returned {} while fetching plugin manifest",
            resp.status()
        );
    }

    let manifest: PluginManifest = resp.json().await.context("Invalid plugin manifest JSON")?;
    if manifest.name != name {
        anyhow::bail!(
            "Plugin name mismatch: requested `{}`, got `{}`",
            name,
            manifest.name
        );
    }
    if manifest.version != resolved_version {
        anyhow::bail!(
            "Plugin version mismatch: requested `{}`, got `{}`",
            resolved_version,
            manifest.version
        );
    }

    let dest = plugin_manifest_path(&manifest.name, &manifest.version)?;
    write_json_file(&dest, &manifest)?;
    set_plugin_enabled(&manifest.name, true)?;
    println!(
        "{} Installed plugin {}@{}",
        "✓".green(),
        manifest.name.bold(),
        manifest.version
    );
    Ok(())
}

pub fn uninstall(name: &str, version: Option<&str>) -> Result<()> {
    let root = installed_root()?;
    let target = match version {
        Some(v) => root.join(name).join(v),
        None => root.join(name),
    };
    if !target.exists() {
        anyhow::bail!("Plugin not installed: {}", target.display());
    }
    fs::remove_dir_all(&target)
        .with_context(|| format!("Failed to remove {}", target.display()))?;
    println!("{} Uninstalled {}", "✓".green(), name.bold());
    Ok(())
}

pub async fn fetch_marketplace(api_url: &str) -> Result<MarketplaceIndexResponse> {
    let client = crate::net::client();
    let resp = client
        .get(format!(
            "{}/api/plugins/marketplace",
            api_url.trim_end_matches('/')
        ))
        .send_with_retry()
        .await
        .context("Failed to fetch plugin marketplace")?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "Registry returned {} while fetching marketplace",
            resp.status()
        );
    }
    resp.json::<MarketplaceIndexResponse>()
        .await
        .context("Invalid marketplace JSON")
}

pub async fn run_installed_command(
    api_url: &str,
    network: &str,
    command_name: &str,
    args: Vec<String>,
) -> Result<PluginRunResult> {
    let installed = discover_installed()?;
    let index = build_command_index(&installed)?;
    let Some((plugin, command)) = index.get(command_name).cloned() else {
        anyhow::bail!(
            "Unknown command `{}`. Try `soroban-registry plugins list`.",
            command_name
        );
    };

    let plugin_config = get_plugin_config(&plugin.name)?;
    let ctx = TemplateContext {
        api_url: api_url.to_string(),
        network: network.to_string(),
        args,
        plugin_config,
        vars: HashMap::new(),
    };

    execute_command_steps(&ctx, &command.steps).await
}

async fn execute_command_steps(
    ctx: &TemplateContext,
    steps: &[PluginStep],
) -> Result<PluginRunResult> {
    let mut stdout = String::new();
    let mut runtime_ctx = ctx.clone();

    for step in steps {
        match step {
            PluginStep::Print { text } => {
                let rendered = render_template(text, &runtime_ctx)?;
                stdout.push_str(&rendered);
                if !rendered.ends_with('\n') {
                    stdout.push('\n');
                }
            }
            PluginStep::HttpGet {
                path,
                json_pointer,
                save_as,
                pretty,
            } => {
                let rendered_path = render_template(path, &runtime_ctx)?;
                if !rendered_path.starts_with('/') {
                    anyhow::bail!(
                        "Plugin http_get path must be a relative path starting with '/': {}",
                        rendered_path
                    );
                }

                let url = format!(
                    "{}{}",
                    runtime_ctx.api_url.trim_end_matches('/'),
                    rendered_path
                );
                let client = crate::net::client();
                let resp = client
                    .get(url)
                    .send_with_retry()
                    .await
                    .context("HTTP GET failed")?;
                let status = resp.status();
                let body = resp
                    .text()
                    .await
                    .context("Failed reading HTTP response body")?;

                if !status.is_success() {
                    anyhow::bail!("Registry returned {}: {}", status, body);
                }

                let extracted = if json_pointer.is_some() || save_as.is_some() {
                    let parsed: Value =
                        serde_json::from_str(&body).context("Response was not valid JSON")?;
                    if let Some(ptr) = json_pointer {
                        parsed
                            .pointer(ptr)
                            .cloned()
                            .ok_or_else(|| anyhow!("JSON pointer not found: {}", ptr))?
                    } else {
                        parsed
                    }
                } else {
                    Value::String(body)
                };

                if let Some(var) = save_as {
                    runtime_ctx.vars.insert(var.clone(), extracted.clone());
                }

                match extracted {
                    Value::String(s) => {
                        stdout.push_str(&s);
                        if !s.ends_with('\n') {
                            stdout.push('\n');
                        }
                    }
                    v => {
                        let want_pretty = pretty.unwrap_or(true);
                        let out = if want_pretty {
                            serde_json::to_string_pretty(&v)?
                        } else {
                            serde_json::to_string(&v)?
                        };
                        stdout.push_str(&out);
                        if !out.ends_with('\n') {
                            stdout.push('\n');
                        }
                    }
                }
            }
        }
    }

    Ok(PluginRunResult { stdout })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;
    use std::sync::OnceLock;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn marketplace_response(name: &str, version: &str) -> Value {
        json!({
            "plugins": [{
                "name": name,
                "version": version,
                "description": "test plugin",
                "commands": [{ "name": name, "description": "test command" }]
            }]
        })
    }

    fn print_manifest(name: &str, version: &str, text: &str) -> Value {
        json!({
            "name": name,
            "version": version,
            "description": "test plugin",
            "commands": [{
                "name": name,
                "description": "test command",
                "steps": [{ "type": "print", "text": text }]
            }]
        })
    }

    fn http_get_manifest(name: &str, version: &str, path_value: &str) -> Value {
        json!({
            "name": name,
            "version": version,
            "description": "test plugin",
            "commands": [{
                "name": name,
                "description": "test command",
                "steps": [{ "type": "http_get", "path": path_value }]
            }]
        })
    }

    #[tokio::test]
    async fn install_and_run_print_plugin() -> Result<()> {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let tmp = tempfile::tempdir()?;
        std::env::set_var(ENV_HOME_OVERRIDE, tmp.path().to_string_lossy().to_string());

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/plugins/marketplace"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(marketplace_response("hello", "0.1.0")),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/plugins/hello/0.1.0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(print_manifest(
                "hello",
                "0.1.0",
                "Hello from plugin!",
            )))
            .mount(&server)
            .await;

        install_from_registry(&server.uri(), "hello", None).await?;
        let result = run_installed_command(&server.uri(), "testnet", "hello", vec![]).await?;
        assert!(result.stdout.contains("Hello from plugin!"));
        std::env::remove_var(ENV_HOME_OVERRIDE);
        Ok(())
    }

    #[tokio::test]
    async fn template_uses_plugin_config() -> Result<()> {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let tmp = tempfile::tempdir()?;
        std::env::set_var(ENV_HOME_OVERRIDE, tmp.path().to_string_lossy().to_string());

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/plugins/marketplace"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(marketplace_response("cfg", "1.0.0")),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/plugins/cfg/1.0.0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(print_manifest(
                "cfg",
                "1.0.0",
                "Value={{config.foo}}",
            )))
            .mount(&server)
            .await;

        install_from_registry(&server.uri(), "cfg", None).await?;
        set_plugin_config_json("cfg", r#"{ "foo": "bar" }"#)?;

        let result = run_installed_command(&server.uri(), "testnet", "cfg", vec![]).await?;
        assert!(result.stdout.contains("Value=bar"));
        std::env::remove_var(ENV_HOME_OVERRIDE);
        Ok(())
    }

    #[tokio::test]
    async fn http_get_rejects_absolute_urls() -> Result<()> {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let tmp = tempfile::tempdir()?;
        std::env::set_var(ENV_HOME_OVERRIDE, tmp.path().to_string_lossy().to_string());

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/plugins/marketplace"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(marketplace_response("evil", "0.0.1")),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/plugins/evil/0.0.1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(http_get_manifest(
                "evil",
                "0.0.1",
                "https://example.com/steal",
            )))
            .mount(&server)
            .await;

        install_from_registry(&server.uri(), "evil", None).await?;
        let err = run_installed_command(&server.uri(), "testnet", "evil", vec![])
            .await
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("must be a relative path starting with '/'"));
        std::env::remove_var(ENV_HOME_OVERRIDE);
        Ok(())
    }
}
