"use client";

import { useMemo, useState } from "react";
import type { ComparableContract } from "@/utils/comparison";
import { diffLines, diffMethodSets } from "@/utils/comparison";

type Props = {
  contracts: ComparableContract[];
  baselineId: string | null;
  onBaselineIdChange: (id: string) => void;
};

type Tab = "abi" | "source";

function lineClass(type: "context" | "add" | "remove") {
  if (type === "add")
    return "bg-green-500/10 text-green-700 dark:text-green-300";
  if (type === "remove") return "bg-red-500/10 text-red-700 dark:text-red-300";
  return "text-muted-foreground";
}

export default function DiffViewer({
  contracts,
  baselineId,
  onBaselineIdChange,
}: Props) {
  const [tab, setTab] = useState<Tab>("abi");

  const baseline = useMemo(
    () => contracts.find((c) => c.id === baselineId) ?? contracts[0] ?? null,
    [contracts, baselineId],
  );
  const others = useMemo(
    () => contracts.filter((c) => c.id !== baseline?.id),
    [contracts, baseline?.id],
  );

  const abiDiffs = useMemo(() => {
    if (!baseline) return [];
    return others.map((c) => ({
      contract: c,
      diff: diffMethodSets(baseline.abiMethods, c.abiMethods),
    }));
  }, [baseline, others]);

  const sourceDiffs = useMemo(() => {
    if (!baseline) return [];
    return others.map((c) => ({
      contract: c,
      lines: diffLines(baseline.sourceCode, c.sourceCode),
    }));
  }, [baseline, others]);

  if (!baseline) {
    return (
      <div className="rounded-2xl border border-border bg-card p-6">
        <div className="text-sm font-semibold text-foreground">Diff view</div>
        <div className="mt-1 text-xs text-muted-foreground">
          Select contracts to compare.
        </div>
      </div>
    );
  }

  return (
    <div className="rounded-2xl border border-border bg-card p-5">
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <div className="text-sm font-semibold text-foreground">
              Diff view
            </div>
            <div className="mt-1 text-xs text-muted-foreground">
              Baseline: compare other contracts against {baseline.name} (
              {baseline.latestVersion}).
            </div>
          </div>

          <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
            <select
              value={baseline.id}
              onChange={(e) => onBaselineIdChange(e.target.value)}
              className="rounded-xl border border-border bg-background px-3 py-2 text-sm text-foreground"
            >
              {contracts.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name} ({c.latestVersion})
                </option>
              ))}
            </select>

            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => setTab("abi")}
                className={`rounded-xl border px-3 py-2 text-sm font-semibold transition-colors ${
                  tab === "abi"
                    ? "border-primary/30 bg-primary/10 text-primary"
                    : "border-border bg-background text-muted-foreground hover:bg-accent hover:text-foreground"
                }`}
              >
                ABI methods
              </button>
              <button
                type="button"
                onClick={() => setTab("source")}
                className={`rounded-xl border px-3 py-2 text-sm font-semibold transition-colors ${
                  tab === "source"
                    ? "border-primary/30 bg-primary/10 text-primary"
                    : "border-border bg-background text-muted-foreground hover:bg-accent hover:text-foreground"
                }`}
              >
                Source code
              </button>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
          <div className="rounded-xl border border-border bg-background p-4">
            <div className="text-xs font-semibold text-muted-foreground">
              Baseline version
            </div>
            <div className="mt-1 text-sm font-semibold text-foreground">
              {baseline.latestVersion}
            </div>
          </div>
          <div className="rounded-xl border border-border bg-background p-4">
            <div className="text-xs font-semibold text-muted-foreground">
              Known versions
            </div>
            <div className="mt-1 text-sm font-semibold text-foreground">
              {baseline.versionCount}
            </div>
          </div>
          <div className="rounded-xl border border-border bg-background p-4">
            <div className="text-xs font-semibold text-muted-foreground">
              ABI methods
            </div>
            <div className="mt-1 text-sm font-semibold text-foreground">
              {baseline.abiMethods.length}
            </div>
          </div>
        </div>

        {others.length === 0 ? (
          <div className="rounded-xl border border-border bg-accent/30 p-4 text-sm text-muted-foreground">
            Add at least 2 contracts to see diffs.
          </div>
        ) : tab === "abi" ? (
          <div className="grid grid-cols-1 gap-4">
            {abiDiffs.map(({ contract, diff }) => (
              <div
                key={contract.id}
                className="rounded-2xl border border-border bg-background p-5"
              >
                <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
                  <div>
                    <div className="text-sm font-semibold text-foreground">
                      {contract.name}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {contract.latestVersion} vs {baseline.latestVersion}
                    </div>
                  </div>
                  <div className="text-xs text-muted-foreground">
                    +{diff.added.length} / -{diff.removed.length}
                  </div>
                </div>

                <div className="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2">
                  <div>
                    <div className="text-xs font-semibold text-muted-foreground">
                      Added
                    </div>
                    <div className="mt-2 rounded-xl border border-border bg-card p-3">
                      {diff.added.length === 0 ? (
                        <div className="text-xs text-muted-foreground">
                          None
                        </div>
                      ) : (
                        <ul className="flex flex-col gap-1">
                          {diff.added.map((m) => (
                            <li
                              key={m}
                              className="font-mono text-xs text-green-700 dark:text-green-300"
                            >
                              + {m}
                            </li>
                          ))}
                        </ul>
                      )}
                    </div>
                  </div>
                  <div>
                    <div className="text-xs font-semibold text-muted-foreground">
                      Removed
                    </div>
                    <div className="mt-2 rounded-xl border border-border bg-card p-3">
                      {diff.removed.length === 0 ? (
                        <div className="text-xs text-muted-foreground">
                          None
                        </div>
                      ) : (
                        <ul className="flex flex-col gap-1">
                          {diff.removed.map((m) => (
                            <li
                              key={m}
                              className="font-mono text-xs text-red-700 dark:text-red-300"
                            >
                              - {m}
                            </li>
                          ))}
                        </ul>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-4">
            {sourceDiffs.map(({ contract, lines }) => (
              <div
                key={contract.id}
                className="rounded-2xl border border-border bg-background p-5"
              >
                <div>
                  <div className="text-sm font-semibold text-foreground">
                    {contract.name}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {contract.latestVersion} vs {baseline.latestVersion}
                  </div>
                </div>
                <div className="mt-3 overflow-hidden rounded-xl border border-border bg-card">
                  <div className="max-h-[560px] overflow-auto">
                    <pre className="p-3 font-mono text-xs leading-5">
                      {lines.map((l, idx) => (
                        <div
                          key={`${idx}-${l.type}-${l.value}`}
                          className={lineClass(l.type)}
                        >
                          <span className="inline-block w-5 select-none text-muted-foreground">
                            {l.type === "add"
                              ? "+"
                              : l.type === "remove"
                                ? "-"
                                : " "}
                          </span>
                          <span>{l.value}</span>
                        </div>
                      ))}
                    </pre>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
