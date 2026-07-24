"use client";

import { useEffect } from "react";
import { usePathname } from "next/navigation";
import { useAnalytics } from "@/hooks/useAnalytics";

export default function UserInteractionTracker() {
  const pathname = usePathname();
  const { logEvent } = useAnalytics();

  useEffect(() => {
    const handleDocumentClick = (event: MouseEvent) => {
      const target = event.target as HTMLElement | null;
      const anchor = target?.closest("a[href]") as HTMLAnchorElement | null;
      if (!anchor) return;

      try {
        const href = anchor.getAttribute("href");
        if (!href) return;
        const url = new URL(href, window.location.origin);
        const isExternal = url.origin !== window.location.origin;
        if (!isExternal) return;

        logEvent("external_link_clicked", {
          href: url.toString(),
          host: url.host,
          path: pathname,
          link_text: anchor.textContent?.trim() || undefined,
        });
      } catch {
        // Ignore malformed href values.
      }
    };

    const handleFormSubmit = (event: SubmitEvent) => {
      const form = event.target as HTMLFormElement | null;
      if (!form) return;

      logEvent("form_submitted", {
        path: pathname,
        form_id: form.id || undefined,
        form_name: form.getAttribute("name") || undefined,
        form_action: form.getAttribute("action") || undefined,
        method: (form.getAttribute("method") || "get").toLowerCase(),
      });
    };

    const handleWindowError = (event: ErrorEvent) => {
      logEvent("error_event", {
        source: "window_error",
        path: pathname,
        message: event.message || "Unknown runtime error",
        filename: event.filename || undefined,
        lineno: event.lineno || undefined,
        colno: event.colno || undefined,
      });
    };

    const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
      const reason =
        typeof event.reason === "string"
          ? event.reason
          : event.reason instanceof Error
            ? event.reason.message
            : "Unhandled promise rejection";

      logEvent("error_event", {
        source: "unhandled_rejection",
        path: pathname,
        message: reason,
      });
    };

    document.addEventListener("click", handleDocumentClick);
    document.addEventListener("submit", handleFormSubmit);
    window.addEventListener("error", handleWindowError);
    window.addEventListener("unhandledrejection", handleUnhandledRejection);

    return () => {
      document.removeEventListener("click", handleDocumentClick);
      document.removeEventListener("submit", handleFormSubmit);
      window.removeEventListener("error", handleWindowError);
      window.removeEventListener(
        "unhandledrejection",
        handleUnhandledRejection,
      );
    };
  }, [pathname, logEvent]);

  return null;
}
