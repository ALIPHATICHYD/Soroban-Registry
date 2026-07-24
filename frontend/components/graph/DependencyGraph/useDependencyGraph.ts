"use client";

import { useState, useRef, useCallback, useMemo } from "react";
import * as d3 from "d3";
import { useTheme } from "@/hooks/useTheme";
import type { GraphNode, GraphEdge } from "@/lib/api";

export interface SimNode extends d3.SimulationNodeDatum {
  id: string;
  data: GraphNode;
  radius: number;
}

export interface SimLink extends d3.SimulationLinkDatum<SimNode> {
  data: GraphEdge;
}

export interface TooltipState {
  x: number;
  y: number;
  node: GraphNode;
  dependents: number;
}

export interface EdgeTooltipState {
  x: number;
  y: number;
  edge: GraphEdge;
  sourceName: string;
  targetName: string;
}

export function useDependencyGraph(
  _nodes: GraphNode[],
  edges: GraphEdge[],
  _dependentCounts: Map<string, number>,
  selectedNode: GraphNode | null,
) {
  const { resolvedTheme } = useTheme();
  const svgRef = useRef<SVGSVGElement>(null);
  const zoomRef = useRef<d3.ZoomBehavior<SVGSVGElement, unknown> | null>(null);
  const gRef = useRef<d3.Selection<
    SVGGElement,
    unknown,
    null,
    undefined
  > | null>(null);

  const [tooltip, setTooltip] = useState<TooltipState | null>(null);
  const [edgeTooltip, setEdgeTooltip] = useState<EdgeTooltipState | null>(null);
  const pinnedRef = useRef<Set<string>>(new Set());

  const computeChain = useCallback(
    (startNodeId: string) => {
      const chainNodes = new Set<string>([startNodeId]);
      const chainEdges = new Set<string>();

      const outEdges = new Map<string, string[]>();
      const inEdges = new Map<string, string[]>();

      edges.forEach((e) => {
        const src = e.source;
        const tgt = e.target;

        if (!outEdges.has(src)) outEdges.set(src, []);
        if (!inEdges.has(tgt)) inEdges.set(tgt, []);
        outEdges.get(src)!.push(tgt);
        inEdges.get(tgt)!.push(src);
      });

      let queue: string[] = [startNodeId];
      while (queue.length > 0) {
        const current = queue.shift()!;
        (outEdges.get(current) || []).forEach((target) => {
          if (!chainNodes.has(target)) {
            chainNodes.add(target);
            chainEdges.add(`${current}-${target}`);
            queue.push(target);
          } else if (!chainEdges.add(`${current}-${target}`)) {
            chainEdges.add(`${current}-${target}`);
          }
        });
      }

      queue = [startNodeId];
      while (queue.length > 0) {
        const current = queue.shift()!;
        (inEdges.get(current) || []).forEach((source) => {
          if (!chainNodes.has(source)) {
            chainNodes.add(source);
            chainEdges.add(`${source}-${current}`);
            queue.push(source);
          } else if (!chainEdges.add(`${source}-${current}`)) {
            chainEdges.add(`${source}-${current}`);
          }
        });
      }

      return { nodes: chainNodes, edges: chainEdges };
    },
    [edges],
  );

  const highlightedChain = useMemo(
    () => (selectedNode ? computeChain(selectedNode.id) : null),
    [selectedNode, computeChain],
  );

  return {
    svgRef,
    zoomRef,
    gRef,
    tooltip,
    setTooltip,
    edgeTooltip,
    setEdgeTooltip,
    highlightedChain,
    pinnedRef,
    resolvedTheme,
  };
}
