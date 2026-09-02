//! `soroban-registry contract list` - browse registered contracts.
//!
//! Paginates through the registry with `--limit`/`--offset`, and renders as a
//! table, JSON or CSV. The machine-readable formats carry no decoration, so the
//! command pipes cleanly into `jq`, `column`, or a spreadsheet.

use anyhow::{Context, Result};
use colored::Colorize;
use registry_client::{Contract, ContractSearchParams, PaginatedResponse};
use std::io::{self, IsTerminal, Write};

/// Rows per page when the caller does not choose.
pub const DEFAULT_LIMIT: usize = 10;
/// The registry clamps `limit` to this, so reject a larger one up front rather
/// than silently returning fewer rows than asked for.
pub const MAX_LIMIT: usize = 100;

/// How to render the listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFormat {
    Table,
    Json,
    Csv,
}

impl ListFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_lowercase().as_str() {
            "table" => Ok(ListFormat::Table),
            "json" => Ok(ListFormat::Json),
            "csv" => Ok(ListFormat::Csv),
            other => anyhow::bail!(
                "Unknown format '{other}'. Use one of: table, json, csv.\n\
                 Example: soroban-registry contract list --format json"
            ),
        }
    }
}

/// Everything `contract list` accepts.
#[derive(Debug, Clone)]
pub struct ListOptions {
    pub limit: usize,
    pub offset: usize,
    pub networks: Option<String>,
    pub category: Option<String>,
    pub format: String,
}

pub async fn run(api_url: &str, options: ListOptions) -> Result<()> {
    let format = ListFormat::parse(&options.format)?;
    let limit = validate_limit(options.limit)?;

    let params = build_params(&options, limit)?;

    let page = crate::registry::client(api_url)
        .await?
        .list_contracts(&params)
        .await
        .map_err(|err| explain(err, api_url))?;

    // Write through a locked handle rather than `println!`: `println!` panics
    // when the reader goes away, so `contract list | head` would crash instead
    // of stopping. A command meant to be piped treats a closed pipe as a normal
    // end of output.
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let result = match format {
        ListFormat::Json => print_json(&mut out, &page, options.offset, limit),
        ListFormat::Csv => print_csv(&mut out, &page.items),
        ListFormat::Table => print_table(&mut out, &page, options.offset, limit),
    }
    .and_then(|()| out.flush());

    match result {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(err).context("Failed to write the contract list"),
    }
}

fn validate_limit(limit: usize) -> Result<usize> {
    if limit == 0 {
        anyhow::bail!(
            "--limit must be at least 1.\n\
             Example: soroban-registry contract list --limit {DEFAULT_LIMIT}"
        );
    }
    if limit > MAX_LIMIT {
        anyhow::bail!(
            "--limit is capped at {MAX_LIMIT} (asked for {limit}).\n\
             Page through the registry instead:\n  \
             soroban-registry contract list --limit {MAX_LIMIT} --offset {MAX_LIMIT}"
        );
    }
    Ok(limit)
}

fn build_params(options: &ListOptions, limit: usize) -> Result<ContractSearchParams> {
    let mut params = ContractSearchParams {
        limit: Some(limit as i64),
        offset: Some(options.offset as i64),
        ..Default::default()
    };

    // Reuse the filter normalisation `list` and `search` already share, so an
    // unknown network fails here rather than being dropped server-side.
    let networks = crate::commands::normalize_network_list(options.networks.as_deref())?;
    if !networks.is_empty() {
        params.networks = Some(networks);
    }
    let categories = crate::commands::normalize_list_values(options.category.as_deref());
    if !categories.is_empty() {
        params.categories = Some(categories);
    }

    Ok(params)
}

/// Turn a client error into something with a next step attached.
fn explain(err: registry_client::Error, api_url: &str) -> anyhow::Error {
    let hint = match &err {
        registry_client::Error::Transport { .. } => format!(
            "Could not reach the registry at {api_url}.\n\
             Check the URL, or set one with --api-url or `soroban-registry config set api-url`."
        ),
        registry_client::Error::Timeout { .. } => {
            "The registry took too long to respond. Try a smaller --limit, or retry shortly."
                .to_string()
        }
        error if error.is_auth() => {
            "The registry rejected the credentials. Run `soroban-registry auth login`.".to_string()
        }
        registry_client::Error::Validation(_) => {
            "The registry rejected the filters. Check --networks and --category values.".to_string()
        }
        registry_client::Error::RateLimited { retry_after, .. } => match retry_after {
            Some(after) => format!("Rate limited. Retry in {}s.", after.as_secs()),
            None => "Rate limited. Retry shortly.".to_string(),
        },
        _ => "Run with -vv to see the full request and response.".to_string(),
    };

    anyhow::anyhow!("Failed to list contracts: {err}\n{hint}")
}

// ── Rendering ────────────────────────────────────────────────────────────────

/// The columns this command promises, in order.
fn row(contract: &Contract) -> [String; 5] {
    [
        contract.contract_id.clone(),
        contract.name.clone(),
        contract.network.to_string(),
        contract.category.clone().unwrap_or_default(),
        contract.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    ]
}

