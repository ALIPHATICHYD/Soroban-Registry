"use client";

import DependencyGraph from "@/components/DependencyGraph";
import type {
  ContractInteroperabilityResponse,
  InteroperabilityCapability,
  InteroperabilityProtocolMatch,
} from "@/types";
import {
  AlertTriangle,
  CheckCircle2,
  GitCompare,
  Link2,
  PlugZap,
  Puzzle,
  ShieldCheck,
} from "lucide-react";

interface InteroperabilityAnalysisProps {
  data: ContractInteroperabilityResponse;
}

function CapabilityBadge({
  capability,
}: {
  capability: InteroperabilityCapability;
}) {
  const isBridge = capability.kind === "bridge";
  return (
    <span
      className={`inline-flex items-center gap-2 rounded-full px-3 py-1 text-xs font-semibold ${isBridge ? "bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-300" : "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300"}`}
    >
      {isBridge ? (
        <Link2 className="h-3.5 w-3.5" />
      ) : (
        <PlugZap className="h-3.5 w-3.5" />
      )}
      {capability.label}
      <span className="opacity-70">
        {Math.round(capability.confidence * 100)}%
      </span>
    </span>
  );
}

function ProtocolCard({
  protocol,
}: {
  protocol: InteroperabilityProtocolMatch;
}) {
  const statusTone =
    protocol.status === "compliant"
      ? "border-green-200 bg-green-50 dark:border-green-900/40 dark:bg-green-900/10"
      : protocol.status === "partial"
        ? "border-amber-200 bg-amber-50 dark:border-amber-900/40 dark:bg-amber-900/10"
        : "border-border bg-card";

  return (
    <article className={`rounded-2xl border p-4 ${statusTone}`}>
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold text-foreground">
            {protocol.name}
          </h3>
          <p className="mt-1 text-sm text-muted-foreground">
            {protocol.description}
          </p>
        </div>
        <span className="rounded-full bg-background/80 px-2.5 py-1 text-xs font-semibold capitalize text-foreground">
          {protocol.status}
        </span>
      </div>
      <div className="mt-4 flex items-center justify-between text-sm">
        <span className="text-muted-foreground">Compliance score</span>
        <span className="font-semibold text-foreground">
          {protocol.compliance_score}%
        </span>
      </div>
      <div className="mt-4 space-y-2 text-sm">
        <div>
          <span className="font-medium text-foreground">Matched:</span>{" "}
          <span className="text-muted-foreground">
            {protocol.matched_functions.length
              ? protocol.matched_functions.join(", ")
              : "None yet"}
          </span>
        </div>
        {protocol.missing_functions.length > 0 && (
          <div>
            <span className="font-medium text-foreground">Missing:</span>{" "}
            <span className="text-muted-foreground">
              {protocol.missing_functions.join(", ")}
            </span>
          </div>
        )}
      </div>
    </article>
  );
}

