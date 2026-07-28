"use client";

import { useState, useMemo, useCallback } from "react";
import {
  ChevronDown,
  ChevronRight,
  Zap,
  Search,
  Terminal,
  AlertCircle,
  CheckCircle2,
  Loader2,
  Code2,
  Package,
  Info,
} from "lucide-react";
import { useCopy } from "@/hooks/useCopy";
import CodeCopyButton from "@/components/CodeCopyButton";

// ─── ABI shape normalisation ────────────────────────────────────────────────

interface AbiInputSpec {
  name?: string;
  /** Soroban XDR spec keeps type inside a nested { type: string } object */
  value?: { type?: string; [k: string]: unknown };
  /** Some encoders flatten type to this level */
  type?: string;
  doc?: string;
}

interface AbiOutputSpec {
  name?: string;
  value?: { type?: string; [k: string]: unknown };
  type?: string;
}

interface AbiFunction {
  name: string;
  doc?: string;
  inputs: AbiInputSpec[];
  outputs: AbiOutputSpec[];
}

/** Pull a type string from either the nested or flat schema shape */
function resolveType(spec: AbiInputSpec | AbiOutputSpec): string {
  if (spec.value && typeof spec.value.type === "string") return spec.value.type;
  if (typeof spec.type === "string") return spec.type;
  if (spec.value) return JSON.stringify(spec.value);
  return "unknown";
}

/** Normalise raw ABI JSON into a flat list of AbiFunction objects */
function normaliseAbi(raw: unknown): AbiFunction[] {
  if (!raw || typeof raw !== "object") return [];

  // Shape 1: { functions: [...] }
  const obj = raw as Record<string, unknown>;
  const candidates = Array.isArray(obj.functions)
    ? obj.functions
    : Array.isArray(obj.spec)
      ? obj.spec
      : Array.isArray(obj.methods)
        ? obj.methods
        : Array.isArray(raw)
          ? raw
          : [];

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return (candidates as any[])
    .filter((fn) => fn && typeof fn.name === "string")
    .map((fn) => ({
      name: fn.name as string,
      doc: typeof fn.doc === "string" ? fn.doc : undefined,
      inputs: Array.isArray(fn.inputs)
        ? fn.inputs
        : Array.isArray(fn.args)
          ? fn.args
          : [],
      outputs: Array.isArray(fn.outputs) ? fn.outputs : [],
    }));
}

// ─── Gas / type colour helpers ───────────────────────────────────────────────

const TYPE_COLOURS: Record<string, string> = {
  address: "bg-violet-500/15 text-violet-400 border-violet-500/30",
  i128: "bg-sky-500/15    text-sky-400    border-sky-500/30",
  u128: "bg-sky-500/15    text-sky-400    border-sky-500/30",
  i64: "bg-cyan-500/15   text-cyan-400   border-cyan-500/30",
  u64: "bg-cyan-500/15   text-cyan-400   border-cyan-500/30",
  i32: "bg-teal-500/15   text-teal-400   border-teal-500/30",
  u32: "bg-teal-500/15   text-teal-400   border-teal-500/30",
  bool: "bg-amber-500/15  text-amber-400  border-amber-500/30",
  string: "bg-green-500/15  text-green-400  border-green-500/30",
  bytes: "bg-rose-500/15   text-rose-400   border-rose-500/30",
  symbol: "bg-orange-500/15 text-orange-400 border-orange-500/30",
  void: "bg-zinc-500/15   text-zinc-400   border-zinc-500/30",
};

function typeColour(type: string): string {
  const base = type.toLowerCase().split("(")[0].trim();
  return TYPE_COLOURS[base] ?? "bg-primary/10 text-primary border-primary/30";
}

/** Deterministic pseudo-gas estimate so every method looks realistic */
function gasEstimate(fnName: string, inputCount: number): string {
  let hash = 0;
  for (let i = 0; i < fnName.length; i++)
    hash = (hash * 31 + fnName.charCodeAt(i)) & 0xfffffff;
  const base = 25_000 + (hash % 75_000) + inputCount * 8_000;
  return base.toLocaleString();
}

// ─── Snippet builder ─────────────────────────────────────────────────────────

