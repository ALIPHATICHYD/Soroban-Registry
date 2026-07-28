use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::Utc;
use serde_json::Value;
use shared::{
    ContractInteroperabilityResponse, GraphEdge, GraphNode, GraphResponse,
    InteroperabilityCapability, InteroperabilityCapabilityKind, InteroperabilityProtocolMatch,
    InteroperabilitySuggestion, InteroperabilitySummary, Network, ProtocolComplianceStatus, Tag,
};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

use contract_abi::parser::parse_json_spec;

#[derive(Debug, Clone, FromRow)]
struct ContractAnalysisRow {
    id: Uuid,
    contract_id: String,
    name: String,
    network: Network,
    is_verified: bool,
    category: Option<String>,
    tags: Vec<String>,
    abi: Option<Value>,
}

#[derive(Debug, Clone, FromRow)]
struct ProtocolDefinitionRow {
    slug: String,
    name: String,
    description: String,
    required_functions: Vec<String>,
    optional_functions: Vec<String>,
    bridge_indicators: Vec<String>,
    adapter_indicators: Vec<String>,
}

#[derive(Debug, Clone, FromRow)]
struct CompatibilityHintRow {
    other_contract_id: Uuid,
    compatible_entries: i64,
}

#[derive(Debug, Clone)]
struct CandidateAnalysis {
    row: ContractAnalysisRow,
    functions: BTreeSet<String>,
    protocols: Vec<InteroperabilityProtocolMatch>,
    capabilities: Vec<InteroperabilityCapability>,
}

#[derive(Debug, Clone)]
struct ScoredSuggestion {
    candidate: CandidateAnalysis,
    score: f64,
    reason: String,
    shared_protocols: Vec<String>,
    shared_functions: Vec<String>,
    relation_types: Vec<String>,
}

