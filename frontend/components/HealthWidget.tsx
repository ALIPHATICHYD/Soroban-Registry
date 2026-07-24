"use client";

import type { Contract } from "@/types";
import { api } from "@/lib/api";
import { useQuery } from "@tanstack/react-query";
import {
  Activity,
  AlertTriangle,
  Shield,
  ShieldAlert,
  ShieldCheck,
} from "lucide-react";

interface HealthWidgetProps {
  contract: Contract;
}

export default function HealthWidget({ contract }: HealthWidgetProps) {
  const { data: health, isLoading } = useQuery({
    queryKey: ["health", contract.id],
    queryFn: () => api.getContractHealth(contract.id),
    retry: false,
  });

  if (isLoading) {
    return (
      <div className="animate-pulse flex items-center gap-2">
        <div className="h-4 w-4 bg-muted rounded-full" />
        <div className="h-2 w-16 bg-muted rounded" />
      </div>
    );
  }

  if (!health) return null;

  const statusColors = {
    healthy:
      "text-green-600 bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800",
    warning:
      "text-yellow-600 bg-yellow-50 dark:bg-yellow-900/20 border-yellow-200 dark:border-yellow-800",
    critical:
      "text-red-600 bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800",
  };

  const StatusIcon = {
    healthy: ShieldCheck,
    warning: AlertTriangle,
    critical: ShieldAlert,
  }[health.status];

  return (
    <div
      className={`mt-3 p-3 rounded-lg border ${statusColors[health.status]}`}
    >
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <StatusIcon className="w-4 h-4" />
          <span className="text-sm font-semibold capitalize">
            {health.status} Health
          </span>
        </div>
        <span className="text-sm font-bold">{health.total_score}/100</span>
      </div>

      <div className="grid grid-cols-2 gap-2 text-xs opacity-90 mb-3">
        <div className="flex items-center gap-1">
          <Activity className="w-3 h-3" />
          <span>
            Activity: {new Date(health.last_activity).toLocaleDateString()}
          </span>
        </div>
        <div className="flex items-center gap-1">
          <Shield className="w-3 h-3" />
          <span>Security: {health.security_score}/50</span>
        </div>
      </div>

      {health.recommendations && health.recommendations.length > 0 && (
        <div className="mt-2 text-xs border-t border-black/10 dark:border-white/10 pt-2 opacity-90">
          <p className="font-semibold mb-1">Recommendations:</p>
          <ul className="list-disc list-inside space-y-1">
            {health.recommendations.map((rec, i) => (
              <li key={i}>{rec}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
