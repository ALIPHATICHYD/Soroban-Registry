use anyhow::{Context, Result};
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};
use shared::{Contract, Network, Publisher};
use sqlx::PgPool;
use std::collections::HashMap;

const CONTRACT_NAMES: &[&str] = &[
    "TokenSwap",
    "LiquidityPool",
    "PriceOracle",
    "StakingContract",
    "VotingSystem",
    "NFTMarketplace",
    "MultiSigWallet",
    "EscrowService",
    "PaymentGateway",
    "IdentityVerifier",
    "DecentralizedExchange",
    "YieldFarming",
    "LendingProtocol",
    "InsurancePool",
    "GovernanceToken",
    "AssetRegistry",
    "CrossChainBridge",
    "DataFeed",
    "AutomatedMarketMaker",
    "RewardDistributor",
];

const DESCRIPTIONS: &[&str] = &[
    "A decentralized token swap protocol enabling seamless asset exchanges",
    "Liquidity pool implementation for automated market making",
    "Real-time price oracle aggregating data from multiple sources",
    "Staking contract with flexible reward distribution mechanisms",
    "On-chain voting system with quadratic voting support",
    "NFT marketplace with royalty enforcement and batch operations",
    "Multi-signature wallet with configurable threshold requirements",
    "Escrow service with time-locked releases and dispute resolution",
    "Payment gateway supporting multiple payment methods",
    "Identity verification system with privacy-preserving credentials",
];

const CATEGORIES: &[&str] = &[
    "DeFi",
    "NFT",
    "Governance",
    "Infrastructure",
    "Payment",
    "Identity",
    "Gaming",
    "Social",
];

const TAGS_POOL: &[&str] = &[
    "defi",
    "nft",
    "governance",
    "staking",
    "liquidity",
    "swap",
    "oracle",
    "bridge",
    "marketplace",
    "wallet",
    "payment",
    "identity",
    "verification",
    "voting",
    "token",
    "yield",
    "lending",
    "insurance",
    "automation",
    "cross-chain",
];

const PUBLISHER_NAMES: &[&str] = &[
    "Stellar Labs",
    "Soroban Dev",
    "Crypto Innovations",
    "Blockchain Solutions",
    "DeFi Protocol",
    "Smart Contracts Inc",
    "Web3 Builders",
    "Decentralized Systems",
    "Chain Developers",
    "Crypto Ventures",
];

pub async fn create_publishers(
    pool: &PgPool,
    count: usize,
    rng: &mut StdRng,
    custom_data: Option<&HashMap<String, serde_json::Value>>,
) -> Result<Vec<Publisher>> {
    let mut publishers = Vec::new();

    for i in 0..count {
        let name = if let Some(data) = custom_data {
            if let Some(names) = data.get("publisher_names").and_then(|v| v.as_array()) {
                if let Some(name_val) = names.get(i % names.len()) {
                    name_val
                        .as_str()
                        .unwrap_or(PUBLISHER_NAMES[i % PUBLISHER_NAMES.len()])
                } else {
                    PUBLISHER_NAMES[i % PUBLISHER_NAMES.len()]
                }
            } else {
                PUBLISHER_NAMES[i % PUBLISHER_NAMES.len()]
            }
        } else {
            PUBLISHER_NAMES[i % PUBLISHER_NAMES.len()]
        };

        let stellar_address = generate_stellar_address(rng);
        let username = format!("{}_{}", name.to_lowercase().replace(" ", "_"), i);
        let email = format!("{}@example.com", username);
        let github_url = format!("https://github.com/{}", username);
        let website = format!("https://{}.example.com", username);

        let publisher: Publisher = sqlx::query_as(
            "INSERT INTO publishers (stellar_address, username, email, github_url, website)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (stellar_address) DO UPDATE SET
                 username = EXCLUDED.username,
                 email = EXCLUDED.email,
                 github_url = EXCLUDED.github_url,
                 website = EXCLUDED.website
             RETURNING *",
        )
        .bind(&stellar_address)
        .bind(&username)
        .bind(&email)
        .bind(&github_url)
        .bind(&website)
        .fetch_one(pool)
        .await
        .context("Failed to create publisher")?;

        publishers.push(publisher);
    }

    Ok(publishers)
}

