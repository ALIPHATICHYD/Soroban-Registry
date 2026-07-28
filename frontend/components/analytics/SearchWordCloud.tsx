"use client";

import React, { useMemo, useState } from "react";
import { TopSearchTerm } from "@/types/analytics";
import { TrendingUp, TrendingDown, Minus } from "lucide-react";

interface SearchWordCloudProps {
  data: TopSearchTerm[];
}

const PALETTE = [
  "#3b82f6",
  "#8b5cf6",
  "#06b6d4",
  "#10b981",
  "#f59e0b",
  "#ef4444",
  "#ec4899",
  "#6366f1",
];

const SearchWordCloud: React.FC<SearchWordCloudProps> = ({ data }) => {
  const [hovered, setHovered] = useState<string | null>(null);

  const maxCount = useMemo(
    () => Math.max(...data.map((d) => d.count), 1),
    [data],
  );
  const minCount = useMemo(
    () => Math.min(...data.map((d) => d.count), 1),
    [data],
  );

  const sized = useMemo(
    () =>
      data.map((item, i) => {
        const ratio =
          (item.count - minCount) / Math.max(maxCount - minCount, 1);
        const fontSize = 12 + Math.round(ratio * 28); // 12px to 40px
        const opacity = 0.65 + ratio * 0.35;
        return {
          ...item,
          fontSize,
          opacity,
          color: PALETTE[i % PALETTE.length],
        };
      }),
    [data, maxCount, minCount],
  );

  const hoveredItem = hovered ? sized.find((d) => d.term === hovered) : null;

  return (
    <div className="bg-card rounded-2xl border border-border p-6 flex flex-col h-full">
      <div className="mb-4">
        <h3 className="text-lg font-semibold text-foreground">
          Popular Search Terms
        </h3>
        <p className="text-xs text-muted-foreground mt-0.5">
          Size reflects search frequency. Hover for details.
        </p>
      </div>

      <div className="flex-1 flex flex-wrap gap-2 items-center justify-center content-center min-h-[240px] px-2">
        {sized.map((item) => (
          <button
            key={item.term}
            onMouseEnter={() => setHovered(item.term)}
            onMouseLeave={() => setHovered(null)}
            className="transition-all duration-150 rounded-md px-1.5 py-0.5 hover:bg-accent focus:outline-none"
            style={{
              fontSize: item.fontSize,
              color: item.color,
              opacity: hovered && hovered !== item.term ? 0.3 : item.opacity,
              fontWeight:
                item.fontSize > 24 ? 700 : item.fontSize > 18 ? 600 : 500,
              transform: hovered === item.term ? "scale(1.1)" : "scale(1)",
            }}
          >
            {item.term}
          </button>
        ))}
      </div>

      {/* Detail tooltip */}
      <div
        className={`mt-4 rounded-xl border border-border bg-accent/50 p-3 transition-all duration-200 ${
          hoveredItem ? "opacity-100" : "opacity-0 pointer-events-none"
        }`}
      >
        {hoveredItem && (
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span
                className="text-base font-bold"
                style={{ color: hoveredItem.color }}
              >
                {hoveredItem.term}
              </span>
              <span className="text-sm text-muted-foreground">
                {hoveredItem.count.toLocaleString()} searches
              </span>
            </div>
            <div className="flex items-center gap-1 text-sm font-medium">
              {hoveredItem.growth > 5 ? (
                <>
                  <TrendingUp className="w-4 h-4 text-green-500" />
                  <span className="text-green-500">+{hoveredItem.growth}%</span>
                </>
              ) : hoveredItem.growth < -5 ? (
                <>
                  <TrendingDown className="w-4 h-4 text-red-500" />
                  <span className="text-red-500">{hoveredItem.growth}%</span>
                </>
              ) : (
                <>
                  <Minus className="w-4 h-4 text-muted-foreground" />
                  <span className="text-muted-foreground">
                    {hoveredItem.growth}%
                  </span>
                </>
              )}
              <span className="text-muted-foreground text-xs ml-1">
                vs prior period
              </span>
            </div>
          </div>
        )}
        {!hoveredItem && (
          <p className="text-xs text-muted-foreground text-center">
            Hover a term to see details
          </p>
        )}
      </div>
    </div>
  );
};

export default SearchWordCloud;
