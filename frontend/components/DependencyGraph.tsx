"use client";

import * as d3 from "d3";
import { useEffect, useRef, useCallback, forwardRef, useImperativeHandle, useState } from "react";
import type { GraphNode, GraphEdge } from "@/lib/api";

// ─── Public handle type ──────────────────────────────────────────────────────
export interface DependencyGraphHandle {
  zoomIn: () => void;
  zoomOut: () => void;
  resetZoom: () => void;
  exportSVG: () => void;
  exportPNG: () => void;
  focusOnNode: (id: string) => void;
  panUp: () => void;
  panDown: () => void;
  panLeft: () => void;
  panRight: () => void;
}

// ─── D3 internal node type ────────────────────────────────────────────────────
interface SimNode extends d3.SimulationNodeDatum {
  id: string;
  data: GraphNode;
  radius: number;
}

interface SimLink extends d3.SimulationLinkDatum<SimNode> {
  data: GraphEdge;
}

interface D3EdgeWithNode {
  source: string | { id: string };
  target: string | { id: string };
}

// ─── Colour helpers ───────────────────────────────────────────────────────────
const NETWORK_COLOR: Record<string, string> = {
  mainnet: "#22c55e",
  testnet: "#3b82f6",
  futurenet: "#a855f7",
};

function nodeColor(node: GraphNode): string {
  return NETWORK_COLOR[node.network] ?? "#6b7280";
}

// ─── Props ────────────────────────────────────────────────────────────────────
interface DependencyGraphProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  searchQuery?: string;
  dependentCounts?: Map<string, number>;
  onNodeClick?: (node: GraphNode | null) => void;
  selectedNode?: GraphNode | null;
}

// ─── Tooltip state ────────────────────────────────────────────────────────────
interface TooltipState {
  x: number;
  y: number;
  node: GraphNode;
  dependents: number;
}

interface EdgeTooltipState {
  x: number;
  y: number;
  edge: GraphEdge;
  sourceName: string;
  targetName: string;
}

