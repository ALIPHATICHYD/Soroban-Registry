"use client";

import React, { useMemo } from "react";
import {
  Columns2,
  AlignLeft,
  Download,
  MessageSquare,
  GitCompare,
} from "lucide-react";
import { useDiff } from "./diff/ContractDiffViewer/useDiff";
import { VersionSelect } from "./diff/ContractDiffViewer/VersionSelect";
import { DiffStats } from "./diff/ContractDiffViewer/DiffStats";
import { UnifiedView } from "./diff/ContractDiffViewer/UnifiedView";
import { SideBySideView } from "./diff/ContractDiffViewer/SideBySideView";

interface ContractDiffViewerProps {
  contractId: string;
  contractName?: string;
}

export default function ContractDiffViewer({
  contractId,
  contractName,
}: ContractDiffViewerProps) {
  const {
    viewMode,
    setViewMode,
    setFromVersion,
    setToVersion,
    comments,
    openThread,
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
    isPending,
  } = useDiff(contractId, contractName);

  const totalComments = useMemo(
    () => Object.values(comments).reduce((s, arr) => s + arr.length, 0),
    [comments],
  );

  if (versionsQuery.isPending) {
    return (
      <div className="rounded-2xl border border-border bg-card p-6 animate-pulse">
        <div className="h-5 w-40 rounded bg-border mb-3" />
        <div className="h-3 w-64 rounded bg-border" />
      </div>
    );
  }

  if (versionsQuery.isError || versions.length === 0) {
    return (
      <div className="rounded-2xl border border-border bg-card p-6">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <GitCompare size={16} />
          {versions.length === 0
            ? "No versions available to diff."
            : "Failed to load contract versions."}
        </div>
      </div>
    );
  }

  const sameVersion = effectiveFrom === effectiveTo;

  return (
    <div className="flex flex-col gap-4">
      <div className="rounded-2xl border border-border bg-card p-4">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
          <div className="flex flex-col gap-1">
            <div className="flex items-center gap-2">
              <GitCompare size={16} className="text-primary" />
              <span className="text-sm font-semibold text-foreground">
                Code Diff
              </span>
              {totalComments > 0 && (
                <span className="flex items-center gap-1 rounded-full bg-primary/10 px-2 py-0.5 text-xs font-semibold text-primary">
                  <MessageSquare size={11} />
                  {totalComments}
                </span>
              )}
            </div>
            <div className="text-xs text-muted-foreground">
              Compare source code between two versions of this contract.
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => setViewMode("unified")}
              className={`flex items-center gap-1.5 rounded-xl border px-3 py-1.5 text-xs font-semibold transition-colors ${
                viewMode === "unified"
                  ? "border-primary/30 bg-primary/10 text-primary"
                  : "border-border bg-background text-muted-foreground"
              }`}
            >
              <AlignLeft size={13} /> Unified
            </button>
            <button
              onClick={() => setViewMode("side-by-side")}
              className={`flex items-center gap-1.5 rounded-xl border px-3 py-1.5 text-xs font-semibold transition-colors ${
                viewMode === "side-by-side"
                  ? "border-primary/30 bg-primary/10 text-primary"
                  : "border-border bg-background text-muted-foreground"
              }`}
            >
              <Columns2 size={13} /> Side by side
            </button>
          </div>
        </div>

        <div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-2 md:grid-cols-4">
          <div className="col-span-2 md:col-span-1">
            <VersionSelect
              label="From (base)"
              versions={versions}
              value={effectiveFrom}
              onChange={setFromVersion}
              exclude={effectiveTo}
            />
          </div>
          <div className="col-span-2 md:col-span-1">
            <VersionSelect
              label="To (compare)"
              versions={versions}
              value={effectiveTo}
              onChange={setToVersion}
              exclude={effectiveFrom}
            />
          </div>
          <div className="col-span-2 flex items-end gap-2">
            <DiffStats stats={stats} />
            <button
              disabled={diffResult.length === 0}
              onClick={handleDownloadPatch}
              className="ml-auto flex items-center gap-1.5 rounded-xl border border-border bg-background px-3 py-1.5 text-xs font-semibold text-muted-foreground hover:text-foreground disabled:opacity-40 transition-colors"
            >
              <Download size={13} /> Download patch
            </button>
          </div>
        </div>
      </div>

      <div className="rounded-2xl border border-border bg-card overflow-hidden">
        {sameVersion ? (
          <div className="p-6 text-sm text-muted-foreground">
            Select different versions to compare.
          </div>
        ) : isPending ? (
          <div className="p-6 animate-pulse space-y-2">
            {[1, 2, 3, 4, 5, 6].map((i) => (
              <div
                key={i}
                className="h-3 rounded bg-border"
                style={{ width: `${60 + (i % 3) * 15}%` }}
              />
            ))}
          </div>
        ) : diffResult.length === 0 ? (
          <div className="p-6 text-sm text-muted-foreground">
            No differences found.
          </div>
        ) : viewMode === "unified" ? (
          <UnifiedView
            lines={diffResult}
            comments={comments}
            openThread={openThread}
            onToggleThread={handleToggleThread}
            onAddComment={handleAddComment}
            onCloseThread={() => handleToggleThread("")}
          />
        ) : (
          <SideBySideView
            rows={sideBySideRows}
            comments={comments}
            openThread={openThread}
            onToggleThread={handleToggleThread}
            onAddComment={handleAddComment}
          />
        )}
      </div>
    </div>
  );
}
