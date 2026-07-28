import React from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";

interface DeploymentTrend {
  date: string;
  count: number;
}

interface DeploymentTrendGraphProps {
  data: DeploymentTrend[];
  dataKey?: string;
  metricLabel?: string;
}

export default function DeploymentTrendGraph({
  data,
  dataKey = "count",
  metricLabel = "Interactions",
}: DeploymentTrendGraphProps) {
  if (!data || data.length === 0) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground text-sm">
        No trend data available
      </div>
    );
  }

  // Format date correctly
  const formattedData = data.map((d) => {
    const dateObj = new Date(d.date);
    return {
      ...d,
      displayDate: dateObj.toLocaleDateString(undefined, {
        month: "short",
        day: "numeric",
      }),
    };
  });

  return (
    <div className="h-[300px] w-full">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart
          data={formattedData}
          margin={{ top: 10, right: 10, left: -20, bottom: 0 }}
        >
          <CartesianGrid
            strokeDasharray="3 3"
            vertical={false}
            stroke="hsl(var(--border))"
          />
          <XAxis
            dataKey="displayDate"
            axisLine={false}
            tickLine={false}
            tick={{ fill: "hsl(var(--muted-foreground))", fontSize: 12 }}
            dy={10}
          />
          <YAxis
            axisLine={false}
            tickLine={false}
            tick={{ fill: "hsl(var(--muted-foreground))", fontSize: 12 }}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: "hsl(var(--card))",
              borderColor: "hsl(var(--border))",
              borderRadius: "0.5rem",
              boxShadow: "0 4px 6px -1px rgb(0 0 0 / 0.1)",
            }}
            itemStyle={{ color: "#6366f1", fontWeight: 600 }}
            labelStyle={{
              color: "hsl(var(--foreground))",
              marginBottom: "4px",
            }}
          />
          <Line
            type="monotone"
            dataKey={dataKey}
            name={metricLabel}
            stroke="#6366f1"
            strokeWidth={3}
            dot={false}
            activeDot={{ r: 4 }}
            animationDuration={1500}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