export default function InteroperabilityAnalysis({
  data,
}: InteroperabilityAnalysisProps) {
  return (
    <div className="space-y-8">
      {data.warnings.length > 0 && (
        <section className="rounded-2xl border border-amber-300 bg-amber-50 p-4 dark:border-amber-700 dark:bg-amber-900/20">
          <div className="flex items-center gap-2 text-sm font-semibold text-amber-800 dark:text-amber-300">
            <AlertTriangle className="h-4 w-4" />
            Analysis warnings
          </div>
          <ul className="mt-2 space-y-1 text-sm text-amber-700 dark:text-amber-300">
            {data.warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </section>
      )}

      <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <div className="rounded-2xl border border-border bg-card p-5">
          <div className="text-sm text-muted-foreground">Protocols matched</div>
          <div className="mt-2 text-3xl font-bold text-foreground">
            {data.summary.protocol_matches}
          </div>
        </div>
        <div className="rounded-2xl border border-border bg-card p-5">
          <div className="text-sm text-muted-foreground">
            Suggested contracts
          </div>
          <div className="mt-2 text-3xl font-bold text-foreground">
            {data.summary.suggested_contracts}
          </div>
        </div>
        <div className="rounded-2xl border border-border bg-card p-5">
          <div className="text-sm text-muted-foreground">Bridge signals</div>
          <div className="mt-2 text-3xl font-bold text-foreground">
            {data.summary.bridge_signals}
          </div>
        </div>
        <div className="rounded-2xl border border-border bg-card p-5">
          <div className="text-sm text-muted-foreground">Graph edges</div>
          <div className="mt-2 text-3xl font-bold text-foreground">
            {data.summary.graph_edges}
          </div>
        </div>
      </section>

      <section className="rounded-2xl border border-border bg-card p-6">
        <div className="flex items-center gap-2 text-lg font-semibold text-foreground">
          <ShieldCheck className="h-5 w-5 text-primary" />
          Protocol compliance
        </div>
        <div className="mt-4 grid gap-4 lg:grid-cols-2">
          {data.protocols.map((protocol) => (
            <ProtocolCard key={protocol.slug} protocol={protocol} />
          ))}
        </div>
      </section>

      <section className="rounded-2xl border border-border bg-card p-6">
        <div className="flex items-center gap-2 text-lg font-semibold text-foreground">
          <Puzzle className="h-5 w-5 text-primary" />
          Bridges and adapters
        </div>
        {data.capabilities.length > 0 ? (
          <div className="mt-4 space-y-4">
            <div className="flex flex-wrap gap-2">
              {data.capabilities.map((capability) => (
                <CapabilityBadge
                  key={`${capability.kind}-${capability.label}`}
                  capability={capability}
                />
              ))}
            </div>
            <div className="grid gap-4 md:grid-cols-2">
              {data.capabilities.map((capability) => (
                <article
                  key={capability.label}
                  className="rounded-2xl border border-border bg-background p-4"
                >
                  <h3 className="text-sm font-semibold text-foreground">
                    {capability.label}
                  </h3>
                  <ul className="mt-3 space-y-2 text-sm text-muted-foreground">
                    {capability.evidence.map((evidence) => (
                      <li key={evidence}>{evidence}</li>
                    ))}
                  </ul>
                </article>
              ))}
            </div>
          </div>
        ) : (
          <p className="mt-3 text-sm text-muted-foreground">
            No bridge or adapter signals were detected from the current ABI and
            metadata.
          </p>
        )}
      </section>

      <section className="rounded-2xl border border-border bg-card p-6">
        <div className="flex items-center gap-2 text-lg font-semibold text-foreground">
          <CheckCircle2 className="h-5 w-5 text-primary" />
          Suggested interoperable contracts
        </div>
        {data.suggestions.length > 0 ? (
          <div className="mt-4 grid gap-4 xl:grid-cols-2">
            {data.suggestions.map((suggestion) => (
              <article
                key={suggestion.contract_id}
                className="rounded-2xl border border-border bg-background p-5"
              >
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <h3 className="text-base font-semibold text-foreground">
                      {suggestion.contract_name}
                    </h3>
                    <p className="mt-1 text-xs font-mono text-muted-foreground">
                      {suggestion.contract_address}
                    </p>
                  </div>
                  <span className="rounded-full bg-primary/10 px-3 py-1 text-xs font-semibold text-primary">
                    Score {Math.round(suggestion.score)}
                  </span>
                </div>
                <p className="mt-3 text-sm text-muted-foreground">
                  {suggestion.reason}
                </p>
                <div className="mt-4 flex flex-wrap gap-2 text-xs">
                  {suggestion.shared_protocols.map((protocol) => (
                    <span
                      key={protocol}
                      className="rounded-full bg-green-100 px-2.5 py-1 font-medium text-green-700 dark:bg-green-900/20 dark:text-green-300"
                    >
                      {protocol}
                    </span>
                  ))}
                  {suggestion.relation_types.map((relation) => (
                    <span
                      key={relation}
                      className="rounded-full bg-accent px-2.5 py-1 font-medium text-foreground"
                    >
                      {relation.replace(/_/g, " ")}
                    </span>
                  ))}
                </div>
              </article>
            ))}
          </div>
        ) : (
          <p className="mt-3 text-sm text-muted-foreground">
            No strong interoperability suggestions were found for this contract
            yet.
          </p>
        )}
      </section>

      <section className="rounded-2xl border border-border bg-card p-6">
        <div className="flex items-center gap-2 text-lg font-semibold text-foreground">
          <GitCompare className="h-5 w-5 text-primary" />
          Interoperability graph
        </div>
        {data.graph.nodes.length > 1 ? (
          <div className="mt-4 h-[480px]">
            <DependencyGraph
              nodes={data.graph.nodes}
              edges={data.graph.edges}
            />
          </div>
        ) : (
          <p className="mt-3 text-sm text-muted-foreground">
            The analysis did not find enough strong relationships to build a
            graph yet.
          </p>
        )}
      </section>
    </div>
  );
}
