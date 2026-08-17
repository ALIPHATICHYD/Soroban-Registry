//! `soroban-registry contract search` — paginated contract search.
//!
//! Pagination is handled by the shared `registry_client` crate, so this command
//! never manages continuation tokens itself: it picks a mode, sets safety
//! bounds, and renders whatever the walk produced. `--all` walks every page
//! within `--max-items` / `--max-pages`, and Ctrl-C stops the walk and prints
//! what was already fetched.
//!
//! The flag-to-request logic lives in [`crate::search_pagination`].

use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use registry_client::{
    ContractHit, PageCollection, PageCursor, PaginationMode, RegistryClient, StopReason,
};

use crate::search_pagination::{build_limits, build_request, resolve_mode, SearchOptions};

pub async fn run(api_url: &str, options: SearchOptions) -> Result<()> {
    let mode = resolve_mode(&options)?;
    let request = build_request(&options, mode)?;
    let limits = build_limits(&options);

    let token = crate::auth::access_token_for_requests(api_url).await?;
    let client =
        RegistryClient::with_http_client(api_url, crate::net::client()).with_bearer_token(token);

    let mut walk = client
        .search_paginator(request, limits)
        .map_err(|err| anyhow!("Failed to search contracts: {err}"))?;

    // Ctrl-C ends a multi-page walk cleanly, keeping the pages already fetched,
    // instead of tearing the process down mid-render.
    if options.all {
        let cancel = walk.cancel_token();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                cancel.cancel();
            }
        });
    }

    let collected = walk
        .collect_all()
        .await
        .context("Failed to search contracts")?;

    if options.json {
        print_json(&options, mode, &collected)?;
    } else {
        print_human(api_url, &options, mode, &collected);
    }

    Ok(())
}

// ── Output ────────────────────────────────────────────────────────────────────

/// How the walk ended, from the command's point of view.
///
/// Without `--all` the walk is deliberately one page long, so the paginator's
/// internal page bound is reported as `single_page` rather than as a bound the
/// user set.
fn stop_reason_label(
    options: &SearchOptions,
    collected: &PageCollection<ContractHit>,
) -> &'static str {
    if !options.all && collected.stop_reason == StopReason::MaxPages {
        return "single_page";
    }
    collected.stop_reason.as_str()
}

fn print_json(
    options: &SearchOptions,
    mode: PaginationMode,
    collected: &PageCollection<ContractHit>,
) -> Result<()> {
    let contracts: Vec<serde_json::Value> = collected
        .items
        .iter()
        .map(|hit| {
            serde_json::json!({
                "id": hit.id,
                "contract_id": hit.contract_id,
                "name": hit.name,
                "description": hit.description,
                "network": hit.network,
                "category": hit.category,
                "is_verified": hit.is_verified,
                "is_deprecated": hit.is_deprecated,
                "deprecation_status": hit.deprecation_status,
                "relevance_score": hit.relevance_score,
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "query": options.query,
            "contracts": contracts,
            "count": contracts.len(),
            "pagination": {
                "mode": mode.as_str(),
                "page_size": options.limit,
                "pages_fetched": collected.pages_fetched,
                "total": collected.total,
                "complete": collected.is_complete(),
                "stop_reason": stop_reason_label(options, collected),
                "cancelled": collected.stop_reason == StopReason::Cancelled,
                "next_cursor": collected.next.as_ref().and_then(PageCursor::as_cursor),
                "next_offset": collected.next.as_ref().and_then(PageCursor::as_offset),
                "max_items": options.all.then(|| options.effective_max_items()),
                "max_pages": options.all.then(|| options.effective_max_pages()),
            }
        }))?
    );

    Ok(())
}

fn print_human(
    api_url: &str,
    options: &SearchOptions,
    mode: PaginationMode,
    collected: &PageCollection<ContractHit>,
) {
    println!("\n{}", "Search Results:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let mut filters: Vec<String> = Vec::new();
    if let Some(networks) = options.networks.as_deref() {
        filters.push(format!("networks: {networks}"));
    }
    if let Some(category) = options.category.as_deref() {
        filters.push(format!("category: {category}"));
    }
    if let Some(tags) = options.tags.as_deref() {
        filters.push(format!("tags: {tags}"));
    }
    if options.verified_only {
        filters.push("verified only".to_string());
    }
    if !filters.is_empty() {
        println!(
            "  {} {}\n",
            "Active filters:".bold(),
            filters.join(" | ").bright_blue()
        );
    }

    if collected.items.is_empty() {
        println!("{}", "No contracts found matching your query.".yellow());
        print_pagination_footer(options, mode, collected);
        return;
    }

    for hit in &collected.items {
        let verified = if hit.is_verified {
            " ✓ verified".green().to_string()
        } else {
            String::new()
        };
        let deprecated = if hit.is_deprecated {
            " ⚠ deprecated".yellow().to_string()
        } else {
            String::new()
        };

        println!(" {}{}{}", hit.name.bold(), verified, deprecated);
        println!(
            "   {}",
            hit.description.as_deref().unwrap_or("No description")
        );
        println!(
            "   {} {} | {} {}",
            "Network:".dimmed(),
            hit.network,
            "Category:".dimmed(),
            hit.category.as_deref().unwrap_or("unknown")
        );
        println!(
            "   {} {}/contracts/{}",
            "Link:".dimmed(),
            api_url,
            hit.contract_id
        );
        println!();
    }

    print_pagination_footer(options, mode, collected);
}

fn print_pagination_footer(
    options: &SearchOptions,
    mode: PaginationMode,
    collected: &PageCollection<ContractHit>,
) {
    let of_total = match collected.total {
        Some(total) => format!(" of {total} match(es)"),
        None => String::new(),
    };

    println!(
        "  {} {} result(s){} · {} page(s) · {} pagination",
        "Showing".bold(),
        collected.items.len(),
        of_total,
        collected.pages_fetched,
        mode
    );

    // Bound notices only make sense for a `--all` walk: a single-page search
    // stops after one page by design.
    match collected.stop_reason {
        StopReason::Cancelled => println!(
            "  {}",
            "Cancelled — showing the pages fetched before interruption.".yellow()
        ),
        StopReason::MaxItems if options.all => println!(
            "  {}",
            format!(
                "Stopped at the --max-items bound ({}).",
                options.effective_max_items()
            )
            .yellow()
        ),
        StopReason::MaxPages if options.all => println!(
            "  {}",
            format!(
                "Stopped at the --max-pages bound ({}).",
                options.effective_max_pages()
            )
            .yellow()
        ),
        StopReason::Exhausted | StopReason::MaxItems | StopReason::MaxPages => {}
    }

    // Say exactly how to continue, so nobody has to hand-roll continuation state.
    match collected.next.as_ref() {
        Some(PageCursor::Cursor(token)) => println!(
            "  {} {}",
            "Next page:".dimmed(),
            format!("--cursor {token}").bright_blue()
        ),
        Some(PageCursor::Offset { offset }) => println!(
            "  {} {}",
            "Next page:".dimmed(),
            format!("--offset {offset}").bright_blue()
        ),
        None => {}
    }
    println!();
}
