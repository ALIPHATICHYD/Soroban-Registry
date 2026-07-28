"use client";

import { useEffect, useCallback } from "react";
import { useSearchParams, useRouter, usePathname } from "next/navigation";
import type { QueryNode } from "@/types";

export function useSearchUrlSync(
  query: QueryNode | null,
  setQuery: (query: QueryNode) => void,
) {
  const searchParams = useSearchParams();
  const router = useRouter();
  const pathname = usePathname();

  // Load from URL on mount
  useEffect(() => {
    const adv = searchParams?.get("adv");
    if (adv) {
      try {
        const decoded = JSON.parse(atob(adv));
        setQuery(decoded);
      } catch (e) {
        console.error("Failed to decode advanced search query from URL", e);
      }
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Sync to URL
  const syncToUrl = useCallback(
    (newQuery: QueryNode) => {
      const params = new URLSearchParams(searchParams?.toString() ?? "");
      const encoded = btoa(JSON.stringify(newQuery));
      params.set("adv", encoded);

      // Clear basic search params to avoid confusion
      params.delete("query");
      params.delete("category");
      params.delete("network");
      params.delete("verified_only");
      params.delete("tag");

      router.replace(`${pathname}?${params.toString()}`, { scroll: false });
    },
    [pathname, router, searchParams],
  );

  const clearUrl = useCallback(() => {
    const params = new URLSearchParams(searchParams?.toString() ?? "");
    params.delete("adv");
    router.replace(`${pathname}?${params.toString()}`, { scroll: false });
  }, [pathname, router, searchParams]);

  return { syncToUrl, clearUrl };
}