pub async fn create_contracts(
    pool: &PgPool,
    count: usize,
    publishers: &[Publisher],
    rng: &mut StdRng,
    custom_data: Option<&HashMap<String, serde_json::Value>>,
) -> Result<Vec<Contract>> {
    let mut contracts = Vec::new();
    let networks = [Network::Mainnet, Network::Testnet, Network::Futurenet];

    for i in 0..count {
        let publisher = &publishers[i % publishers.len()];
        let network = networks[i % networks.len()].clone();

        let name = if let Some(data) = custom_data {
            if let Some(names) = data.get("contract_names").and_then(|v| v.as_array()) {
                if let Some(name_val) = names.get(i % names.len()) {
                    name_val
                        .as_str()
                        .unwrap_or(CONTRACT_NAMES[i % CONTRACT_NAMES.len()])
                } else {
                    CONTRACT_NAMES[i % CONTRACT_NAMES.len()]
                }
            } else {
                CONTRACT_NAMES[i % CONTRACT_NAMES.len()]
            }
        } else {
            CONTRACT_NAMES[i % CONTRACT_NAMES.len()]
        };

        let description = DESCRIPTIONS[i % DESCRIPTIONS.len()];
        let category = Some(CATEGORIES[i % CATEGORIES.len()].to_string());

        let tag_count = rng.gen_range(2..=5);
        let tags: Vec<String> = (0..tag_count)
            .map(|_| TAGS_POOL[rng.gen_range(0..TAGS_POOL.len())].to_string())
            .collect();

        let contract_id = generate_contract_id(rng);
        let wasm_hash = generate_wasm_hash(rng);
        let slug = shared::slugify(name);

        let contract: Contract = sqlx::query_as(
            "INSERT INTO contracts (
                contract_id, wasm_hash, name, slug, description, publisher_id, network,
                category, tags, is_verified
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (contract_id, network) DO UPDATE SET
                wasm_hash = EXCLUDED.wasm_hash,
                name = EXCLUDED.name,
                slug = EXCLUDED.slug,
                description = EXCLUDED.description,
                updated_at = NOW()
            RETURNING *",
        )
        .bind(&contract_id)
        .bind(&wasm_hash)
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(publisher.id)
        .bind(&network)
        .bind(&category)
        .bind(&tags)
        .bind(i % 3 == 0)
        .fetch_one(pool)
        .await
        .context("Failed to create contract")?;

        contracts.push(contract);
    }

    Ok(contracts)
}

pub async fn create_versions(
    pool: &PgPool,
    contracts: &[Contract],
    rng: &mut StdRng,
) -> Result<usize> {
    let mut count = 0;

    for contract in contracts.iter().step_by(3) {
        let version_count = rng.gen_range(1..=3);

        for v in 1..=version_count {
            let version = format!("1.{}.0", v);
            let wasm_hash = generate_wasm_hash(rng);

            sqlx::query(
                "INSERT INTO contract_versions (contract_id, version, wasm_hash, source_url, commit_hash)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (contract_id, version) DO NOTHING",
            )
            .bind(contract.id)
            .bind(&version)
            .bind(&wasm_hash)
            .bind(format!("https://github.com/example/repo/commit/{}", generate_hash(rng, 40)))
            .bind(Some(generate_hash(rng, 40)))
            .execute(pool)
            .await
            .context("Failed to create version")?;

            count += 1;
        }
    }

    Ok(count)
}

pub async fn create_verifications(
    pool: &PgPool,
    contracts: &[Contract],
    rng: &mut StdRng,
) -> Result<usize> {
    let mut count = 0;

    for contract in contracts.iter().step_by(2) {
        if contract.is_verified {
            let status = if rng.gen::<f64>() < 0.9 {
                "verified"
            } else {
                "pending"
            };

            let result = sqlx::query(
                "INSERT INTO verifications (contract_id, status, compiler_version, verified_at)
                 SELECT $1, $2::verification_status, $3, $4
                 WHERE NOT EXISTS (
                     SELECT 1 FROM verifications WHERE contract_id = $1
                 )",
            )
            .bind(contract.id)
            .bind(status)
            .bind("soroban-sdk-20.0.0")
            .bind(if status == "verified" {
                Some(chrono::Utc::now())
            } else {
                None
            })
            .execute(pool)
            .await
            .context("Failed to create verification")?;

            if result.rows_affected() > 0 {
                count += 1;
            }
        }
    }

    Ok(count)
}

fn generate_stellar_address(rng: &mut StdRng) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut address = String::from("G");
    for _ in 0..55 {
        address.push(CHARS[rng.gen_range(0..CHARS.len())] as char);
    }
    address
}

fn generate_contract_id(rng: &mut StdRng) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut id = String::from("C");
    for _ in 0..55 {
        id.push(CHARS[rng.gen_range(0..CHARS.len())] as char);
    }
    id
}

fn generate_wasm_hash(rng: &mut StdRng) -> String {
    generate_hash(rng, 64)
}

fn generate_hash(rng: &mut StdRng, length: usize) -> String {
    const CHARS: &[u8] = b"0123456789abcdef";
    (0..length)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}
