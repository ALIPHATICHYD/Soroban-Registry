"use client";

import React, { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, MetricCatalogEntry } from "@/lib/api";
import { Activity, BarChart3, Clock3, LineChart } from "lucide-react";

function toNumber(value?: number) {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  return 0;
}

function buildSparkline(values: number[], width = 320, height = 80) {
  if (values.length === 0) return "";
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = max - min || 1;

  const points = values.map((value, idx) => {
    const x = (idx / Math.max(values.length - 1, 1)) * (width - 8) + 4;
    const y = height - 4 - ((value - min) / range) * (height - 8);
    return `${x},${y}`;
  });

  return `M ${points.join(" L ")}`;
}

function formatMetricValue(value?: number) {
  if (value === undefined || value === null || !Number.isFinite(value))
    return "—";
  if (Math.abs(value) >= 1000)
    return value.toLocaleString(undefined, { maximumFractionDigits: 2 });
  return value.toFixed(2);
}

const resolutionOptions = [
  { value: "hour", label: "Hourly" },
  { value: "day", label: "Daily" },
];

type Props = {
  contractId: string;
};

export default function CustomMetricsPanel({ contractId }: Props) {
  const [selectedMetric, setSelectedMetric] = useState<string | null>(null);
  const [resolution, setResolution] = useState<"hour" | "day">("hour");

  const {
    data: catalog,
    isLoading: catalogLoading,
    isError: catalogError,
  } = useQuery({
    queryKey: ["custom-metrics-catalog", contractId],
    queryFn: () => api.getCustomMetricCatalog(contractId),
  });

  const metricName = selectedMetric || (catalog?.[0]?.metric_name ?? "");

  const {
    data: series,
    isLoading: seriesLoading,
    isError: seriesError,
  } = useQuery({
    queryKey: ["custom-metrics-series", contractId, metricName, resolution],
    queryFn: () =>
      api.getCustomMetricSeries(contractId, metricName || "", {
        resolution,
        limit: 48,
      }),
    enabled: !!metricName,
  });

  const latestPoint = useMemo(() => {
    if (!series || !series.points || series.points.length === 0) return null;
    return series.points[0];
  }, [series]);

  const sparkline = useMemo(() => {
    if (!series || !series.points) return "";
    const values = [...series.points]
      .reverse()
      .map((point) =>
        toNumber(point.avg_value ?? point.sum_value ?? point.p50_value),
      );
    return buildSparkline(values);
  }, [series]);

  const metricTypeLabel = series?.metric_type ?? "unknown";

  const metrics = useMemo(() => {
    if (!series) return [];
    const avg = latestPoint?.avg_value;
    const p95 = latestPoint?.p95_value;
    const max = latestPoint?.max_value;
    const sum = latestPoint?.sum_value;

    return [
      { label: "Latest Avg", value: formatMetricValue(avg), icon: Activity },
      { label: "P95", value: formatMetricValue(p95), icon: BarChart3 },
      { label: "Max", value: formatMetricValue(max), icon: LineChart },
      { label: "Sum", value: formatMetricValue(sum), icon: Clock3 },
    ];
  }, [series, latestPoint]);

  return (
    <section className="bg-card rounded-2xl border border-border p-6 space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="text-lg font-semibold text-foreground">
            Custom Metrics
          </h3>
          <p className="text-sm text-muted-foreground">
            Contract-emitted counters, gauges, and histograms.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <select
            className="text-sm rounded-md border border-border bg-card px-2 py-1"
            value={resolution}
            onChange={(event) =>
              setResolution(event.target.value as "hour" | "day")
            }
          >
            {resolutionOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      {catalogLoading ? (
        <div className="text-sm text-muted-foreground">Loading metrics…</div>
      ) : catalogError ? (
        <div className="text-sm text-red-600 dark:text-red-400">
          Failed to load custom metrics.
        </div>
      ) : catalog && catalog.length > 0 ? (
        <div className="flex flex-wrap gap-2">
          {catalog.map((entry: MetricCatalogEntry) => (
            <button
              key={entry.metric_name}
              onClick={() => setSelectedMetric(entry.metric_name)}
              className={`px-3 py-1.5 rounded-full text-sm border transition-colors ${
                entry.metric_name === metricName
                  ? "bg-primary text-primary-foreground border-primary"
                  : "bg-card text-muted-foreground border-border"
              }`}
            >
              {entry.metric_name}
            </button>
          ))}
        </div>
      ) : (
        <div className="text-sm text-muted-foreground">
          No custom metrics yet.
        </div>
      )}

      {seriesLoading ? (
        <div className="text-sm text-muted-foreground">Loading series…</div>
      ) : seriesError ? (
        <div className="text-sm text-red-600 dark:text-red-400">
          Failed to load metric series.
        </div>
      ) : series && series.points && series.points.length > 0 ? (
        <div className="space-y-4">
          <div className="rounded-lg border border-border bg-accent p-4">
            <div className="flex items-center justify-between">
              <div className="text-sm text-muted-foreground">
                Last {series.points.length} buckets
              </div>
              <div className="text-xs uppercase tracking-wide text-muted-foreground">
                {metricTypeLabel}
              </div>
            </div>
            <svg viewBox="0 0 320 80" className="w-full h-20 mt-2">
              {sparkline ? (
                <path
                  d={sparkline}
                  stroke="#2563eb"
                  strokeWidth="2"
                  fill="none"
                />
              ) : null}
            </svg>
          </div>

          <div className="grid grid-cols-2 gap-3">
            {metrics.map((metric) => (
              <div
                key={metric.label}
                className="rounded-lg border border-border bg-card p-3"
              >
                <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
                  <metric.icon className="w-4 h-4" />
                  {metric.label}
                </div>
                <div className="text-lg font-semibold text-foreground mt-1">
                  {metric.value}
                </div>
              </div>
            ))}
          </div>
        </div>
      ) : series ? (
        <div className="text-sm text-muted-foreground">
          No data points for this metric.
        </div>
      ) : null}
    </section>
  );
}
