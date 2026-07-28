"use client";

import React, { useState } from "react";
import { NetworkRegionPoint } from "@/types/analytics";

interface NetworkMapProps {
  data: NetworkRegionPoint[];
}

const NETWORK_COLORS: Record<string, { bg: string; dot: string }> = {
  Mainnet: { bg: "rgba(59,130,246,0.15)", dot: "#3b82f6" },
  Testnet: { bg: "rgba(139,92,246,0.15)", dot: "#8b5cf6" },
  Futurenet: { bg: "rgba(16,185,129,0.15)", dot: "#10b981" },
};

// Rough SVG coordinates for world regions (simplified)
const REGION_COORDS: Record<string, { x: number; y: number }> = {
  "North America": { x: 160, y: 120 },
  "Latin America": { x: 210, y: 240 },
  Europe: { x: 390, y: 95 },
  Africa: { x: 390, y: 195 },
  "Middle East": { x: 460, y: 145 },
  "Asia Pacific": { x: 580, y: 140 },
  Oceania: { x: 600, y: 265 },
};

const NetworkMap: React.FC<NetworkMapProps> = ({ data }) => {
  const [hovered, setHovered] = useState<string | null>(null);

  const grouped = data.reduce<Record<string, NetworkRegionPoint[]>>(
    (acc, d) => {
      if (!acc[d.region]) acc[d.region] = [];
      acc[d.region].push(d);
      return acc;
    },
    {},
  );

  const networks = Array.from(new Set(data.map((d) => d.network)));

  return (
    <div className="bg-card rounded-2xl border border-border p-6 flex flex-col h-full">
      <div className="mb-4">
        <h3 className="text-lg font-semibold text-foreground">
          Network Distribution
        </h3>
        <p className="text-xs text-muted-foreground mt-0.5">
          Contract activity by region and network type
        </p>
      </div>

      {/* Network legend */}
      <div className="flex gap-4 mb-3 flex-wrap">
        {networks.map((net) => {
          const c = NETWORK_COLORS[net] ?? {
            bg: "rgba(100,100,100,0.15)",
            dot: "#888",
          };
          return (
            <div
              key={net}
              className="flex items-center gap-1.5 text-xs text-muted-foreground"
            >
              <span
                className="w-2.5 h-2.5 rounded-full"
                style={{ backgroundColor: c.dot }}
              />
              {net}
            </div>
          );
        })}
        <span className="text-xs text-muted-foreground ml-auto">
          Bubble size = contract count
        </span>
      </div>

      {/* Simplified world SVG map */}
      <div className="flex-1 relative overflow-hidden rounded-xl bg-muted/30 border border-border/40">
        <svg
          viewBox="0 0 760 320"
          className="w-full h-full"
          style={{ minHeight: 200 }}
        >
          {/* Simple landmass shapes */}
          {/* North America */}
          <path
            d="M80,60 L240,60 L260,180 L200,220 L120,200 L80,140 Z"
            fill="var(--muted)"
            opacity={0.4}
          />
          {/* South America */}
          <path
            d="M160,230 L240,230 L250,310 L180,310 Z"
            fill="var(--muted)"
            opacity={0.4}
          />
          {/* Europe */}
          <path
            d="M330,50 L460,50 L470,120 L380,140 L330,110 Z"
            fill="var(--muted)"
            opacity={0.4}
          />
          {/* Africa */}
          <path
            d="M340,145 L450,145 L440,280 L360,280 Z"
            fill="var(--muted)"
            opacity={0.4}
          />
          {/* Asia */}
          <path
            d="M460,40 L700,40 L700,200 L520,200 L460,160 Z"
            fill="var(--muted)"
            opacity={0.4}
          />
          {/* Australia */}
          <path
            d="M560,245 L680,245 L675,305 L565,305 Z"
            fill="var(--muted)"
            opacity={0.4}
          />

          {/* Region bubbles */}
          {(() => {
            // Compute max over region totals to normalize bubble radii consistently
            const regionTotals = Object.values(grouped).map((points) =>
              points.reduce((sum, p) => sum + p.count, 0),
            );
            const maxRegionTotal = Math.max(...regionTotals, 1);

            return Object.entries(grouped).map(([region, points]) => {
              const coords = REGION_COORDS[region];
              if (!coords) return null;

              const totalCount = points.reduce((s, p) => s + p.count, 0);
              const radius = 10 + (totalCount / maxRegionTotal) * 28;
              const isHovered = hovered === region;

              // Show dominant network color
              const dominant = points.reduce((a, b) =>
                a.count > b.count ? a : b,
              );
              const col = NETWORK_COLORS[dominant.network] ?? {
                bg: "rgba(100,100,100,0.25)",
                dot: "#888",
              };

              return (
                <g
                  key={region}
                  onMouseEnter={() => setHovered(region)}
                  onMouseLeave={() => setHovered(null)}
                  className="cursor-pointer"
                >
                  <circle
                    cx={coords.x}
                    cy={coords.y}
                    r={radius + (isHovered ? 4 : 0)}
                    fill={col.dot}
                    opacity={isHovered ? 0.35 : 0.2}
                    className="transition-all duration-200"
                  />
                  <circle
                    cx={coords.x}
                    cy={coords.y}
                    r={isHovered ? 7 : 5}
                    fill={col.dot}
                    className="transition-all duration-200"
                  />
                  {isHovered && (
                    <foreignObject
                      x={coords.x + 10}
                      y={coords.y - 36}
                      width={180}
                      height={80}
                      className="overflow-visible"
                    >
                      <div className="bg-card border border-border rounded-lg p-2 shadow-lg text-xs">
                        <p className="font-semibold text-foreground mb-1">
                          {region}
                        </p>
                        {points.map((p) => (
                          <div
                            key={p.network}
                            className="flex justify-between gap-3"
                          >
                            <span className="text-muted-foreground">
                              {p.network}
                            </span>
                            <span className="font-medium text-foreground">
                              {p.count.toLocaleString()} ({p.percentage}%)
                            </span>
                          </div>
                        ))}
                      </div>
                    </foreignObject>
                  )}
                </g>
              );
            });
          })()}
        </svg>
      </div>

      {/* Table summary */}
      <div className="mt-4 grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-2">
        {Object.entries(grouped).map(([region, points]) => {
          const totalCount = points.reduce((s, p) => s + p.count, 0);
          const dominant = points.reduce((a, b) => (a.count > b.count ? a : b));
          const col = NETWORK_COLORS[dominant.network] ?? { dot: "#888" };
          return (
            <div
              key={region}
              className="flex items-center gap-2 p-2 rounded-lg border border-border/60 hover:border-primary/40 transition-colors cursor-default"
              onMouseEnter={() => setHovered(region)}
              onMouseLeave={() => setHovered(null)}
            >
              <span
                className="w-2 h-2 rounded-full shrink-0"
                style={{ backgroundColor: col.dot }}
              />
              <div className="min-w-0">
                <p className="text-xs font-medium text-foreground truncate">
                  {region}
                </p>
                <p className="text-[10px] text-muted-foreground">
                  {totalCount.toLocaleString()} contracts
                </p>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default NetworkMap;
