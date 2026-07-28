"use client";

import { useEffect, type RefObject } from "react";
import * as d3 from "d3";
import type { GraphNode, GraphEdge } from "@/lib/api";
import type {
  EdgeTooltipState,
  SimLink,
  SimNode,
  TooltipState,
} from "./useDependencyGraph";

interface GraphSVGProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  dependentCounts: Map<string, number>;
  resolvedTheme: string;
  svgRef: RefObject<SVGSVGElement | null>;
  onGraphReady: (
    graph: d3.Selection<SVGGElement, unknown, null, undefined> | null,
  ) => void;
  onZoomReady: (zoom: d3.ZoomBehavior<SVGSVGElement, unknown> | null) => void;
  setTooltip: (state: TooltipState | null) => void;
  setEdgeTooltip: (state: EdgeTooltipState | null) => void;
  selectedNode: GraphNode | null;
  onNodeClick?: (node: GraphNode | null) => void;
  pinnedRef: RefObject<Set<string>>;
  isLargeGraph: boolean;
  isVeryLargeGraph: boolean;
}

const NETWORK_COLOR: Record<string, string> = {
  mainnet: "#22c55e",
  testnet: "#3b82f6",
  futurenet: "#a855f7",
};

function nodeColor(node: GraphNode): string {
  return NETWORK_COLOR[node.network] ?? "#6b7280";
}

