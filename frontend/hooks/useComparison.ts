"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useSearchParams } from "next/navigation";
import { api } from "@/lib/api";
import type { Contract, ContractVersion } from "@/types";
import {
  toComparableContract,
  getMetricValue,
  toneForMetricCell,
  type ComparableContract,
  type ComparisonMetricKey,
  type CellTone,
} from "@/utils/comparison";
import {
  parseContractIdsFromSearch,
  replaceUrlContractIds,
} from "@/utils/urlState";

type ComparisonMetric = {
  key: ComparisonMetricKey;
  label: string;
  getDisplayValue: (c: ComparableContract) => string;
  getRawValue: (c: ComparableContract) => string | number | boolean;
};

type MetricTones = Record<ComparisonMetricKey, Record<string, CellTone>>;

function uniq(ids: string[]) {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const id of ids) {
    if (!seen.has(id)) {
      seen.add(id);
      out.push(id);
    }
  }
  return out;
}

export function useComparison() {
  const searchParams = useSearchParams();

  const initialSelectedIds = useMemo(() => {
    const ids = parseContractIdsFromSearch(searchParams?.toString() ?? "");
    return uniq(ids).slice(0, 4);
  }, [searchParams]);

  const [selectedIds, setSelectedIds] = useState<string[]>(
    () => initialSelectedIds,
  );
  const [selectionError, setSelectionError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [baselineId, setBaselineId] = useState<string | null>(
    () => initialSelectedIds[0] ?? null,
  );

  useEffect(() => {
    replaceUrlContractIds(selectedIds);
  }, [selectedIds]);

  const contractsSearchQuery = useQuery({
    queryKey: ["compare", "contracts-search", searchQuery],
    queryFn: async () => {
      const res = await api.getContracts({
        query: searchQuery || undefined,
        page: 1,
        page_size: 25,
        sort_by: searchQuery ? "relevance" : "created_at",
        sort_order: "desc",
      });
      return res.items;
    },
    staleTime: 30_000,
  });

  const selectedContractsQuery = useQuery({
    queryKey: ["compare", "selected-contracts", selectedIds],
    queryFn: async () => {
      const ids = selectedIds.slice(0, 4);
      if (ids.length === 0) return [];
      const results = await Promise.all(
        ids.map(async (id) => {
          const c = await api.getContract(id);
          const versions = await api
            .getContractVersions(id)
            .catch(() => [] as ContractVersion[]);
          const latestVersion =
            [...versions].sort(
              (a, b) =>
                new Date(b.created_at).getTime() -
                new Date(a.created_at).getTime(),
            )[0] ?? null;

          const abiResponse =
            latestVersion != null
              ? await api
                  .getContractAbi(id, latestVersion.version)
                  .catch(() => ({ abi: null }))
              : { abi: null };

          let sourceCode = "";
          if (latestVersion?.source_url) {
            try {
              const normalizedSourceUrl = latestVersion.source_url.replace(
                /^https:\/\/github\.com\/([^/]+)\/([^/]+)\/blob\/([^/]+)\/(.+)$/,
                "https://raw.githubusercontent.com/$1/$2/$3/$4",
              );
              const res = await fetch(normalizedSourceUrl);
              if (res.ok) {
                sourceCode = await res.text();
              }
            } catch {
              sourceCode = "";
            }
          }

          return {
            contract: c,
            versions,
            abi: abiResponse.abi,
            sourceCode,
          };
        }),
      );
      return results;
    },
    enabled: selectedIds.length > 0,
    staleTime: 30_000,
  });

  const selectedContracts = useMemo<ComparableContract[]>(() => {
    const contracts = selectedContractsQuery.data ?? [];
    const byId = new Map<string, ComparableContract>();
    for (const item of contracts) {
      byId.set(
        item.contract.id,
        toComparableContract(item.contract, {
          versions: item.versions,
          abi: item.abi,
          sourceCode: item.sourceCode,
        }),
      );
    }
    return selectedIds
      .map((id) => byId.get(id))
      .filter((c): c is ComparableContract => Boolean(c));
  }, [selectedContractsQuery.data, selectedIds]);

  const effectiveBaselineId = useMemo(() => {
    if (!baselineId) return selectedContracts[0]?.id ?? null;
    return selectedContracts.some((c) => c.id === baselineId)
      ? baselineId
      : (selectedContracts[0]?.id ?? null);
  }, [baselineId, selectedContracts]);

  const metrics = useMemo<ComparisonMetric[]>(
    () => [
      {
        key: "contract_id",
        label: "Contract ID",
        getDisplayValue: (c) => c.contractId,
        getRawValue: (c) => c.contractId,
      },
      {
        key: "network",
        label: "Network",
        getDisplayValue: (c) => c.network,
        getRawValue: (c) => c.network,
      },
      {
        key: "category",
        label: "Category",
        getDisplayValue: (c) => c.category,
        getRawValue: (c) => c.category,
      },
      {
        key: "publisher",
        label: "Publisher",
        getDisplayValue: (c) => c.publisherId,
        getRawValue: (c) => c.publisherId,
      },
      {
        key: "verification_status",
        label: "Verification",
        getDisplayValue: (c) => (c.isVerified ? "Verified" : "Unverified"),
        getRawValue: (c) => c.isVerified,
      },
      {
        key: "wasm_hash",
        label: "WASM hash",
        getDisplayValue: (c) =>
          c.wasmHash ? `${c.wasmHash.slice(0, 12)}…` : "—",
        getRawValue: (c) => c.wasmHash,
      },
      {
        key: "deployment_count",
        label: "Deployments",
        getDisplayValue: (c) => String(c.deploymentCount),
        getRawValue: (c) => c.deploymentCount,
      },
      {
        key: "popularity_score",
        label: "Popularity score",
        getDisplayValue: (c) => String(c.popularityScore),
        getRawValue: (c) => c.popularityScore,
      },
    ],
    [],
  );

  const metricTones = useMemo<MetricTones>(() => {
    const tones = {} as MetricTones;
    for (const metric of metrics) {
      const values = selectedContracts.map((c) =>
        getMetricValue(c, metric.key),
      );
      tones[metric.key] = {};
      for (const c of selectedContracts) {
        tones[metric.key][c.id] = toneForMetricCell(
          metric.key,
          getMetricValue(c, metric.key),
          values,
        );
      }
    }
    return tones;
  }, [metrics, selectedContracts]);

  const addContract = useCallback(
    (contract: Pick<Contract, "id">) => {
      setSelectionError(null);

      setSelectedIds((prev) => {
        if (prev.includes(contract.id)) {
          setSelectionError("That contract is already selected.");
          return prev;
        }
        if (prev.length >= 4) {
          setSelectionError("You can compare up to 4 contracts.");
          return prev;
        }
        const next = [...prev, contract.id];
        if (!baselineId) setBaselineId(next[0] ?? null);
        return next;
      });
    },
    [baselineId],
  );

  const removeContract = useCallback((contractId: string) => {
    setSelectionError(null);
    setSelectedIds((prev) => prev.filter((id) => id !== contractId));
  }, []);

  const replaceSelectedIds = useCallback((ids: string[]) => {
    const next = uniq(ids).slice(0, 4);
    setSelectionError(null);
    setSelectedIds(next);
    setBaselineId(next[0] ?? null);
  }, []);

  const selectionCountValid =
    selectedIds.length >= 2 && selectedIds.length <= 4;
  const selectionCountError =
    selectedIds.length > 4
      ? "You can compare up to 4 contracts."
      : selectedIds.length > 0 && selectedIds.length < 2
        ? "Select at least 2 contracts to compare."
        : null;

  return {
    searchQuery,
    setSearchQuery,
    contractsSearch: {
      items: contractsSearchQuery.data ?? [],
      isLoading: contractsSearchQuery.isLoading,
    },
    selectedIds,
    selectedContracts,
    selectedContractsQuery: {
      isLoading: selectedContractsQuery.isLoading,
      isError: selectedContractsQuery.isError,
    },
    selectionError,
    selectionCountError,
    selectionCountValid,
    addContract,
    removeContract,
    replaceSelectedIds,
    metrics,
    metricTones,
    baselineId: effectiveBaselineId,
    setBaselineId,
  };
}
