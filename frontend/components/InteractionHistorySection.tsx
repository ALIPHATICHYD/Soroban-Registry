"use client";

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { InteractionsQueryParams } from "@/lib/api";
import { formatPublicKey, formatTransactionHash } from "@/lib/utils/formatting";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from "recharts";
import { AlertCircle, Users, Activity } from "lucide-react";

interface InteractionHistorySectionProps {
  contractId: string;
}

export default function InteractionHistorySection({
  contractId,
}: InteractionHistorySectionProps) {
  const [listParams, setListParams] = useState<InteractionsQueryParams>({
    limit: 20,
    offset: 0,
  });
  const [accountFilter, setAccountFilter] = useState("");
  const [methodFilter, setMethodFilter] = useState("");

  const {
    data: analytics,
    isLoading: analyticsLoading,
    error: analyticsError,
  } = useQuery({
    queryKey: ["contract-analytics", contractId],
    queryFn: () => api.getContractAnalytics(contractId),
  });

  const {
    data: interactions,
    isLoading: listLoading,
    error: listError,
  } = useQuery({
    queryKey: ["contract-interactions", contractId, listParams],
    queryFn: () => api.getContractInteractions(contractId, listParams),
  });

  const applyFilters = () => {
    setListParams((p) => ({
      ...p,
      offset: 0,
      account: accountFilter || undefined,
      method: methodFilter || undefined,
    }));
  };

  return (
    <section className="space-y-8">
      <h2 className="text-2xl font-bold text-foreground flex items-center gap-2">
        <Activity className="w-6 h-6" />
        Interaction History
      </h2>

      {/* Timeline + Top users row */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 bg-card rounded-2xl border border-border p-6">
          <h3 className="text-lg font-semibold text-foreground mb-4">
            Interaction frequency (last 30 days)
          </h3>
          {analyticsLoading && (
            <div className="h-64 flex items-center justify-center text-muted-foreground">
              Loading…
            </div>
          )}
          {analyticsError && (
            <div className="h-64 flex items-center gap-2 text-red-600 dark:text-red-400">
              <AlertCircle className="w-5 h-5" />
              Failed to load timeline
            </div>
          )}
          {analytics && !analyticsLoading && !analyticsError && (
            <div className="h-64">
              {analytics.timeline.every((d) => d.count === 0) ? (
                <div className="h-full flex items-center justify-center text-muted-foreground text-sm">
                  No interactions in this period
                </div>
              ) : (
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart
                    data={analytics.timeline}
                    margin={{ top: 8, right: 8, left: 0, bottom: 0 }}
                  >
                    <CartesianGrid
                      strokeDasharray="3 3"
                      className="stroke-gray-200 dark:stroke-gray-700"
                    />
                    <XAxis
                      dataKey="date"
                      tick={{ fontSize: 11, fill: "currentColor" }}
                      tickFormatter={(v) =>
                        typeof v === "string" ? v.slice(5) : v
                      }
                    />
                    <YAxis
                      tick={{ fontSize: 11, fill: "currentColor" }}
                      allowDecimals={false}
                    />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: "var(--tooltip-bg, #1e293b)",
                        border: "1px solid rgba(255,255,255,0.1)",
                        borderRadius: "8px",
                      }}
                      labelFormatter={(v) => `Date: ${v}`}
                      formatter={(value: unknown) => [
                        String(value),
                        "Interactions",
                      ]}
                    />
                    <Bar
                      dataKey="count"
                      fill="rgb(59, 130, 246)"
                      radius={[4, 4, 0, 0]}
                    />
                  </BarChart>
                </ResponsiveContainer>
              )}
            </div>
          )}
        </div>

        <div className="bg-card rounded-2xl border border-border p-6">
          <h3 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
            <Users className="w-5 h-5" />
            Top users
          </h3>
          {analyticsLoading && (
            <div className="space-y-2">
              {[1, 2, 3].map((i) => (
                <div key={i} className="h-10 bg-muted rounded animate-pulse" />
              ))}
            </div>
          )}
          {analyticsError && (
            <p className="text-sm text-red-600 dark:text-red-400 flex items-center gap-2">
              <AlertCircle className="w-4 h-4" />
              Failed to load
            </p>
          )}
          {analytics && !analyticsLoading && !analyticsError && (
            <>
              {analytics.interactors.top_users.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  No interaction data yet
                </p>
              ) : (
                <ul className="space-y-2">
                  {analytics.interactors.top_users.map((u) => (
                    <li
                      key={u.address}
                      className="flex justify-between items-center text-sm"
                    >
                      <span
                        className="font-mono text-muted-foreground truncate max-w-[140px]"
                        title={u.address}
                      >
                        {formatPublicKey(u.address)}
                      </span>
                      <span className="font-medium text-foreground tabular-nums">
                        {u.count}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
              {analytics.interactors.unique_count > 0 && (
                <p className="text-xs text-muted-foreground mt-3">
                  {analytics.interactors.unique_count} unique account(s)
                </p>
              )}
            </>
          )}
        </div>
      </div>

      {/* Filters + Interactions list */}
      <div className="bg-card rounded-2xl border border-border p-6">
        <h3 className="text-lg font-semibold text-foreground mb-4">
          Recent interactions
        </h3>
        <div className="flex flex-wrap gap-3 mb-4">
          <input
            type="text"
            placeholder="Filter by account"
            value={accountFilter}
            onChange={(e) => setAccountFilter(e.target.value)}
            className="px-3 py-2 rounded-lg border border-border bg-card text-foreground text-sm font-mono w-48"
          />
          <input
            type="text"
            placeholder="Filter by method"
            value={methodFilter}
            onChange={(e) => setMethodFilter(e.target.value)}
            className="px-3 py-2 rounded-lg border border-border bg-card text-foreground text-sm w-40"
          />
          <button
            type="button"
            onClick={applyFilters}
            className="px-4 py-2 rounded-lg bg-primary hover:bg-primary/90 text-primary-foreground text-sm font-medium"
          >
            Apply
          </button>
        </div>
        {listLoading && (
          <div className="space-y-2">
            {[1, 2, 3, 4, 5].map((i) => (
              <div key={i} className="h-12 bg-muted rounded animate-pulse" />
            ))}
          </div>
        )}
        {listError && (
          <p className="text-red-600 dark:text-red-400 flex items-center gap-2">
            <AlertCircle className="w-4 h-4" />
            Failed to load interactions
          </p>
        )}
        {interactions && !listLoading && !listError && (
          <>
            {interactions.items.length === 0 ? (
              <p className="text-muted-foreground py-6 text-center">
                No interactions match the filters
              </p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-border text-left text-muted-foreground">
                      <th className="py-2 pr-4">Account</th>
                      <th className="py-2 pr-4">Method</th>
                      <th className="py-2 pr-4">Tx hash</th>
                      <th className="py-2 pr-4">Time</th>
                    </tr>
                  </thead>
                  <tbody>
                    {interactions.items.map((row) => (
                      <tr key={row.id} className="border-b border-border">
                        <td
                          className="py-2 pr-4 font-mono text-xs text-muted-foreground max-w-[120px] truncate"
                          title={row.account ?? ""}
                        >
                          {row.account ? formatPublicKey(row.account) : "—"}
                        </td>
                        <td className="py-2 pr-4 font-medium text-foreground">
                          {row.method ?? "—"}
                        </td>
                        <td
                          className="py-2 pr-4 font-mono text-xs text-muted-foreground max-w-[80px] truncate"
                          title={row.transaction_hash ?? ""}
                        >
                          {row.transaction_hash
                            ? formatTransactionHash(row.transaction_hash)
                            : "—"}
                        </td>
                        <td className="py-2 pr-4 text-muted-foreground whitespace-nowrap">
                          {new Date(row.created_at).toLocaleString()}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
            {interactions.total > interactions.items.length && (
              <div className="mt-4 flex items-center justify-between">
                <p className="text-sm text-muted-foreground">
                  Showing {interactions.offset + 1}–
                  {interactions.offset + interactions.items.length} of{" "}
                  {interactions.total}
                </p>
                <div className="flex gap-2">
                  <button
                    type="button"
                    disabled={listParams.offset === 0}
                    onClick={() =>
                      setListParams((p) => ({
                        ...p,
                        offset: Math.max(0, (p.offset ?? 0) - (p.limit ?? 50)),
                      }))
                    }
                    className="px-3 py-1 rounded border border-border text-sm disabled:opacity-50"
                  >
                    Previous
                  </button>
                  <button
                    type="button"
                    disabled={
                      interactions.offset + interactions.items.length >=
                      interactions.total
                    }
                    onClick={() =>
                      setListParams((p) => ({
                        ...p,
                        offset: (p.offset ?? 0) + (p.limit ?? 50),
                      }))
                    }
                    className="px-3 py-1 rounded border border-border text-sm disabled:opacity-50"
                  >
                    Next
                  </button>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </section>
  );
}
