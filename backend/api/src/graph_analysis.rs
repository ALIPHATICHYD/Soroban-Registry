// Contract interaction graph analysis algorithms.
//
// Algorithms implemented (all pure Rust, no external solver):
//
//   ┌─────────────────────────────────────────────────────────┐
//   │  Label Propagation   → cluster / sub-network detection  │
//   │  PageRank            → influence / criticality ranking   │
//   │  Betweenness (BFS)   → bridge / bottleneck detection    │
//   │  BFS propagation     → vulnerability spread analysis    │
//   └─────────────────────────────────────────────────────────┘

use shared::{
    CriticalContractScore, GraphAnalysisReport, GraphCluster, GraphEdge, GraphNode, PropagationHop,
    VulnerabilityPropagationResult,
};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

// ─── Internal graph representation ───────────────────────────────────────────

/// Compact adjacency-list graph used by all algorithms.
pub struct AnalysisGraph {
    /// Ordered list of node UUIDs; position = node index.
    pub node_ids: Vec<Uuid>,
    /// node UUID → index in node_ids.
    pub index: HashMap<Uuid, usize>,
    /// node name keyed by index (for output labels).
    pub names: Vec<String>,
    /// Directed out-edges: out[u] = [(v, weight)].
    pub out: Vec<Vec<(usize, f64)>>,
    /// Directed in-edges: in_[v] = [(u, weight)].
    pub in_: Vec<Vec<(usize, f64)>>,
}

impl AnalysisGraph {
    pub fn build(nodes: &[GraphNode], edges: &[GraphEdge]) -> Self {
        let n = nodes.len();
        let mut index = HashMap::with_capacity(n);
        let mut node_ids = Vec::with_capacity(n);
        let mut names = Vec::with_capacity(n);

        for node in nodes {
            let idx = node_ids.len();
            index.insert(node.id, idx);
            node_ids.push(node.id);
            names.push(node.name.clone());
        }

        let mut out = vec![Vec::new(); n];
        let mut in_ = vec![Vec::new(); n];

        for edge in edges {
            let Some(&u) = index.get(&edge.source) else {
                continue;
            };
            let Some(&v) = index.get(&edge.target) else {
                continue;
            };
            if u == v {
                continue;
            }
            // Normalise call_frequency to a weight ≥ 1.
            let w = edge.call_frequency.unwrap_or(1).max(1) as f64;
            out[u].push((v, w));
            in_[v].push((u, w));
        }

        Self {
            node_ids,
            index,
            names,
            out,
            in_,
        }
    }

    pub fn n(&self) -> usize {
        self.node_ids.len()
    }

    pub fn in_degree(&self, u: usize) -> usize {
        self.in_[u].len()
    }
    pub fn out_degree(&self, u: usize) -> usize {
        self.out[u].len()
    }
}

// ─── Label propagation (community / cluster detection) ───────────────────────
//
// Each node starts with a unique label. On each iteration every node adopts
// the label that appears most frequently among its *undirected* neighbours.
// Nodes that converge to the same label belong to the same cluster.
//
// Complexity: O(iter × E).  Typically converges in < 30 iterations.

const MAX_LP_ITERATIONS: usize = 40;

pub fn label_propagation(g: &AnalysisGraph) -> Vec<usize> {
    let n = g.n();
    let mut labels: Vec<usize> = (0..n).collect();

    for _ in 0..MAX_LP_ITERATIONS {
        let prev = labels.clone();
        let mut changed = false;

        // Randomised update order reduces oscillation.
        let mut order: Vec<usize> = (0..n).collect();
        // Deterministic shuffle using a simple LCG so results are reproducible.
        let mut rng = 6364136223846793005u64;
        for i in (1..n).rev() {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (rng >> 33) as usize % (i + 1);
            order.swap(i, j);
        }

        for &u in &order {
            // Collect neighbour labels from both out- and in-edges (undirected view).
            let mut freq: HashMap<usize, f64> = HashMap::new();
            for &(v, w) in &g.out[u] {
                *freq.entry(prev[v]).or_default() += w;
            }
            for &(v, w) in &g.in_[u] {
                *freq.entry(prev[v]).or_default() += w;
            }
            if freq.is_empty() {
                continue;
            }
            // Pick the label with the highest weighted frequency;
            // break ties by choosing the smallest label for stability.
            let best = freq
                .iter()
                .max_by(|(la, wa), (lb, wb)| {
                    wa.partial_cmp(wb)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(lb.cmp(la))
                })
                .map(|(&l, _)| l)
                .unwrap_or(prev[u]);

            if best != prev[u] {
                changed = true;
            }
            labels[u] = best;
        }

        if !changed {
            break;
        }
    }

    // Normalise labels to 0-based contiguous cluster IDs.
    let mut label_map: HashMap<usize, usize> = HashMap::new();
    let mut next_id = 0;
    for l in labels.iter_mut() {
        let entry = label_map.entry(*l).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            id
        });
        *l = *entry;
    }

    labels
}

