"use client";

import type React from "react";
import { useMemo, useState } from "react";
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Code2,
  Bug,
  GitBranch,
  Globe2,
  MessageSquare,
  Package,
  Play,
  ShieldCheck,
  Terminal,
  Users,
  Zap,
} from "lucide-react";
import {
  analyzeContractSource,
  type AuditFinding,
  type AuditReport,
  type AuditSeverity,
} from "@/lib/contractAudit";
import {
  compileContractSource,
  createDebugTrace,
  createPackageManifest,
  createVersionSnapshot,
  diffSnapshots,
  runContractTests,
  type VersionSnapshot,
} from "@/lib/contractIde";
import {
  addCollaborativeComment,
  applyCollaborativeEdit,
  createCollaborativeDocument,
} from "@/lib/collaboration";
import {
  pinContractMetadata,
  retrievePinnedContract,
  verifyPinnedContract,
  type PinnedContractMetadata,
} from "@/lib/ipfsMirror";

const STARTER_SOURCE = `#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

const STORAGE_KEY_BALANCE: &str = "balance";
const STORAGE_KEY_ALLOWANCE: &str = "balance";

#[contract]
pub struct RegistryToken;

#[contractimpl]
impl RegistryToken {
    pub fn set_balance(env: Env, owner: Address, amount: i128) {
        owner.require_auth();
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, STORAGE_KEY_BALANCE), &amount);
    }

    pub fn mint(env: Env, amount: i128) {
        let supply = amount + 1;
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "total_supply"), &supply);
    }

    pub fn balance(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get::<_, i128>(&Symbol::new(&env, STORAGE_KEY_BALANCE))
            .expect("missing balance")
    }
}`;

type WorkspaceTab = "audit" | "build" | "collab" | "package" | "ipfs";

const TABS: Array<{
  id: WorkspaceTab;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
}> = [
  { id: "audit", label: "Audit", icon: ShieldCheck },
  { id: "build", label: "Build", icon: Terminal },
  { id: "collab", label: "Live", icon: Users },
  { id: "package", label: "Package", icon: Package },
  { id: "ipfs", label: "IPFS", icon: Globe2 },
];

const RUST_KEYWORDS = new Set([
  "pub",
  "fn",
  "impl",
  "struct",
  "enum",
  "use",
  "let",
  "mut",
  "const",
  "Result",
  "Ok",
]);

function severityClass(severity: AuditSeverity) {
  switch (severity) {
    case "critical":
      return "border-red-500/40 bg-red-500/10 text-red-500";
    case "high":
      return "border-orange-500/40 bg-orange-500/10 text-orange-500";
    case "medium":
      return "border-amber-500/40 bg-amber-500/10 text-amber-500";
    case "low":
      return "border-sky-500/40 bg-sky-500/10 text-sky-500";
    default:
      return "border-border bg-accent text-muted-foreground";
  }
}

function FindingRow({ finding }: { finding: AuditFinding }) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <span
              className={`rounded-md border px-2 py-1 text-xs font-semibold ${severityClass(finding.severity)}`}
            >
              {finding.severity.toUpperCase()}
            </span>
            <span className="text-xs text-muted-foreground">
              Line {finding.line}
            </span>
            <span className="text-xs text-muted-foreground">
              {Math.round(finding.confidence * 100)}% confidence
            </span>
          </div>
          <h3 className="mt-3 text-base font-semibold text-foreground">
            {finding.title}
          </h3>
        </div>
      </div>
      <code className="mt-3 block overflow-x-auto rounded-md bg-muted px-3 py-2 text-xs text-muted-foreground">
        {finding.evidence}
      </code>
      <p className="mt-3 text-sm text-muted-foreground">
        {finding.explanation}
      </p>
      <p className="mt-2 text-sm text-foreground">{finding.recommendation}</p>
    </div>
  );
}

