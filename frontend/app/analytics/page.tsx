"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import Navbar from "@/components/Navbar";
import CategoryDistributionPie from "@/components/analytics/CategoryDistributionPie";
import DeploymentTrendGraph from "@/components/analytics/DeploymentTrendGraph";
import NetworkUsageBarChart from "@/components/analytics/NetworkUsageBarChart";
import TopContractsBarChart from "@/components/analytics/TopContractsBarChart";
import {
  AlertCircle,
  Download,
  RefreshCw,
  BarChart3,
  Clock,
  PieChart,
  Activity,
  CalendarRange,
} from "lucide-react";

type Timeframe = "7d" | "30d" | "90d" | "custom";
type VerificationFilter = "all" | "verified" | "unverified";

interface DashboardMetricResponse {
  total_contracts: number;
  active_deployments: number;
  this_month_interactions: number;
  time_range_start: string;
  time_range_end: string;
  category_distribution: Array<{ category: string; count: number }>;
  network_usage: Array<{ network: string; count: number }>;
  deployment_trends: Array<{ date: string; count: number }>;
  interaction_trends: Array<{ date: string; count: number }>;
  top_contracts: Array<{
    id: string;
    contract_id: string;
    name: string;
    network: string;
    category: string | null;
    is_verified: boolean;
    interaction_count: number;
  }>;
}

const POLL_INTERVAL_MS = 5 * 60 * 1000;

