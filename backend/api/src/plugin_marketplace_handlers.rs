use crate::error::ApiError;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

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
    pub steps: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceIndexResponse {
    #[serde(default)]
    pub plugins: Vec<MarketplaceEntry>,
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
struct MarketplaceIndexFile {
    #[serde(default)]
    plugins: Vec<MarketplacePluginFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MarketplacePluginFileEntry {
    name: String,
    version: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    commands: Vec<MarketplaceCommand>,
    manifest: PluginManifest,
}

static MARKETPLACE: Lazy<MarketplaceIndexFile> = Lazy::new(|| {
    serde_json::from_str(include_str!("../plugin_marketplace/index.json"))
        .expect("Invalid plugin marketplace index.json")
});

pub async fn get_marketplace() -> Json<MarketplaceIndexResponse> {
    let plugins = MARKETPLACE
        .plugins
        .iter()
        .map(|p| MarketplaceEntry {
            name: p.name.clone(),
            version: p.version.clone(),
            description: p.description.clone(),
            commands: p.commands.clone(),
        })
        .collect();
    Json(MarketplaceIndexResponse { plugins })
}

pub async fn get_plugin_manifest(
    Path((name, version)): Path<(String, String)>,
) -> Result<Json<PluginManifest>, ApiError> {
    let plugin = MARKETPLACE
        .plugins
        .iter()
        .find(|p| p.name == name && p.version == version)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "PLUGIN_NOT_FOUND",
                "Plugin not found",
            )
        })?;

    Ok(Json(plugin.manifest.clone()))
}
