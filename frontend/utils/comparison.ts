import type { Contract } from "@/types";

export type ComparisonMetricKey =
  | "contract_id"
  | "network"
  | "category"
  | "publisher"
  | "verification_status"
  | "wasm_hash"
  | "deployment_count"
  | "popularity_score"
  | "health_score";

export type CellTone = "neutral" | "best" | "worst" | "different";

export type DiffLine =
  | { type: "context"; value: string }
  | { type: "add"; value: string }
  | { type: "remove"; value: string };

export interface ComparableContract {
  id: string;
  name: string;
  contractId: string;
  network: string;
  category: string;
  publisherId: string;
  latestVersion: string;
  versionCount: number;
  abiMethods: string[];
  isVerified: boolean;
  tags: string[];
  sourceCode: string;
  wasmHash: string;
  deploymentCount: number;
  popularityScore: number;
  healthScore: number;
  base?: Pick<
    Contract,
    "contract_id" | "network" | "publisher_id" | "wasm_hash" | "updated_at"
  >;
}

function indentBlock(text: string, spaces: number) {
  const pad = " ".repeat(spaces);
  return text
    .split("\n")
    .map((l) => (l.length > 0 ? `${pad}${l}` : l))
    .join("\n");
}

function latestVersionFromVersions(
  versions: Array<{ version: string; created_at: string }>,
) {
  if (versions.length === 0) return null;
  return (
    [...versions].sort(
      (a, b) =>
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
    )[0] ?? null
  );
}

function collectStringValues(input: unknown, sink: string[]) {
  if (typeof input === "string") {
    sink.push(input);
    return;
  }
  if (Array.isArray(input)) {
    for (const item of input) collectStringValues(item, sink);
    return;
  }
  if (input && typeof input === "object") {
    for (const value of Object.values(input)) collectStringValues(value, sink);
  }
}

export function extractAbiMethods(abi: unknown): string[] {
  const methods = new Set<string>();

  const visit = (node: unknown) => {
    if (Array.isArray(node)) {
      for (const item of node) visit(item);
      return;
    }
    if (!node || typeof node !== "object") return;

    const record = node as Record<string, unknown>;
    const directCandidates = [
      record.name,
      record.method,
      record.function,
      record.fn,
      record.symbol,
      record.export,
    ];

    for (const candidate of directCandidates) {
      if (typeof candidate === "string" && candidate.trim()) {
        methods.add(candidate.trim());
      }
    }

    for (const [key, value] of Object.entries(record)) {
      if (
        ["functions", "methods", "entries"].includes(key) &&
        Array.isArray(value)
      ) {
        for (const item of value) visit(item);
        continue;
      }

      if (["name_scval", "nameScVal", "identifier"].includes(key)) {
        const values: string[] = [];
        collectStringValues(value, values);
        for (const v of values) {
          if (v.trim()) methods.add(v.trim());
        }
      }
    }
  };

  visit(abi);
  return [...methods].sort((a, b) => a.localeCompare(b));
}

function buildFallbackSourceCode(
  name: string,
  isVerified: boolean,
  abiMethods: string[],
  latestVersion: string,
) {
  const versionLine = latestVersion
    ? `const VERSION: &str = "${latestVersion}";\n`
    : "";
  const verifiedLine = isVerified
    ? "const VERIFIED: bool = true;\n"
    : "const VERIFIED: bool = false;\n";
  const functions = abiMethods
    .slice(0, 12)
    .map(
      (m) =>
        `pub fn ${m}(env: Env) -> i128 {\n    env.ledger().sequence() as i128\n}\n`,
    )
    .join("\n");

  return `// ${name}\n// Generated comparison fallback source\n\nuse soroban_sdk::{contract, contractimpl, Env};\n\n${versionLine}${verifiedLine}\n#[contract]\npub struct Contract;\n\n#[contractimpl]\nimpl Contract {\n${indentBlock(functions.trimEnd() || "  // No ABI methods available", 2)}\n}\n`;
}

export function toComparableContract(
  contract: Contract,
  options?: {
    versions?: Array<{ version: string; created_at: string }>;
    abi?: unknown;
    sourceCode?: string;
  },
): ComparableContract {
  const isVerified = Boolean(contract.is_verified);
  const versions = options?.versions ?? [];
  const latestVersion =
    latestVersionFromVersions(versions)?.version ?? "Unversioned";
  const abiMethods = extractAbiMethods(options?.abi);
  const sourceCode =
    options?.sourceCode && options.sourceCode.trim().length > 0
      ? options.sourceCode
      : buildFallbackSourceCode(
          contract.name,
          isVerified,
          abiMethods,
          latestVersion,
        );

  return {
    id: contract.id,
    name: contract.name,
    contractId: contract.contract_id,
    network: contract.network,
    category: contract.category || "Uncategorized",
    publisherId: contract.publisher_id,
    latestVersion,
    versionCount: versions.length,
    abiMethods,
    isVerified,
    tags: contract.tags,
    sourceCode,
    wasmHash: contract.wasm_hash,
    deploymentCount: contract.deployment_count ?? 0,
    popularityScore: contract.popularity_score ?? 0,
    healthScore: 0,
    base: {
      contract_id: contract.contract_id,
      network: contract.network,
      publisher_id: contract.publisher_id,
      wasm_hash: contract.wasm_hash,
      updated_at: contract.updated_at,
    },
  };
}

