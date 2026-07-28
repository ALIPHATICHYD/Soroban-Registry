export interface CompileDiagnostic {
  line: number;
  severity: "error" | "warning" | "info";
  message: string;
}

export interface CompileResult {
  ok: boolean;
  target: "wasm32-unknown-unknown";
  diagnostics: CompileDiagnostic[];
  artifact?: {
    name: string;
    wasmBytes: number;
    optimizedBytes: number;
    exportedFunctions: string[];
  };
}

export interface TestResult {
  name: string;
  status: "passed" | "failed";
  durationMs: number;
  details: string;
}

export interface VersionSnapshot {
  id: string;
  label: string;
  createdAt: string;
  source: string;
  summary: string;
}

export interface DebugTraceStep {
  step: number;
  label: string;
  status: "ok" | "warning" | "error";
  detail: string;
}

export function extractPublicFunctions(source: string): string[] {
  return Array.from(source.matchAll(/pub\s+fn\s+([A-Za-z0-9_]+)\s*\(/g)).map(
    (match) => match[1],
  );
}

function braceDiagnostics(source: string): CompileDiagnostic[] {
  const diagnostics: CompileDiagnostic[] = [];
  const stack: Array<{ char: string; line: number }> = [];
  const pairs: Record<string, string> = { "}": "{", ")": "(", "]": "[" };
  let line = 1;

  for (const char of source) {
    if (char === "\n") line += 1;
    if (char === "{" || char === "(" || char === "[") {
      stack.push({ char, line });
    }
    if (char === "}" || char === ")" || char === "]") {
      const previous = stack.pop();
      if (!previous || previous.char !== pairs[char]) {
        diagnostics.push({
          line,
          severity: "error",
          message: `Unmatched ${char}`,
        });
      }
    }
  }

  for (const item of stack) {
    diagnostics.push({
      line: item.line,
      severity: "error",
      message: `Unclosed ${item.char}`,
    });
  }

  return diagnostics;
}

export function compileContractSource(source: string): CompileResult {
  const diagnostics = braceDiagnostics(source);
  const functions = extractPublicFunctions(source);

  if (!/#!\[no_std\]/.test(source)) {
    diagnostics.push({
      line: 1,
      severity: "warning",
      message:
        "Soroban contracts should compile without the Rust standard library.",
    });
  }

  if (!/#\[contractimpl\]/.test(source)) {
    diagnostics.push({
      line: 1,
      severity: "error",
      message: "No #[contractimpl] block was found.",
    });
  }

  if (functions.length === 0) {
    diagnostics.push({
      line: 1,
      severity: "warning",
      message: "No public contract functions were exported.",
    });
  }

  const ok = diagnostics.every((diagnostic) => diagnostic.severity !== "error");
  const wasmBytes = Math.max(512, source.length * 3 + functions.length * 96);

  return {
    ok,
    target: "wasm32-unknown-unknown",
    diagnostics,
    artifact: ok
      ? {
          name: "contract.wasm",
          wasmBytes,
          optimizedBytes: Math.round(wasmBytes * 0.72),
          exportedFunctions: functions,
        }
      : undefined,
  };
}

export function runContractTests(source: string): TestResult[] {
  const compile = compileContractSource(source);
  const hasAuth = /require_auth\s*\(/.test(source);
  const hasStorage = /storage\(\)/.test(source);
  const hasPanics = /\b(panic!|unwrap\s*\(|expect\s*\()/.test(source);

  return [
    {
      name: "wasm compilation",
      status: compile.ok ? "passed" : "failed",
      durationMs: 42,
      details: compile.ok
        ? "Compilation produced a WASM artifact."
        : "Compilation diagnostics contain errors.",
    },
    {
      name: "authorization path",
      status: hasAuth ? "passed" : "failed",
      durationMs: 18,
      details: hasAuth
        ? "At least one authorization check is present."
        : "No require_auth call was detected.",
    },
    {
      name: "state access smoke test",
      status: hasStorage ? "passed" : "failed",
      durationMs: 23,
      details: hasStorage
        ? "Storage access paths are reachable."
        : "No storage calls were detected.",
    },
    {
      name: "recoverable error policy",
      status: hasPanics ? "failed" : "passed",
      durationMs: 15,
      details: hasPanics
        ? "Panic-prone calls were found in contract source."
        : "No panic-prone calls were found.",
    },
  ];
}

export function createDebugTrace(source: string): DebugTraceStep[] {
  const functions = extractPublicFunctions(source);
  const writesStorage =
    /storage\(\)\.(persistent|temporary|instance)\(\)\.set/.test(source);
  const hasAuth = /require_auth\s*\(/.test(source);
  const hasPanic = /\b(panic!|unwrap\s*\(|expect\s*\()/.test(source);
  const emitsEvents = /events\(\)\.publish/.test(source);

  return [
    {
      step: 1,
      label: "Load contract module",
      status: functions.length > 0 ? "ok" : "warning",
      detail: `${functions.length} exported function${functions.length === 1 ? "" : "s"} discovered.`,
    },
    {
      step: 2,
      label: "Execute authorization gate",
      status: hasAuth ? "ok" : writesStorage ? "error" : "warning",
      detail: hasAuth
        ? "require_auth was reached before privileged actions."
        : "No require_auth call was found.",
    },
    {
      step: 3,
      label: "Commit storage changes",
      status: writesStorage ? "ok" : "warning",
      detail: writesStorage
        ? "Persistent storage write path detected."
        : "No persistent write path was detected.",
    },
    {
      step: 4,
      label: "Emit registry events",
      status: emitsEvents ? "ok" : "warning",
      detail: emitsEvents
        ? "Event publication path detected."
        : "No events().publish call was found.",
    },
    {
      step: 5,
      label: "Return caller result",
      status: hasPanic ? "error" : "ok",
      detail: hasPanic
        ? "Panic-prone calls may abort instead of returning a recoverable error."
        : "No panic-prone calls detected.",
    },
  ];
}

export function createPackageManifest(name: string, source: string) {
  const exports = extractPublicFunctions(source);

  return {
    package:
      name.toLowerCase().replace(/[^a-z0-9_-]+/g, "-") || "soroban-contract",
    version: "0.1.0",
    target: "wasm32-unknown-unknown",
    exports,
    dependencies: ["soroban-sdk"],
  };
}

export function createVersionSnapshot(
  source: string,
  label: string,
  createdAt = new Date().toISOString(),
): VersionSnapshot {
  const lines = source.split("\n").length;
  const functions = extractPublicFunctions(source).length;

  return {
    id: `${Date.parse(createdAt)}-${source.length}`,
    label,
    createdAt,
    source,
    summary: `${lines} lines, ${functions} exported function${functions === 1 ? "" : "s"}`,
  };
}

export function diffSnapshots(
  previous: VersionSnapshot,
  next: VersionSnapshot,
) {
  const before = previous.source.split("\n");
  const after = next.source.split("\n");
  const max = Math.max(before.length, after.length);
  const changes: Array<{
    line: number;
    before?: string;
    after?: string;
    type: "added" | "removed" | "changed";
  }> = [];

  for (let index = 0; index < max; index += 1) {
    if (before[index] === after[index]) continue;
    if (before[index] === undefined)
      changes.push({ line: index + 1, after: after[index], type: "added" });
    else if (after[index] === undefined)
      changes.push({ line: index + 1, before: before[index], type: "removed" });
    else
      changes.push({
        line: index + 1,
        before: before[index],
        after: after[index],
        type: "changed",
      });
  }

  return changes;
}
