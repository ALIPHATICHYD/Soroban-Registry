use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use flate2::write::GzEncoder;
use flate2::Compression;
use reqwest::Url;
use serde_json::{Map, Value};
use tar::Builder;

use crate::io_utils::{compute_sha256_streaming, BUF_SIZE};
use crate::manifest::{ExportManifest, ManifestEntry};

pub fn create_archive(
    contract_dir: &Path,
    output_path: &Path,
    contract_id: &str,
    name: &str,
    network: &str,
) -> Result<()> {
    let tmp_dir = tempfile::tempdir().context("failed to create temp dir")?;
    let inner_path = tmp_dir.path().join("contract.tar.gz");

    let mut manifest = ExportManifest::new(contract_id.into(), name.into(), network.into());

    build_inner_archive(contract_dir, &inner_path, &mut manifest)?;
    manifest.sha256 = compute_sha256_streaming(&inner_path)?;

    let manifest_path = tmp_dir.path().join("manifest.json");
    let manifest_json = serde_json::to_vec_pretty(&manifest)?;
    fs::write(&manifest_path, &manifest_json)?;

    build_outer_archive(output_path, &manifest_path, &inner_path)?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryExportFormat {
    Json,
    Csv,
    JsonLines,
    Sqlite,
    Markdown,
    Archive,
}

impl RegistryExportFormat {
    pub fn resolve(raw: Option<&str>, id: Option<&str>, output: Option<&str>) -> Result<Self> {
        if let Some(format) = raw {
            return Self::parse(format);
        }

        if id.is_some() && output.is_some_and(|path| path.ends_with(".tar.gz")) {
            return Ok(Self::Archive);
        }

        if id.is_some() && output.is_none() {
            return Ok(Self::Archive);
        }

        Ok(Self::Json)
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            "jsonl" | "ndjson" => Ok(Self::JsonLines),
            "sqlite" | "sqlite3" => Ok(Self::Sqlite),
            "markdown" | "md" => Ok(Self::Markdown),
            "archive" | "tar" | "tar.gz" => Ok(Self::Archive),
            other => anyhow::bail!(
                "unsupported export format `{}`; expected json, csv, jsonl, sqlite, markdown, or archive",
                other
            ),
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
            Self::JsonLines => "jsonl",
            Self::Sqlite => "sqlite",
            Self::Markdown => "md",
            Self::Archive => "tar.gz",
        }
    }
}

pub struct RegistryExportOptions<'a> {
    pub api_url: &'a str,
    pub id: Option<&'a str>,
    pub output: Option<&'a str>,
    pub contract_dir: &'a str,
    pub format: RegistryExportFormat,
    pub filters: Vec<String>,
    pub page_size: usize,
    pub include_related: bool,
    pub compress: bool,
}

pub struct RegistryExportSummary {
    pub output_path: String,
    pub checksum_path: String,
    pub sha256: String,
    pub items_exported: usize,
    pub format: RegistryExportFormat,
}