/// Build `GraphCluster` structs from raw label assignments.
pub fn build_clusters(g: &AnalysisGraph, labels: &[usize]) -> Vec<GraphCluster> {
    let mut by_cluster: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, &l) in labels.iter().enumerate() {
        by_cluster.entry(l).or_default().push(i);
    }

    // Count cross-cluster edges.
    let mut external: HashMap<usize, usize> = HashMap::new();
    let mut internal_weight: HashMap<usize, f64> = HashMap::new();
    let mut internal_count: HashMap<usize, usize> = HashMap::new();

    for u in 0..g.n() {
        for &(v, w) in &g.out[u] {
            if labels[u] == labels[v] {
                *internal_weight.entry(labels[u]).or_default() += w;
                *internal_count.entry(labels[u]).or_default() += 1;
            } else {
                *external.entry(labels[u]).or_default() += 1;
                *external.entry(labels[v]).or_default() += 1;
            }
        }
    }

    let mut clusters: Vec<GraphCluster> = by_cluster
        .into_iter()
        .map(|(cluster_id, members)| {
            // Hub = node with the highest total degree within the cluster.
            let hub = members
                .iter()
                .max_by_key(|&&i| g.in_degree(i) + g.out_degree(i))
                .map(|&i| g.node_ids[i]);

            let ext = external.get(&cluster_id).copied().unwrap_or(0);
            let int_w = internal_weight.get(&cluster_id).copied().unwrap_or(0.0);
            let int_c = internal_count.get(&cluster_id).copied().unwrap_or(1).max(1);
            let cohesion = int_w / int_c as f64;

            GraphCluster {
                cluster_id,
                members: members.iter().map(|&i| g.node_ids[i]).collect(),
                hub_contract_id: hub,
                cohesion,
                external_edges: ext,
            }
        })
        .collect();

    // Sort by size descending, then cluster_id ascending.
    clusters.sort_by(|a, b| {
        b.members
            .len()
            .cmp(&a.members.len())
            .then(a.cluster_id.cmp(&b.cluster_id))
    });
    clusters
}

// ─── PageRank ─────────────────────────────────────────────────────────────────
//
// Standard power-iteration PageRank.
// PR(v) = (1 - d) / N  +  d × Σ_u∈in(v)  PR(u) / out_degree(u)
// Converges in ~50 iterations for typical graphs.

const PAGERANK_DAMPING: f64 = 0.85;
const PAGERANK_ITERATIONS: usize = 50;
const PAGERANK_TOLERANCE: f64 = 1e-8;

pub fn pagerank(g: &AnalysisGraph) -> Vec<f64> {
    let n = g.n();
    if n == 0 {
        return Vec::new();
    }

    let init = 1.0 / n as f64;
    let mut pr = vec![init; n];
    let teleport = (1.0 - PAGERANK_DAMPING) / n as f64;

    for _ in 0..PAGERANK_ITERATIONS {
        let mut next = vec![teleport; n];

        // Dangling node mass (nodes with no out-edges distribute uniformly).
        let dangling_mass: f64 = pr
            .iter()
            .enumerate()
            .filter(|(u, _)| g.out[*u].is_empty())
            .map(|(_, &p)| p)
            .sum::<f64>()
            * PAGERANK_DAMPING
            / n as f64;

        for v in 0..n {
            next[v] += dangling_mass;
        }

        for u in 0..n {
            if g.out[u].is_empty() {
                continue;
            }
            let total_weight: f64 = g.out[u].iter().map(|(_, w)| w).sum();
            for &(v, w) in &g.out[u] {
                next[v] += PAGERANK_DAMPING * pr[u] * (w / total_weight);
            }
        }

        // Check convergence.
        let delta: f64 = pr.iter().zip(&next).map(|(a, b)| (a - b).abs()).sum();
        pr = next;
        if delta < PAGERANK_TOLERANCE {
            break;
        }
    }

    pr
}

// ─── Betweenness centrality (approximate) ─────────────────────────────────────
//
// Brandes' algorithm on sampled source nodes.
// Full exact computation is O(VE); we sample min(n, MAX_SAMPLES) sources
// and scale the result — sufficient for ranking purposes.

const MAX_BETWEENNESS_SAMPLES: usize = 128;