function ScorePanel({ report }: { report: AuditReport }) {
  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-[220px_1fr]">
      <div className="rounded-lg border border-border bg-card p-5">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Activity className="h-4 w-4 text-primary" />
          Security Score
        </div>
        <div className="mt-4 flex items-end gap-2">
          <span className="text-5xl font-bold text-foreground">
            {report.score}
          </span>
          <span className="pb-2 text-lg font-semibold text-primary">
            Grade {report.grade}
          </span>
        </div>
        <p className="mt-3 text-sm text-muted-foreground">{report.summary}</p>
      </div>

      <div className="rounded-lg border border-border bg-card p-5">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Zap className="h-4 w-4 text-primary" />
          Explainable Model Signals
        </div>
        <div className="mt-4 grid gap-3 sm:grid-cols-2">
          {report.signals.map((signal) => (
            <div
              key={signal.name}
              className="rounded-md border border-border bg-background p-3"
            >
              <div className="flex items-center justify-between gap-3">
                <span className="text-sm font-medium text-foreground">
                  {signal.name}
                </span>
                <span className="text-sm font-semibold text-primary">
                  {signal.value}
                </span>
              </div>
              <p className="mt-1 text-xs text-muted-foreground">
                {signal.explanation}
              </p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function HighlightedSource({ source }: { source: string }) {
  return (
    <pre className="max-h-72 overflow-auto rounded-b-lg border-t border-border bg-background p-4 font-mono text-xs leading-6 text-foreground">
      {source.split("\n").map((line, lineIndex) => (
        <div key={`${lineIndex}-${line.slice(0, 12)}`}>
          {line.split(/(\s+|::|[(){}.,;&])/).map((token, tokenIndex) => {
            const tokenWord = token.replace(/[^A-Za-z0-9_]/g, "");
            const className = line.trim().startsWith("//")
              ? "text-emerald-500"
              : RUST_KEYWORDS.has(tokenWord)
                ? "text-primary font-semibold"
                : /^".*"$/.test(token)
                  ? "text-amber-500"
                  : "";
            return (
              <span key={`${lineIndex}-${tokenIndex}`} className={className}>
                {token}
              </span>
            );
          })}
        </div>
      ))}
    </pre>
  );
}

export default function DeveloperWorkspace() {
  const [source, setSource] = useState(STARTER_SOURCE);
  const [activeTab, setActiveTab] = useState<WorkspaceTab>("audit");
  const [snapshots, setSnapshots] = useState<VersionSnapshot[]>([
    createVersionSnapshot(
      STARTER_SOURCE,
      "Initial workspace",
      "2026-04-23T00:00:00.000Z",
    ),
  ]);
  const [collabDoc, setCollabDoc] = useState(() =>
    createCollaborativeDocument(STARTER_SOURCE),
  );
  const [pins, setPins] = useState<PinnedContractMetadata[]>([]);
  const [lastResolution, setLastResolution] = useState(
    "Connected to local collaboration room.",
  );

  const report = useMemo(() => analyzeContractSource(source), [source]);
  const compile = useMemo(() => compileContractSource(source), [source]);
  const tests = useMemo(() => runContractTests(source), [source]);
  const debugTrace = useMemo(() => createDebugTrace(source), [source]);
  const manifest = useMemo(
    () => createPackageManifest("RegistryToken", source),
    [source],
  );
  const latestPin = pins[0];
  const latestDiff =
    snapshots.length > 1
      ? diffSnapshots(
          snapshots[snapshots.length - 2],
          snapshots[snapshots.length - 1],
        )
      : [];

  function saveSnapshot() {
    setSnapshots((current) => [
      ...current,
      createVersionSnapshot(source, `Revision ${current.length + 1}`),
    ]);
  }

  function simulateCollaborativeEdit() {
    const insertion =
      "\n        // Reviewed live: require_auth owner before storage writes.";
    const result = applyCollaborativeEdit(collabDoc, {
      userId: "u2",
      baseRevision: Math.max(1, collabDoc.revision - 1),
      from: source.indexOf("env.storage()"),
      to: source.indexOf("env.storage()"),
      text: insertion,
    });
    setCollabDoc(
      addCollaborativeComment(
        result.document,
        "u3",
        16,
        "Live reviewer requested an authorization trace.",
      ),
    );
    setSource(result.document.source);
    setLastResolution(result.resolution);
  }

  function pinCurrentMetadata() {
    const pin = pinContractMetadata({
      contractId: "CCONTRACTREGISTRYTOKEN",
      name: "RegistryToken",
      network: "testnet",
      wasmHash: compile.artifact
        ? `sha256-${compile.artifact.optimizedBytes}`
        : "sha256-pending",
      version: `0.1.${pins.length}`,
      sourceHash: `source-${source.length}`,
      auditScore: report.score,
      tags: ["ide", "audit", "registry"],
    });
    setPins((current) => [pin, ...current]);
  }

  return (
    <div className="min-h-screen bg-background text-foreground">
      <section className="border-b border-border bg-card">
        <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
          <div className="flex flex-col gap-5 lg:flex-row lg:items-end lg:justify-between">
            <div>
              <div className="flex items-center gap-2 text-sm font-semibold text-primary">
                <Code2 className="h-4 w-4" />
                Contract Developer Workspace
              </div>
              <h1 className="mt-3 text-3xl font-bold tracking-tight text-foreground sm:text-4xl">
                Audit, build, collaborate, and mirror Soroban contracts
              </h1>
              <p className="mt-3 max-w-3xl text-sm leading-6 text-muted-foreground">
                A browser IDE for contract source review with explainable audit
                scoring, WASM build checks, live editing state, package
                snapshots, and deterministic IPFS metadata verification.
              </p>
            </div>
            <div className="grid grid-cols-3 gap-2 text-center sm:w-[360px]">
              <div className="rounded-lg border border-border bg-background p-3">
                <div className="text-2xl font-bold text-foreground">
                  {report.score}
                </div>
                <div className="text-xs text-muted-foreground">Audit</div>
              </div>
              <div className="rounded-lg border border-border bg-background p-3">
                <div className="text-2xl font-bold text-foreground">
                  {compile.ok ? "OK" : "ERR"}
                </div>
                <div className="text-xs text-muted-foreground">Build</div>
              </div>
              <div className="rounded-lg border border-border bg-background p-3">
                <div className="text-2xl font-bold text-foreground">
                  {pins.length}
                </div>
                <div className="text-xs text-muted-foreground">Pins</div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <main className="mx-auto grid max-w-7xl gap-6 px-4 py-6 sm:px-6 lg:grid-cols-[minmax(0,1fr)_420px] lg:px-8">
        <section className="min-w-0 rounded-lg border border-border bg-card">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border px-4 py-3">
            <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
              <Code2 className="h-4 w-4 text-primary" />
              src/lib.rs
            </div>
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                onClick={saveSnapshot}
                className="inline-flex items-center gap-2 rounded-md border border-border px-3 py-2 text-sm font-medium text-foreground hover:bg-accent"
              >
                <GitBranch className="h-4 w-4" />
                Snapshot
              </button>
              <button
                type="button"
                onClick={simulateCollaborativeEdit}
                className="inline-flex items-center gap-2 rounded-md border border-border px-3 py-2 text-sm font-medium text-foreground hover:bg-accent"
              >
                <Users className="h-4 w-4" />
                Live Edit
              </button>
              <button
                type="button"
                onClick={pinCurrentMetadata}
                className="inline-flex items-center gap-2 rounded-md bg-primary px-3 py-2 text-sm font-semibold text-primary-foreground hover:brightness-105"
              >
                <Globe2 className="h-4 w-4" />
                Pin
              </button>
            </div>
          </div>

          <div className="grid grid-cols-[52px_1fr] overflow-hidden">
            <div className="select-none border-r border-border bg-muted px-2 py-4 text-right font-mono text-xs leading-6 text-muted-foreground">
              {source.split("\n").map((_, index) => (
                <div key={index}>{index + 1}</div>
              ))}
            </div>
            <textarea
              value={source}
              onChange={(event) => {
                setSource(event.target.value);
                setCollabDoc((current) => ({
                  ...current,
                  source: event.target.value,
                  revision: current.revision + 1,
                }));
              }}
              spellCheck={false}
              className="min-h-[680px] resize-y bg-background p-4 font-mono text-sm leading-6 text-foreground outline-none"
              aria-label="Soroban contract source editor"
            />
          </div>
          <div className="border-t border-border">
            <div className="flex items-center gap-2 px-4 py-3 text-sm font-semibold text-foreground">
              <Code2 className="h-4 w-4 text-primary" />
              Syntax Highlight View
            </div>
            <HighlightedSource source={source} />
          </div>
        </section>

        <aside className="min-w-0">
          <div className="sticky top-20 space-y-4">
            <div className="rounded-lg border border-border bg-card p-2">
              <div className="grid grid-cols-5 gap-1">
                {TABS.map(({ id, label, icon: Icon }) => (
                  <button
                    key={id}
                    type="button"
                    onClick={() => setActiveTab(id)}
                    className={`flex h-16 flex-col items-center justify-center gap-1 rounded-md text-xs font-medium transition-colors ${
                      activeTab === id
                        ? "bg-primary text-primary-foreground"
                        : "text-muted-foreground hover:bg-accent hover:text-foreground"
                    }`}
                  >
                    <Icon className="h-4 w-4" />
                    {label}
                  </button>
                ))}
              </div>
            </div>

            {activeTab === "audit" ? (
              <div className="space-y-4">
                <ScorePanel report={report} />
                <div className="space-y-3">
                  {report.findings.length === 0 ? (
                    <div className="rounded-lg border border-border bg-card p-4 text-sm text-muted-foreground">
                      No findings detected.
                    </div>
                  ) : (
                    report.findings.map((finding, index) => (
                      <FindingRow
                        key={`${finding.id}-${index}`}
                        finding={finding}
                      />
                    ))
                  )}
                </div>
              </div>
            ) : null}

            {activeTab === "build" ? (
              <div className="space-y-4">
                <div className="rounded-lg border border-border bg-card p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div className="flex items-center gap-2 font-semibold text-foreground">
                      <Terminal className="h-4 w-4 text-primary" />
                      WASM Compile
                    </div>
                    <span
                      className={`rounded-md px-2 py-1 text-xs font-semibold ${compile.ok ? "bg-green-500/10 text-green-500" : "bg-red-500/10 text-red-500"}`}
                    >
                      {compile.ok ? "PASSED" : "FAILED"}
                    </span>
                  </div>
                  {compile.artifact ? (
                    <div className="mt-4 grid grid-cols-2 gap-3 text-sm">
                      <div className="rounded-md bg-background p-3">
                        <div className="text-muted-foreground">WASM bytes</div>
                        <div className="font-semibold text-foreground">
                          {compile.artifact.wasmBytes}
                        </div>
                      </div>
                      <div className="rounded-md bg-background p-3">
                        <div className="text-muted-foreground">Optimized</div>
                        <div className="font-semibold text-foreground">
                          {compile.artifact.optimizedBytes}
                        </div>
                      </div>
                    </div>
                  ) : null}
                  <div className="mt-4 space-y-2">
                    {compile.diagnostics.map((diagnostic, index) => (
                      <div
                        key={index}
                        className="flex gap-2 rounded-md border border-border bg-background p-3 text-sm"
                      >
                        <AlertTriangle className="mt-0.5 h-4 w-4 text-amber-500" />
                        <div>
                          <div className="font-medium text-foreground">
                            Line {diagnostic.line}
                          </div>
                          <div className="text-muted-foreground">
                            {diagnostic.message}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="rounded-lg border border-border bg-card p-4">
                  <div className="mb-3 flex items-center gap-2 font-semibold text-foreground">
                    <Play className="h-4 w-4 text-primary" />
                    Test Execution
                  </div>
                  <div className="space-y-2">
                    {tests.map((test) => (
                      <div
                        key={test.name}
                        className="rounded-md border border-border bg-background p-3"
                      >
                        <div className="flex items-center justify-between gap-2">
                          <span className="text-sm font-medium text-foreground">
                            {test.name}
                          </span>
                          <span
                            className={
                              test.status === "passed"
                                ? "text-green-500"
                                : "text-red-500"
                            }
                          >
                            {test.status}
                          </span>
                        </div>
                        <p className="mt-1 text-xs text-muted-foreground">
                          {test.details}
                        </p>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="rounded-lg border border-border bg-card p-4">
                  <div className="mb-3 flex items-center gap-2 font-semibold text-foreground">
                    <Bug className="h-4 w-4 text-primary" />
                    Debug Trace
                  </div>
                  <div className="space-y-2">
                    {debugTrace.map((step) => (
                      <div
                        key={step.step}
                        className="rounded-md border border-border bg-background p-3"
                      >
                        <div className="flex items-center justify-between gap-2">
                          <span className="text-sm font-medium text-foreground">
                            {step.step}. {step.label}
                          </span>
                          <span
                            className={
                              step.status === "ok"
                                ? "text-green-500"
                                : step.status === "error"
                                  ? "text-red-500"
                                  : "text-amber-500"
                            }
                          >
                            {step.status}
                          </span>
                        </div>
                        <p className="mt-1 text-xs text-muted-foreground">
                          {step.detail}
                        </p>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            ) : null}

            {activeTab === "collab" ? (
              <div className="space-y-4">
                <div className="rounded-lg border border-border bg-card p-4">
                  <div className="flex items-center gap-2 font-semibold text-foreground">
                    <Users className="h-4 w-4 text-primary" />
                    Collaboration Room
                  </div>
                  <p className="mt-2 text-sm text-muted-foreground">
                    {lastResolution}
                  </p>
                  <div className="mt-4 space-y-2">
                    {collabDoc.cursors.map((cursor) => (
                      <div
                        key={cursor.userId}
                        className="flex items-center justify-between rounded-md bg-background p-3 text-sm"
                      >
                        <span className="flex items-center gap-2 text-foreground">
                          <span
                            className="h-3 w-3 rounded-full"
                            style={{ backgroundColor: cursor.color }}
                          />
                          {cursor.name}
                        </span>
                        <span className="text-muted-foreground">
                          {cursor.line}:{cursor.column}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="rounded-lg border border-border bg-card p-4">
                  <div className="mb-3 flex items-center gap-2 font-semibold text-foreground">
                    <MessageSquare className="h-4 w-4 text-primary" />
                    Live Comments
                  </div>
                  <div className="space-y-2">
                    {collabDoc.comments.map((comment) => (
                      <div
                        key={comment.id}
                        className="rounded-md border border-border bg-background p-3"
                      >
                        <div className="text-xs text-muted-foreground">
                          Line {comment.line}
                        </div>
                        <p className="mt-1 text-sm text-foreground">
                          {comment.body}
                        </p>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            ) : null}

            {activeTab === "package" ? (
              <div className="space-y-4">
                <div className="rounded-lg border border-border bg-card p-4">
                  <div className="flex items-center gap-2 font-semibold text-foreground">
                    <Package className="h-4 w-4 text-primary" />
                    Package Manifest
                  </div>
                  <pre className="mt-4 overflow-x-auto rounded-md bg-background p-3 text-xs text-muted-foreground">
                    {JSON.stringify(manifest, null, 2)}
                  </pre>
                </div>

                <div className="rounded-lg border border-border bg-card p-4">
                  <div className="mb-3 flex items-center gap-2 font-semibold text-foreground">
                    <GitBranch className="h-4 w-4 text-primary" />
                    Version History
                  </div>
                  <div className="space-y-2">
                    {snapshots.map((snapshot) => (
                      <div
                        key={snapshot.id}
                        className="rounded-md border border-border bg-background p-3"
                      >
                        <div className="text-sm font-medium text-foreground">
                          {snapshot.label}
                        </div>
                        <div className="text-xs text-muted-foreground">
                          {snapshot.summary}
                        </div>
                      </div>
                    ))}
                  </div>
                  {latestDiff.length > 0 ? (
                    <div className="mt-4 rounded-md bg-background p-3 text-xs text-muted-foreground">
                      Latest diff: {latestDiff.length} changed line
                      {latestDiff.length === 1 ? "" : "s"}
                    </div>
                  ) : null}
                </div>
              </div>
            ) : null}

            {activeTab === "ipfs" ? (
              <div className="space-y-4">
                <div className="rounded-lg border border-border bg-card p-4">
                  <div className="flex items-center gap-2 font-semibold text-foreground">
                    <Globe2 className="h-4 w-4 text-primary" />
                    Decentralized Registry Mirror
                  </div>
                  {latestPin ? (
                    <div className="mt-4 space-y-3">
                      <div className="rounded-md border border-border bg-background p-3">
                        <div className="text-xs text-muted-foreground">
                          Content ID
                        </div>
                        <div className="mt-1 break-all font-mono text-sm text-foreground">
                          {latestPin.cid}
                        </div>
                      </div>
                      <div className="flex items-center gap-2 text-sm">
                        {verifyPinnedContract(latestPin) &&
                        retrievePinnedContract(latestPin.cid, pins) ? (
                          <CheckCircle2 className="h-4 w-4 text-green-500" />
                        ) : (
                          <AlertTriangle className="h-4 w-4 text-red-500" />
                        )}
                        <span className="text-foreground">
                          Hash verification successful
                        </span>
                      </div>
                      <div className="space-y-2">
                        {latestPin.gateways.map((gateway) => (
                          <a
                            key={gateway}
                            href={gateway}
                            className="block break-all rounded-md bg-background p-3 text-xs text-primary hover:underline"
                          >
                            {gateway}
                          </a>
                        ))}
                      </div>
                    </div>
                  ) : (
                    <p className="mt-3 text-sm text-muted-foreground">
                      Pin the current contract metadata to create a
                      content-addressed mirror entry.
                    </p>
                  )}
                </div>
              </div>
            ) : null}
          </div>
        </aside>
      </main>
    </div>
  );
}