pub async fn export_registry_data(
    options: RegistryExportOptions<'_>,
) -> Result<RegistryExportSummary> {
    if options.format == RegistryExportFormat::Archive {
        let id = options
            .id
            .context("archive export requires --id so the manifest can identify the contract")?;
        let output = options
            .output
            .map(str::to_string)
            .unwrap_or_else(|| "contract-export.tar.gz".to_string());
        let source = Path::new(options.contract_dir);
        anyhow::ensure!(
            source.is_dir(),
            "contract directory does not exist: {}",
            options.contract_dir
        );

        create_archive(source, Path::new(&output), id, "contract", "testnet")?;
        let (sha256, checksum_path) = write_checksum_file(Path::new(&output))?;
        return Ok(RegistryExportSummary {
            output_path: output,
            checksum_path,
            sha256,
            items_exported: 1,
            format: options.format,
        });
    }

    let output = options
        .output
        .map(str::to_string)
        .unwrap_or_else(|| format!("contracts-export.{}", options.format.extension()));
    let output = if options.compress && !output.ends_with(".gz") {
        format!("{}.gz", output)
    } else {
        output
    };
    let output_path = Path::new(&output);
    let filters = parse_filters(&options.filters)?;
    let client = reqwest::Client::new();

    let items_exported = match options.format {
        RegistryExportFormat::Json => export_json(&client, &options, &filters, output_path).await?,
        RegistryExportFormat::JsonLines => {
            export_json_lines(&client, &options, &filters, output_path).await?
        }
        RegistryExportFormat::Csv => export_csv(&client, &options, &filters, output_path).await?,
        RegistryExportFormat::Sqlite => {
            export_sqlite(&client, &options, &filters, output_path).await?
        }
        RegistryExportFormat::Markdown => {
            export_markdown(&client, &options, &filters, output_path).await?
        }
        RegistryExportFormat::Archive => unreachable!("archive handled above"),
    };

    let (sha256, checksum_path) = write_checksum_file(output_path)?;
    Ok(RegistryExportSummary {
        output_path: output,
        checksum_path,
        sha256,
        items_exported,
        format: options.format,
    })
}

async fn export_json(
    client: &reqwest::Client,
    options: &RegistryExportOptions<'_>,
    filters: &[(String, String)],
    output_path: &Path,
) -> Result<usize> {
    let mut writer = create_export_writer(output_path, options.compress)?;
    let started_at = Utc::now().to_rfc3339();

    writeln!(writer, "{{")?;
    writeln!(writer, "  \"schema_version\": 1,")?;
    writeln!(
        writer,
        "  \"exported_at\": {},",
        serde_json::to_string(&started_at)?
    )?;
    writeln!(writer, "  \"format\": \"json\",")?;
    writeln!(
        writer,
        "  \"filters\": {},",
        serde_json::to_string(filters)?
    )?;
    writeln!(writer, "  \"items\": [")?;

    let mut count = 0usize;
    let mut first = true;
    for item in fetch_export_items(client, options, filters).await? {
        if !first {
            writeln!(writer, ",")?;
        }
        first = false;
        write!(writer, "    {}", serde_json::to_string(&item)?)?;
        count += 1;
        eprintln!("exported {} contract(s)...", count);
    }

    writeln!(writer)?;
    writeln!(writer, "  ]")?;
    writeln!(writer, "}}")?;
    writer.flush()?;
    Ok(count)
}

async fn export_json_lines(
    client: &reqwest::Client,
    options: &RegistryExportOptions<'_>,
    filters: &[(String, String)],
    output_path: &Path,
) -> Result<usize> {
    let mut writer = create_export_writer(output_path, options.compress)?;
    let mut count = 0usize;
    for item in fetch_export_items(client, options, filters).await? {
        writeln!(writer, "{}", serde_json::to_string(&item)?)?;
        count += 1;
        eprintln!("exported {} contract(s)...", count);
    }
    writer.flush()?;
    Ok(count)
}

async fn export_csv(
    client: &reqwest::Client,
    options: &RegistryExportOptions<'_>,
    filters: &[(String, String)],
    output_path: &Path,
) -> Result<usize> {
    let mut writer = csv::Writer::from_writer(create_export_writer(output_path, options.compress)?);
    writer.write_record([
        "id",
        "contract_id",
        "name",
        "network",
        "is_verified",
        "category",
        "publisher_id",
        "wasm_hash",
        "created_at",
        "updated_at",
        "relationships_json",
        "metadata_json",
    ])?;

    let mut count = 0usize;
    for item in fetch_export_items(client, options, filters).await? {
        writer.write_record([
            scalar(&item, "id"),
            scalar(&item, "contract_id"),
            scalar(&item, "name"),
            scalar(&item, "network"),
            scalar(&item, "is_verified"),
            scalar(&item, "category"),
            scalar(&item, "publisher_id"),
            scalar(&item, "wasm_hash"),
            scalar(&item, "created_at"),
            scalar(&item, "updated_at"),
            json_field(&item, "relationships"),
            serde_json::to_string(&item)?,
        ])?;
        count += 1;
        eprintln!("exported {} contract(s)...", count);
    }

    writer.flush()?;
    Ok(count)
}

