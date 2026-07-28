"use client";

import React, { useMemo } from "react";
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Clock3,
  RefreshCw,
} from "lucide-react";
import type {
  VerificationLogEntry,
  VerificationMetrics,
  VerificationStatus,
} from "@/types/verification";

function statusPct(status: VerificationStatus): number {
  if (status === "submitted") return 33;
  if (status === "under_review") return 66;
  if (status === "approved" || status === "rejected") return 100;
  return 0;
}

function statusTone(status: VerificationStatus): string {
  if (status === "approved") return "bg-green-500";
  if (status === "rejected") return "bg-red-500";
  return "bg-primary";
}

function levelClass(level: VerificationLogEntry["level"]): string {
  if (level === "error") return "text-red-400";
  if (level === "warn") return "text-yellow-300";
  if (level === "debug") return "text-cyan-300";
  return "text-emerald-300";
}

function highlightOutput(raw: string): React.ReactNode[] {
  const lines = raw.split("\n");
  return lines.map((line, idx) => {
    const tokens = line.split(
      /(".*?"|\btrue\b|\bfalse\b|\bnull\b|-?\d+(?:\.\d+)?)/g,
    );
    return (
      <div key={`${idx}-${line}`} className="whitespace-pre-wrap break-words">
        {tokens.map((token, tIdx) => {
          if (/^".*"$/.test(token)) {
            return (
              <span key={tIdx} className="text-emerald-300">
                {token}
              </span>
            );
          }
          if (/^-?\d+(?:\.\d+)?$/.test(token)) {
            return (
              <span key={tIdx} className="text-orange-300">
                {token}
              </span>
            );
          }
          if (/^(true|false|null)$/.test(token)) {
            return (
              <span key={tIdx} className="text-violet-300">
                {token}
              </span>
            );
          }
          return <span key={tIdx}>{token}</span>;
        })}
      </div>
    );
  });
}

export function VerificationWorkflow(props: {
  status: VerificationStatus;
  updatedAt: string;
}) {
  const { status, updatedAt } = props;
  const pct = statusPct(status);
  return (
    <div className="rounded-2xl border border-border bg-card p-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm font-semibold text-foreground">
          Verification workflow
        </p>
        <div className="inline-flex items-center gap-2 text-xs text-muted-foreground">
          <Activity className="w-3.5 h-3.5 text-primary animate-pulse" />
          Live updates
        </div>
      </div>
      <div className="mt-3 h-2 rounded-full bg-muted overflow-hidden">
        <div
          className={`h-full transition-all duration-500 ${statusTone(status)}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      <div className="mt-3 grid grid-cols-3 text-[11px] gap-2 text-muted-foreground">
        <span className={pct >= 33 ? "text-foreground font-semibold" : ""}>
          Submitted
        </span>
        <span
          className={
            pct >= 66
              ? "text-foreground font-semibold text-center"
              : "text-center"
          }
        >
          Review
        </span>
        <span
          className={
            pct >= 100
              ? "text-foreground font-semibold text-right"
              : "text-right"
          }
        >
          Decision
        </span>
      </div>
      <p className="mt-2 text-xs text-muted-foreground">
        Last update: {new Date(updatedAt).toLocaleString()}
      </p>
    </div>
  );
}

export function VerificationMetricsPanel(props: {
  metrics: VerificationMetrics;
}) {
  const { metrics } = props;
  const cards = [
    { label: "Attempts", value: metrics.attemptCount, icon: RefreshCw },
    { label: "Checks passed", value: metrics.checksPassed, icon: CheckCircle2 },
    {
      label: "Checks failed",
      value: metrics.checksFailed,
      icon: AlertTriangle,
    },
    {
      label: "Duration",
      value: `${(metrics.durationMs / 1000).toFixed(1)}s`,
      icon: Clock3,
    },
  ];
  return (
    <div className="rounded-2xl border border-border bg-card p-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm font-semibold text-foreground">
          Verification metrics
        </p>
        <p className="text-xs text-muted-foreground">
          Coverage {metrics.coveragePct}%
        </p>
      </div>
      <div className="mt-3 grid grid-cols-2 gap-2">
        {cards.map((card) => (
          <div
            key={card.label}
            className="rounded-xl border border-border bg-background px-3 py-2"
          >
            <div className="flex items-center gap-1.5 text-muted-foreground">
              <card.icon className="w-3.5 h-3.5" />
              <p className="text-[11px] uppercase tracking-wide">
                {card.label}
              </p>
            </div>
            <p className="mt-1 text-lg font-semibold text-foreground">
              {card.value}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}

export function VerificationLogs(props: { logs: VerificationLogEntry[] }) {
  const sorted = useMemo(
    () => [...props.logs].sort((a, b) => a.at.localeCompare(b.at)),
    [props.logs],
  );

  return (
    <div className="rounded-2xl border border-border bg-card p-4">
      <p className="text-sm font-semibold text-foreground">Verification logs</p>
      <div className="mt-3 space-y-3 max-h-[420px] overflow-y-auto pr-1">
        {sorted.map((entry) => (
          <div
            key={entry.id}
            className="rounded-xl border border-border bg-background p-3"
          >
            <div className="flex items-center justify-between gap-2">
              <p className="text-xs text-muted-foreground">
                {new Date(entry.at).toLocaleTimeString()}
              </p>
              <span
                className={`text-[10px] uppercase tracking-wide font-semibold ${levelClass(entry.level)}`}
              >
                {entry.level}
              </span>
            </div>
            <p className="mt-1 text-sm text-foreground">{entry.message}</p>
            {entry.output && (
              <pre className="mt-2 overflow-x-auto rounded-lg border border-border bg-zinc-950 p-3 text-[11px] leading-5 text-zinc-200 font-mono">
                <code>{highlightOutput(entry.output)}</code>
              </pre>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
