'use client';

import { useReportWebVitals } from 'next/web-vitals';

// Inline type definition for Metric
type Metric = {
  id: string;
  name: string;
  value: number;
  rating: 'good' | 'needs-improvement' | 'poor';
  delta: number;
  entries: PerformanceEntry[];
  navigationType: 'navigate' | 'reload' | 'back-forward' | 'prerender';
};

export function useWebVitals() {
  useReportWebVitals((metric: Metric) => {
    const body = JSON.stringify(metric);
    const url = '/api/analytics/vitals';

    // Use `navigator.sendBeacon()` if available, falling back to `fetch()`.
    if (navigator.sendBeacon) {
      navigator.sendBeacon(url, body);
    } else {
      fetch(url, { body, method: 'POST', keepalive: true });
    }

    console.log('[Web Vitals]', metric);
  });
}