async fn export_sqlite(
    client: &reqwest::Client,
    options: &RegistryExportOptions<'_>,
    filters: &[(String, String)],
    output_path: &Path,
) -> Result<usize> {
    let mut writer = create_export_writer(output_path, options.compress)?;
    writeln!(writer, "BEGIN TRANSACTION;")?;
    writeln!(writer, "CREATE TABLE IF NOT EXISTS contracts (id TEXT PRIMARY KEY, contract_id TEXT, name TEXT, network TEXT, is_verified INTEGER, category TEXT, publisher_id TEXT, wasm_hash TEXT, created_at TEXT, updated_at TEXT, metadata_json TEXT NOT NULL);")?;

    let mut count = 0usize;
    for item in fetch_export_items(client, options, filters).await? {
        writeln!(
            writer,
            "INSERT OR REPLACE INTO contracts (id, contract_id, name, network, is_verified, category, publisher_id, wasm_hash, created_at, updated_at, metadata_json) VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {});",
            sql_string(&scalar(&item, "id")),
            sql_string(&scalar(&item, "contract_id")),
            sql_string(&scalar(&item, "name")),
            sql_string(&scalar(&item, "network")),
            sql_bool(&item, "is_verified"),
            sql_string(&scalar(&item, "category")),
            sql_string(&scalar(&item, "publisher_id")),
            sql_string(&scalar(&item, "wasm_hash")),
            sql_string(&scalar(&item, "created_at")),
            sql_string(&scalar(&item, "updated_at")),
            sql_string(&serde_json::to_string(&item)?),
        )?;
        count += 1;
        eprintln!("exported {} contract(s)...", count);
    }

    writeln!(writer, "COMMIT;")?;
    writer.flush()?;
    Ok(count)
}

async fn export_markdown(
    client: &reqwest::Client,
    options: &RegistryExportOptions<'_>,
    filters: &[(String, String)],
    output_path: &Path,
) -> Result<usize> {
    let mut writer = create_export_writer(output_path, options.compress)?;
    writeln!(writer, "# Soroban Registry Export")?;
    writeln!(writer)?;
    writeln!(writer, "- Exported at: {}", Utc::now().to_rfc3339())?;
    writeln!(writer, "- Filters: `{}`", filters_as_string(filters))?;
    writeln!(writer)?;
    writeln!(
        writer,
        "| Name | Contract ID | Network | Verified | Category | Relationships |"
    )?;
    writeln!(writer, "|---|---|---|---|---|---|")?;

    let mut count = 0usize;
    for item in fetch_export_items(client, options, filters).await? {
        writeln!(
            writer,
            "| {} | `{}` | {} | {} | {} | `{}` |",
            md_cell(&scalar(&item, "name")),
            md_cell(&scalar(&item, "contract_id")),
            md_cell(&scalar(&item, "network")),
            md_cell(&scalar(&item, "is_verified")),
            md_cell(&scalar(&item, "category")),
            md_cell(&relationship_summary(&item))
        )?;
        count += 1;
        eprintln!("exported {} contract(s)...", count);
    }

    writer.flush()?;
    Ok(count)
}

async fn fetch_export_items(
    client: &reqwest::Client,
    options: &RegistryExportOptions<'_>,
    filters: &[(String, String)],
) -> Result<Vec<Value>> {
    if let Some(id) = options.id {
        let detail = fetch_json(
            client,
            &format!("{}/api/contracts/{}", base_url(options.api_url), id),
        )
        .await
        .with_context(|| format!("failed to fetch contract {}", id))?;
        return Ok(vec![maybe_enrich_contract(client, options, detail).await]);
    }

    let mut output = Vec::new();
    let page_size = options.page_size.clamp(1, 1_000);
    let mut offset = 0usize;

    loop {
        let page = fetch_contract_page(client, options.api_url, filters, page_size, offset).await?;
        let items = extract_items(&page);
        if items.is_empty() {
            break;
        }

        for item in items {
            output.push(maybe_enrich_contract(client, options, item).await);
        }

        offset += page_size;
        let total = page
            .get("total")
            .and_then(Value::as_u64)
            .map(|v| v as usize);
        if total.is_some_and(|total| offset >= total) || output.len() < offset {
            break;
        }
    }

    Ok(output)
}

