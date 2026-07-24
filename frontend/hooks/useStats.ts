import { useState, useEffect, useCallback } from "react";
import { fetchStats } from "@/lib/api/stats";
import type { StatsResponse, TimePeriod } from "@/types";

interface UseStatsReturn {
  data: StatsResponse | null;
  loading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

export function useStats(period: TimePeriod): UseStatsReturn {
  const [data, setData] = useState<StatsResponse | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<Error | null>(null);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await fetchStats(period);
      setData(result);
    } catch (err) {
      setError(err instanceof Error ? err : new Error("Failed to fetch stats"));
    } finally {
      setLoading(false);
    }
  }, [period]);

  useEffect(() => {
    requestAnimationFrame(() => void loadData());

    const intervalId = setInterval(() => {
      // Skip polling when the tab is not visible
      if (
        typeof document !== "undefined" &&
        document.visibilityState === "hidden"
      )
        return;
      // Background refresh without setting loading state
      fetchStats(period)
        .then(setData)
        .catch(() => {
          /* swallow polling errors silently */
        });
    }, 30000);

    return () => clearInterval(intervalId);
  }, [loadData, period]);

  return { data, loading, error, refetch: loadData };
}
