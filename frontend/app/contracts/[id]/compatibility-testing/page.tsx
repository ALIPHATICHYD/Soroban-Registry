"use client";

import { useQuery } from "@tanstack/react-query";
import { useParams } from "next/navigation";
import Link from "next/link";
import { api } from "@/lib/api";
import CompatibilityTestingMatrix from "@/components/CompatibilityTestingMatrix";
import { ArrowLeft, FlaskConical } from "lucide-react";
import Navbar from "@/components/Navbar";

export default function CompatibilityTestingPage() {
  const params = useParams<{ id?: string | string[] }>() ?? {};
  const idParam = params.id;
  const contractId = Array.isArray(idParam) ? idParam[0] : idParam;

  const { data: contract } = useQuery({
    queryKey: ["contract", contractId],
    queryFn: () => api.fetchContract(contractId!),
    enabled: !!contractId,
  });

  if (!contractId) {
    return (
      <div className="min-h-screen bg-background text-foreground">
        <Navbar />
        <div className="max-w-4xl mx-auto px-4 py-10">
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
                className="inline-flex items-center gap-2 rounded-xl border border-border bg-background px-3 py-2 text-sm font-semibold text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
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
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Back navigation */}
        <Link
          href={`/contracts/${contractId}`}
          className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground mb-6 transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          Back to contract
        </Link>

        {/* Header */}
        <div className="mb-8">
          <div className="flex items-center gap-3 mb-2">
            <span className="flex items-center justify-center w-9 h-9 rounded-lg bg-secondary/10">
              <FlaskConical className="w-5 h-5 text-secondary" />
            </span>
            <h1 className="text-2xl font-bold text-foreground">
              SDK Compatibility Testing
            </h1>
          </div>
          {contract && (
            <p className="text-muted-foreground ml-12">
              {contract.name}{" "}
              <span className="font-mono text-xs bg-accent px-1.5 py-0.5 rounded">
                {contract.contract_id.slice(0, 12)}…
              </span>
            </p>
          )}
          <p className="text-sm text-muted-foreground mt-1 ml-12">
            Test contract compatibility across Soroban SDK versions, Wasm
            runtimes, and Stellar networks.
          </p>
        </div>

        {/* Content */}
        <div className="bg-card rounded-2xl border border-border p-6">
          <CompatibilityTestingMatrix contractId={contractId} />
        </div>
      </div>
    </div>
  );
}