async fn fetch_contract_page(
    client: &reqwest::Client,
    api_url: &str,
    filters: &[(String, String)],
    limit: usize,
    offset: usize,
) -> Result<Value> {
    let mut url = Url::parse(&format!("{}/api/contracts", base_url(api_url)))
        .context("invalid registry API URL")?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("limit", &limit.to_string());
        query.append_pair("offset", &offset.to_string());
        for (key, value) in filters {
            query.append_pair(key, value);
        }
    }
    fetch_json(client, url.as_str()).await
}

async fn maybe_enrich_contract(
    client: &reqwest::Client,
    options: &RegistryExportOptions<'_>,
    item: Value,
) -> Value {
    if options.include_related {
        enrich_contract(client, options.api_url, item).await
    } else {
        item
    }
}

async fn enrich_contract(client: &reqwest::Client, api_url: &str, mut item: Value) -> Value {
    let Some(id) = item
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| item.get("contract_id").and_then(Value::as_str))
        .map(str::to_string)
    else {
        return item;
    };

    let mut relationships = Map::new();
    for (name, path) in [
        ("versions", format!("/api/contracts/{}/versions", id)),
        (
            "dependencies",
            format!("/api/contracts/{}/dependencies", id),
        ),
        ("analytics", format!("/api/contracts/{}/analytics", id)),
        ("reviews", format!("/api/contracts/{}/reviews", id)),
    ] {
        if let Ok(value) = fetch_json(client, &format!("{}{}", base_url(api_url), path)).await {
            relationships.insert(name.to_string(), value);
        }
    }

    if !relationships.is_empty() {
        if let Some(obj) = item.as_object_mut() {
            obj.insert("relationships".to_string(), Value::Object(relationships));
        }
    }

    item
}

async fn fetch_json(client: &reqwest::Client, url: &str) -> Result<Value> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("request failed: {}", url))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .with_context(|| format!("failed to read response body: {}", url))?;
    anyhow::ensure!(
        status.is_success(),
        "registry returned {} for {}",
        status,
        url
    );
    serde_json::from_str(&text).with_context(|| format!("invalid JSON from {}", url))
}

fn extract_items(page: &Value) -> Vec<Value> {
    if let Some(items) = page.get("items").and_then(Value::as_array) {
        return items.clone();
    }
    if let Some(items) = page.get("contracts").and_then(Value::as_array) {
        return items.clone();
    }
    if let Some(items) = page.as_array() {
        return items.clone();
    }
    Vec::new()
}

fn parse_filters(filters: &[String]) -> Result<Vec<(String, String)>> {
    filters
        .iter()
        .map(|filter| {
            let (key, value) = filter
                .split_once('=')
                .with_context(|| format!("invalid filter `{}`; expected key=value", filter))?;
            let key = key.trim();
            anyhow::ensure!(!key.is_empty(), "filter key cannot be empty");
            anyhow::ensure!(
                key.chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'),
                "filter key `{}` contains unsupported characters",
                key
            );
            Ok((key.to_string(), value.trim().to_string()))
        })
        .collect()
}

fn write_checksum_file(output_path: &Path) -> Result<(String, String)> {
    let sha256 = compute_sha256_streaming(output_path)?;
    let checksum_path = format!("{}.sha256", output_path.display());
    let file_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("export");
    fs::write(&checksum_path, format!("{}  {}\n", sha256, file_name))
        .with_context(|| format!("failed to write checksum file {}", checksum_path))?;
    Ok((sha256, checksum_path))
}

fn base_url(api_url: &str) -> String {
    api_url.trim_end_matches('/').to_string()
}