export function GraphSVG({
  nodes,
  edges,
  dependentCounts,
  resolvedTheme,
  svgRef,
  onGraphReady,
  onZoomReady,
  setTooltip,
  setEdgeTooltip,
  selectedNode,
  onNodeClick,
  pinnedRef,
  isLargeGraph,
  isVeryLargeGraph,
}: GraphSVGProps) {
  useEffect(() => {
    if (!svgRef.current) return;

    const svgEl = svgRef.current;
    const rect = svgEl.getBoundingClientRect();
    const W = rect.width || 1000;
    const H = rect.height || 700;

    d3.select(svgEl).selectAll("*").remove();

    const svg = d3.select(svgEl).attr("width", W).attr("height", H);

    svg
      .append("defs")
      .append("marker")
      .attr("id", "arrow")
      .attr("viewBox", "0 -5 10 10")
      .attr("refX", 18)
      .attr("refY", 0)
      .attr("markerWidth", 5)
      .attr("markerHeight", 5)
      .attr("orient", "auto")
      .append("path")
      .attr("d", "M0,-5L10,0L0,5")
      .attr("fill", resolvedTheme === "dark" ? "#6b7280" : "#9ca3af");

    const g = svg.append("g").attr("class", "graph-root");
    onGraphReady(g);

    const zoom = d3
      .zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.05, 8])
      .on("zoom", (event) => {
        g.attr("transform", event.transform.toString());
      });

    onZoomReady(zoom);
    svg.call(zoom).on("dblclick.zoom", null);

    const simNodes: SimNode[] = nodes.map((n) => {
      const deps = dependentCounts.get(n.id) ?? 0;
      const radius = Math.max(6, Math.min(22, 8 + deps * 1.5));
      return { id: n.id, data: n, radius, x: 0, y: 0 };
    });

    const nodeById = new Map(simNodes.map((n) => [n.id, n]));

    const simLinks: SimLink[] = edges
      .filter(
        (e) =>
          nodeById.has(e.source as string) && nodeById.has(e.target as string),
      )
      .map((e) => ({
        source: nodeById.get(e.source as string)!,
        target: nodeById.get(e.target as string)!,
        data: e,
      }));

    const baseEdgeColor = (link: SimLink) => {
      if (link.data.is_circular) return "#ef4444";
      return resolvedTheme === "dark" ? "#4b5563" : "#cbd5e1";
    };
    const baseEdgeOpacity = (link: SimLink) => {
      if (link.data.is_circular) return isLargeGraph ? 0.8 : 1.0;
      return isLargeGraph ? 0.4 : 0.7;
    };

    const simulation = d3
      .forceSimulation<SimNode>(simNodes)
      .force(
        "link",
        d3
          .forceLink<SimNode, SimLink>(simLinks)
          .id((d) => d.id)
          .distance(isLargeGraph ? 50 : 80)
          .strength(0.4),
      )
      .force(
        "charge",
        d3
          .forceManyBody()
          .strength(isVeryLargeGraph ? -60 : isLargeGraph ? -120 : -180)
          .distanceMax(isLargeGraph ? 200 : 400),
      )
      .force("center", d3.forceCenter(W / 2, H / 2))
      .force(
        "collision",
        d3
          .forceCollide<SimNode>()
          .radius((d) => d.radius + (isLargeGraph ? 2 : 4)),
      )
      .alphaDecay(isVeryLargeGraph ? 0.06 : isLargeGraph ? 0.04 : 0.025);

    const linkGroup = g.append("g").attr("class", "links");
    const linkEls = linkGroup
      .selectAll<SVGLineElement, SimLink>("line")
      .data(simLinks)
      .join("line")
      .attr("stroke", (d) => baseEdgeColor(d))
      .attr("stroke-width", isLargeGraph ? 0.8 : 1.5)
      .attr("stroke-opacity", (d) => baseEdgeOpacity(d))
      .attr("marker-end", isVeryLargeGraph ? null : "url(#arrow)");

    const nodeGroup = g.append("g").attr("class", "nodes");
    const nodeEls = nodeGroup
      .selectAll<SVGGElement, SimNode>("g.node")
      .data(simNodes, (d) => d.id)
      .join("g")
      .attr("class", "node")
      .style("cursor", "pointer");

    nodeEls
      .append("circle")
      .attr("r", (d) => d.radius)
      .attr("fill", (d) => nodeColor(d.data))
      .attr("fill-opacity", 0.85)
      .attr("stroke", (d) =>
        selectedNode?.id === d.id ? "currentColor" : "transparent",
      )
      .attr("stroke-width", (d) => (selectedNode?.id === d.id ? 3 : 0));

    nodeEls
      .on("mouseenter", (event, d) => {
        setTooltip({
          x: event.clientX,
          y: event.clientY,
          node: d.data,
          dependents: dependentCounts.get(d.id) ?? 0,
        });
      })
      .on("mouseleave", () => setTooltip(null))
      .on("click", (event, d) => {
        event.stopPropagation();
        if (pinnedRef.current.has(d.id)) {
          pinnedRef.current.delete(d.id);
        } else {
          pinnedRef.current.add(d.id);
        }
        onNodeClick?.(d.data);
      });

    linkEls
      .on("mouseenter", (event, d) => {
        const source = d.source as SimNode;
        const target = d.target as SimNode;
        setEdgeTooltip({
          x: event.clientX,
          y: event.clientY,
          edge: d.data,
          sourceName: source.data.name,
          targetName: target.data.name,
        });
      })
      .on("mouseleave", () => setEdgeTooltip(null));

    if (!isLargeGraph) {
      nodeEls
        .append("text")
        .attr("dy", (d) => d.radius + 12)
        .attr("text-anchor", "middle")
        .attr("fill", resolvedTheme === "dark" ? "#d1d5db" : "#374151")
        .attr("font-size", "10px")
        .text((d) => d.data.name);
    }

    simulation.on("tick", () => {
      linkEls
        .attr("x1", (d) => (d.source as SimNode).x ?? 0)
        .attr("y1", (d) => (d.source as SimNode).y ?? 0)
        .attr("x2", (d) => (d.target as SimNode).x ?? 0)
        .attr("y2", (d) => (d.target as SimNode).y ?? 0);

      nodeEls.attr("transform", (d) => `translate(${d.x ?? 0},${d.y ?? 0})`);
    });

    return () => {
      simulation.stop();
      onGraphReady(null);
      onZoomReady(null);
    };
  }, [
    nodes,
    edges,
    dependentCounts,
    resolvedTheme,
    isLargeGraph,
    isVeryLargeGraph,
    onGraphReady,
    onZoomReady,
    onNodeClick,
    pinnedRef,
    selectedNode,
    setEdgeTooltip,
    setTooltip,
    svgRef,
  ]);

  return (
    <svg
      ref={svgRef}
      className="w-full h-full"
      style={{ display: "block", touchAction: "none" }}
    />
  );
}