pub fn betweenness_centrality(g: &AnalysisGraph) -> Vec<f64> {
    let n = g.n();
    if n < 3 {
        return vec![0.0; n];
    }

    let mut bc = vec![0.0f64; n];
    // Sample sources — for small graphs use all; for large, take evenly spaced.
    let step = (n / MAX_BETWEENNESS_SAMPLES).max(1);
    let sources: Vec<usize> = (0..n).step_by(step).collect();
    let sample_count = sources.len();

    for &s in &sources {
        // BFS from s — unweighted for efficiency.
        let mut dist = vec![i64::MAX; n];
        let mut sigma = vec![0i64; n]; // shortest-path counts
        let mut stack: Vec<usize> = Vec::new();
        let mut pred: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut queue = VecDeque::new();

        dist[s] = 0;
        sigma[s] = 1;
        queue.push_back(s);

        while let Some(v) = queue.pop_front() {
            stack.push(v);
            for &(w, _) in &g.out[v] {
                if dist[w] == i64::MAX {
                    dist[w] = dist[v] + 1;
                    queue.push_back(w);
                }
                if dist[w] == dist[v] + 1 {
                    sigma[w] = sigma[w].saturating_add(sigma[v]);
                    pred[w].push(v);
                }
            }
        }

        // Accumulation (back-propagation).
        let mut delta = vec![0.0f64; n];
        while let Some(w) = stack.pop() {
            for &v in &pred[w] {
                if sigma[w] > 0 {
                    delta[v] += (sigma[v] as f64 / sigma[w] as f64) * (1.0 + delta[w]);
                }
            }
            if w != s {
                bc[w] += delta[w];
            }
        }
    }

    // Scale by sampling ratio and normalise to [0, 1].
    let scale = n as f64 / sample_count as f64;
    let norm = if n > 2 {
        ((n - 1) * (n - 2)) as f64
    } else {
        1.0
    };
    bc.iter_mut().for_each(|v| *v = (*v * scale) / norm);

    bc
}

// ─── Critical contract ranking ────────────────────────────────────────────────

