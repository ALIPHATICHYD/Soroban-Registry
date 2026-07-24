"use client";

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useRealtime } from "./useRealtime";

export function useContractAutoRefresh(contractId?: string) {
  const { subscribe } = useRealtime();
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!contractId) return;

    // Subscribe to contract update events
    const unsubscribe = subscribe("contract_updated", (data: unknown) => {
      const typedData = data as Record<string, unknown>;
      if (typedData.contractId === contractId) {
        // Invalidate query to trigger refetch
        queryClient.invalidateQueries({
          queryKey: ["contract", contractId],
        });
        queryClient.invalidateQueries({
          queryKey: ["contract-dependencies", contractId],
        });
        queryClient.invalidateQueries({
          queryKey: ["contract-deprecation", contractId],
        });
      }
    });

    // Subscribe to deployment events for new contract information
    const unsubscribeDeploy = subscribe(
      "contract_deployed",
      (data: unknown) => {
        const typedData = data as Record<string, unknown>;
        if (typedData.contractId === contractId) {
          queryClient.invalidateQueries({
            queryKey: ["contract", contractId],
          });
        }
      },
    );

    return () => {
      unsubscribe();
      unsubscribeDeploy();
    };
  }, [contractId, queryClient, subscribe]);
}

export function useContractListAutoRefresh() {
  const { subscribe } = useRealtime();
  const queryClient = useQueryClient();

  useEffect(() => {
    // Subscribe to new deployments for the contract list
    const unsubscribe = subscribe("contract_deployed", () => {
      // Invalidate contract list queries
      queryClient.invalidateQueries({
        queryKey: ["contracts"],
      });
    });

    return () => {
      unsubscribe();
    };
  }, [queryClient, subscribe]);
}