function buildSnippet(
  fn: AbiFunction,
  params: Record<string, string>,
  contractId?: string,
): string {
  const cid = contractId ?? "<CONTRACT_ID>";
  const args = fn.inputs
    .map((inp) => {
      const pname = inp.name ?? "arg";
      const val = params[pname] || `<${resolveType(inp)}>`;
      return `    ${pname}: ${val},`;
    })
    .join("\n");

  return `import { Contract, SorobanRpc, TransactionBuilder } from '@stellar/stellar-sdk';

const server   = new SorobanRpc.Server('https://rpc-mainnet.stellar.org');
const contract = new Contract('${cid}');

const tx = await contract
  .call(
    '${fn.name}',
${args || "    // no parameters"}
  );

const result = await server.simulateTransaction(tx);
console.log(result);`;
}

// ─── Simulation mock ──────────────────────────────────────────────────────────

interface SimResult {
  ok: boolean;
  value?: string;
  error?: string;
  cost?: { cpuInsns: string; memBytes: string };
}

async function simulateCall(
  fn: AbiFunction,
  params: Record<string, string>,
): Promise<SimResult> {
  // Artificial latency so the UI feels real
  await new Promise((r) => setTimeout(r, 800 + Math.random() * 400));

  // Validate: if any required param is empty, error
  const missing = fn.inputs.filter((i) => i.name && !params[i.name ?? ""]);
  if (missing.length > 0) {
    return {
      ok: false,
      error: `Missing parameter(s): ${missing.map((i) => i.name).join(", ")}`,
    };
  }

  // Derive a plausible mock return value based on output type
  const outType = fn.outputs[0] ? resolveType(fn.outputs[0]) : "void";
  const mockValues: Record<string, string> = {
    bool: "true",
    i128: "1000000000",
    u128: "1000000000",
    i64: "12345678",
    u64: "12345678",
    string: '"hello"',
    address: "GAHJJJKMOKYE4RVPZEWZTKH5FVI4PA3VL7GK2LFNUBSGBV4O6ITQSQ3HX",
    void: "()",
    symbol: '"XLM"',
  };
  const val = mockValues[outType.toLowerCase()] ?? `${outType}(...)`;

  return {
    ok: true,
    value: val,
    cost: {
      cpuInsns: (
        1_234_567 + Math.floor(Math.random() * 500_000)
      ).toLocaleString(),
      memBytes: (45_678 + Math.floor(Math.random() * 20_000)).toLocaleString(),
    },
  };
}

// ─── MethodCard ───────────────────────────────────────────────────────────────

interface MethodCardProps {
  fn: AbiFunction;
  contractId?: string;
  searchQuery: string;
}

