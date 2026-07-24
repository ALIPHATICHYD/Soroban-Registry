"use client";

import { Suspense } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import Navbar from "@/components/Navbar";
import dynamic from "next/dynamic";
import { ArrowLeft, GitCompare } from "lucide-react";

const ContractDiffViewer = dynamic(
  () => import("@/components/ContractDiffViewer"),
  {
    ssr: false,
    loading: () => (
      <div className="rounded-2xl border border-border bg-card p-6 animate-pulse">
        <div className="h-5 w-40 rounded bg-border mb-3" />
        <div className="h-3 w-64 rounded bg-border" />
      </div>
    ),
  },
);

function DiffPageContent() {
  const params = useParams<{ id?: string | string[] }>();

  const idParam = params?.id;
  const contractId = Array.isArray(idParam) ? idParam[0] : (idParam ?? "");

  const contractQuery = useQuery({
    queryKey: ["contract", contractId],
    queryFn: () => api.fetchContract(contractId),
    enabled: !!contractId,
  });

  const contract = contractQuery.data;

  return (
    <div className="min-h-screen bg-background">
      <Navbar />

      <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        {/* Breadcrumb */}
        <div className="mb-6 flex items-center gap-2 text-sm">
          <Link
            href={`/contracts/${contractId}`}
            className="flex items-center gap-1.5 text-muted-foreground hover:text-foreground transition-colors"
          >
            <ArrowLeft size={15} />
            Back to contract
          </Link>
          {contract && (
            <>
              <span className="text-muted-foreground">/</span>
              <span className="font-semibold text-foreground">
                {contract.name}
              </span>
            </>
          )}
          <span className="text-muted-foreground">/</span>
          <span className="flex items-center gap-1 text-primary font-semibold">
            <GitCompare size={15} />
            Diff
          </span>
        </div>

        {/* Page heading */}
        <div className="mb-6">
          <h1 className="text-xl font-bold text-foreground">
            {contract
              ? `${contract.name} — Version Diff`
              : "Contract Version Diff"}
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Select two versions to compare source code changes side-by-side or
            in unified view.
          </p>
        </div>

        {/* Diff viewer */}
        {contractId ? (
          <ContractDiffViewer
            contractId={contractId}
            contractName={contract?.name}
          />
        ) : (
          <div className="rounded-2xl border border-border bg-card p-6 text-sm text-muted-foreground">
            No contract ID provided.
          </div>
        )}
      </main>
    </div>
  );
}

export default function DiffPage() {
  return (
    <Suspense
      fallback={
        <div className="min-h-screen bg-background animate-pulse">
          <div className="h-14 border-b border-border bg-card" />
          <div className="mx-auto max-w-7xl px-4 py-8">
            <div className="h-4 w-40 rounded bg-border mb-6" />
            <div className="h-6 w-72 rounded bg-border mb-8" />
            <div className="h-80 rounded-2xl bg-border" />
          </div>
        </div>
      }
    >
      <DiffPageContent />
    </Suspense>
  );
}
