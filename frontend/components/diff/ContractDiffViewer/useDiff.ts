"use client";

import { useState, useMemo, useCallback } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { ContractVersion } from "@/types";
import { diffLines } from "@/utils/comparison";
import {
  calcStats,
  toSideBySideRows,
  buildPatch,
  downloadPatch,
} from "./utils";
import type { DiffComment } from "./CommentThread";

export function useDiff(contractId: string, contractName?: string) {
  const [viewMode, setViewMode] = useState<"unified" | "side-by-side">(
    "unified",
  );
  const [fromVersion, setFromVersion] = useState<string>("");
  const [toVersion, setToVersion] = useState<string>("");
  const [comments, setComments] = useState<Record<string, DiffComment[]>>({});
  const [openThread, setOpenThread] = useState<string | null>(null);

  const versionsQuery = useQuery({
    queryKey: ["contract-versions", contractId],
    queryFn: () => api.getContractVersions(contractId),
    enabled: !!contractId,
  });

  const versions = useMemo(
    () => versionsQuery.data ?? [],
    [versionsQuery.data],
  );

  const effectiveFrom = useMemo(() => {
    if (fromVersion) return fromVersion;
    if (versions.length >= 2) return versions[versions.length - 2].version;
    return versions[0]?.version ?? "";
  }, [fromVersion, versions]);

  const effectiveTo = useMemo(() => {
    if (toVersion) return toVersion;
    return versions[versions.length - 1]?.version ?? "";
  }, [toVersion, versions]);

  const fromMeta = versions.find((v) => v.version === effectiveFrom) ?? null;
  const toMeta = versions.find((v) => v.version === effectiveTo) ?? null;

  const sourceQuery = useCallback((meta: ContractVersion | null) => {
    if (!meta?.source_url) return Promise.resolve<string>("");
    const rawUrl = meta.source_url.replace(
      /^https:\/\/github\.com\/([^/]+)\/([^/]+)\/blob\/([^/]+)\/(.+)$/,
      "https://raw.githubusercontent.com/$1/$2/$3/$4",
    );
    return fetch(rawUrl)
      .then((r) => (r.ok ? r.text() : ""))
      .catch(() => "");
  }, []);

  const fromSourceQuery = useQuery({
    queryKey: ["diff-source", contractId, effectiveFrom],
    queryFn: () => sourceQuery(fromMeta),
    enabled: !!fromMeta,
  });

  const toSourceQuery = useQuery({
    queryKey: ["diff-source", contractId, effectiveTo],
    queryFn: () => sourceQuery(toMeta),
    enabled: !!toMeta,
  });

  const fromSource =
    fromSourceQuery.data ??
    (fromMeta?.wasm_hash
      ? `// ${contractName ?? contractId} — v${effectiveFrom}\n// Source unavailable\n`
      : "");
  const toSource =
    toSourceQuery.data ??
    (toMeta?.wasm_hash
      ? `// ${contractName ?? contractId} — v${effectiveTo}\n// Source unavailable\n`
      : "");

  const diffResult = useMemo(() => {
    if (!effectiveFrom || !effectiveTo || effectiveFrom === effectiveTo)
      return [];
    return diffLines(fromSource, toSource);
  }, [fromSource, toSource, effectiveFrom, effectiveTo]);

  const stats = useMemo(() => calcStats(diffResult), [diffResult]);
  const sideBySideRows = useMemo(
    () => (viewMode === "side-by-side" ? toSideBySideRows(diffResult) : []),
    [diffResult, viewMode],
  );

  const handleToggleThread = useCallback(
    (key: string) => setOpenThread((prev) => (prev === key ? null : key)),
    [],
  );

  const handleAddComment = useCallback((lineKey: string, text: string) => {
    setComments((prev) => ({
      ...prev,
      [lineKey]: [
        ...(prev[lineKey] ?? []),
        {
          id: `${Date.now()}-${Math.random()}`,
          lineKey,
          text,
          createdAt: new Date().toISOString(),
        },
      ],
    }));
  }, []);

  const handleDownloadPatch = useCallback(() => {
    if (diffResult.length === 0) return;
    const patch = buildPatch(
      diffResult,
      `${contractName ?? contractId}@${effectiveFrom}`,
      `${contractName ?? contractId}@${effectiveTo}`,
    );
    downloadPatch(
      patch,
      `${contractName ?? contractId}-${effectiveFrom}-to-${effectiveTo}.patch`,
    );
  }, [diffResult, contractId, contractName, effectiveFrom, effectiveTo]);

  return {
    viewMode,
    setViewMode,
    fromVersion,
    setFromVersion,
    toVersion,
    setToVersion,
    comments,
    setComments,
    openThread,
    setOpenThread,
    versionsQuery,
    versions,
    effectiveFrom,
    effectiveTo,
    diffResult,
    stats,
    sideBySideRows,
    handleToggleThread,
    handleAddComment,
    handleDownloadPatch,
    isPending:
      versionsQuery.isPending ||
      fromSourceQuery.isPending ||
      toSourceQuery.isPending,
  };
}
