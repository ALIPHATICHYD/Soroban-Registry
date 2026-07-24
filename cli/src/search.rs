use crate::net::RequestBuilderExt;
use colored::Colorize;
use shared::models::Contract;
use std::time::Instant;

pub async fn run(
    query: &str,
    verified_only: bool,
    networks: Option<&String>,
    category: Option<&String>,
    sort: Option<&String>,
    limit: usize,
    offset: usize,
    output_json: bool,
    api_url: &str,
) -> anyhow::Result<()> {
    let start = Instant::now();

    let url = format!("{}/api/contracts", api_url);
    let client = crate::net::client();
    let mut all_contracts: Vec<Contract> = client.get(&url).send_with_retry().await?.json().await?;

    let q = query.to_lowercase();
    all_contracts.retain(|c| {
        c.name.to_lowercase().contains(&q)
            || c.description
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains(&q)
    });

    if let Some(nets) = networks {
        let network_list: Vec<&str> = nets.split(',').map(|s| s.trim()).collect();
        all_contracts.retain(|c| {
            let net_str = format!("{:?}", c.network).to_lowercase();
            network_list.iter().any(|n| n.to_lowercase() == net_str)
        });
    }

    if let Some(category_filter) = category {
        all_contracts.retain(|c| {
            c.category
                .as_deref()
                .unwrap_or("")
                .eq_ignore_ascii_case(category_filter)
        });
    }

    if verified_only {
        all_contracts.retain(|c| c.is_verified);
    }

    let sort_mode = sort.map(|s| s.as_str()).unwrap_or("relevance");
    match sort_mode {
        "updated" => all_contracts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        "created" => all_contracts.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        "name" => all_contracts.sort_by(|a, b| a.name.cmp(&b.name)),
        _ => {
            all_contracts.sort_by(|a, b| {
                b.relevance_score
                    .unwrap_or(0.0)
                    .partial_cmp(&a.relevance_score.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    if offset < all_contracts.len() {
        all_contracts = all_contracts.split_off(offset);
    } else {
        all_contracts.clear();
    }
    all_contracts.truncate(limit);
    let elapsed = start.elapsed();

    if output_json {
        let contracts: Vec<serde_json::Value> = all_contracts
            .iter()
            .map(|c| {
                let tag_names: Vec<String> = c.tags.iter().map(|t| t.name.clone()).collect();
                serde_json::json!({
                    "id": c.id,
                    "name": c.name,
                    "contract_id": c.contract_id,
                    "network": c.network,
                    "category": c.category.as_deref().unwrap_or(""),
                    "is_verified": c.is_verified,
                    "health_score": c.health_score,
                    "created_at": c.created_at.to_rfc3339(),
                    "tags": tag_names,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "contracts": contracts,
                "count": contracts.len()
            }))?
        );
        return Ok(());
    }

    if all_contracts.is_empty() {
        println!("{}", "No contracts found matching your query.".yellow());
        return Ok(());
    }

    println!(
        "{} {} result(s) in {:.0}ms\n",
        "Found".green().bold(),
        all_contracts.len().to_string().green().bold(),
        elapsed.as_millis()
    );

    let mut filters: Vec<String> = Vec::new();
    if let Some(nets) = networks {
        filters.push(format!("networks: {}", nets));
    }
    if let Some(cat) = category {
        filters.push(format!("category: {}", cat));
    }
    if verified_only {
        filters.push("verified".to_string());
    }
    if !filters.is_empty() {
        println!("  {}\n", filters.join(" | ").bright_blue());
    }

    for contract in &all_contracts {
        let highlighted_name = highlight_match(&contract.name, query);
        let desc = contract.description.as_deref().unwrap_or("No description");
        let highlighted_desc = highlight_match(desc, query);

        let verified_badge = if contract.is_verified {
            " ✓ verified".green().to_string()
        } else {
            String::new()
        };

        println!(" {}{}", highlighted_name.bold(), verified_badge);
        println!("   {}", highlighted_desc);
        println!(
            "   {} {:?} | {} {}",
            "Network:".dimmed(),
            contract.network,
            "Category:".dimmed(),
            contract.category.as_deref().unwrap_or("unknown")
        );
        println!(
            "   {} {}",
            "Updated:".dimmed(),
            contract.updated_at.format("%Y-%m-%d %H:%M:%S")
        );
        println!();
    }

    Ok(())
}

fn highlight_match(text: &str, query: &str) -> String {
    if query.is_empty() {
        return text.to_string();
    }
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut result = String::new();
    let mut last = 0;
    while let Some(pos) = lower_text[last..].find(&lower_query) {
        let abs = last + pos;
        result.push_str(&text[last..abs]);
        result.push_str(&text[abs..abs + query.len()].yellow().bold().to_string());
        last = abs + query.len();
    }
    result.push_str(&text[last..]);
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_multiple_networks_comma_separated() {
        let input = "testnet,mainnet,futurenet";
        let networks: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
        assert_eq!(networks.len(), 3);
        assert!(networks.contains(&"testnet"));
        assert!(networks.contains(&"mainnet"));
        assert!(networks.contains(&"futurenet"));
    }

    #[test]
    fn parse_single_network() {
        let input = "testnet";
        let networks: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0], "testnet");
    }

    #[test]
    fn parse_networks_with_spaces() {
        let input = "testnet, mainnet , futurenet";
        let networks: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
        assert_eq!(networks.len(), 3);
        assert_eq!(networks[0], "testnet");
        assert_eq!(networks[1], "mainnet");
        assert_eq!(networks[2], "futurenet");
    }
}
