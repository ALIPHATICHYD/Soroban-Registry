"use client";

import React, { useState } from "react";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
} from "recharts";
import { CategoryAnalytic } from "@/types/analytics";
import { BarChart2, PieChart as PieIcon } from "lucide-react";

interface CategoryPopularityChartProps {
  data: CategoryAnalytic[];
}

const COLORS = [
  "#3b82f6",
  "#8b5cf6",
  "#06b6d4",
  "#10b981",
  "#f59e0b",
  "#ef4444",
  "#ec4899",
  "#6366f1",
];

const CategoryPopularityChart: React.FC<CategoryPopularityChartProps> = ({
  data,
}) => {
  const [view, setView] = useState<"bar" | "pie">("bar");
  const [metric, setMetric] = useState<"searches" | "views" | "deployments">(
    "searches",
  );

  const pieData = data.map((d) => ({ name: d.category, value: d[metric] }));

  return (
    <div className="bg-card rounded-2xl border border-border p-6 flex flex-col h-full">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold text-foreground">
            Category Popularity
          </h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            Search, view, and deployment counts per category
          </p>
        </div>
        <div className="flex items-center gap-2">
          {/* Metric selector */}
          <div className="flex rounded-lg border border-border overflow-hidden">
            {(["searches", "views", "deployments"] as const).map((m) => (
              <button
                key={m}
                onClick={() => setMetric(m)}
                className={`px-2.5 py-1 text-xs font-medium transition-colors ${
                  metric === m
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:bg-accent"
                }`}
              >
                {m.charAt(0).toUpperCase() + m.slice(1)}
              </button>
            ))}
          </div>
          {/* View toggle */}
          <div className="flex rounded-lg border border-border overflow-hidden">
            <button
              onClick={() => setView("bar")}
              className={`p-1.5 transition-colors ${
                view === "bar"
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:bg-accent"
              }`}
              title="Bar chart"
            >
              <BarChart2 className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => setView("pie")}
              className={`p-1.5 transition-colors ${
                view === "pie"
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:bg-accent"
              }`}
              title="Pie chart"
            >
              <PieIcon className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>

      <div className="flex-1 min-h-[260px]">
        <ResponsiveContainer width="100%" height="100%">
          {view === "bar" ? (
            <BarChart
              data={data}
              margin={{ top: 5, right: 10, left: 0, bottom: 5 }}
            >
              <CartesianGrid
                strokeDasharray="3 3"
                stroke="var(--border)"
                opacity={0.5}
              />
              <XAxis
                dataKey="category"
                tick={{ fontSize: 11 }}
                stroke="var(--muted-foreground)"
              />
              <YAxis
                tick={{ fontSize: 11 }}
                stroke="var(--muted-foreground)"
                width={40}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: "var(--card)",
                  borderRadius: "10px",
                  border: "1px solid var(--border)",
                  fontSize: 12,
                }}
              />
              <Legend wrapperStyle={{ fontSize: 11 }} />
              {metric === "searches" && (
                <Bar
                  dataKey="searches"
                  name="Searches"
                  fill="#3b82f6"
                  radius={[3, 3, 0, 0]}
                />
              )}
              {metric === "views" && (
                <Bar
                  dataKey="views"
                  name="Views"
                  fill="#8b5cf6"
                  radius={[3, 3, 0, 0]}
                />
              )}
              {metric === "deployments" && (
                <Bar
                  dataKey="deployments"
                  name="Deployments"
                  fill="#10b981"
                  radius={[3, 3, 0, 0]}
                />
              )}
            </BarChart>
          ) : (
            <PieChart>
              <Pie
                data={pieData}
                cx="50%"
                cy="50%"
                outerRadius="70%"
                dataKey="value"
                nameKey="name"
                label={({ name, percent }) =>
                  (percent ?? 0) > 0.05
                    ? `${name} ${((percent ?? 0) * 100).toFixed(0)}%`
                    : ""
                }
                labelLine={false}
              >
                {pieData.map((_, index) => (
                  <Cell
                    key={`cell-${index}`}
                    fill={COLORS[index % COLORS.length]}
                  />
                ))}
              </Pie>
              <Tooltip
                contentStyle={{
                  backgroundColor: "var(--card)",
                  borderRadius: "10px",
                  border: "1px solid var(--border)",
                  fontSize: 12,
                }}
                formatter={(value) => [Number(value).toLocaleString(), metric]}
              />
              <Legend wrapperStyle={{ fontSize: 11 }} />
            </PieChart>
          )}
        </ResponsiveContainer>
      </div>
    </div>
  );
};

export default CategoryPopularityChart;
