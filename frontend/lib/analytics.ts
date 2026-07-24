// lib/analytics.ts
declare global {
  interface Window {
    gtag?: (...args: unknown[]) => void;
    plausible?: (eventName: string, options?: Record<string, unknown>) => void;
    mixpanel?: {
      init: (token: string) => void;
      track: (name: string, params?: Record<string, unknown>) => void;
    };
    dataLayer?: unknown[];
  }
}

export type AnalyticsProvider = "ga" | "plausible" | "mixpanel";

const provider = process.env
  .NEXT_PUBLIC_ANALYTICS_PROVIDER as AnalyticsProvider;

// Initialize based on provider
export const initAnalytics = () => {
  if (provider === "ga" && process.env.NEXT_PUBLIC_GA_ID) {
    if (!window.gtag) {
      const script = document.createElement("script");
      script.src = `https://www.googletagmanager.com/gtag/js?id=${process.env.NEXT_PUBLIC_GA_ID}`;
      script.async = true;
      document.head.appendChild(script);

      window.dataLayer = window.dataLayer || [];
      function gtag(...args: unknown[]) {
        window.dataLayer?.push(args);
      }
      window.gtag = gtag;

      window.gtag("js", new Date());

      // Wait for GA script to fully load before calling config
      script.onload = () => {
        if (window.gtag) {
          window.gtag("config", process.env.NEXT_PUBLIC_GA_ID, {
            anonymize_ip: true,
            ...(process.env.NODE_ENV !== "production" && { debug_mode: true }),
          });
        }
      };
    }
  }

  if (provider === "plausible" && process.env.NEXT_PUBLIC_PLAUSIBLE_DOMAIN) {
    if (!window.plausible) {
      const script = document.createElement("script");
      script.src = `https://plausible.io/js/plausible.js`;
      script.defer = true;
      script.dataset.domain = process.env.NEXT_PUBLIC_PLAUSIBLE_DOMAIN;
      document.head.appendChild(script);
      window.plausible = (
        eventName: string,
        options?: Record<string, unknown>,
      ) => {
        if (
          typeof (
            window as Window & {
              plausible?: (e: string, o?: Record<string, unknown>) => void;
            }
          ).plausible === "function"
        ) {
          (
            window as Window & {
              plausible: (e: string, o?: Record<string, unknown>) => void;
            }
          ).plausible(eventName, options);
        }
      };
    }
  }

  if (provider === "mixpanel" && process.env.NEXT_PUBLIC_MIXPANEL_TOKEN) {
    if (!window.mixpanel) {
      const script = document.createElement("script");
      script.src = "https://cdn.mxpnl.com/libs/mixpanel-2-latest.min.js";
      script.async = true;
      script.onload = () => {
        window.mixpanel?.init(process.env.NEXT_PUBLIC_MIXPANEL_TOKEN ?? "");
      };
      document.head.appendChild(script);
    }
  }
};

// Track events in a unified way
export const trackEvent = (name: string, params?: Record<string, unknown>) => {
  if (provider === "ga" && window.gtag) {
    window.gtag("event", name, params);
  }
  if (provider === "plausible" && window.plausible)
    window.plausible(name, params);
  if (provider === "mixpanel" && window.mixpanel)
    window.mixpanel.track(name, params);
};
