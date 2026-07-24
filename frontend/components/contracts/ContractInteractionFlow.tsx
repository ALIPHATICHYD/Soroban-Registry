"use client";

import { useQuery } from "@tanstack/react-query";
import { api, GraphNode } from "@/lib/api";
import DependencyGraph from "@/components/DependencyGraph";
import GraphControls from "@/components/GraphControls";
import { useState, useCallback, useRef, useMemo, useEffect } from "react";
import { AlertCircle, ExternalLink, X, Info } from "lucide-react";
import type { DependencyGraphHandle } from "@/components/DependencyGraph";
import { useRouter } from "next/navigation";

interface ContractInteractionFlowProps {
  contractId: string;
}

export default function ContractInteractionFlow({
  contractId,
}: ContractInteractionFlowProps) {
  const [depth, setDepth] = useState(1);
  const [networkFilter, setNetworkFilter] = useState<string>("");
  const [dependencyTypeFilter, setDependencyTypeFilter] = useState<string>("");
  const [showCyclesOnly, setShowCyclesOnly] = useState(false);
  const [minCallFrequency, setMinCallFrequency] = useState(0);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [searchMatchIndex, setSearchMatchIndex] = useState(0);
  const graphRef = useRef<DependencyGraphHandle | null>(null);
  const router = useRouter();

  const {
    data: graphData,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ["contract-local-graph", contractId, depth],
    queryFn: () => api.getContractLocalGraph(contractId, depth),
  });

  const rawNodes = useMemo(
    () => (graphData && Array.isArray(graphData.nodes) ? graphData.nodes : []),
    [graphData],
  );
  const rawEdges = useMemo(
    () => (graphData && Array.isArray(graphData.edges) ? graphData.edges : []),
    [graphData],
  );

  const filteredGraph = useMemo(() => {
    const edgeFiltered = rawEdges.filter((edge) => {
      if (dependencyTypeFilter && edge.dependency_type !== dependencyTypeFilter)
        return false;
      if (showCyclesOnly && !edge.is_circular) return false;
      if (minCallFrequency > 0 && (edge.call_frequency ?? 0) < minCallFrequency)
        return false;
      return true;
    });

    const nodeIds = new Set<string>();
    for (const edge of edgeFiltered) {
      nodeIds.add(edge.source);
      nodeIds.add(edge.target);
    }

    // Always keep the root contract
    const rootNode = rawNodes.find(
      (n) => n.id === contractId || n.contract_id === contractId,
    );
    if (rootNode) nodeIds.add(rootNode.id);

    return {
      nodes: rawNodes.filter(
        (node) =>
          nodeIds.has(node.id) &&
          (!networkFilter || node.network === networkFilter),
      ),
      edges: edgeFiltered,
    };
  }, [
    rawNodes,
    rawEdges,
    dependencyTypeFilter,
    showCyclesOnly,
    minCallFrequency,
    networkFilter,
    contractId,
  ]);

  const nodes = filteredGraph.nodes;
  const edges = filteredGraph.edges;

  const dependentCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const edge of edges) {
      counts.set(edge.target, (counts.get(edge.target) || 0) + 1);
    }
    return counts;
  }, [edges]);

  const dependencyCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const edge of edges) {
      counts.set(edge.source, (counts.get(edge.source) || 0) + 1);
    }
    return counts;
  }, [edges]);

  const handleNodeClick = useCallback(
    (node: GraphNode | null) => {
      if (!node) {
        setSelectedNode(null);
        return;
      }
      if (node.contract_id === contractId) {
        setSelectedNode(node);
        return;
      }
      router.push(`/contracts/${node.contract_id}`);
    },
    [router, contractId],
  );

  const searchMatches = useMemo(() => {
    if (!searchQuery || nodes.length === 0) return [];
    const q = searchQuery.toLowerCase();
    return nodes
      .filter(
        (n) =>
          n.name.toLowerCase().includes(q) ||
          n.contract_id.toLowerCase().includes(q),
      )
      .map((n) => n.id);
  }, [searchQuery, nodes]);

  const handleSearchChange = useCallback((value: string) => {
    setSearchQuery(value);
    setSearchMatchIndex(0);
  }, []);

  useEffect(() => {
    if (searchMatches.length > 0 && graphRef.current) {
      graphRef.current.focusOnNode(
        searchMatches[searchMatchIndex] || searchMatches[0],
      );
    }
  }, [searchMatches, searchMatchIndex]);

  const cyclicEdgeCount = useMemo(
    () => edges.filter((edge) => edge.is_circular).length,
    [edges],
  );

  const networkCounts = useMemo(() => {
    const counts = { mainnet: 0, testnet: 0, futurenet: 0, other: 0 };
    for (const node of nodes) {
      const n = node.network?.toLowerCase() ?? "";
      if (n === "mainnet") counts.mainnet++;
      else if (n === "testnet") counts.testnet++;
      else if (n === "futurenet") counts.futurenet++;
      else counts.other++;
    }
    return counts;
  }, [nodes]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-[600px] bg-card/50 rounded-2xl border border-dashed border-border">
        <div className="text-center">
          <div className="inline-block w-8 h-8 border-4 border-primary border-t-transparent rounded-full animate-spin mb-4" />
          <p className="text-muted-foreground text-sm">
            Building interaction flow…
          </p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-[600px] bg-card/50 rounded-2xl border border-dashed border-red-500/50">
        <div className="text-center p-6">
          <AlertCircle className="w-10 h-10 text-red-500 mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-foreground mb-2">
            Failed to load graph
          </h3>
          <p className="text-muted-foreground text-sm max-w-xs mx-auto mb-4">
            Could not retrieve interaction data for this contract.
          </p>
          <button
            onClick={() => refetch()}
            className="btn-secondary px-4 py-2 rounded-lg text-sm"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="relative h-[700px] w-full bg-background rounded-2xl border border-border overflow-hidden group">
      {/* Graph Canvas */}
      <div className="w-full h-full bg-background relative">
        <DependencyGraph
          ref={graphRef}
          nodes={nodes}
          edges={edges}
          searchQuery={searchQuery}
          dependentCounts={dependentCounts}
          onNodeClick={handleNodeClick}
          selectedNode={selectedNode}
        />
      </div>

      {/* Depth Selector Floating UI */}
      <div className="absolute top-4 left-4 z-30 flex items-center gap-2 pointer-events-auto">
        <div className="bg-card/90 backdrop-blur-xl border border-border p-1 rounded-lg flex items-center gap-1 shadow-lg">
          {[1, 2, 3].map((d) => (
            <button
              key={d}
              onClick={() => setDepth(d)}
              className={`px-3 py-1.5 rounded-md text-xs font-medium transition-all ${
                depth === d
                  ? "bg-primary text-primary-foreground shadow-sm"
                  : "text-muted-foreground hover:bg-accent hover:text-foreground"
              }`}
            >
              Depth {d}
            </button>
          ))}
          <div
            className="ml-2 px-2 py-1.5 border-l border-border flex items-center gap-1.5 text-muted-foreground"
            title="Controls traversal distance from this contract"
          >
            <Info className="w-3.5 h-3.5" />
            <span className="text-[10px] uppercase tracking-wider font-bold">
              Scope
            </span>
          </div>
        </div>
      </div>

      {/* Controls Overlay */}
      <div className="absolute top-0 right-0 h-full pointer-events-none">
        <GraphControls
          searchQuery={searchQuery}
          onSearchChange={handleSearchChange}
          networkFilter={networkFilter}
          onNetworkFilterChange={setNetworkFilter}
          dependencyTypeFilter={dependencyTypeFilter}
          onDependencyTypeFilterChange={setDependencyTypeFilter}
          showCyclesOnly={showCyclesOnly}
          onShowCyclesOnlyChange={setShowCyclesOnly}
          minCallFrequency={minCallFrequency}
          onMinCallFrequencyChange={setMinCallFrequency}
          totalNodes={nodes.length}
          totalEdges={edges.length}
          cyclicEdgeCount={cyclicEdgeCount}
          criticalCount={0} // Not used here
          searchMatchCount={searchMatches.length}
          searchMatchIndex={searchMatchIndex}
          onPrevMatch={() =>
            setSearchMatchIndex(
              (i) => (i - 1 + searchMatches.length) % searchMatches.length,
            )
          }
          onNextMatch={() =>
            setSearchMatchIndex((i) => (i + 1) % searchMatches.length)
          }
          onZoomIn={() => graphRef.current?.zoomIn()}
          onZoomOut={() => graphRef.current?.zoomOut()}
          onResetZoom={() => graphRef.current?.resetZoom()}
          onExportSVG={() => graphRef.current?.exportSVG()}
          onExportPNG={() => graphRef.current?.exportPNG()}
          onPanUp={() => graphRef.current?.panUp()}
          onPanDown={() => graphRef.current?.panDown()}
          onPanLeft={() => graphRef.current?.panLeft()}
          onPanRight={() => graphRef.current?.panRight()}
          networkCounts={networkCounts}
          // Note: we might want to hide some global controls in this view
        />
      </div>

      {/* Selected Node Panel (Simplified) */}
      {selectedNode && (
        <div className="absolute bottom-4 left-4 z-30 w-72 bg-card/95 backdrop-blur-xl border border-border rounded-xl shadow-2xl overflow-hidden animate-in fade-in slide-in-from-bottom-2">
          <div className="p-4">
            <div className="flex items-start justify-between mb-3">
              <div className="min-w-0">
                <h3 className="font-semibold text-foreground text-sm truncate">
                  {selectedNode.name}
                </h3>
                <p className="text-[10px] text-muted-foreground font-mono truncate">
                  {selectedNode.contract_id}
                </p>
              </div>
              <button
                onClick={() => setSelectedNode(null)}
                className="p-1 hover:bg-accent rounded text-muted-foreground"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="grid grid-cols-2 gap-2 mb-4">
              <div className="bg-muted/50 p-2 rounded-lg text-center">
                <div className="text-sm font-bold text-foreground">
                  {dependentCounts.get(selectedNode.id) || 0}
                </div>
                <div className="text-[10px] text-muted-foreground italic">
                  Inflow
                </div>
              </div>
              <div className="bg-muted/50 p-2 rounded-lg text-center">
                <div className="text-sm font-bold text-foreground">
                  {dependencyCounts.get(selectedNode.id) || 0}
                </div>
                <div className="text-[10px] text-muted-foreground italic">
                  Outflow
                </div>
              </div>
            </div>

            {selectedNode.contract_id !== contractId && (
              <button
                onClick={() =>
                  router.push(`/contracts/${selectedNode.contract_id}`)
                }
                className="w-full py-2 bg-primary text-primary-foreground rounded-lg text-xs font-bold flex items-center justify-center gap-1.5 hover:brightness-110 transition-all"
              >
                <ExternalLink className="w-3.5 h-3.5" />
                Go to Contract
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