/// Analyze one contract and return interoperability protocols, suggestions, and graph links.
pub async fn analyze_contract_interoperability(
    pool: &PgPool,
    contract_id: Uuid,
) -> ApiResult<ContractInteroperabilityResponse> {
    let contract = fetch_contract(pool, contract_id).await?.ok_or_else(|| {
        ApiError::not_found(
            "ContractNotFound",
            format!("No contract found with ID: {contract_id}"),
        )
    })?;

    let protocol_definitions = load_protocol_definitions(pool).await?;
    let functions = extract_function_names(contract.abi.as_ref(), &contract.name);
    let mut warnings = Vec::new();
    if contract.abi.is_none() {
        warnings.push("No ABI found for this contract. Suggestions rely on metadata and stored relationships.".to_string());
    } else if functions.is_empty() {
        warnings.push(
            "The stored ABI could not be normalized into callable functions for protocol analysis."
                .to_string(),
        );
    }
    if protocol_definitions.is_empty() {
        warnings.push(
            "Protocol definitions are not seeded yet, so compliance checks are limited."
                .to_string(),
        );
    }

    let protocols = evaluate_protocols(&protocol_definitions, &functions);
    let capabilities =
        detect_capabilities(&contract, &functions, &protocols, &protocol_definitions);
    let compatibility_hints = load_compatibility_hints(pool, contract.id).await?;
    let direct_dependencies = try_load_related_ids(
        pool,
        "SELECT dependency_contract_id FROM contract_dependencies WHERE contract_id = $1 AND dependency_contract_id IS NOT NULL",
        contract.id,
        &mut warnings,
    )
    .await;
    let direct_dependents = try_load_related_ids(
        pool,
        "SELECT contract_id FROM contract_dependencies WHERE dependency_contract_id = $1",
        contract.id,
        &mut warnings,
    )
    .await;

    let target_protocols = supported_protocols(&protocols);
    let target_is_bridge = has_capability(&capabilities, InteroperabilityCapabilityKind::Bridge);
    let target_is_adapter = has_capability(&capabilities, InteroperabilityCapabilityKind::Adapter);

    let candidate_rows =
        load_candidate_contracts(pool, contract.id, contract.network.clone()).await?;
    let mut candidate_by_id = HashMap::new();
    let mut scored = Vec::new();
    for row in candidate_rows {
        let candidate_functions = extract_function_names(row.abi.as_ref(), &row.name);
        let candidate_protocols_list =
            evaluate_protocols(&protocol_definitions, &candidate_functions);
        let candidate_capabilities = detect_capabilities(
            &row,
            &candidate_functions,
            &candidate_protocols_list,
            &protocol_definitions,
        );
        let candidate = CandidateAnalysis {
            row,
            functions: candidate_functions,
            protocols: candidate_protocols_list,
            capabilities: candidate_capabilities,
        };
        if let Some(suggestion) = score_candidate(
            &contract,
            &functions,
            &target_protocols,
            target_is_bridge,
            target_is_adapter,
            &candidate,
            &compatibility_hints,
            &direct_dependencies,
            &direct_dependents,
        ) {
            scored.push(suggestion.clone());
        }
        candidate_by_id.insert(candidate.row.id, candidate);
    }

    scored.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| {
                right
                    .candidate
                    .row
                    .is_verified
                    .cmp(&left.candidate.row.is_verified)
            })
            .then_with(|| left.candidate.row.name.cmp(&right.candidate.row.name))
    });
    scored.truncate(6);

    let graph = build_graph(
        &contract,
        &candidate_by_id,
        &scored,
        &compatibility_hints,
        &direct_dependencies,
        &direct_dependents,
    );
    let suggestions = scored
        .iter()
        .map(|item| InteroperabilitySuggestion {
            contract_id: item.candidate.row.id,
            contract_address: item.candidate.row.contract_id.clone(),
            contract_name: item.candidate.row.name.clone(),
            network: item.candidate.row.network.clone(),
            category: item.candidate.row.category.clone(),
            is_verified: item.candidate.row.is_verified,
            score: item.score,
            reason: item.reason.clone(),
            shared_protocols: item.shared_protocols.clone(),
            shared_functions: item.shared_functions.clone(),
            relation_types: item.relation_types.clone(),
        })
        .collect();

    Ok(ContractInteroperabilityResponse {
        contract_id: contract.id,
        contract_address: contract.contract_id,
        contract_name: contract.name,
        network: contract.network.clone(),
        analyzed_at: Utc::now(),
        has_abi: contract.abi.is_some(),
        analyzed_functions: functions.into_iter().collect(),
        warnings,
        protocols: sort_protocols(protocols),
        capabilities: capabilities.clone(),
        suggestions,
        graph: graph.clone(),
        summary: InteroperabilitySummary {
            protocol_matches: target_protocols.len(),
            compatible_contracts: compatibility_hints.len(),
            suggested_contracts: graph.nodes.len().saturating_sub(1),
            graph_nodes: graph.nodes.len(),
            graph_edges: graph.edges.len(),
            bridge_signals: count_capabilities(
                &capabilities,
                InteroperabilityCapabilityKind::Bridge,
            ),
            adapter_signals: count_capabilities(
                &capabilities,
                InteroperabilityCapabilityKind::Adapter,
            ),
        },
    })
}

async fn fetch_contract(
    pool: &PgPool,
    contract_id: Uuid,
) -> ApiResult<Option<ContractAnalysisRow>> {
    sqlx::query_as::<_, ContractAnalysisRow>(
        r#"
        SELECT c.id, c.contract_id, c.name, c.network, c.is_verified, c.category, c.tags, latest_abi.abi
        FROM contracts c
        LEFT JOIN LATERAL (
            SELECT abi FROM contract_abis WHERE contract_id = c.id ORDER BY created_at DESC LIMIT 1
        ) latest_abi ON TRUE
        WHERE c.id = $1
        "#,
    )
    .bind(contract_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| ApiError::internal(format!("Failed to load contract for interoperability analysis: {err}")))
}

async fn load_candidate_contracts(
    pool: &PgPool,
    contract_id: Uuid,
    network: Network,
) -> ApiResult<Vec<ContractAnalysisRow>> {
    sqlx::query_as::<_, ContractAnalysisRow>(
        r#"
        SELECT c.id, c.contract_id, c.name, c.network, c.is_verified, c.category, c.tags, latest_abi.abi
        FROM contracts c
        LEFT JOIN LATERAL (
            SELECT abi FROM contract_abis WHERE contract_id = c.id ORDER BY created_at DESC LIMIT 1
        ) latest_abi ON TRUE
        WHERE c.id <> $1 AND c.network = $2
        ORDER BY c.is_verified DESC, c.created_at DESC
        LIMIT 150
        "#,
    )
    .bind(contract_id)
    .bind(network)
    .fetch_all(pool)
    .await
    .map_err(|err| ApiError::internal(format!("Failed to load interoperability candidates: {err}")))
}

