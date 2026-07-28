"use client";

import { usePathname } from "next/navigation";
import { useEffect } from "react";
import { useAnalytics } from "@/hooks/useAnalytics";

export default function PageViewTracker() {
  const pathname = usePathname();
  const { logEvent } = useAnalytics();

  // Track page views on route change
  useEffect(() => {
    if (!pathname) return;

    logEvent("page_view", {
      path: pathname,
      referrer: document.referrer || "direct",
    });
    // console.log('Page view tracked:', pathname)
  }, [pathname, logEvent]);

  return null;
}
