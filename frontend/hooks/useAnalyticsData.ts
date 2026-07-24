import { useState, useEffect, useCallback } from "react";
import { fetchAnalytics } from "@/lib/api/analytics";
import type { AnalyticsResponse, TimePeriod } from "@/types";

interface UseAnalyticsDataReturn {
  data: AnalyticsResponse | null;
  loading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

export function useAnalyticsData(period: TimePeriod): UseAnalyticsDataReturn {
  const [data, setData] = useState<AnalyticsResponse | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<Error | null>(null);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await fetchAnalytics(period);
      setData(result);
    } catch (err) {
      setError(
        err instanceof Error ? err : new Error("Failed to fetch analytics"),
      );
    } finally {
      setLoading(false);
    }
  }, [period]);

  useEffect(() => {
    requestAnimationFrame(() => void loadData());

    const intervalId = setInterval(() => {
      if (
        typeof document !== "undefined" &&
        document.visibilityState === "hidden"
      )
        return;
      fetchAnalytics(period)
        .then(setData)
        .catch(() => {});
    }, 60000);

    return () => clearInterval(intervalId);
  }, [loadData, period]);

  return { data, loading, error, refetch: loadData };
}