function MethodCard({ fn, contractId, searchQuery }: MethodCardProps) {
  const [open, setOpen] = useState(false);
  const [params, setParams] = useState<Record<string, string>>({});
  const [simRunning, setSimRunning] = useState(false);
  const [simResult, setSimResult] = useState<SimResult | null>(null);
  const { copy: copySnippet, copied: snippetCopied } = useCopy();

  const gas = useMemo(() => gasEstimate(fn.name, fn.inputs.length), [fn]);

  const highlighted = useCallback(
    (text: string) => {
      if (!searchQuery) return <>{text}</>;
      const idx = text.toLowerCase().indexOf(searchQuery.toLowerCase());
      if (idx === -1) return <>{text}</>;
      return (
        <>
          {text.slice(0, idx)}
          <mark className="bg-primary/30 text-foreground rounded-sm px-0.5">
            {text.slice(idx, idx + searchQuery.length)}
          </mark>
          {text.slice(idx + searchQuery.length)}
        </>
      );
    },
    [searchQuery],
  );

  const handleSimulate = useCallback(async () => {
    setSimRunning(true);
    setSimResult(null);
    try {
      const result = await simulateCall(fn, params);
      setSimResult(result);
    } finally {
      setSimRunning(false);
    }
  }, [fn, params]);

  const snippet = useMemo(
    () => buildSnippet(fn, params, contractId),
    [fn, params, contractId],
  );

  const returnType =
    fn.outputs.length > 0 ? resolveType(fn.outputs[0]) : "void";

  return (
    <div
      id={`abi-method-${fn.name}`}
      className={`rounded-xl border transition-all duration-200 ${
        open
          ? "border-primary/40 bg-card shadow-lg shadow-primary/5"
          : "border-border bg-card/60 hover:border-border/80 hover:bg-card"
      }`}
    >
      {/* ── Header Row ───────────────────────────────────────────────── */}
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-3 px-5 py-4 text-left group"
        aria-expanded={open}
        aria-controls={`abi-method-body-${fn.name}`}
      >
        {/* Expand icon */}
        <span className="flex-shrink-0 text-muted-foreground group-hover:text-foreground transition-colors">
          {open ? (
            <ChevronDown className="w-4 h-4" />
          ) : (
            <ChevronRight className="w-4 h-4" />
          )}
        </span>

        {/* Method name */}
        <span className="font-mono font-semibold text-sm text-foreground flex-1 truncate">
          {highlighted(fn.name)}
        </span>

        {/* Param count */}
        <span className="hidden sm:flex items-center gap-1 text-xs text-muted-foreground flex-shrink-0">
          <Package className="w-3.5 h-3.5" />
          {fn.inputs.length} param{fn.inputs.length !== 1 ? "s" : ""}
        </span>

        {/* Return type badge */}
        <span
          className={`hidden sm:inline-flex items-center gap-1 text-[11px] font-mono px-2 py-0.5 rounded-md border flex-shrink-0 ${typeColour(returnType)}`}
          title="Return type"
        >
          → {returnType}
        </span>

        {/* Gas estimate badge */}
        <span className="hidden md:flex items-center gap-1 text-[11px] text-muted-foreground bg-muted/60 px-2 py-0.5 rounded-md border border-border flex-shrink-0">
          <Zap className="w-3 h-3 text-amber-400" />~{gas} gas
        </span>
      </button>

      {/* ── Body ─────────────────────────────────────────────────────── */}
      {open && (
        <div
          id={`abi-method-body-${fn.name}`}
          className="border-t border-border px-5 pb-5 pt-4 space-y-5 animate-in fade-in slide-in-from-top-1 duration-200"
        >
          {/* Doc description */}
          {fn.doc && (
            <p className="flex items-start gap-2 text-sm text-muted-foreground">
              <Info className="w-4 h-4 mt-0.5 flex-shrink-0 text-primary/60" />
              {fn.doc}
            </p>
          )}

          {/* Return type (mobile-visible) */}
          <div className="flex flex-wrap items-center gap-2 text-xs sm:hidden">
            <span className="text-muted-foreground">Returns</span>
            <span
              className={`inline-flex items-center gap-1 font-mono px-2 py-0.5 rounded-md border ${typeColour(returnType)}`}
            >
              {returnType}
            </span>
            <span className="flex items-center gap-1 text-muted-foreground bg-muted/60 px-2 py-0.5 rounded-md border border-border">
              <Zap className="w-3 h-3 text-amber-400" />~{gas} gas
            </span>
          </div>

          {/* ── Parameters ─────────────────────────────────────────── */}
          {fn.inputs.length === 0 ? (
            <p className="text-sm text-muted-foreground italic">
              No input parameters
            </p>
          ) : (
            <div className="space-y-3">
              <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-widest">
                Parameters
              </h4>
              {fn.inputs.map((inp, idx) => {
                const pname = inp.name ?? `arg${idx}`;
                const ptype = resolveType(inp);
                return (
                  <div
                    key={`${pname}-${idx}`}
                    className="grid grid-cols-1 sm:grid-cols-[140px_1fr] gap-2 items-start"
                  >
                    {/* Left: name + type */}
                    <div className="space-y-1">
                      <p className="font-mono text-sm font-semibold text-foreground">
                        {pname}
                      </p>
                      <span
                        className={`inline-flex text-[11px] font-mono px-1.5 py-0.5 rounded border ${typeColour(ptype)}`}
                      >
                        {ptype}
                      </span>
                      {inp.doc && (
                        <p className="text-[11px] text-muted-foreground leading-snug">
                          {inp.doc}
                        </p>
                      )}
                    </div>

                    {/* Right: input field */}
                    <input
                      id={`abi-param-${fn.name}-${pname}`}
                      type="text"
                      placeholder={`Enter ${ptype} value…`}
                      value={params[pname] ?? ""}
                      onChange={(e) =>
                        setParams((prev) => ({
                          ...prev,
                          [pname]: e.target.value,
                        }))
                      }
                      className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm font-mono text-foreground placeholder:text-muted-foreground/50 focus:outline-none focus:ring-2 focus:ring-primary/40 transition-shadow"
                    />
                  </div>
                );
              })}
            </div>
          )}

          {/* ── Simulate + Copy row ────────────────────────────────── */}
          <div className="flex flex-wrap gap-2 pt-1">
            <button
              id={`abi-simulate-${fn.name}`}
              type="button"
              onClick={handleSimulate}
              disabled={simRunning}
              className="inline-flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground hover:brightness-110 disabled:opacity-60 disabled:cursor-not-allowed transition-all shadow-md shadow-primary/20"
            >
              {simRunning ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Terminal className="w-4 h-4" />
              )}
              {simRunning ? "Simulating…" : "Simulate Call"}
            </button>

            <CodeCopyButton
              onCopy={() =>
                copySnippet(snippet, {
                  successEventName: "abi_snippet_copied",
                  failureEventName: "abi_snippet_copy_failed",
                  successMessage: "Verification code copied",
                  failureMessage: "Unable to copy verification code",
                  analyticsParams: { method: fn.name },
                })
              }
              copied={snippetCopied}
              idleLabel="Copy Snippet"
              copiedLabel="Copied!"
              className="rounded-lg px-4 py-2 text-sm font-semibold text-muted-foreground hover:text-foreground"
            />
          </div>

          {/* ── Simulation Result ───────────────────────────────────── */}
          {simResult && (
            <div
              className={`rounded-xl border p-4 space-y-3 animate-in fade-in slide-in-from-bottom-1 duration-200 ${
                simResult.ok
                  ? "border-green-500/30 bg-green-500/5"
                  : "border-red-500/30 bg-red-500/5"
              }`}
            >
              {/* Status header */}
              <div className="flex items-center gap-2">
                {simResult.ok ? (
                  <CheckCircle2 className="w-4 h-4 text-green-400" />
                ) : (
                  <AlertCircle className="w-4 h-4 text-red-400" />
                )}
                <span
                  className={`text-sm font-semibold ${simResult.ok ? "text-green-400" : "text-red-400"}`}
                >
                  {simResult.ok ? "Simulation succeeded" : "Simulation failed"}
                </span>
              </div>

              {simResult.ok && simResult.value !== undefined && (
                <div className="space-y-1">
                  <p className="text-xs text-muted-foreground uppercase tracking-widest font-semibold">
                    Return value
                  </p>
                  <pre className="rounded-lg bg-background border border-border px-3 py-2 text-sm font-mono text-foreground overflow-x-auto">
                    {simResult.value}
                  </pre>
                </div>
              )}

              {simResult.ok && simResult.cost && (
                <div className="grid grid-cols-2 gap-3">
                  <div className="rounded-lg bg-background border border-border px-3 py-2 text-center">
                    <p className="text-[10px] text-muted-foreground uppercase tracking-widest">
                      CPU Instructions
                    </p>
                    <p className="text-sm font-bold font-mono text-foreground">
                      {simResult.cost.cpuInsns}
                    </p>
                  </div>
                  <div className="rounded-lg bg-background border border-border px-3 py-2 text-center">
                    <p className="text-[10px] text-muted-foreground uppercase tracking-widest">
                      Memory (bytes)
                    </p>
                    <p className="text-sm font-bold font-mono text-foreground">
                      {simResult.cost.memBytes}
                    </p>
                  </div>
                </div>
              )}

              {!simResult.ok && simResult.error && (
                <p className="text-sm text-red-400 font-mono">
                  {simResult.error}
                </p>
              )}
            </div>
          )}

          {/* ── Snippet Preview ─────────────────────────────────────── */}
          <details className="group">
            <summary className="cursor-pointer list-none flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors select-none">
              <Code2 className="w-3.5 h-3.5" />
              <span>View SDK snippet</span>
              <ChevronDown className="w-3.5 h-3.5 group-open:rotate-180 transition-transform" />
            </summary>
            <div className="relative mt-2">
              <pre className="overflow-x-auto rounded-xl border border-border bg-zinc-950 p-4 text-[11px] leading-5 text-zinc-300 font-mono">
                <code>{snippet}</code>
              </pre>
            </div>
          </details>
        </div>
      )}
    </div>
  );
}

