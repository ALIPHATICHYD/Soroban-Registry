'use client';

import { useMemo, useState } from "react";
import dynamic from "next/dynamic";
import { BarChart2, Code2, Download, Link2 } from "lucide-react";
import { useComparison } from "@/hooks/useComparison";
import ContractSelector from "@/components/comparison/ContractSelector";
import ComparisonTable from "@/components/comparison/ComparisonTable";
import MobileComparisonCard from "@/components/comparison/MobileComparisonCard";
import { useCopy } from "@/hooks/useCopy";
import { exportComparisonToCsv, exportComparisonToPdf } from "@/utils/export";
import {
  uniqueMethodsPerContract,
  type ComparableContract,
} from "@/utils/comparison";

// ── Statistics section ────────────────────────────────────────────────────────

function StatisticsSection({ contracts }: { contracts: ComparableContract[] }) {
  const maxDeployments = Math.max(...contracts.map((c) => c.deploymentCount));
  const maxPopularity = Math.max(...contracts.map((c) => c.popularityScore));

  return (
    <div className="rounded-2xl border border-border bg-card p-5">
      <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Statistics</div>
      <div className="mt-3 overflow-x-auto">
        <table className="min-w-full text-sm">
          <thead>
            <tr className="border-b border-border">
              <th className="py-2 pr-4 text-left text-xs font-semibold text-muted-foreground">Metric</th>
              {contracts.map((c) => (
                <th key={c.id} className="py-2 px-3 text-left text-xs font-semibold text-foreground min-w-[160px]">
                  {c.name}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            <tr className="border-b border-border/50">
              <td className="py-2 pr-4 text-xs text-muted-foreground font-medium">Deployments</td>
              {contracts.map((c) => (
                <td key={c.id} className="py-2 px-3">
                  <span className={`rounded-lg px-2 py-1 text-xs font-semibold ${
                    c.deploymentCount === maxDeployments && maxDeployments > 0
                      ? 'bg-green-500/10 text-green-700 dark:text-green-300'
                      : 'text-foreground'
                  }`}>
                    {c.deploymentCount}
                  </span>
                </td>
              ))}
            </tr>
            <tr className="border-b border-border/50">
              <td className="py-2 pr-4 text-xs text-muted-foreground font-medium">Popularity score</td>
              {contracts.map((c) => (
                <td key={c.id} className="py-2 px-3">
                  <span className={`rounded-lg px-2 py-1 text-xs font-semibold ${
                    c.popularityScore === maxPopularity && maxPopularity > 0
                      ? 'bg-green-500/10 text-green-700 dark:text-green-300'
                      : 'text-foreground'
                  }`}>
                    {c.popularityScore}
                  </span>
                </td>
              ))}
            </tr>
            <tr className="border-b border-border/50">
              <td className="py-2 pr-4 text-xs text-muted-foreground font-medium">Versions</td>
              {contracts.map((c) => (
                <td key={c.id} className="py-2 px-3 text-xs text-foreground">{c.versionCount}</td>
              ))}
            </tr>
            <tr>
              <td className="py-2 pr-4 text-xs text-muted-foreground font-medium">ABI methods</td>
              {contracts.map((c) => (
                <td key={c.id} className="py-2 px-3 text-xs text-foreground">{c.abiMethods.length}</td>
              ))}
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ── Unique methods section ────────────────────────────────────────────────────

function UniqueMethodsSection({ contracts }: { contracts: ComparableContract[] }) {
  const uniqueMap = uniqueMethodsPerContract(contracts);
  const hasAnyUnique = contracts.some((c) => uniqueMap[c.id]?.length > 0);

  if (!hasAnyUnique) {
    return (
      <div className="rounded-2xl border border-border bg-card p-5">
        <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Unique to each contract</div>
        <p className="mt-3 text-xs text-muted-foreground">All selected contracts share the same ABI methods — no unique methods found.</p>
      </div>
    );
  }

  return (
    <div className="rounded-2xl border border-border bg-card p-5">
      <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Unique to each contract</div>
      <p className="mt-1 text-xs text-muted-foreground">Methods present in one contract but absent from all others.</p>
      <div className="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {contracts.map((c) => {
          const methods = uniqueMap[c.id] ?? [];
          return (
            <div key={c.id} className="rounded-xl border border-border bg-accent/30 p-3">
              <div className="mb-2 truncate text-xs font-semibold text-foreground">{c.name}</div>
              {methods.length === 0 ? (
                <p className="text-xs text-muted-foreground italic">No unique methods</p>
              ) : (
                <ul className="space-y-1">
                  {methods.map((m) => (
                    <li key={m} className="rounded bg-primary/10 px-2 py-0.5 font-mono text-[11px] text-primary">
                      {m}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ── Main page component ───────────────────────────────────────────────────────

const DiffViewer = dynamic(() => import('@/components/comparison/DiffViewer'), {
  ssr: false,
  loading: () => (
    <div className="rounded-2xl border border-border bg-card p-6">
      <div className="text-sm font-semibold text-foreground">Loading diff viewer...</div>
    </div>
  ),
});

export default function CompareContracts() {
  const [viewMode, setViewMode] = useState<'table' | 'diff'>('table');
  const {
    searchQuery,
    setSearchQuery,
    contractsSearch,
    selectedContracts,
    selectionError,
    selectionCountError,
    selectionCountValid,
    addContract,
    removeContract,
    metrics,
    metricTones,
    baselineId,
    setBaselineId,
  } = useComparison();

  const DiffViewer = useMemo(
    () =>
      dynamic(() => import("@/components/comparison/DiffViewer"), {
        ssr: false,
        loading: () => (
          <div className="rounded-2xl border border-border bg-card p-6">
            <div className="text-sm font-semibold text-foreground">
              Loading diff viewer...
            </div>
          </div>
        ),
      }),
    [],
  );

  const selectedChips = useMemo(
    () => selectedContracts.map((c) => ({ id: c.id, name: c.name })),
    [selectedContracts],
  );

  const canExport = selectedContracts.length >= 2;
  const { copy, copied, isCopying } = useCopy();

  const exportRowsForPdf = useMemo(
    () => [
      ...metrics.map((m) => ({
        label: m.label,
        values: selectedContracts.map((c) => m.getDisplayValue(c)),
      })),
      {
        label: 'Latest version',
        values: selectedContracts.map((c) => c.latestVersion),
      },
      {
        label: 'Version count',
        values: selectedContracts.map((c) => String(c.versionCount)),
      },
      {
        label: 'ABI methods',
        values: selectedContracts.map((c) => String(c.abiMethods.length)),
      },
      {
        label: 'Tags',
        values: selectedContracts.map((c) => (c.tags.length > 0 ? c.tags.join(', ') : 'None')),
      },
    ],
    [metrics, selectedContracts],
  );

  const exportMetricsForCsv = useMemo(
    () => [
      ...metrics.map((m) => ({
        key: m.key,
        label: m.label,
        getValue: (c: (typeof selectedContracts)[number]) =>
          m.key === 'verification_status' ? m.getDisplayValue(c) : m.getRawValue(c),
      })),
      {
        key: 'latest_version' as const,
        label: 'Latest version',
        getValue: (c: (typeof selectedContracts)[number]) => c.latestVersion,
      },
      {
        key: 'version_count' as const,
        label: 'Version count',
        getValue: (c: (typeof selectedContracts)[number]) => c.versionCount,
      },
      {
        key: 'abi_methods' as const,
        label: 'ABI methods',
        getValue: (c: (typeof selectedContracts)[number]) => c.abiMethods.length,
      },
    ],
    [metrics],
  );

  return (
    <main className="max-w-7xl mx-auto px-4 py-10 sm:px-6 lg:px-8">
      <div className="flex flex-col gap-6">
        <div className="rounded-2xl border border-border bg-card p-6">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <h1 className="text-2xl font-bold text-foreground">Compare Contracts</h1>
              <p className="mt-2 text-sm text-muted-foreground">
                Select 2-4 contracts, compare real contract details side-by-side, and inspect ABI and source differences.
              </p>
            </div>
            <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setViewMode('table')}
                  className={`inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-sm font-semibold transition-colors ${
                    viewMode === 'table'
                      ? 'border-primary/30 bg-primary/10 text-primary'
                      : 'border-border bg-background text-muted-foreground hover:bg-accent hover:text-foreground'
                  }`}
                >
                  <BarChart2 className="h-4 w-4" />
                  Table
                </button>
                <button
                  type="button"
                  onClick={() => setViewMode('diff')}
                  className={`inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-sm font-semibold transition-colors ${
                    viewMode === 'diff'
                      ? 'border-primary/30 bg-primary/10 text-primary'
                      : 'border-border bg-background text-muted-foreground hover:bg-accent hover:text-foreground'
                  }`}
                >
                  <Code2 className="h-4 w-4" />
                  Diff
                </button>
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  disabled={!canExport}
                  onClick={() => exportComparisonToCsv(selectedContracts, exportMetricsForCsv)}
                  className={`inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-sm font-semibold transition-colors ${
                    canExport
                      ? 'border-border bg-background text-muted-foreground hover:bg-accent hover:text-foreground'
                      : 'cursor-not-allowed border-border bg-muted text-muted-foreground opacity-70'
                  }`}
                >
                  <Download className="h-4 w-4" />
                  CSV
                </button>
                <button
                  type="button"
                  disabled={!canExport}
                  onClick={() => exportComparisonToPdf(selectedContracts, exportRowsForPdf)}
                  className={`inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-sm font-semibold transition-colors ${
                    canExport
                      ? 'border-border bg-background text-muted-foreground hover:bg-accent hover:text-foreground'
                      : 'cursor-not-allowed border-border bg-muted text-muted-foreground opacity-70'
                  }`}
                >
                  <Download className="h-4 w-4" />
                  PDF
                </button>
                <button
                  type="button"
                  onClick={() => copy(window.location.href, { successEventName: 'comparison_link_copied' })}
                  disabled={isCopying}
                  className="inline-flex items-center gap-2 rounded-xl border border-border bg-background px-3 py-2 text-sm font-semibold text-muted-foreground transition-colors hover:bg-accent hover:text-foreground disabled:opacity-60"
                >
                  <Link2 className="h-4 w-4" />
                  {copied ? 'Copied' : 'Share'}
                </button>
              </div>
            </div>
          </div>
        </div>

        <ContractSelector
          available={contractsSearch.items}
          isLoading={contractsSearch.isLoading}
          searchQuery={searchQuery}
          onSearchQueryChange={setSearchQuery}
          selected={selectedChips}
          onAdd={addContract}
          onRemove={removeContract}
          selectionError={selectionError}
          selectionCountError={selectionCountError}
        />

        {viewMode === 'table' ? (
          <div className="flex flex-col gap-4">
            {!selectionCountValid && (
              <div className="rounded-2xl border border-border bg-card p-6">
                <div className="text-sm font-semibold text-foreground">Add at least 2 contracts to compare.</div>
                <div className="mt-1 text-xs text-muted-foreground">Use the selector above to pick contracts.</div>
              </div>
            )}

            {selectedContracts.length > 0 && (
              <>
                {/* Summary cards — version, ABI, tags */}
                <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
                  <div className="rounded-2xl border border-border bg-card p-5">
                    <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Version comparison</div>
                    <div className="mt-3 space-y-2">
                      {selectedContracts.map((contract) => (
                        <div key={contract.id} className="flex items-center justify-between gap-3 rounded-xl bg-accent/40 px-3 py-2 text-sm">
                          <span className="truncate text-foreground">{contract.name}</span>
                          <span className="shrink-0 font-mono text-muted-foreground">
                            {contract.latestVersion} ({contract.versionCount})
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                  <div className="rounded-2xl border border-border bg-card p-5">
                    <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">ABI coverage</div>
                    <div className="mt-3 space-y-2">
                      {selectedContracts.map((contract) => (
                        <div key={contract.id} className="flex items-center justify-between gap-3 rounded-xl bg-accent/40 px-3 py-2 text-sm">
                          <span className="truncate text-foreground">{contract.name}</span>
                          <span className="shrink-0 font-semibold text-foreground">{contract.abiMethods.length} methods</span>
                        </div>
                      ))}
                    </div>
                  </div>
                  <div className="rounded-2xl border border-border bg-card p-5">
                    <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Tag summary</div>
                    <div className="mt-3 space-y-2">
                      {selectedContracts.map((contract) => (
                        <div key={contract.id} className="rounded-xl bg-accent/40 px-3 py-2 text-sm">
                          <div className="truncate font-medium text-foreground">{contract.name}</div>
                          <div className="mt-1 text-xs text-muted-foreground">
                            {contract.tags.length > 0 ? contract.tags.join(', ') : 'No tags'}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>

                {/* Statistics card — deployments & popularity */}
                <StatisticsSection contracts={selectedContracts} />

                {/* Main comparison table (metadata + ABI + deployments) */}
                <ComparisonTable contracts={selectedContracts} metrics={metrics} tones={metricTones} />

                {/* Unique methods per contract */}
                <UniqueMethodsSection contracts={selectedContracts} />

                {/* Mobile stacked view */}
                <div className="grid grid-cols-1 gap-4 lg:hidden">
                  {selectedContracts.map((c) => (
                    <MobileComparisonCard key={c.id} contract={c} metrics={metrics} tones={metricTones} />
                  ))}
                </div>
              </>
            )}
          </div>
        ) : (
          <DiffViewer contracts={selectedContracts} baselineId={baselineId} onBaselineIdChange={setBaselineId} />
        )}
      </div>
    </main>
  );
}
