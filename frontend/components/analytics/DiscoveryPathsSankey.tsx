"use client";

import React, { useRef, useEffect, useState } from "react";
import { DiscoveryPaths, SankeyNode } from "@/types/analytics";

interface DiscoveryPathsSankeyProps {
  data: DiscoveryPaths;
}

interface LayoutNode {
  id: string;
  name: string;
  category: string;
  x0: number;
  x1: number;
  y0: number;
  y1: number;
  value: number;
}

interface LayoutLink {
  source: LayoutNode;
  target: LayoutNode;
  value: number;
  y0: number; // source exit y
  y1: number; // target entry y
  width: number;
}

const CATEGORY_COLORS: Record<string, string> = {
  entry: "#3b82f6",
  search: "#8b5cf6",
  filter: "#06b6d4",
  contract: "#10b981",
  action: "#f59e0b",
};

const NODE_WIDTH = 14;
const NODE_PAD = 18;

function computeLayout(
  nodes: DiscoveryPaths["nodes"],
  links: DiscoveryPaths["links"],
  width: number,
  height: number,
): { layoutNodes: LayoutNode[]; layoutLinks: LayoutLink[] } {
  // Assign columns by category order
  const COLUMN_ORDER: SankeyNode["category"][] = [
    "entry",
    "search",
    "filter",
    "contract",
    "action",
  ];

  const nodeMap = new Map<string, LayoutNode>();
  const columnGroups = new Map<number, string[]>();

  nodes.forEach((n) => {
    const col = COLUMN_ORDER.indexOf(n.category);
    if (!columnGroups.has(col)) columnGroups.set(col, []);
    columnGroups.get(col)!.push(n.id);
    nodeMap.set(n.id, {
      ...n,
      x0: 0,
      x1: 0,
      y0: 0,
      y1: 0,
      value: 0,
    });
  });

  // Compute node values from links using max(total_in, total_out)
  const inTotals = new Map<string, number>();
  const outTotals = new Map<string, number>();

  links.forEach((l) => {
    if (l.source) {
      outTotals.set(l.source, (outTotals.get(l.source) ?? 0) + l.value);
    }
    if (l.target) {
      inTotals.set(l.target, (inTotals.get(l.target) ?? 0) + l.value);
    }
  });

  nodeMap.forEach((node, id) => {
    const inTotal = inTotals.get(id) ?? 0;
    const outTotal = outTotals.get(id) ?? 0;
    node.value = Math.max(inTotal, outTotal);
  });
  const numCols = COLUMN_ORDER.length;
  const colWidth = (width - NODE_WIDTH) / (numCols - 1);

  COLUMN_ORDER.forEach((cat, colIdx) => {
    const ids = columnGroups.get(colIdx) ?? [];
    const totalValue = ids.reduce(
      (s, id) => s + (nodeMap.get(id)?.value ?? 0),
      0,
    );
    const availH = height - NODE_PAD * (ids.length - 1);
    let y = 0;

    ids.forEach((id) => {
      const node = nodeMap.get(id)!;
      const nodeH = Math.max(
        4,
        (node.value / Math.max(totalValue, 1)) * availH,
      );
      node.x0 = colIdx * colWidth;
      node.x1 = node.x0 + NODE_WIDTH;
      node.y0 = y;
      node.y1 = y + nodeH;
      y += nodeH + NODE_PAD;
    });
  });

  // Build layout links with exit/entry y positions
  const srcExitY = new Map<string, number>();
  const tgtEntryY = new Map<string, number>();
  nodes.forEach((n) => {
    srcExitY.set(n.id, nodeMap.get(n.id)!.y0);
    tgtEntryY.set(n.id, nodeMap.get(n.id)!.y0);
  });

  const layoutLinks: LayoutLink[] = links.map((l) => {
    const src = nodeMap.get(l.source)!;
    const tgt = nodeMap.get(l.target)!;
    const srcH = src.y1 - src.y0;
    const tgtH = tgt.y1 - tgt.y0;
    const srcTotal = src.value;
    const tgtTotal = tgt.value;
    const lw = Math.max(1, (l.value / Math.max(srcTotal, 1)) * srcH);
    const lh = Math.max(1, (l.value / Math.max(tgtTotal, 1)) * tgtH);

    const y0 = srcExitY.get(l.source)!;
    const y1 = tgtEntryY.get(l.target)!;
    srcExitY.set(l.source, y0 + lw);
    tgtEntryY.set(l.target, y1 + lh);

    return {
      source: src,
      target: tgt,
      value: l.value,
      y0,
      y1,
      width: (lw + lh) / 2,
    };
  });

  return { layoutNodes: Array.from(nodeMap.values()), layoutLinks };
}