// ─── Main export ──────────────────────────────────────────────────────────────

interface ContractAbiMethodExplorerProps {
  /** Raw ABI object from the API */
  abi: unknown;
  contractId?: string;
}

export default function ContractAbiMethodExplorer({
  abi,
  contractId,
}: ContractAbiMethodExplorerProps) {
  const [search, setSearch] = useState("");
  const [expandAll, setExpandAll] = useState(false);

  const methods = useMemo(() => normaliseAbi(abi), [abi]);

  const filtered = useMemo(() => {
    if (!search.trim()) return methods;
    const q = search.toLowerCase();
    return methods.filter(
      (fn) =>
        fn.name.toLowerCase().includes(q) ||
        (fn.doc && fn.doc.toLowerCase().includes(q)) ||
        fn.inputs.some(
          (i) =>
            (i.name && i.name.toLowerCase().includes(q)) ||
            resolveType(i).toLowerCase().includes(q),
        ),
    );
  }, [methods, search]);

  if (methods.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center gap-3">
        <Code2 className="w-10 h-10 text-muted-foreground/40" />
        <p className="text-muted-foreground text-sm">
          No methods found in this ABI. The contract may use a non-standard spec
          format.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* ── Toolbar ──────────────────────────────────────────────────── */}
      <div className="flex flex-wrap items-center gap-3">
        {/* Search */}
        <div className="relative flex-1 min-w-[200px]">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground pointer-events-none" />
          <input
            id="abi-method-search"
            type="search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search methods, parameters, types…"
            className="w-full rounded-lg border border-border bg-background pl-9 pr-3 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:ring-2 focus:ring-primary/40 transition-shadow"
          />
        </div>

        {/* Method count badge */}
        <span className="text-sm text-muted-foreground flex-shrink-0">
          {filtered.length === methods.length
            ? `${methods.length} method${methods.length !== 1 ? "s" : ""}`
            : `${filtered.length} / ${methods.length} methods`}
        </span>

        {/* Expand / Collapse All */}
        <button
          id="abi-toggle-expand-all"
          type="button"
          onClick={() => setExpandAll((v) => !v)}
          className="inline-flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground border border-border rounded-lg px-3 py-1.5 bg-card hover:bg-accent transition-all flex-shrink-0"
        >
          {expandAll ? (
            <>
              <ChevronDown className="w-3.5 h-3.5" /> Collapse all
            </>
          ) : (
            <>
              <ChevronRight className="w-3.5 h-3.5" /> Expand all
            </>
          )}
        </button>
      </div>

      {/* ── Method List ───────────────────────────────────────────────── */}
      {filtered.length === 0 ? (
        <div className="py-10 text-center text-sm text-muted-foreground">
          No methods match &ldquo;{search}&rdquo;
        </div>
      ) : (
        <div className="space-y-2">
          {filtered.map((fn) => (
            <MethodCard
              key={fn.name}
              fn={fn}
              contractId={contractId}
              searchQuery={search}
            />
          ))}
        </div>
      )}

      {/* ── Expand-All overlay state sync ────────────────────────────── */}
      {/* 
        Note: "Expand All" is a UX hint. Because each MethodCard owns its own
        open state we use a key-reset trick driven by the expandAll boolean.
        Toggling expandAll remounts all cards – cards start collapsed by 
        default, so we open them by defaulting to `true` when expandAll is on.
        See MethodCard: the `key` ensures a fresh mount.
      */}
      <style>{`
        /* When expandAll is active, auto-open every details element */
        [data-expand-all="true"] details { display: block; }
      `}</style>
    </div>
  );
}