export function toneForMetricCell(
  metric: ComparisonMetricKey,
  value: string | number | boolean,
  allValues: Array<string | number | boolean>,
): CellTone {
  const isAllEqual =
    allValues.length > 0 && allValues.every((v) => v === allValues[0]);
  if (isAllEqual) return "neutral";

  if (metric === "verification_status") {
    const v = Boolean(value);
    return v ? "best" : "worst";
  }

  if (
    metric === "deployment_count" ||
    metric === "popularity_score" ||
    metric === "health_score"
  ) {
    const nums = allValues.map(Number).filter(isFinite);
    const max = Math.max(...nums);
    if (max === 0) return "neutral";
    return Number(value) === max ? "best" : "different";
  }

  if (metric === "wasm_hash") {
    return "different";
  }

  return "different";
}

export function getMetricValue(
  contract: ComparableContract,
  metric: ComparisonMetricKey,
): string | number | boolean {
  switch (metric) {
    case "contract_id":
      return contract.contractId;
    case "network":
      return contract.network;
    case "category":
      return contract.category;
    case "publisher":
      return contract.publisherId;
    case "verification_status":
      return contract.isVerified;
    case "wasm_hash":
      return contract.wasmHash;
    case "deployment_count":
      return contract.deploymentCount;
    case "popularity_score":
      return contract.popularityScore;
    case "health_score":
      return contract.healthScore;
  }
}

/**
 * For each contract in `contracts`, returns the ABI methods that appear in
 * that contract but in none of the others — i.e. truly unique methods.
 */
export function uniqueMethodsPerContract(
  contracts: ComparableContract[],
): Record<string, string[]> {
  const result: Record<string, string[]> = {};
  for (const contract of contracts) {
    const others = contracts.filter((c) => c.id !== contract.id);
    const otherMethods = new Set(others.flatMap((c) => c.abiMethods));
    result[contract.id] = contract.abiMethods.filter(
      (m) => !otherMethods.has(m),
    );
  }
  return result;
}

export function diffMethodSets(base: string[], other: string[]) {
  const baseSet = new Set(base);
  const otherSet = new Set(other);
  const added = [...otherSet]
    .filter((m) => !baseSet.has(m))
    .sort((a, b) => a.localeCompare(b));
  const removed = [...baseSet]
    .filter((m) => !otherSet.has(m))
    .sort((a, b) => a.localeCompare(b));
  return { added, removed };
}

export function diffLines(aText: string, bText: string): DiffLine[] {
  const a = normalizeNewlines(aText).split("\n");
  const b = normalizeNewlines(bText).split("\n");
  const result = myersDiff(a, b);
  return result;
}

function normalizeNewlines(s: string) {
  return s.replaceAll("\r\n", "\n").replaceAll("\r", "\n");
}

function myersDiff(a: string[], b: string[]): DiffLine[] {
  const n = a.length;
  const m = b.length;
  const v: Record<number, number> = { 1: 0 };
  const trace: Record<number, number>[] = [];

  for (let d = 0; d <= n + m; d++) {
    const vCopy = { ...v };
    for (let k = -d; k <= d; k += 2) {
      let x: number;
      if (k === -d || (k !== d && (v[k - 1] ?? 0) < (v[k + 1] ?? 0))) {
        x = v[k + 1] ?? 0;
      } else {
        x = (v[k - 1] ?? 0) + 1;
      }
      let y = x - k;
      while (x < n && y < m && a[x] === b[y]) {
        x++;
        y++;
      }
      v[k] = x;
      if (x >= n && y >= m) {
        return backtrackDiff(a, b, [...trace, v]);
      }
    }
    trace.push({ ...v });
  }
  return [];
}

function backtrackDiff(
  a: string[],
  b: string[],
  trace: Record<number, number>[],
): DiffLine[] {
  let x = a.length;
  let y = b.length;
  const result: DiffLine[] = [];

  for (let d = trace.length - 1; d > 0; d--) {
    const v = trace[d];
    const prevV = trace[d - 1];
    const k = x - y;

    let prevK: number;
    if (k === -d || (k !== d && (prevV[k - 1] ?? 0) < (prevV[k + 1] ?? 0))) {
      prevK = k + 1;
    } else {
      prevK = k - 1;
    }

    const prevX = prevV[prevK] ?? 0;
    const prevY = prevX - prevK;

    while (x > prevX && y > prevY) {
      result.push({ type: "context", value: a[x - 1] });
      x--;
      y--;
    }

    if (x > prevX) {
      result.push({ type: "remove", value: a[x - 1] });
      x--;
    } else if (y > prevY) {
      result.push({ type: "add", value: b[y - 1] });
      y--;
    }
  }

  while (x > 0 && y > 0) {
    result.push({ type: "context", value: a[x - 1] });
    x--;
    y--;
  }

  return result.reverse();
}
