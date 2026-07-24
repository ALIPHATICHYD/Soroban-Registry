"use client";

import { trackEvent } from "../lib/analytics";

export const useAnalytics = () => {
  const logEvent = (name: string, params?: Record<string, unknown>) => {
    trackEvent(name, params);
  };

  return { logEvent };
};
