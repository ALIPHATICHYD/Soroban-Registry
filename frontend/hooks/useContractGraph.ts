"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { api, GraphEdge, GraphNode } from "@/lib/api";

export interface ContractGraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export function useContractGraph(contractId: string, depth = 2) {
  const [graph, setGraph] = useState<ContractGraphData>({ nodes: [], edges: [] });
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await api.getContractLocalGraph(contractId, depth);
      setGraph({ nodes: response.nodes, edges: response.edges });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load graph";
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, [contractId, depth]);

  useEffect(() => {
    if (!contractId) {
      setGraph({ nodes: [], edges: [] });
      setIsLoading(false);
      return;
    }
    void refresh();
  }, [contractId, refresh]);

  const exportAsJson = useCallback(() => JSON.stringify(graph, null, 2), [graph]);

  const stats = useMemo(
    () => ({
      nodeCount: graph.nodes.length,
      edgeCount: graph.edges.length,
    }),
    [graph.edges.length, graph.nodes.length],
  );

  return {
    graph,
    isLoading,
    error,
    stats,
    refresh,
    exportAsJson,
  };
}
