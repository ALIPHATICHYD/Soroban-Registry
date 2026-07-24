"use client";

import { useQuery } from "@tanstack/react-query";
import { useParams } from "next/navigation";
import Link from "next/link";
import { api, type CompatibilityMatrix } from "@/lib/api";
import { CompatibilityMatrixDisplay } from "@/components/CompatibilityMatrix";
import { ArrowLeft, GitCompare, Loader2 } from "lucide-react";
import Navbar from "@/components/Navbar";

export default function CompatibilityPage() {
  const params = useParams<{ id?: string | string[] }>() ?? {};
  const idParam = params.id;
  const contractId = Array.isArray(idParam) ? idParam[0] : idParam;

  const { data: contract } = useQuery({
    queryKey: ["contract", contractId],
    queryFn: () => api.fetchContract(contractId!),
    enabled: !!contractId,
  });

  const {
    data: compatibility,
    isLoading,
    isError,
    error,
  } = useQuery({
    queryKey: ["compatibility", contractId],
    queryFn: async (): Promise<CompatibilityMatrix> => {
      const versions = await api.fetchContractVersions(contractId!);
      return {
        warnings: [],
        version_order: versions.map((version) => version.version),
        total_pairs: 0,
        rows: versions.map((version) => ({
          source_version: version.version,
          targets: [],
        })),
      };
    },
    enabled: !!contractId,
  });

  if (!contractId) {
    return (
      <div className="min-h-screen bg-background text-foreground">
        <Navbar />
        <div className="mx-auto max-w-4xl px-4 py-10">
          <div className="rounded-2xl border border-border bg-card p-6">
            <div className="text-sm font-semibold text-foreground">
              Missing contract id
            </div>
            <div className="mt-1 text-sm text-muted-foreground">
              Open this page from a contract details view.
            </div>
            <div className="mt-4">
              <Link
                href="/contracts"
                className="inline-flex items-center gap-2 rounded-xl border border-border bg-background px-3 py-2 text-sm font-semibold text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
              >
                Browse contracts
              </Link>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />
      <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <Link
          href={`/contracts/${contractId}`}
          className="mb-6 inline-flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to contract
        </Link>

        <div className="mb-8">
          <div className="mb-2 flex items-center gap-3">
            <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/10">
              <GitCompare className="h-5 w-5 text-primary" />
            </span>
            <h1 className="text-2xl font-bold text-foreground">
              Contract version compatibility
            </h1>
          </div>
          {contract && (
            <p className="ml-12 text-muted-foreground">
              {contract.name}{" "}
              <span className="rounded bg-accent px-1.5 py-0.5 font-mono text-xs">
                {contract.contract_id.slice(0, 12)}...
              </span>
            </p>
          )}
          <p className="ml-12 mt-1 text-sm text-muted-foreground">
            Compare ABI changes across published versions, detect breaking
            changes, and review the upgrade matrix before shipping an upgrade.
          </p>
        </div>

        <div className="rounded-2xl border border-border bg-card p-6">
          {isLoading ? (
            <div className="flex items-center justify-center gap-3 py-16 text-muted-foreground">
              <Loader2 className="h-6 w-6 animate-spin" />
              <span className="text-sm">Loading compatibility matrix...</span>
            </div>
          ) : isError ? (
            <div className="py-12 text-center">
              <p className="text-sm text-red-500 dark:text-red-400">
                {(error as Error)?.message ??
                  "Failed to load compatibility matrix."}
              </p>
            </div>
          ) : compatibility ? (
            <CompatibilityMatrixDisplay
              data={compatibility}
              contractId={contractId}
            />
          ) : null}
        </div>
      </div>
    </div>
  );
}