fn scalar(item: &Value, key: &str) -> String {
    match item.get(key) {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Bool(value)) => value.to_string(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(value) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn create_export_writer(output_path: &Path, compress: bool) -> Result<Box<dyn Write>> {
    let file = File::create(output_path)
        .with_context(|| format!("failed to create {}", output_path.display()))?;
    if compress {
        Ok(Box::new(GzEncoder::new(file, Compression::default())))
    } else {
        Ok(Box::new(BufWriter::new(file)))
    }
}

fn sql_string(value: &str) -> String {
    if value.is_empty() {
        "NULL".to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

fn sql_bool(item: &Value, key: &str) -> &'static str {
    match item.get(key).and_then(Value::as_bool) {
        Some(true) => "1",
        Some(false) => "0",
        None => "NULL",
    }
}

fn json_field(item: &Value, key: &str) -> String {
    item.get(key)
        .map(|value| serde_json::to_string(value).unwrap_or_default())
        .unwrap_or_default()
}

fn filters_as_string(filters: &[(String, String)]) -> String {
    if filters.is_empty() {
        "none".to_string()
    } else {
        filters
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn relationship_summary(item: &Value) -> String {
    let Some(relationships) = item.get("relationships").and_then(Value::as_object) else {
        return "none".to_string();
    };

    relationships
        .iter()
        .map(|(name, value)| {
            let count = value
                .as_array()
                .map(Vec::len)
                .or_else(|| value.get("items").and_then(Value::as_array).map(Vec::len))
                .unwrap_or(1);
            format!("{}:{}", name, count)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn md_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn build_inner_archive(
    source_dir: &Path,
    archive_path: &Path,
    manifest: &mut ExportManifest,
) -> Result<()> {
    let file = BufWriter::new(File::create(archive_path)?);
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    walk_and_append(&mut builder, source_dir, source_dir, manifest)?;

    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn walk_and_append<W: Write>(
    builder: &mut Builder<W>,
    base: &Path,
    dir: &Path,
    manifest: &mut ExportManifest,
) -> Result<()> {
    let entries = fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap_or(&path);

        if path.is_dir() {
            walk_and_append(builder, base, &path, manifest)?;
        } else {
            let metadata = entry.metadata()?;
            let modified: DateTime<Utc> = metadata
                .modified()
                .ok()
                .and_then(|t| {
                    t.duration_since(std::time::UNIX_EPOCH).ok().and_then(|d| {
                        Utc.timestamp_opt(d.as_secs() as i64, d.subsec_nanos())
                            .single()
                    })
                })
                .unwrap_or_else(Utc::now);

            manifest.contents.push(ManifestEntry {
                path: rel.to_string_lossy().replace('\\', "/"),
                size: metadata.len(),
                modified_at: modified,
            });

            let mut header = tar::Header::new_gnu();
            header.set_size(metadata.len());
            header.set_mode(0o644);
            header.set_cksum();

            let f = BufReader::new(File::open(&path)?);
            builder.append_data(&mut header, rel.to_string_lossy().replace('\\', "/"), f)?;
        }
    }
    Ok(())
}

fn build_outer_archive(
    output_path: &Path,
    manifest_path: &Path,
    inner_archive_path: &Path,
) -> Result<()> {
    let file = BufWriter::new(File::create(output_path)?);
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    append_file_streaming(&mut builder, manifest_path, "manifest.json")?;
    append_file_streaming(&mut builder, inner_archive_path, "contract.tar.gz")?;

    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn append_file_streaming<W: Write>(
    builder: &mut Builder<W>,
    file_path: &Path,
    archive_name: &str,
) -> Result<()> {
    let metadata = fs::metadata(file_path)?;
    let mut header = tar::Header::new_gnu();
    header.set_size(metadata.len());
    header.set_mode(0o644);
    header.set_cksum();

    let reader = BufReader::with_capacity(BUF_SIZE, File::open(file_path)?);
    builder.append_data(&mut header, archive_name, reader)?;
    Ok(())
}