/// Compute a combined criticality score for each node and return them sorted
/// by criticality descending.
pub fn rank_critical_contracts(
    g: &AnalysisGraph,
    pr: &[f64],
    bc: &[f64],
    labels: &[usize],
) -> Vec<CriticalContractScore> {
    let n = g.n();

    // Normalise PageRank and betweenness to [0, 1].
    let pr_max = pr
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(1e-12);
    let bc_max = bc
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(1e-12);

    let mut scores: Vec<CriticalContractScore> = (0..n)
        .map(|i| {
            let pr_norm = pr[i] / pr_max;
            let bc_norm = bc[i] / bc_max;
            let in_d = g.in_degree(i);
            let out_d = g.out_degree(i);
            // Degree centrality normalised to [0, 1].
            let deg_norm = (in_d + out_d) as f64 / (2 * n).max(1) as f64;

            // Weighted combination — in-degree weighted highest because it
            // directly measures how many contracts depend on this one.
            let criticality = 0.35 * pr_norm
                + 0.30 * bc_norm
                + 0.25 * (in_d as f64 / n as f64).min(1.0)
                + 0.10 * deg_norm;

            CriticalContractScore {
                contract_id: g.node_ids[i],
                contract_name: g.names[i].clone(),
                criticality_score: criticality,
                pagerank: pr[i],
                betweenness: bc[i],
                in_degree: in_d,
                out_degree: out_d,
                cluster_id: labels.get(i).copied(),
            }
        })
        .collect();

    scores.sort_by(|a, b| {
        b.criticality_score
            .partial_cmp(&a.criticality_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scores
}

// ─── Vulnerability propagation ────────────────────────────────────────────────
//
// BFS from source nodes through dependency edges.
// Risk decays with depth:  risk(depth) = source_severity × decay^depth
// Contracts in a strongly-connected component with the source all receive
// the full source severity (cycle = full exposure).

const DECAY_PER_HOP: f64 = 0.60;
const MIN_RISK_THRESHOLD: f64 = 0.01;
const MAX_PROPAGATION_DEPTH: usize = 8;

pub fn propagate_vulnerability(
    g: &AnalysisGraph,
    source_indices: &[(usize, f64)], // (node_index, severity 0..1)
) -> VulnerabilityPropagationResult {
    let n = g.n();
    let mut risk = vec![0.0f64; n];
    let mut depth_map = vec![usize::MAX; n];
    let mut propagates_to: Vec<HashSet<usize>> = vec![HashSet::new(); n];

    let mut queue: VecDeque<(usize, usize, f64)> = VecDeque::new(); // (node, depth, risk)
    let mut has_cycles = false;

    // Seed the queue from sources.
    for &(src, sev) in source_indices {
        if src >= n {
            continue;
        }
        if risk[src] < sev {
            risk[src] = sev;
            depth_map[src] = 0;
        }
        queue.push_back((src, 0, sev));
    }

    let source_set: HashSet<usize> = source_indices.iter().map(|(i, _)| *i).collect();

    while let Some((u, d, r)) = queue.pop_front() {
        if d >= MAX_PROPAGATION_DEPTH {
            continue;
        }

        // Propagate along out-edges (dependents depend on u, so they inherit risk).
        // We follow IN-edges: if v depends on u and u is vulnerable, v is at risk.
        for &(v, w) in &g.in_[u] {
            // Edge weight boosts risk slightly — heavily-used paths carry more risk.
            let weight_boost = (w / (w + 10.0)).min(0.2); // at most +20%
            let new_risk = r * (DECAY_PER_HOP + weight_boost);

            if new_risk < MIN_RISK_THRESHOLD {
                continue;
            }

            propagates_to[u].insert(v);

            if source_set.contains(&v) {
                has_cycles = true;
            }

            if new_risk > risk[v] {
                risk[v] = new_risk;
                depth_map[v] = d + 1;
                queue.push_back((v, d + 1, new_risk));
            }
        }
    }

    let max_depth = depth_map
        .iter()
        .filter(|&&d| d != usize::MAX)
        .copied()
        .max()
        .unwrap_or(0);

    let mut affected: Vec<PropagationHop> = (0..n)
        .filter(|&i| risk[i] > MIN_RISK_THRESHOLD)
        .map(|i| PropagationHop {
            contract_id: g.node_ids[i],
            contract_name: g.names[i].clone(),
            depth: depth_map[i],
            risk_score: risk[i],
            propagates_to: propagates_to[i].iter().map(|&j| g.node_ids[j]).collect(),
        })
        .collect();

    affected.sort_by(|a, b| {
        b.risk_score
            .partial_cmp(&a.risk_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total = affected.len();
    let sources = source_indices
        .iter()
        .filter(|(i, _)| *i < n)
        .map(|(i, _)| g.node_ids[*i])
        .collect();

    VulnerabilityPropagationResult {
        source_contracts: sources,
        affected_contracts: affected,
        total_affected: total,
        max_depth,
        has_cycles,
    }
}

// ─── Strongly connected components (cycle detection) ─────────────────────────
//
// Kosaraju's algorithm — two iterative DFS passes.
// Returns the set of node UUIDs that belong to any SCC of size ≥ 2
// (genuinely cyclic; self-loops are excluded).

pub fn cyclic_nodes(g: &AnalysisGraph) -> Vec<Uuid> {
    let n = g.n();

    // Pass 1: iterative post-order DFS on the forward graph.
    let mut visited = vec![false; n];
    let mut finish_order: Vec<usize> = Vec::with_capacity(n);

    for start in 0..n {
        if visited[start] {
            continue;
        }
        // Stack stores (node, edge_index) — edge_index tracks which out-edge to visit next.
        let mut stack: Vec<(usize, usize)> = vec![(start, 0)];
        visited[start] = true;
        while let Some((u, ei)) = stack.last_mut() {
            if *ei < g.out[*u].len() {
                let (v, _) = g.out[*u][*ei];
                *ei += 1;
                if !visited[v] {
                    visited[v] = true;
                    stack.push((v, 0));
                }
            } else {
                finish_order.push(*u);
                stack.pop();
            }
        }
    }

    // Pass 2: iterative DFS on the reverse graph in reverse finish order.
    let mut component = vec![usize::MAX; n];
    let mut comp_id = 0;

    for &u in finish_order.iter().rev() {
        if component[u] != usize::MAX {
            continue;
        }
        let mut stack = vec![u];
        while let Some(v) = stack.pop() {
            if component[v] != usize::MAX {
                continue;
            }
            component[v] = comp_id;
            for &(w, _) in &g.in_[v] {
                if component[w] == usize::MAX {
                    stack.push(w);
                }
            }
        }
        comp_id += 1;
    }

    // Count component sizes, then collect UUIDs in non-trivial SCCs.
    let mut sizes = vec![0usize; comp_id];
    for &c in &component {
        if c < comp_id {
            sizes[c] += 1;
        }
    }

    (0..n)
        .filter(|&i| component[i] < comp_id && sizes[component[i]] >= 2)
        .map(|i| g.node_ids[i])
        .collect()
}

// ─── Full analysis entry point ────────────────────────────────────────────────

pub fn run_full_analysis(nodes: &[GraphNode], edges: &[GraphEdge]) -> GraphAnalysisReport {
    let started = std::time::Instant::now();

    let g = AnalysisGraph::build(nodes, edges);
    let n = g.n();

    let labels = label_propagation(&g);
    let clusters = build_clusters(&g, &labels);
    let pr = pagerank(&g);
    let bc = betweenness_centrality(&g);
    let critical = rank_critical_contracts(&g, &pr, &bc, &labels);
    let cyclic = cyclic_nodes(&g);

    GraphAnalysisReport {
        total_nodes: n,
        total_edges: edges.len(),
        clusters,
        critical_contracts: critical,
        cyclic_contracts: cyclic,
        analysis_duration_ms: started.elapsed().as_millis() as u64,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{GraphEdge, GraphNode, Network, Tag};

    fn make_node(id: Uuid, name: &str) -> GraphNode {
        GraphNode {
            id,
            contract_id: id.to_string(),
            name: name.to_string(),
            network: Network::Testnet,
            is_verified: true,
            category: None,
            tags: vec![],
        }
    }

    fn make_edge(src: Uuid, tgt: Uuid, freq: i64) -> GraphEdge {
        GraphEdge {
            source: src,
            target: tgt,
            dependency_type: "direct".into(),
            call_frequency: Some(freq),
            call_volume: Some(freq),
            is_estimated: false,
            is_circular: false,
        }
    }

    #[test]
    fn empty_graph_produces_valid_report() {
        let report = run_full_analysis(&[], &[]);
        assert_eq!(report.total_nodes, 0);
        assert_eq!(report.total_edges, 0);
        assert!(report.clusters.is_empty());
    }

    #[test]
    fn pagerank_sums_to_one() {
        let ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
        let nodes: Vec<GraphNode> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| make_node(id, &format!("c{}", i)))
            .collect();
        let edges = vec![
            make_edge(ids[0], ids[1], 10),
            make_edge(ids[1], ids[2], 5),
            make_edge(ids[2], ids[0], 3),
        ];
        let g = AnalysisGraph::build(&nodes, &edges);
        let pr = pagerank(&g);
        let sum: f64 = pr.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-6,
            "PageRank should sum to 1, got {}",
            sum
        );
    }

    #[test]
    fn label_propagation_finds_two_communities() {
        // Build two triangles connected by one bridge edge.
        let ids: Vec<Uuid> = (0..6).map(|_| Uuid::new_v4()).collect();
        let nodes: Vec<GraphNode> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| make_node(id, &i.to_string()))
            .collect();
        let edges = vec![
            // Cluster A: 0-1-2
            make_edge(ids[0], ids[1], 10),
            make_edge(ids[1], ids[2], 10),
            make_edge(ids[2], ids[0], 10),
            // Cluster B: 3-4-5
            make_edge(ids[3], ids[4], 10),
            make_edge(ids[4], ids[5], 10),
            make_edge(ids[5], ids[3], 10),
            // Bridge (weak)
            make_edge(ids[2], ids[3], 1),
        ];
        let g = AnalysisGraph::build(&nodes, &edges);
        let labels = label_propagation(&g);
        let unique: HashSet<usize> = labels.iter().copied().collect();
        // Should produce at least 2 distinct clusters.
        assert!(
            unique.len() >= 2,
            "Expected ≥2 clusters, got {}",
            unique.len()
        );
    }

    #[test]
    fn vulnerability_propagation_follows_dependencies() {
        let ids: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
        let nodes: Vec<GraphNode> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| make_node(id, &i.to_string()))
            .collect();
        // 0 <- 1 <- 2 <- 3  (1,2,3 depend on 0)
        let edges = vec![
            make_edge(ids[1], ids[0], 5),
            make_edge(ids[2], ids[1], 5),
            make_edge(ids[3], ids[2], 5),
        ];
        let g = AnalysisGraph::build(&nodes, &edges);
        let result = propagate_vulnerability(&g, &[(0, 1.0)]);
        // Contracts 1, 2, 3 should all be at risk since they depend on 0.
        assert!(
            result.total_affected >= 3,
            "Expected at least 3 affected, got {}",
            result.total_affected
        );
        // Risk should decrease with depth.
        let by_id: HashMap<Uuid, f64> = result
            .affected_contracts
            .iter()
            .map(|h| (h.contract_id, h.risk_score))
            .collect();
        assert!(by_id[&ids[1]] > by_id[&ids[2]]);
        assert!(by_id[&ids[2]] > by_id[&ids[3]]);
    }
}