fn print_json(
    out: &mut impl Write,
    page: &PaginatedResponse<Contract>,
    offset: usize,
    limit: usize,
) -> io::Result<()> {
    let contracts: Vec<serde_json::Value> = page
        .items
        .iter()
        .map(|contract| {
            serde_json::json!({
                "address": contract.contract_id,
                "name": contract.name,
                "network": contract.network.to_string(),
                "category": contract.category,
                "last_update": contract.updated_at.to_rfc3339(),
            })
        })
        .collect();

    let document = serde_json::json!({
        "contracts": contracts,
        "pagination": pagination_json(page, offset, limit),
    });
    // Serializing a value this crate built cannot fail for any reason the user
    // can act on, so a failure here is a bug, not a report.
    let rendered = serde_json::to_string_pretty(&document)
        .expect("the listing document is always serializable");
    writeln!(out, "{rendered}")
}

fn pagination_json(
    page: &PaginatedResponse<Contract>,
    offset: usize,
    limit: usize,
) -> serde_json::Value {
    let total = page.total.max(0) as usize;
    serde_json::json!({
        "count": page.items.len(),
        "total": total,
        "limit": limit,
        "offset": offset,
        "page": current_page(offset, limit),
        "total_pages": total_pages(total, limit),
        "has_more": offset + page.items.len() < total,
    })
}

fn print_csv(out: &mut impl Write, items: &[Contract]) -> io::Result<()> {
    // No header decoration and no colour: this is the format people pipe.
    writeln!(out, "address,name,network,category,last_update")?;
    for contract in items {
        let cells = row(contract).map(|cell| csv_escape(&cell));
        writeln!(out, "{}", cells.join(","))?;
    }
    Ok(())
}

/// Quote a CSV field only when it needs it, doubling any embedded quotes.
fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn print_table(
    out: &mut impl Write,
    page: &PaginatedResponse<Contract>,
    offset: usize,
    limit: usize,
) -> io::Result<()> {
    let headers = ["ADDRESS", "NAME", "NETWORK", "CATEGORY", "LAST UPDATE"];
    let rows: Vec<[String; 5]> = page.items.iter().map(row).collect();

    if rows.is_empty() {
        writeln!(
            out,
            "{}",
            decorate("No contracts found.", Decoration::Warning)
        )?;
        return writeln!(out, "Try widening the filters, or a different --offset.");
    }

    // Size every column to its widest visible cell, so the table stays aligned
    // whatever the data looks like.
    let mut widths: [usize; 5] = headers.map(str::len);
    for row in &rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }

    let header_line: Vec<String> = headers
        .iter()
        .enumerate()
        .map(|(index, header)| format!("{header:<width$}", width = widths[index]))
        .collect();
    writeln!(
        out,
        "{}",
        decorate(&header_line.join("  "), Decoration::Header)
    )?;

    for row in &rows {
        let line: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(index, cell)| format!("{cell:<width$}", width = widths[index]))
            .collect();
        writeln!(out, "{}", line.join("  "))?;
    }

    let total = page.total.max(0) as usize;
    writeln!(out)?;
    writeln!(
        out,
        "Page {} of {} - showing {} of {} contract(s)",
        current_page(offset, limit),
        total_pages(total, limit),
        rows.len(),
        total
    )?;
    if offset + rows.len() < total {
        writeln!(
            out,
            "Next page: soroban-registry contract list --limit {limit} --offset {}",
            offset + limit
        )?;
    }
    Ok(())
}

fn current_page(offset: usize, limit: usize) -> usize {
    offset / limit.max(1) + 1
}

fn total_pages(total: usize, limit: usize) -> usize {
    total.div_ceil(limit.max(1)).max(1)
}

enum Decoration {
    Header,
    Warning,
}

/// Colour only when stdout is a terminal, so a piped table stays plain text.
fn decorate(text: &str, decoration: Decoration) -> String {
    if !std::io::stdout().is_terminal() {
        return text.to_string();
    }
    match decoration {
        Decoration::Header => text.bold().to_string(),
        Decoration::Warning => text.yellow().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_parse_case_insensitively() {
        assert_eq!(ListFormat::parse("table").unwrap(), ListFormat::Table);
        assert_eq!(ListFormat::parse("JSON").unwrap(), ListFormat::Json);
        assert_eq!(ListFormat::parse(" csv ").unwrap(), ListFormat::Csv);
    }

    #[test]
    fn an_unknown_format_names_the_valid_ones() {
        let err = ListFormat::parse("yaml").expect_err("yaml is not supported");
        let message = err.to_string();
        assert!(message.contains("table, json, csv"), "{message}");
        assert!(message.contains("Example:"), "errors suggest a next step");
    }

    #[test]
    fn limits_are_bounded_to_what_the_registry_serves() {
        assert_eq!(validate_limit(1).unwrap(), 1);
        assert_eq!(validate_limit(MAX_LIMIT).unwrap(), MAX_LIMIT);

        let err = validate_limit(MAX_LIMIT + 1).expect_err("over the cap");
        assert!(err.to_string().contains("capped at 100"));
        assert!(
            err.to_string().contains("--offset"),
            "the error should show how to page instead"
        );
        assert!(validate_limit(0).is_err());
    }

    #[test]
    fn page_numbers_are_derived_from_the_offset() {
        assert_eq!(current_page(0, 10), 1);
        assert_eq!(current_page(10, 10), 2);
        assert_eq!(current_page(95, 10), 10);
        // A partial page still counts as a page.
        assert_eq!(total_pages(0, 10), 1);
        assert_eq!(total_pages(10, 10), 1);
        assert_eq!(total_pages(11, 10), 2);
        assert_eq!(total_pages(101, 10), 11);
    }

    #[test]
    fn csv_fields_are_escaped_only_when_needed() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("with,comma"), "\"with,comma\"");
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_escape("two\nlines"), "\"two\nlines\"");
    }
}