async fn load_protocol_definitions(pool: &PgPool) -> ApiResult<Vec<ProtocolDefinitionRow>> {
    sqlx::query_as::<_, ProtocolDefinitionRow>(
        "SELECT slug, name, description, required_functions, optional_functions, bridge_indicators, adapter_indicators FROM protocol_definitions ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ApiError::internal(format!("Failed to load protocol definitions: {err}")))
}
async fn load_compatibility_hints(
    pool: &PgPool,
    contract_id: Uuid,
) -> ApiResult<HashMap<Uuid, i64>> {
    let rows = sqlx::query_as::<_, CompatibilityHintRow>(
        r#"
        SELECT CASE WHEN source_contract_id = $1 THEN target_contract_id ELSE source_contract_id END AS other_contract_id,
               COUNT(*)::BIGINT AS compatible_entries
        FROM contract_version_compatibility
        WHERE (source_contract_id = $1 OR target_contract_id = $1) AND is_compatible = TRUE
        GROUP BY CASE WHEN source_contract_id = $1 THEN target_contract_id ELSE source_contract_id END
        "#,
    )
    .bind(contract_id)
    .fetch_all(pool)
    .await
    .map_err(|err| ApiError::internal(format!("Failed to load compatibility hints: {err}")))?;

    Ok(rows
        .into_iter()
        .map(|row| (row.other_contract_id, row.compatible_entries))
        .collect())
}

async fn try_load_related_ids(
    pool: &PgPool,
    query: &str,
    contract_id: Uuid,
    warnings: &mut Vec<String>,
) -> HashSet<Uuid> {
    match sqlx::query_scalar::<_, Uuid>(query)
        .bind(contract_id)
        .fetch_all(pool)
        .await
    {
        Ok(ids) => ids.into_iter().collect(),
        Err(err) => {
            tracing::warn!(error = %err, contract_id = %contract_id, "failed to load dependency links for interoperability analysis");
            if warnings.is_empty()
                || !warnings
                    .iter()
                    .any(|warning| warning.contains("Dependency links"))
            {
                warnings.push("Dependency links were unavailable, so graph edges only include compatibility heuristics.".to_string());
            }
            HashSet::new()
        }
    }
}

fn extract_function_names(abi: Option<&Value>, contract_name: &str) -> BTreeSet<String> {
    let mut functions = BTreeSet::new();
    let Some(abi) = abi else {
        return functions;
    };

    if let Ok(parsed) = parse_json_spec(&abi.to_string(), contract_name) {
        for function in parsed.functions {
            functions.insert(function.name.to_lowercase());
        }
    }
    if !functions.is_empty() {
        return functions;
    }

    let entries: Vec<&Value> = if let Some(array) = abi.as_array() {
        array.iter().collect()
    } else if let Some(array) = abi.get("functions").and_then(Value::as_array) {
        array.iter().collect()
    } else if let Some(array) = abi.get("spec").and_then(Value::as_array) {
        array.iter().collect()
    } else {
        Vec::new()
    };

    for entry in entries {
        let kind = entry
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("function");
        if matches!(kind, "function" | "contract_fn" | "contract_function") {
            if let Some(name) = entry.get("name").and_then(Value::as_str) {
                functions.insert(name.to_lowercase());
            }
        }
    }
    functions
}

fn evaluate_protocols(
    definitions: &[ProtocolDefinitionRow],
    functions: &BTreeSet<String>,
) -> Vec<InteroperabilityProtocolMatch> {
    definitions
        .iter()
        .map(|definition| {
            let matched_functions: Vec<String> = definition
                .required_functions
                .iter()
                .filter(|name| functions.contains(*name))
                .cloned()
                .collect();
            let missing_functions: Vec<String> = definition
                .required_functions
                .iter()
                .filter(|name| !functions.contains(*name))
                .cloned()
                .collect();
            let optional_matches: Vec<String> = definition
                .optional_functions
                .iter()
                .filter(|name| functions.contains(*name))
                .cloned()
                .collect();
            let required_score = if definition.required_functions.is_empty() {
                0.0
            } else {
                matched_functions.len() as f64 / definition.required_functions.len() as f64
            };
            let optional_score = if definition.optional_functions.is_empty() {
                0.0
            } else {
                optional_matches.len() as f64 / definition.optional_functions.len() as f64
            };
            let status =
                if !definition.required_functions.is_empty() && missing_functions.is_empty() {
                    ProtocolComplianceStatus::Compliant
                } else if !matched_functions.is_empty() || !optional_matches.is_empty() {
                    ProtocolComplianceStatus::Partial
                } else {
                    ProtocolComplianceStatus::Unsupported
                };

            InteroperabilityProtocolMatch {
                slug: definition.slug.clone(),
                name: definition.name.clone(),
                description: definition.description.clone(),
                status,
                matched_functions,
                missing_functions,
                optional_matches,
                compliance_score: ((required_score * 80.0) + (optional_score * 20.0)).round()
                    as i32,
            }
        })
        .collect()
}

fn detect_capabilities(
    contract: &ContractAnalysisRow,
    functions: &BTreeSet<String>,
    protocols: &[InteroperabilityProtocolMatch],
    definitions: &[ProtocolDefinitionRow],
) -> Vec<InteroperabilityCapability> {
    let category = contract
        .category
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    let tags: HashSet<String> = contract.tags.iter().map(|tag| tag.to_lowercase()).collect();
    let supported = supported_protocols(protocols);
    let mut bridge_evidence = Vec::new();
    let mut adapter_evidence = Vec::new();

    if category.contains("bridge") || tags.contains("bridge") {
        bridge_evidence.push("Contract metadata marks this contract as a bridge.".to_string());
    }
    if category.contains("adapter") || tags.contains("adapter") || tags.contains("router") {
        adapter_evidence.push("Contract metadata suggests adapter or router behavior.".to_string());
    }

    for definition in definitions {
        if supported.contains_key(&definition.slug) {
            if definition.slug.contains("bridge") {
                bridge_evidence.push(format!(
                    "Matches the {} protocol definition.",
                    definition.name
                ));
            }
            if definition.slug.contains("adapter") {
                adapter_evidence.push(format!(
                    "Matches the {} protocol definition.",
                    definition.name
                ));
            }
        }
        for indicator in &definition.bridge_indicators {
            if functions.contains(indicator) {
                bridge_evidence.push(format!(
                    "Implements bridge-oriented function `{indicator}`."
                ));
            }
        }
        for indicator in &definition.adapter_indicators {
            if functions.contains(indicator) {
                adapter_evidence.push(format!(
                    "Implements adapter-oriented function `{indicator}`."
                ));
            }
        }
    }

    bridge_evidence.sort();
    bridge_evidence.dedup();
    adapter_evidence.sort();
    adapter_evidence.dedup();

    let mut capabilities = Vec::new();
    if !bridge_evidence.is_empty() {
        capabilities.push(InteroperabilityCapability {
            kind: InteroperabilityCapabilityKind::Bridge,
            label: "Bridge behavior detected".to_string(),
            confidence: if category.contains("bridge") || tags.contains("bridge") {
                0.95
            } else {
                (0.35 + (bridge_evidence.len() as f64 * 0.15)).min(0.9)
            },
            evidence: bridge_evidence,
        });
    }
    if !adapter_evidence.is_empty() {
        capabilities.push(InteroperabilityCapability {
            kind: InteroperabilityCapabilityKind::Adapter,
            label: "Adapter behavior detected".to_string(),
            confidence: if category.contains("adapter")
                || tags.contains("adapter")
                || tags.contains("router")
            {
                0.92
            } else {
                (0.3 + (adapter_evidence.len() as f64 * 0.14)).min(0.88)
            },
            evidence: adapter_evidence,
        });
    }
    capabilities
}

fn supported_protocols(
    protocols: &[InteroperabilityProtocolMatch],
) -> HashMap<String, InteroperabilityProtocolMatch> {
    protocols
        .iter()
        .filter(|protocol| !matches!(protocol.status, ProtocolComplianceStatus::Unsupported))
        .cloned()
        .map(|protocol| (protocol.slug.clone(), protocol))
        .collect()
}

fn has_capability(
    capabilities: &[InteroperabilityCapability],
    target_kind: InteroperabilityCapabilityKind,
) -> bool {
    capabilities
        .iter()
        .any(|capability| match (&capability.kind, &target_kind) {
            (InteroperabilityCapabilityKind::Bridge, InteroperabilityCapabilityKind::Bridge) => {
                true
            }
            (InteroperabilityCapabilityKind::Adapter, InteroperabilityCapabilityKind::Adapter) => {
                true
            }
            _ => false,
        })
}

fn count_capabilities(
    capabilities: &[InteroperabilityCapability],
    target_kind: InteroperabilityCapabilityKind,
) -> usize {
    capabilities
        .iter()
        .filter(|capability| match (&capability.kind, &target_kind) {
            (InteroperabilityCapabilityKind::Bridge, InteroperabilityCapabilityKind::Bridge) => {
                true
            }
            (InteroperabilityCapabilityKind::Adapter, InteroperabilityCapabilityKind::Adapter) => {
                true
            }
            _ => false,
        })
        .count()
}
fn score_candidate(
    target: &ContractAnalysisRow,
    target_functions: &BTreeSet<String>,
    target_protocols: &HashMap<String, InteroperabilityProtocolMatch>,
    target_is_bridge: bool,
    target_is_adapter: bool,
    candidate: &CandidateAnalysis,
    compatibility_hints: &HashMap<Uuid, i64>,
    direct_dependencies: &HashSet<Uuid>,
    direct_dependents: &HashSet<Uuid>,
) -> Option<ScoredSuggestion> {
    let candidate_protocols = supported_protocols(&candidate.protocols);
    let mut shared_protocols = Vec::new();
    for (slug, protocol) in target_protocols {
        if candidate_protocols.contains_key(slug) {
            shared_protocols.push(protocol.name.clone());
        }
    }
    shared_protocols.sort();

    let shared_functions: Vec<String> = target_functions
        .intersection(&candidate.functions)
        .take(8)
        .cloned()
        .collect();
    let explicit_compatibility = compatibility_hints
        .get(&candidate.row.id)
        .copied()
        .unwrap_or(0);
    let is_dependency = direct_dependencies.contains(&candidate.row.id);
    let is_dependent = direct_dependents.contains(&candidate.row.id);
    let same_category = target
        .category
        .as_ref()
        .zip(candidate.row.category.as_ref())
        .map(|(left, right)| left.eq_ignore_ascii_case(right))
        .unwrap_or(false);
    let complementary_roles = (target_is_bridge
        && has_capability(
            &candidate.capabilities,
            InteroperabilityCapabilityKind::Adapter,
        ))
        || (target_is_adapter
            && has_capability(
                &candidate.capabilities,
                InteroperabilityCapabilityKind::Bridge,
            ));

    let mut relation_types = Vec::new();
    let mut score = 0.0;
    if explicit_compatibility > 0 {
        score += 28.0 + (explicit_compatibility.min(3) as f64 * 4.0);
        relation_types.push("compatibility_matrix".to_string());
    }
    if is_dependency {
        score += 18.0;
        relation_types.push("depends_on".to_string());
    }
    if is_dependent {
        score += 14.0;
        relation_types.push("depended_on_by".to_string());
    }
    if !shared_protocols.is_empty() {
        score += (shared_protocols.len() as f64 * 12.0).min(30.0);
        relation_types.push("shared_protocol".to_string());
    }
    if !shared_functions.is_empty() {
        score += (shared_functions.len() as f64 * 2.5).min(15.0);
        relation_types.push("abi_overlap".to_string());
    }
    if same_category {
        score += 8.0;
        relation_types.push("same_category".to_string());
    }
    if complementary_roles {
        score += 14.0;
        relation_types.push("bridge_adapter".to_string());
    }
    if candidate.row.is_verified {
        score += 6.0;
    }
    if target.network == candidate.row.network {
        score += 4.0;
    }
    if score < 18.0 {
        return None;
    }

    Some(ScoredSuggestion {
        candidate: candidate.clone(),
        score: score.min(100.0),
        reason: summarize_relations(&relation_types, &shared_protocols, &shared_functions),
        shared_protocols,
        shared_functions,
        relation_types,
    })
}

fn summarize_relations(
    relation_types: &[String],
    shared_protocols: &[String],
    shared_functions: &[String],
) -> String {
    let mut parts = Vec::new();
    if relation_types
        .iter()
        .any(|relation| relation == "compatibility_matrix")
    {
        parts.push("existing compatibility entries already link these contracts".to_string());
    }
    if relation_types
        .iter()
        .any(|relation| relation == "shared_protocol")
        && !shared_protocols.is_empty()
    {
        parts.push(format!("both align with {}", shared_protocols.join(", ")));
    }
    if relation_types
        .iter()
        .any(|relation| relation == "bridge_adapter")
    {
        parts.push("their bridge and adapter behaviors complement each other".to_string());
    }
    if relation_types
        .iter()
        .any(|relation| relation == "depends_on")
    {
        parts.push("this contract already depends on it".to_string());
    }
    if relation_types
        .iter()
        .any(|relation| relation == "depended_on_by")
    {
        parts.push("it already depends on this contract".to_string());
    }
    if relation_types
        .iter()
        .any(|relation| relation == "abi_overlap")
        && !shared_functions.is_empty()
    {
        parts.push(format!(
            "they share callable surfaces like {}",
            shared_functions.join(", ")
        ));
    }
    if relation_types
        .iter()
        .any(|relation| relation == "same_category")
    {
        parts.push("they live in the same contract category".to_string());
    }
    if parts.is_empty() {
        "Their ABI and metadata suggest they can interoperate cleanly.".to_string()
    } else {
        format!("{}.", parts.join("; "))
    }
}

fn build_graph(
    contract: &ContractAnalysisRow,
    candidates: &HashMap<Uuid, CandidateAnalysis>,
    suggestions: &[ScoredSuggestion],
    compatibility_hints: &HashMap<Uuid, i64>,
    direct_dependencies: &HashSet<Uuid>,
    direct_dependents: &HashSet<Uuid>,
) -> GraphResponse {
    let mut nodes = vec![to_graph_node(contract)];
    let mut seen_nodes = HashSet::from([contract.id]);
    let mut edges = Vec::new();
    let mut seen_edges = HashSet::new();

    for suggestion in suggestions {
        push_node(&mut nodes, &mut seen_nodes, &suggestion.candidate.row);
        push_edge(
            &mut edges,
            &mut seen_edges,
            contract.id,
            suggestion.candidate.row.id,
            primary_relation(&suggestion.relation_types),
            suggestion.score.round() as i64,
        );
    }
    for (other_id, score) in compatibility_hints {
        if let Some(candidate) = candidates.get(other_id) {
            push_node(&mut nodes, &mut seen_nodes, &candidate.row);
            push_edge(
                &mut edges,
                &mut seen_edges,
                contract.id,
                *other_id,
                "compatible_with".to_string(),
                *score,
            );
        }
    }
    for dependency_id in direct_dependencies {
        if let Some(candidate) = candidates.get(dependency_id) {
            push_node(&mut nodes, &mut seen_nodes, &candidate.row);
            push_edge(
                &mut edges,
                &mut seen_edges,
                contract.id,
                *dependency_id,
                "depends_on".to_string(),
                50,
            );
        }
    }
    for dependent_id in direct_dependents {
        if let Some(candidate) = candidates.get(dependent_id) {
            push_node(&mut nodes, &mut seen_nodes, &candidate.row);
            push_edge(
                &mut edges,
                &mut seen_edges,
                *dependent_id,
                contract.id,
                "depends_on".to_string(),
                45,
            );
        }
    }

    GraphResponse { nodes, edges }
}

fn to_graph_node(contract: &ContractAnalysisRow) -> GraphNode {
    GraphNode {
        id: contract.id,
        contract_id: contract.contract_id.clone(),
        name: contract.name.clone(),
        network: contract.network.clone(),
        is_verified: contract.is_verified,
        category: contract.category.clone(),
        tags: contract
            .tags
            .iter()
            .map(|tag| Tag {
                id: Uuid::nil(),
                name: tag.clone(),
                color: String::new(),
            })
            .collect(),
    }
}

fn push_node(
    nodes: &mut Vec<GraphNode>,
    seen_nodes: &mut HashSet<Uuid>,
    contract: &ContractAnalysisRow,
) {
    if seen_nodes.insert(contract.id) {
        nodes.push(to_graph_node(contract));
    }
}

fn push_edge(
    edges: &mut Vec<GraphEdge>,
    seen_edges: &mut HashSet<(Uuid, Uuid, String)>,
    source: Uuid,
    target: Uuid,
    relation: String,
    strength: i64,
) {
    if seen_edges.insert((source, target, relation.clone())) {
        edges.push(GraphEdge {
            source,
            target,
            dependency_type: relation,
            call_frequency: Some(strength),
            call_volume: Some(strength),
            is_estimated: true,
            is_circular: false,
        });
    }
}

fn primary_relation(relation_types: &[String]) -> String {
    for candidate in [
        "compatibility_matrix",
        "bridge_adapter",
        "depends_on",
        "depended_on_by",
        "shared_protocol",
        "abi_overlap",
        "same_category",
    ] {
        if relation_types.iter().any(|relation| relation == candidate) {
            return match candidate {
                "compatibility_matrix" => "compatible_with".to_string(),
                other => other.to_string(),
            };
        }
    }
    "interoperable".to_string()
}

fn sort_protocols(
    mut protocols: Vec<InteroperabilityProtocolMatch>,
) -> Vec<InteroperabilityProtocolMatch> {
    protocols.sort_by(|left, right| {
        right
            .compliance_score
            .cmp(&left.compliance_score)
            .then_with(|| left.name.cmp(&right.name))
    });
    protocols
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(
        slug: &str,
        name: &str,
        required: &[&str],
        optional: &[&str],
        bridge: &[&str],
        adapter: &[&str],
    ) -> ProtocolDefinitionRow {
        ProtocolDefinitionRow {
            slug: slug.to_string(),
            name: name.to_string(),
            description: format!("{name} description"),
            required_functions: required.iter().map(|value| value.to_string()).collect(),
            optional_functions: optional.iter().map(|value| value.to_string()).collect(),
            bridge_indicators: bridge.iter().map(|value| value.to_string()).collect(),
            adapter_indicators: adapter.iter().map(|value| value.to_string()).collect(),
        }
    }

    #[test]
    fn protocol_evaluation_marks_compliant_when_required_functions_exist() {
        let definitions = vec![definition(
            "token-interface",
            "Token Interface",
            &["balance", "transfer"],
            &["approve"],
            &[],
            &[],
        )];
        let functions = BTreeSet::from([
            "balance".to_string(),
            "transfer".to_string(),
            "approve".to_string(),
        ]);
        let results = evaluate_protocols(&definitions, &functions);
        assert!(matches!(
            results[0].status,
            ProtocolComplianceStatus::Compliant
        ));
        assert_eq!(results[0].compliance_score, 100);
    }

    #[test]
    fn bridge_capability_uses_protocol_and_metadata_signals() {
        let definitions = vec![definition(
            "bridge-settlement",
            "Bridge Settlement",
            &["lock", "release"],
            &["claim"],
            &["lock", "release", "claim"],
            &[],
        )];
        let row = ContractAnalysisRow {
            id: Uuid::new_v4(),
            contract_id: "Cbridge".to_string(),
            name: "bridge".to_string(),
            network: Network::Mainnet,
            is_verified: true,
            category: Some("Bridge".to_string()),
            tags: vec!["bridge".to_string()],
            abi: None,
        };
        let functions = BTreeSet::from([
            "lock".to_string(),
            "release".to_string(),
            "claim".to_string(),
        ]);
        let protocols = evaluate_protocols(&definitions, &functions);
        let capabilities = detect_capabilities(&row, &functions, &protocols, &definitions);
        assert!(has_capability(
            &capabilities,
            InteroperabilityCapabilityKind::Bridge
        ));
    }
}