const DiscoveryPathsSankey: React.FC<DiscoveryPathsSankeyProps> = ({
  data,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [dims, setDims] = useState({ width: 700, height: 340 });
  const [tooltip, setTooltip] = useState<{
    x: number;
    y: number;
    content: string;
  } | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    const ro = new ResizeObserver((entries) => {
      const { width } = entries[0].contentRect;
      setDims({ width, height: 340 });
    });
    ro.observe(containerRef.current);
    const { width } = containerRef.current.getBoundingClientRect();
    setDims({ width, height: 340 });
    return () => ro.disconnect();
  }, []);

  const PAD = { top: 10, right: 80, bottom: 10, left: 10 };
  const innerW = dims.width - PAD.left - PAD.right;
  const innerH = dims.height - PAD.top - PAD.bottom;

  const { layoutNodes, layoutLinks } = computeLayout(
    data.nodes,
    data.links,
    innerW,
    innerH,
  );

  const linkPath = (link: LayoutLink) => {
    const x0 = link.source.x1;
    const x1 = link.target.x0;
    const midX = (x0 + x1) / 2;
    const y0 = link.y0 + link.width / 2;
    const y1 = link.y1 + link.width / 2;
    return `M${x0},${y0} C${midX},${y0} ${midX},${y1} ${x1},${y1}`;
  };

  return (
    <div className="bg-card rounded-2xl border border-border p-6 flex flex-col h-full">
      <div className="mb-4">
        <h3 className="text-lg font-semibold text-foreground">
          Contract Discovery Paths
        </h3>
        <p className="text-xs text-muted-foreground mt-0.5">
          How users navigate from entry points to contract actions
        </p>
      </div>

      {/* Legend */}
      <div className="flex flex-wrap gap-3 mb-3">
        {Object.entries(CATEGORY_COLORS).map(([cat, color]) => (
          <div
            key={cat}
            className="flex items-center gap-1.5 text-xs text-muted-foreground"
          >
            <span
              className="w-2.5 h-2.5 rounded-sm"
              style={{ backgroundColor: color }}
            />
            <span className="capitalize">{cat}</span>
          </div>
        ))}
      </div>

      <div ref={containerRef} className="flex-1 relative">
        <svg
          width={dims.width}
          height={dims.height}
          className="overflow-visible"
        >
          <g transform={`translate(${PAD.left},${PAD.top})`}>
            {/* Links */}
            {layoutLinks.map((link, i) => (
              <path
                key={i}
                d={linkPath(link)}
                fill="none"
                stroke={CATEGORY_COLORS[link.source.category] ?? "#94a3b8"}
                strokeWidth={Math.max(1, link.width)}
                strokeOpacity={0.35}
                onMouseEnter={(e) => {
                  const rect = containerRef.current?.getBoundingClientRect();
                  if (!rect) return;
                  setTooltip({
                    x: e.clientX - rect.left,
                    y: e.clientY - rect.top - 10,
                    content: `${link.source.name} → ${link.target.name}: ${link.value.toLocaleString()} users`,
                  });
                }}
                onMouseLeave={() => setTooltip(null)}
                className="cursor-pointer hover:stroke-opacity-60 transition-all"
              />
            ))}

            {/* Nodes */}
            {layoutNodes.map((node) => (
              <g key={node.id}>
                <rect
                  x={node.x0}
                  y={node.y0}
                  width={NODE_WIDTH}
                  height={Math.max(4, node.y1 - node.y0)}
                  fill={CATEGORY_COLORS[node.category] ?? "#94a3b8"}
                  rx={3}
                  className="cursor-pointer"
                  onMouseEnter={(e) => {
                    const rect = containerRef.current?.getBoundingClientRect();
                    if (!rect) return;
                    setTooltip({
                      x: e.clientX - rect.left,
                      y: e.clientY - rect.top - 10,
                      content: `${node.name}: ${node.value.toLocaleString()} users`,
                    });
                  }}
                  onMouseLeave={() => setTooltip(null)}
                />
                <text
                  x={node.x1 + 6}
                  y={(node.y0 + node.y1) / 2}
                  dominantBaseline="middle"
                  fontSize={11}
                  fill="var(--muted-foreground)"
                >
                  {node.name}
                </text>
              </g>
            ))}
          </g>
        </svg>

        {/* Tooltip */}
        {tooltip && (
          <div
            className="absolute z-10 pointer-events-none px-2.5 py-1.5 rounded-lg text-xs bg-card border border-border shadow-lg text-foreground whitespace-nowrap"
            style={{ left: tooltip.x + 8, top: tooltip.y }}
          >
            {tooltip.content}
          </div>
        )}
      </div>
    </div>
  );
};

export default DiscoveryPathsSankey;
