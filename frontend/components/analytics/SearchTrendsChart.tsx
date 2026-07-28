"use client";

import React, { useState } from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";
import { SearchTrendPoint } from "@/types/analytics";

interface SearchTrendsChartProps {
  data: SearchTrendPoint[];
}

const SearchTrendsChart: React.FC<SearchTrendsChartProps> = ({ data }) => {
  const [hiddenLines, setHiddenLines] = useState<Set<string>>(new Set());

  const toggleLine = (key: string) => {
    setHiddenLines((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const formatDate = (dateStr: string) => {
    const d = new Date(dateStr);
    return `${d.getMonth() + 1}/${d.getDate()}`;
  };

  return (
    <div className="bg-card rounded-2xl border border-border p-6 flex flex-col h-full">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold text-foreground">
            Search Trends
          </h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            Daily search volume and unique search terms
          </p>
        </div>
        <div className="flex gap-2">
          {[
            { key: "searches", label: "Searches", color: "#3b82f6" },
            { key: "uniqueTerms", label: "Unique Terms", color: "#8b5cf6" },
          ].map(({ key, label, color }) => (
            <button
              key={key}
              onClick={() => toggleLine(key)}
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium transition-all border ${
                hiddenLines.has(key)
                  ? "border-border text-muted-foreground bg-transparent"
                  : "border-transparent text-white"
              }`}
              style={hiddenLines.has(key) ? {} : { backgroundColor: color }}
            >
              <span
                className="w-2 h-2 rounded-full"
                style={{ backgroundColor: color }}
              />
              {label}
            </button>
          ))}
        </div>
      </div>
      <div className="flex-1 min-h-[280px]">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart
            data={data}
            margin={{ top: 5, right: 20, left: 0, bottom: 5 }}
          >
            <CartesianGrid
              strokeDasharray="3 3"
              stroke="var(--border)"
              opacity={0.5}
            />
            <XAxis
              dataKey="date"
              tickFormatter={formatDate}
              stroke="var(--muted-foreground)"
              tick={{ fontSize: 11 }}
              interval={Math.max(1, Math.floor(data.length / 8))}
            />
            <YAxis
              stroke="var(--muted-foreground)"
              tick={{ fontSize: 11 }}
              width={40}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "var(--card)",
                borderRadius: "10px",
                border: "1px solid var(--border)",
                boxShadow: "0 4px 12px rgba(0,0,0,0.1)",
                fontSize: 12,
              }}
              labelFormatter={(v) =>
                new Date(v).toLocaleDateString("en-US", {
                  month: "short",
                  day: "numeric",
                })
              }
            />
            <Legend wrapperStyle={{ fontSize: 12 }} />
            {!hiddenLines.has("searches") && (
              <Line
                type="monotone"
                dataKey="searches"
                name="Searches"
                stroke="#3b82f6"
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 5 }}
              />
            )}
            {!hiddenLines.has("uniqueTerms") && (
              <Line
                type="monotone"
                dataKey="uniqueTerms"
                name="Unique Terms"
                stroke="#8b5cf6"
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 5 }}
                strokeDasharray="4 2"
              />
            )}
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
};

export default SearchTrendsChart;
