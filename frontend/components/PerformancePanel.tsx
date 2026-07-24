"use client";

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { AlertTriangle, Gauge, TimerReset } from "lucide-react";

interface PerformancePanelProps {
  contractId: string;
}

export default function PerformancePanel({
  contractId,
}: PerformancePanelProps) {
  const { data, isLoading, error } = useQuery({
    queryKey: ["contract-analytics", contractId],
    queryFn: () => api.getContractAnalytics(contractId),
  });

  if (isLoading) {
    return (
      <div className="text-muted-foreground text-sm">
        Loading performance data...
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="flex items-start gap-3 p-4 bg-destructive/10 border border-destructive/30 rounded-xl">
        <AlertTriangle className="w-5 h-5 text-destructive shrink-0 mt-0.5" />
        <div>
          <p className="font-medium text-sm text-destructive">
            Performance data unavailable
          </p>
          <p className="text-xs text-muted-foreground mt-1">
            {error
              ? (error as Error).message
              : "No performance metrics available for this contract."}
          </p>
        </div>
      </div>
    );
  }

  const timeline = data.timeline || [];

  return (
    <section className="space-y-6">
      <div className="flex items-center gap-3">
        <Gauge className="w-6 h-6 text-primary" />
        <div>
          <h2 className="text-2xl font-bold text-foreground">
            Activity Timeline
          </h2>
          <p className="text-sm text-muted-foreground">
            Contract interaction and deployment history across networks.
          </p>
        </div>
      </div>

      {timeline && timeline.length > 0 ? (
        <div className="rounded-2xl border border-border bg-card p-6">
          <div className="space-y-3">
            {timeline.map((entry, idx) => (
              <div
                key={idx}
                className="flex items-center gap-3 text-sm pb-3 border-b border-border last:border-0"
              >
                <TimerReset className="w-4 h-4 text-primary shrink-0" />
                <div>
                  <p className="text-foreground font-medium">{entry.date}</p>
                  <p className="text-xs text-muted-foreground">
                    {entry.count} interaction{entry.count !== 1 ? "s" : ""}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </div>
      ) : (
        <div className="rounded-2xl border border-border bg-card p-6 text-sm text-muted-foreground text-center">
          No activity history available yet.
        </div>
      )}
    </section>
  );
}