// ─── Component ────────────────────────────────────────────────────────────────
const DependencyGraph = forwardRef<DependencyGraphHandle, DependencyGraphProps>(
  function DependencyGraph(
    { nodes, edges, searchQuery = "", dependentCounts = new Map(), onNodeClick, selectedNode }: DependencyGraphProps,
    ref
  ) {
    const svgRef = useRef<SVGSVGElement>(null);
    const zoomRef = useRef<d3.ZoomBehavior<SVGSVGElement, unknown> | null>(null);
    const gRef = useRef<d3.Selection<SVGGElement, unknown, null, undefined> | null>(null);
    const [tooltip, setTooltip] = useState<TooltipState | null>(null);
    const [edgeTooltip, setEdgeTooltip] = useState<EdgeTooltipState | null>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const scrollWrapperRef = useRef<HTMLDivElement>(null); // ← new: scroll wrapper ref
    const [, setPinnedNodes] = useState<Set<string>>(new Set());
    const pinnedRef = useRef<Set<string>>(new Set());
    const [highlightedChain, setHighlightedChain] = useState<{ nodes: Set<string>; edges: Set<string> } | null>(null);

    // ── Compute Chain Highlighting ──────────────────────────────────────────
    const computeChain = useCallback((startNodeId: string) => {
      const chainNodes = new Set<string>([startNodeId]);
      const chainEdges = new Set<string>();
      
      // Map for easy traversal
      const outEdges = new Map<string, string[]>();
      const inEdges = new Map<string, string[]>();
      
      edges.forEach(e => {
        // Handle both raw edges (strings) and D3-mutated edges (objects)
        const edge = e as D3EdgeWithNode;
        const src = typeof edge.source === 'string' ? edge.source : edge.source.id;
        const tgt = typeof edge.target === 'string' ? edge.target : edge.target.id;
        
        if (!outEdges.has(src)) outEdges.set(src, []);
        if (!inEdges.has(tgt)) inEdges.set(tgt, []);
        outEdges.get(src)!.push(tgt);
        inEdges.get(tgt)!.push(src);
      });

      // BFS for descendants
      let queue: string[] = [startNodeId];
      while (queue.length > 0) {
        const current = queue.shift()!;
        (outEdges.get(current) || []).forEach(target => {
          if (!chainNodes.has(target)) {
            chainNodes.add(target);
            chainEdges.add(`${current}-${target}`);
            queue.push(target);
          } else if (!chainEdges.has(`${current}-${target}`)) {
             chainEdges.add(`${current}-${target}`);
          }
        });
      }
      
      // BFS for ancestors
      queue = [startNodeId];
      while (queue.length > 0) {
        const current = queue.shift()!;
        (inEdges.get(current) || []).forEach(source => {
          if (!chainNodes.has(source)) {
            chainNodes.add(source);
            chainEdges.add(`${source}-${current}`);
            queue.push(source);
          } else if (!chainEdges.has(`${source}-${current}`)) {
            chainEdges.add(`${source}-${current}`);
          }
        });
      }
      
      return { nodes: chainNodes, edges: chainEdges };
    }, [edges]);

    useEffect(() => {
      if (selectedNode) {
        setHighlightedChain(computeChain(selectedNode.id));
      } else {
        setHighlightedChain(null);
      }
    }, [selectedNode, computeChain]);

    // ── Large-graph performance flags ─────────────────────────────────────────
    const isLargeGraph = nodes.length > 200;
    const isVeryLargeGraph = nodes.length > 500;

    // ── Zoom helpers ────────────────────────────────────────────────────────
    const getZoom = useCallback(() => zoomRef.current, []);
    const getSvg = useCallback(() => svgRef.current ? d3.select(svgRef.current) : null, []);

    const zoomBy = useCallback((factor: number) => {
      const svg = getSvg();
      const z = getZoom();
      if (!svg || !z) return;
      svg.transition().duration(250).call(z.scaleBy, factor);
    }, [getSvg, getZoom]);

    const panBy = useCallback((dx: number, dy: number) => {
      const svg = getSvg();
      const z = getZoom();
      if (!svg || !z) return;
      svg.transition().duration(150).call(z.translateBy, dx, dy);
    }, [getSvg, getZoom]);

    // ── Imperative handle ────────────────────────────────────────────────────
    useImperativeHandle(ref, () => ({
      zoomIn: () => zoomBy(1.3),
      zoomOut: () => zoomBy(1 / 1.3),
      resetZoom: () => {
        const svg = getSvg();
        const z = getZoom();
        if (!svg || !z) return;
        const rect = svgRef.current!.getBoundingClientRect();
        svg.transition().duration(400).call(
          z.transform,
          d3.zoomIdentity.translate(rect.width / 2, rect.height / 2).scale(1)
        );
      },
      focusOnNode: (id: string) => {
        const svg = getSvg();
        const z = getZoom();
        if (!svg || !z || !svgRef.current) return;
        const gEl = gRef.current;
        if (!gEl) return;
        const circle = gEl.select<SVGCircleElement>(`circle[data-id="${id}"]`);
        if (circle.empty()) return;
        const cx = parseFloat(circle.attr("cx") || "0");
        const cy = parseFloat(circle.attr("cy") || "0");
        const rect = svgRef.current.getBoundingClientRect();
        svg.transition().duration(400).call(
          z.transform,
          d3.zoomIdentity
            .translate(rect.width / 2 - cx * 1.5, rect.height / 2 - cy * 1.5)
            .scale(1.5)
        );
      },
      exportSVG: () => {
        if (!svgRef.current) return;
        const serializer = new XMLSerializer();
        const source = serializer.serializeToString(svgRef.current);
        const blob = new Blob([source], { type: "image/svg+xml" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "dependency-graph.svg";
        a.click();
        URL.revokeObjectURL(url);
      },
      exportPNG: () => {
        if (!svgRef.current) return;
        const serializer = new XMLSerializer();
        const source = serializer.serializeToString(svgRef.current);
        const img = new Image();
        const svg = svgRef.current;
        img.onload = () => {
          const canvas = document.createElement("canvas");
          canvas.width = svg.clientWidth || 1200;
          canvas.height = svg.clientHeight || 800;
          const ctx = canvas.getContext("2d")!;
          ctx.fillStyle = "#030712";
          ctx.fillRect(0, 0, canvas.width, canvas.height);
          ctx.drawImage(img, 0, 0);
          const a = document.createElement("a");
          a.href = canvas.toDataURL("image/png");
          a.download = "dependency-graph.png";
          a.click();
        };
        const utf8Bytes = new TextEncoder().encode(source);
        const binaryString = utf8Bytes.reduce((data, byte) => data + String.fromCharCode(byte), "");
        const base64Svg = btoa(binaryString);
        img.src = "data:image/svg+xml;base64," + base64Svg;
      },
      panUp: () => panBy(0, -60),
      panDown: () => panBy(0, 60),
      panLeft: () => panBy(-60, 0),
      panRight: () => panBy(60, 0),
    }));

    // ── D3 force simulation ────────────────────────────────────────────────
    useEffect(() => {
      if (!svgRef.current) return;

      const svgEl = svgRef.current;
      const rect = svgEl.getBoundingClientRect();
      const W = rect.width || 1000;
      const H = rect.height || 700;

      // Clear previous render
      d3.select(svgEl).selectAll("*").remove();

      const svg = d3.select(svgEl)
        .attr("width", W)
        .attr("height", H);

      // ── Arrow marker ──
      svg.append("defs").append("marker")
        .attr("id", "arrow")
        .attr("viewBox", "0 -5 10 10")
        .attr("refX", 18)
        .attr("refY", 0)
        .attr("markerWidth", 5)
        .attr("markerHeight", 5)
        .attr("orient", "auto")
        .append("path")
        .attr("d", "M0,-5L10,0L0,5")
        .attr("fill", "#4b5563");

      // ── Root group (zoom target) ──
      const g = svg.append("g").attr("class", "graph-root");
      gRef.current = g;

      // ── Zoom ──
      const zoom = d3.zoom<SVGSVGElement, unknown>()
        .scaleExtent([0.05, 8])
        .on("zoom", (event) => {
          g.attr("transform", event.transform.toString());
        });

      zoomRef.current = zoom;
      svg.call(zoom).on("dblclick.zoom", null);

      // ── Build sim nodes/links ──
      const simNodes: SimNode[] = nodes.map((n) => {
        const deps = dependentCounts.get(n.id) ?? 0;
        const radius = Math.max(6, Math.min(22, 8 + deps * 1.5));
        return { id: n.id, data: n, radius };
      });

      const nodeById = new Map(simNodes.map((n) => [n.id, n]));

      const simLinks: SimLink[] = edges
        .filter((e) => nodeById.has(e.source) && nodeById.has(e.target))
        .map((e) => ({
          source: nodeById.get(e.source)!,
          target: nodeById.get(e.target)!,
          data: e,
        }));

      const baseEdgeColor = (link: SimLink) => link.data.is_circular ? "#ef4444" : "#374151";
      const baseEdgeOpacity = (link: SimLink) => {
        if (link.data.is_circular) return isLargeGraph ? 0.72 : 0.9;
        return isLargeGraph ? 0.35 : 0.6;
      };

      // ── Force simulation ──
      const simulation = d3.forceSimulation<SimNode>(simNodes)
        .force("link", d3.forceLink<SimNode, SimLink>(simLinks)
          .id((d) => d.id)
          .distance(isLargeGraph ? 50 : 80)
          .strength(0.4))
        .force("charge", d3.forceManyBody()
          .strength(isVeryLargeGraph ? -60 : isLargeGraph ? -120 : -180)
          .distanceMax(isLargeGraph ? 200 : 400))
        .force("center", d3.forceCenter(0, 0))
        .force("collision", d3.forceCollide<SimNode>().radius((d) => d.radius + (isLargeGraph ? 2 : 4)))
        .alphaDecay(isVeryLargeGraph ? 0.06 : isLargeGraph ? 0.04 : 0.025)
        .alphaMin(0.001);

      // ── Edges ──
      const linkGroup = g.append("g").attr("class", "links");
      const linkEls = linkGroup.selectAll<SVGLineElement, SimLink>("line")
        .data(simLinks)
        .join("line")
        .attr("class", "graph-edge")
        .attr("stroke", (d) => baseEdgeColor(d))
        .attr("stroke-width", isLargeGraph ? 0.8 : 1.5)
        .attr("stroke-opacity", (d) => baseEdgeOpacity(d))
        .attr("stroke-dasharray", (d) => d.data.is_circular ? "4 3" : null)
        .attr("marker-end", isVeryLargeGraph ? null : "url(#arrow)");

      const edgeLabels = !isLargeGraph
        ? g.append("g")
          .attr("class", "edge-labels")
          .selectAll<SVGTextElement, SimLink>("text")
          .data(simLinks)
          .join("text")
          .attr("font-size", "9px")
          .attr("fill", (d) => d.data.is_circular ? "#fca5a5" : "#9ca3af")
          .attr("text-anchor", "middle")
          .attr("pointer-events", "none")
          .text((d) => {
            if (!d.data.call_frequency) return "";
            return d.data.is_estimated ? `~${d.data.call_frequency}` : `${d.data.call_frequency}`;
          })
        : null;

      // ── Nodes ──
      const nodeGroup = g.append("g").attr("class", "nodes");
      const nodeEls = nodeGroup.selectAll<SVGGElement, SimNode>("g.node")
        .data(simNodes, (d) => d.id)
        .join("g")
        .attr("class", "node")
        .style("cursor", "pointer");

      nodeEls.append("circle")
        .attr("r", (d) => d.radius)
        .attr("data-id", (d) => d.id)
        .attr("fill", (d) => nodeColor(d.data))
        .attr("fill-opacity", 0.85)
        .attr("stroke", (d) => {
          const deps = dependentCounts.get(d.id) ?? 0;
          return deps >= 5 ? "#f59e0b" : "transparent";
        })
        .attr("stroke-width", 2.5);

      if (!isLargeGraph) {
        nodeEls.append("text")
          .attr("dy", (d) => d.radius + 12)
          .attr("text-anchor", "middle")
          .attr("fill", "#d1d5db")
          .attr("font-size", "10px")
          .attr("pointer-events", "none")
          .text((d) => { const n = d.data.name ?? d.data.contract_id ?? ""; return n.length > 14 ? n.slice(0, 13) + "…" : n; });
      }

      // ── Drag ──
      const drag = d3.drag<SVGGElement, SimNode>()
        .on("start", (event, d) => {
          if (!event.active) simulation.alphaTarget(0.3).restart();
          d.fx = d.x;
          d.fy = d.y;
        })
        .on("drag", (event, d) => {
          d.fx = event.x;
          d.fy = event.y;
        })
        .on("end", (event, d) => {
          if (!event.active) simulation.alphaTarget(0);
          if (!pinnedRef.current.has(d.id)) {
            d.fx = null;
            d.fy = null;
          }
        });

      // ── Pin indicator ──
      const pinGroup = nodeEls.append("g")
        .attr("class", "pin-indicator")
        .attr("pointer-events", "none")
        .attr("display", "none");

      pinGroup.append("circle")
        .attr("r", 5)
        .attr("cx", (d) => d.radius - 4)
        .attr("cy", -6)
        .attr("fill", "#f97316")
        .attr("stroke", "#1f2937")
        .attr("stroke-width", 1.5);

      pinGroup.append("text")
        .attr("x", (d) => d.radius - 4)
        .attr("y", -3)
        .attr("text-anchor", "middle")
        .attr("fill", "white")
        .attr("font-size", "7px")
        .attr("font-weight", "bold")
        .attr("pointer-events", "none")
        .text("P");

      nodeEls.call(drag as unknown as (selection: d3.Selection<SVGGElement, SimNode, SVGGElement, unknown>) => void);

      // ── Tooltip + hover highlight ──
      nodeEls.on("mouseenter", (event: MouseEvent, d) => {
        setEdgeTooltip(null);
        const svgRect = svgEl.getBoundingClientRect();
        setTooltip({
          x: event.clientX - svgRect.left,
          y: event.clientY - svgRect.top,
          node: d.data,
          dependents: dependentCounts.get(d.id) ?? 0,
        });

        if (!selectedNode) {
          const connected = new Set<string>([d.id]);
          linkEls.each(function (ld) {
            const src = (ld.source as SimNode).id;
            const tgt = (ld.target as SimNode).id;
            if (src === d.id || tgt === d.id) {
              connected.add(src);
              connected.add(tgt);
            }
          });

          linkEls
            .attr("stroke-opacity", (ld) => {
              const src = (ld.source as SimNode).id;
              const tgt = (ld.target as SimNode).id;
              return (src === d.id || tgt === d.id) ? 0.9 : 0.05;
            })
            .attr("stroke", (ld) => {
              const src = (ld.source as SimNode).id;
              const tgt = (ld.target as SimNode).id;
              return (src === d.id || tgt === d.id) ? "#60a5fa" : baseEdgeColor(ld);
            });

          nodeEls.attr("opacity", (nd) => connected.has(nd.id) ? 1 : 0.2);
        }
      });

      nodeEls.on("mousemove", (event: MouseEvent) => {
        const svgRect = svgEl.getBoundingClientRect();
        setTooltip((prev) => prev ? { ...prev, x: event.clientX - svgRect.left, y: event.clientY - svgRect.top } : null);
      });

      nodeEls.on("mouseleave", () => {
        setTooltip(null);
        if (!selectedNode) {
          linkEls
            .attr("stroke-opacity", (d) => baseEdgeOpacity(d))
            .attr("stroke", (d) => baseEdgeColor(d));
          nodeEls.attr("opacity", 1);
        }
      });

      linkEls.on("mouseenter", (event: MouseEvent, d) => {
        setTooltip(null);
        const svgRect = svgEl.getBoundingClientRect();
        const sourceNode = d.source as SimNode;
        const targetNode = d.target as SimNode;
        setEdgeTooltip({
          x: event.clientX - svgRect.left,
          y: event.clientY - svgRect.top,
          edge: d.data,
          sourceName: sourceNode.data.name,
          targetName: targetNode.data.name,
        });

        d3.select(event.currentTarget as SVGLineElement)
          .attr("stroke", d.data.is_circular ? "#dc2626" : "#60a5fa")
          .attr("stroke-opacity", 1)
          .attr("stroke-width", isLargeGraph ? 1.4 : 2.2);
      });

      linkEls.on("mousemove", (event: MouseEvent) => {
        const svgRect = svgEl.getBoundingClientRect();
        setEdgeTooltip((prev) => prev ? { ...prev, x: event.clientX - svgRect.left, y: event.clientY - svgRect.top } : null);
      });

      linkEls.on("mouseleave", (event: MouseEvent, d) => {
        setEdgeTooltip(null);
        if (selectedNode) return;
        d3.select(event.currentTarget as SVGLineElement)
          .attr("stroke", baseEdgeColor(d))
          .attr("stroke-opacity", baseEdgeOpacity(d))
          .attr("stroke-width", isLargeGraph ? 0.8 : 1.5);
      });

      // ── Click: select node ──
      nodeEls.on("click", (event: MouseEvent, d) => {
        event.stopPropagation();
        onNodeClick?.(d.data);
      });

      // ── Double-click: pin / unpin ──
      nodeEls.on("dblclick", (event: MouseEvent, d) => {
        event.stopPropagation();
        const isPinned = pinnedRef.current.has(d.id);
        if (isPinned) {
          pinnedRef.current.delete(d.id);
          d.fx = null;
          d.fy = null;
          d3.select(event.currentTarget as SVGGElement)
            .select(".pin-indicator")
            .attr("display", "none");
        } else {
          pinnedRef.current.add(d.id);
          d.fx = d.x;
          d.fy = d.y;
          d3.select(event.currentTarget as SVGGElement)
            .select(".pin-indicator")
            .attr("display", null);
        }
        setPinnedNodes(new Set(pinnedRef.current));
        simulation.alphaTarget(0.1).restart();
      });

      svg.on("click", () => onNodeClick?.(null));

      // ── Tick ──
      simulation.on("tick", () => {
        linkEls
          .attr("x1", (d) => (d.source as SimNode).x ?? 0)
          .attr("y1", (d) => (d.source as SimNode).y ?? 0)
          .attr("x2", (d) => (d.target as SimNode).x ?? 0)
          .attr("y2", (d) => (d.target as SimNode).y ?? 0);

        nodeEls.attr("transform", (d) => `translate(${d.x ?? 0},${d.y ?? 0})`);
        nodeEls.select("circle")
          .attr("cx", 0)
          .attr("cy", 0);

        if (edgeLabels) {
          edgeLabels
            .attr("x", (d) => (((d.source as SimNode).x ?? 0) + ((d.target as SimNode).x ?? 0)) / 2)
            .attr("y", (d) => ((((d.source as SimNode).y ?? 0) + ((d.target as SimNode).y ?? 0)) / 2) - 4);
        }
      });

      // Initial zoom-to-fit
      const initialZoomTimer = setTimeout(() => {
        const rect2 = svgEl.getBoundingClientRect();
        svg.call(
          zoom.transform,
          d3.zoomIdentity.translate(rect2.width / 2, rect2.height / 2).scale(
            Math.min(1, Math.max(0.05, 5 / Math.sqrt(simNodes.length + 1)))
          )
        );
      }, isVeryLargeGraph ? 1500 : isLargeGraph ? 900 : 600);

      // Responsive resize
      let resizeObserver: ResizeObserver | null = null;
      if (typeof ResizeObserver !== "undefined") {
        resizeObserver = new ResizeObserver((entries) => {
          for (const entry of entries) {
            const { width, height } = entry.contentRect;
            d3.select(svgEl).attr("width", width).attr("height", height);
          }
        });
        if (containerRef.current) resizeObserver.observe(containerRef.current);
      }

      return () => {
        clearTimeout(initialZoomTimer);
        resizeObserver?.disconnect();
        simulation.stop();
        d3.select(svgEl).selectAll("*").remove();
      };
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [nodes, edges, dependentCounts, isLargeGraph, isVeryLargeGraph]);

    // ── Highlight selected node & neighbours ──────────────────────────────
    useEffect(() => {
      const g = gRef.current;
      if (!g) return;

      if (!selectedNode && !highlightedChain) {
        g.selectAll<SVGGElement, SimNode>("g.node").attr("opacity", 1);
        g.selectAll<SVGLineElement, SimLink>("line.graph-edge")
          .attr("opacity", (d) => d.data.is_circular ? 0.9 : 0.6)
          .attr("stroke", (d) => d.data.is_circular ? "#ef4444" : "#374151");
        return;
      }

      const neighbourIds = new Set<string>();
      const chainNodeIds = highlightedChain?.nodes ?? new Set();
      const chainEdgeIds = highlightedChain?.edges ?? new Set();

      if (selectedNode) {
        neighbourIds.add(selectedNode.id);
        g.selectAll<SVGLineElement, SimLink>("line.graph-edge").each(function (d) {
          const src = (d.source as SimNode).id;
          const tgt = (d.target as SimNode).id;
          if (src === selectedNode.id || tgt === selectedNode.id) {
            neighbourIds.add(src);
            neighbourIds.add(tgt);
          }
        });
      }

      g.selectAll<SVGGElement, SimNode>("g.node")
        .attr("opacity", (d) => {
          if (chainNodeIds.has(d.id)) return 1;
          if (neighbourIds.has(d.id)) return 1;
          return 0.1;
        })
        .select("circle")
        .attr("stroke", (d) => {
          if (selectedNode?.id === d.id) return "#60a5fa";
          if (chainNodeIds.has(d.id)) return "#3b82f6";
          const deps = dependentCounts.get(d.id) ?? 0;
          return deps >= 5 ? "#f59e0b" : "transparent";
        })
        .attr("stroke-width", (d) => {
           if (selectedNode?.id === d.id) return 4;
           if (chainNodeIds.has(d.id)) return 2.5;
           const deps = dependentCounts.get(d.id) ?? 0;
           return deps >= 5 ? 2.5 : 0;
        });

      g.selectAll<SVGLineElement, SimLink>("line.graph-edge")
        .attr("opacity", (d) => {
          const src = (d.source as SimNode).id;
          const tgt = (d.target as SimNode).id;
          const edgeId = `${src}-${tgt}`;
          if (chainEdgeIds.has(edgeId)) return 1;
          if (selectedNode && (src === selectedNode.id || tgt === selectedNode.id)) return 0.8;
          return 0.05;
        })
        .attr("stroke", (d) => {
          const src = (d.source as SimNode).id;
          const tgt = (d.target as SimNode).id;
          const edgeId = `${src}-${tgt}`;
          if (chainEdgeIds.has(edgeId)) return "#60a5fa";
          if (selectedNode && (src === selectedNode.id || tgt === selectedNode.id)) return "#93c5fd";
          return d.data.is_circular ? "#ef4444" : "#374151";
        })
        .attr("stroke-width", (d) => {
          const src = (d.source as SimNode).id;
          const tgt = (d.target as SimNode).id;
          const edgeId = `${src}-${tgt}`;
          if (chainEdgeIds.has(edgeId)) return 2.5;
          return isLargeGraph ? 0.8 : 1.5;
        });
    }, [selectedNode, highlightedChain, isLargeGraph, dependentCounts]);

    // ── Search highlight ──────────────────────────────────────────────────
    useEffect(() => {
      const g = gRef.current;
      if (!g) return;
      if (!searchQuery) {
        g.selectAll<SVGGElement, SimNode>("g.node").select("circle")
          .attr("stroke-width", (d) => (dependentCounts.get(d.id) ?? 0) >= 5 ? 2.5 : 0)
          .attr("stroke", (d) => (dependentCounts.get(d.id) ?? 0) >= 5 ? "#f59e0b" : "transparent");
        return;
      }
      const q = searchQuery.toLowerCase();
      g.selectAll<SVGGElement, SimNode>("g.node").select("circle")
        .attr("stroke-width", (d) => {
          const isMatch = d.data.name.toLowerCase().includes(q) || d.data.contract_id.toLowerCase().includes(q);
          return isMatch ? 3 : ((dependentCounts.get(d.id) ?? 0) >= 5 ? 2.5 : 0);
        })
        .attr("stroke", (d) => {
          const isMatch = d.data.name.toLowerCase().includes(q) || d.data.contract_id.toLowerCase().includes(q);
          return isMatch ? "#facc15" : ((dependentCounts.get(d.id) ?? 0) >= 5 ? "#f59e0b" : "transparent");
        });
    }, [searchQuery, dependentCounts]);

    // ── Empty state ───────────────────────────────────────────────────────
    if (nodes.length === 0) {
      return (
        <div className="w-full h-full flex items-center justify-center bg-muted/30">
          <p className="text-muted-foreground text-sm">No nodes to display.</p>
        </div>
      );
    }

    return (

      <div
        ref={scrollWrapperRef}
        className="w-full overflow-x-auto"               // ← enables horizontal scroll on mobile
        style={{ WebkitOverflowScrolling: "touch" }}     // ← smooth momentum scroll on iOS
      >
        {/* Mobile scroll hint — hidden on md+ */}
        <p className="md:hidden text-xs text-muted-foreground text-center pb-1 select-none pointer-events-none">
          ← Scroll to explore →
        </p>

        {/* Inner container: enforces minimum width so graph isn't crushed */}
        <div
          ref={containerRef}
          className="relative h-full bg-surface min-w-[600px] md:min-w-0"  // ← key change
        >
          <svg
            ref={svgRef}
            className="w-full h-full"
            style={{ display: "block", touchAction: "none" }}
          />

          {/* Performance notice for very large graphs */}
          {isVeryLargeGraph && (
            <div className="absolute top-4 left-1/2 -translate-x-1/2 z-30 pointer-events-none">
              <div className="bg-amber-900/80 backdrop-blur border border-amber-700/50 rounded-lg px-3 py-1.5 text-xs text-amber-200">
                Large graph ({nodes.length.toLocaleString()} nodes) — labels hidden for performance
              </div>
            </div>
          )}

          {/* Tooltip */}
          {tooltip && (
            <div
              className="pointer-events-none absolute z-40 bg-background/95 backdrop-blur-xl border border-border rounded-xl px-3 py-2.5 shadow-2xl text-xs max-w-[220px]"
              style={{
                left: tooltip.x + 14,
                top: tooltip.y - 10,
                transform: tooltip.x > (containerRef.current?.clientWidth ?? 0) - 240
                  ? "translateX(-110%)"
                  : "none",
              }}
            >
              <p className="font-semibold text-foreground mb-1 truncate">{tooltip.node.name}</p>
              <p className="font-mono text-muted-foreground truncate text-[10px] mb-1.5">
                {tooltip.node.contract_id ? tooltip.node.contract_id.slice(0, 12) + "…" : "—"}
              </p>
              <div className="space-y-0.5">
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Network</span>
                  <span style={{ color: NETWORK_COLOR[tooltip.node.network] ?? undefined }}>
                    {tooltip.node.network}
                  </span>
                </div>
                {tooltip.node.category && (
                  <div className="flex justify-between gap-4">
                    <span className="text-muted-foreground">Type</span>
                    <span className="text-foreground">{tooltip.node.category}</span>
                  </div>
                )}
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Verified</span>
                  <span className={tooltip.node.is_verified ? "text-green-600 dark:text-green-400" : "text-muted-foreground"}>
                    {tooltip.node.is_verified ? "✓" : "—"}
                  </span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Dependents</span>
                  <span className="text-foreground">{tooltip.dependents}</span>
                </div>
              </div>
            </div>
          )}

          {edgeTooltip && (
            <div
              className="pointer-events-none absolute z-40 bg-background/95 backdrop-blur-xl border border-border rounded-xl px-3 py-2.5 shadow-2xl text-xs max-w-[260px]"
              style={{
                left: edgeTooltip.x + 14,
                top: edgeTooltip.y - 10,
                transform: edgeTooltip.x > (containerRef.current?.clientWidth ?? 0) - 280
                  ? "translateX(-110%)"
                  : "none",
              }}
            >
              <p className="font-semibold text-foreground mb-1 truncate">{edgeTooltip.sourceName} → {edgeTooltip.targetName}</p>
              <div className="space-y-0.5">
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Type</span>
                  <span className="text-foreground uppercase tracking-wide">{edgeTooltip.edge.dependency_type}</span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Call frequency</span>
                  <span className="text-foreground">{edgeTooltip.edge.call_frequency ?? 0}</span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Call volume</span>
                  <span className="text-foreground">{edgeTooltip.edge.call_volume ?? 0}</span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-muted-foreground">Telemetry</span>
                  <span className={edgeTooltip.edge.is_estimated ? "text-amber-400" : "text-emerald-400"}>
                    {edgeTooltip.edge.is_estimated ? "Estimated" : "Exact"}
                  </span>
                </div>
                {edgeTooltip.edge.is_estimated && (
                  <div className="text-amber-300 text-[11px] pt-1">
                    Inferred from source-level aggregate interactions.
                  </div>
                )}
                {edgeTooltip.edge.is_circular && (
                  <div className="text-red-400 text-[11px] pt-1">Circular dependency detected</div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    );
  }
);

DependencyGraph.displayName = "DependencyGraph";
export default DependencyGraph;