function downloadText(content: string, filename: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

function csvEscape(
  value: string | number | boolean | null | undefined,
): string {
  if (value === null || value === undefined) {
    return "";
  }
  const stringified = String(value);
  if (/[",\r\n]/.test(stringified)) {
    return `"${stringified.replace(/"/g, '""')}"`;
  }
  return stringified;
}

function encodeSvgToBase64(svgContent: string): string {
  const utf8Bytes = new TextEncoder().encode(svgContent);
  const binaryString = utf8Bytes.reduce(
    (result, byte) => result + String.fromCharCode(byte),
    "",
  );
  return btoa(binaryString);
}

export default function AnalyticsDashboard() {
  const [data, setData] = useState<DashboardMetricResponse | null>(null);
  const [timeframe, setTimeframe] = useState<Timeframe>("30d");
  const [network, setNetwork] = useState<string>("all");
  const [category, setCategory] = useState<string>("all");
  const [verification, setVerification] = useState<VerificationFilter>("all");
  const [customStartDate, setCustomStartDate] = useState<string>("");
  const [customEndDate, setCustomEndDate] = useState<string>("");
  const [knownCategories, setKnownCategories] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState(false);

  const queryString = useMemo(() => {
    const params = new URLSearchParams();
    params.set("limit", "10");
    params.set("top_limit", "10");
    params.set("timeframe", timeframe);
    if (timeframe === "custom") {
      if (customStartDate) params.set("start_date", customStartDate);
      if (customEndDate) params.set("end_date", customEndDate);
    }
    if (network !== "all") params.set("network", network);
    if (category !== "all") params.set("category", category);
    if (verification === "verified") params.set("verified", "true");
    if (verification === "unverified") params.set("verified", "false");
    return params.toString();
  }, [
    timeframe,
    customStartDate,
    customEndDate,
    network,
    category,
    verification,
  ]);

  const fetchData = useCallback(
    async (isAutoRefresh = false) => {
      if (!isAutoRefresh) setLoading(true);
      else setRefreshing(true);

      setError(false);
      try {
        const baseUrl = process.env.NEXT_PUBLIC_API_URL || "";
        const dashRes = await fetch(
          `${baseUrl}/api/analytics/dashboard?${queryString}`,
        );
        if (!dashRes.ok) throw new Error("Failed to fetch data");

        const dashJson: DashboardMetricResponse = await dashRes.json();
        setData(dashJson);
        setKnownCategories((prev) => {
          const merged = new Set(prev);
          dashJson.category_distribution.forEach((item) => {
            if (item.category && item.category.trim().length > 0) {
              merged.add(item.category);
            }
          });
          return Array.from(merged).sort((a, b) => a.localeCompare(b));
        });
      } catch (e) {
        console.error("Failed to load analytics", e);
        if (!isAutoRefresh) setError(true);
      } finally {
        if (!isAutoRefresh) setLoading(false);
        else setRefreshing(false);
      }
    },
    [queryString],
  );

  const exportDatasetAsJson = useCallback(() => {
    if (!data) return;
    const stamp = new Date().toISOString().slice(0, 10);
    downloadText(
      JSON.stringify(data, null, 2),
      `contract-analytics-${timeframe}-${stamp}.json`,
      "application/json",
    );
  }, [data, timeframe]);

  const exportDatasetAsCsv = useCallback(() => {
    if (!data) return;
    const sections: string[] = [];
    sections.push("Metric,Value");
    sections.push(
      `${csvEscape("Total Contracts")},${csvEscape(data.total_contracts)}`,
    );
    sections.push(
      `${csvEscape("Active Deployments")},${csvEscape(data.active_deployments)}`,
    );
    sections.push(
      `${csvEscape("This Month Interactions")},${csvEscape(data.this_month_interactions)}`,
    );

    sections.push("");
    sections.push("Interaction Trends");
    sections.push("Date,Interactions");
    data.interaction_trends.forEach((item) => {
      sections.push(`${csvEscape(item.date)},${csvEscape(item.count)}`);
    });

    sections.push("");
    sections.push("Top Contracts");
    sections.push("Name,Network,Category,Verified,Interactions");
    data.top_contracts.forEach((item) => {
      sections.push(
        [
          csvEscape(item.name),
          csvEscape(item.network),
          csvEscape(item.category || "Uncategorized"),
          csvEscape(item.is_verified),
          csvEscape(item.interaction_count),
        ].join(","),
      );
    });

    sections.push("");
    sections.push("Category Distribution");
    sections.push("Category,Contracts");
    data.category_distribution.forEach((item) => {
      sections.push(`${csvEscape(item.category)},${csvEscape(item.count)}`);
    });

    const stamp = new Date().toISOString().slice(0, 10);
    downloadText(
      sections.join("\n"),
      `contract-analytics-${timeframe}-${stamp}.csv`,
      "text/csv;charset=utf-8",
    );
  }, [data, timeframe]);

  const exportChartAsPng = useCallback(
    (chartContainerId: string, filename: string) => {
      const container = document.getElementById(chartContainerId);
      const svg = container?.querySelector("svg");
      if (!container || !svg) return;

      const serializer = new XMLSerializer();
      const source = serializer.serializeToString(svg);
      const image = new Image();
      const rect = container.getBoundingClientRect();

      image.onload = () => {
        const canvas = document.createElement("canvas");
        canvas.width = Math.max(Math.round(rect.width), 600);
        canvas.height = Math.max(Math.round(rect.height), 320);
        const ctx = canvas.getContext("2d");
        if (!ctx) return;
        ctx.fillStyle = "#ffffff";
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(image, 0, 0, canvas.width, canvas.height);
        const anchor = document.createElement("a");
        anchor.href = canvas.toDataURL("image/png");
        anchor.download = filename;
        anchor.click();
      };

      image.src = `data:image/svg+xml;base64,${encodeSvgToBase64(source)}`;
    },
    [],
  );

  useEffect(() => {
    fetchData();

    // Poll every 5 minutes for near-real-time updates.
    const interval = setInterval(() => {
      fetchData(true);
    }, POLL_INTERVAL_MS);

    return () => clearInterval(interval);
  }, [fetchData]);

  if (loading) {
    return (
      <div className="min-h-screen bg-background flex flex-col">
        <Navbar />
        <div className="flex-1 flex items-center justify-center">
          <div className="flex flex-col items-center gap-4">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
            <p className="text-sm text-muted-foreground animate-pulse">
              Loading real-time analytics...
            </p>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-background flex flex-col">
        <Navbar />
        <div className="flex-1 flex items-center justify-center p-4">
          <div className="bg-card p-8 rounded-2xl shadow-lg border border-red-500/20 max-w-md w-full text-center">
            <AlertCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
            <h2 className="text-xl font-bold text-foreground mb-2">
              Failed to load analytics
            </h2>
            <p className="text-sm text-muted-foreground mb-6">
              There was an issue connecting to the analytics service. Please
              check your connection and try again.
            </p>
            <button
              onClick={() => fetchData()}
              className="w-full py-2.5 bg-primary hover:opacity-90 text-primary-foreground rounded-lg inline-flex items-center justify-center transition-colors font-medium"
            >
              <RefreshCw className="w-4 h-4 mr-2" /> Try Again
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background flex flex-col">
      <Navbar />
      <main className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-8">
        <header className="flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 mb-1">
              <Activity className="w-5 h-5 text-primary" />
              <span className="text-xs font-bold text-primary uppercase tracking-wider">
                Live Metrics
              </span>
            </div>
            <h1 className="text-4xl font-black text-foreground tracking-tight">
              Ecosystem Insights
            </h1>
            <p className="text-muted-foreground mt-1.5 text-sm">
              Monitoring interaction trends, contract leaders, and category mix
              in real time.
            </p>
          </div>
          <div className="flex items-center gap-2 flex-wrap justify-end">
            <button
              onClick={exportDatasetAsCsv}
              disabled={!data || refreshing}
              className="px-3 py-2 border border-border rounded-lg bg-card hover:bg-muted transition-colors text-sm inline-flex items-center gap-2 disabled:opacity-50"
            >
              <Download className="w-4 h-4" /> CSV
            </button>
            <button
              onClick={exportDatasetAsJson}
              disabled={!data || refreshing}
              className="px-3 py-2 border border-border rounded-lg bg-card hover:bg-muted transition-colors text-sm inline-flex items-center gap-2 disabled:opacity-50"
            >
              <Download className="w-4 h-4" /> JSON
            </button>
            {refreshing && (
              <span className="text-[10px] text-muted-foreground flex items-center gap-1 animate-pulse">
                <RefreshCw className="w-3 h-3 animate-spin" /> Updating every
                5m...
              </span>
            )}
            <button
              onClick={() => fetchData()}
              disabled={refreshing}
              className="p-2 border border-border rounded-lg bg-card hover:bg-muted transition-colors"
            >
              <RefreshCw
                className={`w-4 h-4 ${refreshing ? "animate-spin" : ""}`}
              />
            </button>
          </div>
        </header>

        <section className="bg-card border border-border rounded-2xl p-4 sm:p-5">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-6 gap-3">
            <div className="lg:col-span-2">
              <label className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-1.5 block">
                Time Range
              </label>
              <div className="flex flex-wrap gap-2">
                {(["7d", "30d", "90d", "custom"] as Timeframe[]).map(
                  (value) => (
                    <button
                      key={value}
                      onClick={() => setTimeframe(value)}
                      className={`px-3 py-1.5 rounded-lg text-sm border transition-colors ${
                        timeframe === value
                          ? "bg-primary text-primary-foreground border-primary"
                          : "bg-background border-border hover:bg-muted"
                      }`}
                    >
                      {value.toUpperCase()}
                    </button>
                  ),
                )}
              </div>
            </div>

            <div>
              <label className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-1.5 block">
                Network
              </label>
              <select
                value={network}
                onChange={(e) => setNetwork(e.target.value)}
                className="w-full h-9 rounded-lg border border-border bg-background px-3 text-sm"
              >
                <option value="all">All Networks</option>
                <option value="mainnet">Mainnet</option>
                <option value="testnet">Testnet</option>
                <option value="futurenet">Futurenet</option>
              </select>
            </div>

            <div>
              <label className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-1.5 block">
                Category
              </label>
              <select
                value={category}
                onChange={(e) => setCategory(e.target.value)}
                className="w-full h-9 rounded-lg border border-border bg-background px-3 text-sm"
              >
                <option value="all">All Categories</option>
                {knownCategories.map((value) => (
                  <option key={value} value={value}>
                    {value}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-1.5 block">
                Verification
              </label>
              <select
                value={verification}
                onChange={(e) =>
                  setVerification(e.target.value as VerificationFilter)
                }
                className="w-full h-9 rounded-lg border border-border bg-background px-3 text-sm"
              >
                <option value="all">All</option>
                <option value="verified">Verified</option>
                <option value="unverified">Unverified</option>
              </select>
            </div>

            <div className="lg:col-span-1 flex items.end">
              <button
                onClick={() => fetchData()}
                className="w-full h-9 rounded-lg border border-border bg-background hover:bg-muted text-sm transition-colors inline-flex items-center justify-center gap-2"
              >
                <CalendarRange className="w-4 h-4" /> Apply
              </button>
            </div>
          </div>

          {timeframe === "custom" && (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 mt-3">
              <div>
                <label className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-1.5 block">
                  Custom Start
                </label>
                <input
                  type="date"
                  value={customStartDate}
                  onChange={(e) => setCustomStartDate(e.target.value)}
                  className="w-full h-9 rounded-lg border border-border bg-background px-3 text-sm"
                />
              </div>
              <div>
                <label className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-1.5 block">
                  Custom End
                </label>
                <input
                  type="date"
                  value={customEndDate}
                  onChange={(e) => setCustomEndDate(e.target.value)}
                  className="w-full h-9 rounded-lg border border-border bg-background px-3 text-sm"
                />
              </div>
            </div>
          )}

          <p className="text-xs text-muted-foreground mt-3">
            Active window: {data?.time_range_start || "-"} to{" "}
            {data?.time_range_end || "-"}
          </p>
        </section>

        {/* Top Stats Overview */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <div className="bg-card border border-border rounded-2xl p-6 shadow-sm">
            <p className="text-xs font-bold text-muted-foreground uppercase tracking-widest mb-1">
              Total Contracts
            </p>
            <p className="text-3xl font-black text-foreground">
              {data?.total_contracts?.toLocaleString() || 0}
            </p>
          </div>
          <div className="bg-card border border-border rounded-2xl p-6 shadow-sm">
            <p className="text-xs font-bold text-muted-foreground uppercase tracking-widest mb-1">
              Active Deployments
            </p>
            <p className="text-3xl font-black text-foreground">
              {data?.active_deployments?.toLocaleString() || 0}
            </p>
          </div>
          <div className="bg-card border border-border rounded-2xl p-6 shadow-sm bg-gradient-to-br from-card to-primary/5">
            <p className="text-xs font-bold text-primary uppercase tracking-widest mb-1">
              This Month Interactions
            </p>
            <p className="text-3xl font-black text-foreground">
              {data?.this_month_interactions?.toLocaleString() || 0}
            </p>
          </div>
        </div>

        <div className="grid grid-cols-1 xl:grid-cols-3 gap-8">
          <div className="xl:col-span-2 space-y-8">
            <section className="bg-card border border-border rounded-2xl shadow-sm overflow-hidden p-5 sm:p-6">
              <div className="flex items-center justify-between gap-3 mb-4">
                <div className="flex items-center gap-2">
                  <Clock className="w-5 h-5 text-primary" />
                  <h3 className="font-bold text-lg">Interaction Trends</h3>
                </div>
                <button
                  onClick={() =>
                    exportChartAsPng(
                      "interaction-trend-chart",
                      `interaction-trends-${Date.now()}.png`,
                    )
                  }
                  className="px-2.5 py-1.5 text-xs border border-border rounded-md hover:bg-muted transition-colors inline-flex items-center gap-1.5"
                >
                  <Download className="w-3.5 h-3.5" /> PNG
                </button>
              </div>
              <div
                id="interaction-trend-chart"
                className="h-[320px] sm:h-[360px]"
              >
                <DeploymentTrendGraph
                  data={data?.interaction_trends || []}
                  metricLabel="Interactions"
                />
              </div>
            </section>

            <section className="bg-card border border-border rounded-2xl shadow-sm overflow-hidden p-5 sm:p-6">
              <div className="flex items-center justify-between gap-3 mb-4">
                <div className="flex items-center gap-2">
                  <BarChart3 className="w-5 h-5 text-blue-500" />
                  <h3 className="font-bold text-lg">
                    Top Contracts by Interactions
                  </h3>
                </div>
                <button
                  onClick={() =>
                    exportChartAsPng(
                      "top-contracts-chart",
                      `top-contracts-${Date.now()}.png`,
                    )
                  }
                  className="px-2.5 py-1.5 text-xs border border-border rounded-md hover:bg-muted transition-colors inline-flex items-center gap-1.5"
                >
                  <Download className="w-3.5 h-3.5" /> PNG
                </button>
              </div>
              <div id="top-contracts-chart" className="h-[320px] sm:h-[360px]">
                <TopContractsBarChart data={data?.top_contracts || []} />
              </div>
            </section>
          </div>

          <div className="space-y-8">
            <section className="bg-card border border-border rounded-2xl shadow-sm p-5 sm:p-6">
              <div className="flex items-center justify-between gap-3 mb-4">
                <div className="flex items-center gap-2">
                  <PieChart className="w-5 h-5 text-pink-500" />
                  <h3 className="font-bold">Categories</h3>
                </div>
                <button
                  onClick={() =>
                    exportChartAsPng(
                      "categories-chart",
                      `categories-${Date.now()}.png`,
                    )
                  }
                  className="px-2.5 py-1.5 text-xs border border-border rounded-md hover:bg-muted transition-colors inline-flex items-center gap-1.5"
                >
                  <Download className="w-3.5 h-3.5" /> PNG
                </button>
              </div>
              <div id="categories-chart" className="h-[320px]">
                <CategoryDistributionPie
                  data={data?.category_distribution || []}
                />
              </div>
            </section>

            <section className="bg-card border border-border rounded-2xl shadow-sm p-5 sm:p-6">
              <div className="flex items-center justify-between gap-3 mb-4">
                <div className="flex items-center gap-2">
                  <BarChart3 className="w-5 h-5 text-cyan-500" />
                  <h3 className="font-bold">Network Usage</h3>
                </div>
                <button
                  onClick={() =>
                    exportChartAsPng(
                      "network-chart",
                      `network-usage-${Date.now()}.png`,
                    )
                  }
                  className="px-2.5 py-1.5 text-xs border border-border rounded-md hover:bg-muted transition-colors inline-flex items-center gap-1.5"
                >
                  <Download className="w-3.5 h-3.5" /> PNG
                </button>
              </div>
              <div id="network-chart" className="h-[320px]">
                <NetworkUsageBarChart
                  data={(data?.network_usage || []).map((i) => ({
                    network: i.network,
                    contract_count: i.count,
                  }))}
                />
              </div>
            </section>
          </div>
        </div>
      </main>
    </div>
  );
}
