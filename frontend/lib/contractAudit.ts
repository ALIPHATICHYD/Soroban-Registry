export type AuditSeverity = "critical" | "high" | "medium" | "low" | "info";

export type AuditCategory =
  | "access-control"
  | "runtime-safety"
  | "resource-usage"
  | "storage"
  | "observability"
  | "maintainability"
  | "optimization";

export interface AuditFinding {
  id: string;
  title: string;
  severity: AuditSeverity;
  category: AuditCategory;
  line: number;
  evidence: string;
  explanation: string;
  recommendation: string;
  confidence: number;
  weight: number;
}

export interface AuditRecommendation {
  id: string;
  title: string;
  category: AuditCategory;
  priority: "must-fix" | "should-fix" | "consider";
  rationale: string;
  implementation: string;
}

export interface AuditOptimization {
  id: string;
  title: string;
  impact: "high" | "medium" | "low";
  explanation: string;
}

export interface AuditSignal {
  name: string;
  value: number;
  contribution: number;
  explanation: string;
}

export interface AuditReport {
  score: number;
  grade: "A" | "B" | "C" | "D" | "F";
  generatedAt: string;
  model: string;
  findings: AuditFinding[];
  recommendations: AuditRecommendation[];
  optimizations: AuditOptimization[];
  signals: AuditSignal[];
  summary: string;
}

interface RuleMatch {
  id: string;
  title: string;
  severity: AuditSeverity;
  category: AuditCategory;
  test: (
    source: string,
    lines: string[],
  ) => Array<{ line: number; evidence: string }>;
  explanation: string;
  recommendation: string;
  confidence: number;
  weight: number;
}

const SEVERITY_PENALTY: Record<AuditSeverity, number> = {
  critical: 28,
  high: 20,
  medium: 12,
  low: 6,
  info: 2,
};

function lineNumberForIndex(source: string, index: number): number {
  return source.slice(0, index).split("\n").length;
}

function firstEvidence(lines: string[], predicate: (line: string) => boolean) {
  const index = lines.findIndex(predicate);
  if (index === -1) return [];
  return [{ line: index + 1, evidence: lines[index].trim() }];
}

function publicFunctionBlocks(source: string) {
  const blocks: Array<{
    name: string;
    line: number;
    body: string;
    signature: string;
  }> = [];
  const matcher =
    /pub\s+fn\s+([A-Za-z0-9_]+)\s*\([^)]*\)(?:\s*->\s*[^{]+)?\s*\{/g;
  let match: RegExpExecArray | null;

  while ((match = matcher.exec(source)) !== null) {
    let depth = 1;
    let cursor = matcher.lastIndex;
    while (cursor < source.length && depth > 0) {
      const char = source[cursor];
      if (char === "{") depth += 1;
      if (char === "}") depth -= 1;
      cursor += 1;
    }

    blocks.push({
      name: match[1],
      line: lineNumberForIndex(source, match.index),
      body: source.slice(
        matcher.lastIndex,
        Math.max(matcher.lastIndex, cursor - 1),
      ),
      signature: match[0],
    });
  }

  return blocks;
}

function duplicateStorageKeyFindings(source: string) {
  const findings: Array<{ line: number; evidence: string }> = [];
  const constants = Array.from(
    source.matchAll(/const\s+([A-Za-z0-9_]+)\s*:\s*&str\s*=\s*"([^"]+)"/g),
  );
  const byValue = new Map<string, Array<{ name: string; line: number }>>();

  for (const constant of constants) {
    const value = constant[2];
    const item = {
      name: constant[1],
      line: lineNumberForIndex(source, constant.index ?? 0),
    };
    byValue.set(value, [...(byValue.get(value) ?? []), item]);
  }

  for (const [value, items] of byValue.entries()) {
    if (items.length > 1) {
      const names = items.map((item) => item.name).join(", ");
      findings.push({
        line: items[1].line,
        evidence: `${names} share "${value}"`,
      });
    }
  }

  return findings;
}

const AUDIT_RULES: RuleMatch[] = [
  {
    id: "missing-auth",
    title: "State-changing public function lacks authorization",
    severity: "critical",
    category: "access-control",
    test: (source) =>
      publicFunctionBlocks(source)
        .filter((block) =>
          /(set_|update_|mint|burn|transfer|withdraw|admin|upgrade)/.test(
            block.name,
          ),
        )
        .filter((block) => !/require_auth\s*\(/.test(block.body))
        .map((block) => ({
          line: block.line,
          evidence: block.signature.trim(),
        })),
    explanation:
      "The function name and body indicate a privileged state change, but no Soroban Address::require_auth check was found in the function body.",
    recommendation:
      "Require authorization from the account that owns the state transition before writing storage or invoking privileged behavior.",
    confidence: 0.89,
    weight: 1,
  },
  {
    id: "panic-public",
    title: "Public contract path can panic",
    severity: "high",
    category: "runtime-safety",
    test: (_source, lines) =>
      firstEvidence(
        lines,
        (line) =>
          /\b(panic!|unwrap\s*\(|expect\s*\()/.test(line) &&
          !line.includes("#[test]"),
      ),
    explanation:
      "Panics make failure modes harder to compose and can hide recoverable contract errors from callers.",
    recommendation:
      "Return Result values or safe defaults for expected failure cases, and reserve panics for impossible internal states.",
    confidence: 0.82,
    weight: 0.9,
  },
  {
    id: "unchecked-arithmetic",
    title: "Arithmetic is not explicitly checked",
    severity: "high",
    category: "runtime-safety",
    test: (_source, lines) =>
      firstEvidence(
        lines,
        (line) =>
          /(amount|balance|supply|allowance|counter).*(\+=|-=|\*=| \+ | - | \* )/.test(
            line,
          ) && !/checked_|saturating_|MAX_/.test(line),
      ),
    explanation:
      "Financial state transitions should use checked or saturating arithmetic so overflow behavior is explicit.",
    recommendation:
      "Use checked_add, checked_sub, or domain-specific bounds before committing token balances or supply changes.",
    confidence: 0.78,
    weight: 0.85,
  },
  {
    id: "unbounded-loop",
    title: "Loop may consume an unbounded instruction budget",
    severity: "medium",
    category: "resource-usage",
    test: (_source, lines) =>
      firstEvidence(
        lines,
        (line) =>
          /\b(loop|while|for)\b/.test(line) &&
          !/MAX_|take\s*\(|limit|bounded|counter/.test(line),
      ),
    explanation:
      "Soroban contracts execute under strict instruction budgets, so loops need caller-independent bounds.",
    recommendation:
      "Cap iteration counts with constants, pagination, or bounded input validation before entering the loop.",
    confidence: 0.74,
    weight: 0.75,
  },
  {
    id: "duplicate-storage-key",
    title: "Storage key collision risk detected",
    severity: "high",
    category: "storage",
    test: duplicateStorageKeyFindings,
    explanation:
      "Two logical storage keys resolve to the same serialized string, allowing unrelated state to overwrite itself.",
    recommendation:
      "Replace raw string keys with a typed DataKey enum and include account identifiers in key variants.",
    confidence: 0.95,
    weight: 0.95,
  },
  {
    id: "hardcoded-address",
    title: "Hardcoded address or hash-like identifier found",
    severity: "medium",
    category: "maintainability",
    test: (_source, lines) =>
      firstEvidence(lines, (line) =>
        /"(G[A-Z2-7]{20,}|0x[a-fA-F0-9]{32,})"/.test(line),
      ),
    explanation:
      "Hardcoded principals make upgrades, testing, and multi-network deployment brittle.",
    recommendation:
      "Move addresses into initialization state, constructor arguments, or per-network registry metadata.",
    confidence: 0.7,
    weight: 0.65,
  },
  {
    id: "missing-events",
    title: "State changes do not emit events",
    severity: "low",
    category: "observability",
    test: (source) => {
      const mutatesState =
        /storage\(\)\.(persistent|temporary|instance)\(\)\.(set|remove)/.test(
          source,
        );
      const emitsEvent = /events\(\)\.publish/.test(source);
      return mutatesState && !emitsEvent
        ? [{ line: 1, evidence: "storage writes without events().publish" }]
        : [];
    },
    explanation:
      "Events make off-chain indexing, monitoring, and incident response easier for registry consumers.",
    recommendation:
      "Publish typed events for balance changes, admin actions, and registry-visible state transitions.",
    confidence: 0.81,
    weight: 0.45,
  },
  {
    id: "raw-string-storage",
    title: "Raw string storage keys reduce type safety",
    severity: "medium",
    category: "storage",
    test: (_source, lines) =>
      firstEvidence(
        lines,
        (line) =>
          /Symbol::new\([^,]+,\s*"/.test(line) || /&str\s*=\s*"/.test(line),
      ),
    explanation:
      "String keys are easy to duplicate and do not encode the shape of the data stored behind them.",
    recommendation:
      "Use #[contracttype] enum keys so the compiler helps keep storage namespaces distinct.",
    confidence: 0.86,
    weight: 0.7,
  },
];

function calculateSignals(
  source: string,
  findings: AuditFinding[],
): AuditSignal[] {
  const publicFns = publicFunctionBlocks(source);
  const authCoverage =
    publicFns.length === 0
      ? 1
      : publicFns.filter((block) => /require_auth\s*\(/.test(block.body))
          .length / publicFns.length;
  const resultCoverage =
    publicFns.length === 0
      ? 1
      : publicFns.filter((block) => /->\s*Result\b/.test(block.signature))
          .length / publicFns.length;
  const storageTyping = /enum\s+[A-Za-z0-9_]*Key|contracttype/.test(source)
    ? 1
    : 0;
  const eventCoverage = /events\(\)\.publish/.test(source) ? 1 : 0;
  const findingPressure = Math.min(
    1,
    findings.reduce((sum, finding) => sum + finding.weight, 0) / 7,
  );

  return [
    {
      name: "Authorization coverage",
      value: Math.round(authCoverage * 100),
      contribution: Math.round(authCoverage * 12),
      explanation:
        "Share of public functions with an explicit require_auth check.",
    },
    {
      name: "Recoverable errors",
      value: Math.round(resultCoverage * 100),
      contribution: Math.round(resultCoverage * 8),
      explanation:
        "Share of public functions that expose Result-based failures.",
    },
    {
      name: "Typed storage",
      value: storageTyping * 100,
      contribution: storageTyping * 8,
      explanation:
        "Detects contracttype-backed storage keys instead of raw strings.",
    },
    {
      name: "Event observability",
      value: eventCoverage * 100,
      contribution: eventCoverage * 5,
      explanation:
        "Rewards event publication for registry and indexer visibility.",
    },
    {
      name: "Risk pressure",
      value: Math.round((1 - findingPressure) * 100),
      contribution: -Math.round(findingPressure * 20),
      explanation:
        "Aggregates weighted vulnerability and best-practice findings.",
    },
  ];
}

function gradeForScore(score: number): AuditReport["grade"] {
  if (score >= 90) return "A";
  if (score >= 80) return "B";
  if (score >= 70) return "C";
  if (score >= 60) return "D";
  return "F";
}

function recommendationPriority(
  severity: AuditSeverity,
): AuditRecommendation["priority"] {
  if (severity === "critical" || severity === "high") return "must-fix";
  if (severity === "medium") return "should-fix";
  return "consider";
}

export function analyzeContractSource(
  source: string,
  generatedAt = new Date().toISOString(),
): AuditReport {
  const lines = source.split("\n");
  const findings = AUDIT_RULES.flatMap((rule) =>
    rule.test(source, lines).map((match) => ({
      id: rule.id,
      title: rule.title,
      severity: rule.severity,
      category: rule.category,
      line: match.line,
      evidence: match.evidence,
      explanation: rule.explanation,
      recommendation: rule.recommendation,
      confidence: rule.confidence,
      weight: rule.weight,
    })),
  );

  const penalty = findings.reduce(
    (sum, finding) =>
      sum +
      SEVERITY_PENALTY[finding.severity] * finding.weight * finding.confidence,
    0,
  );
  const signals = calculateSignals(source, findings);
  const signalBonus = signals.reduce(
    (sum, signal) => sum + signal.contribution,
    0,
  );
  const score = Math.max(
    0,
    Math.min(100, Math.round(82 - penalty + signalBonus)),
  );
  const topFindings = findings.slice(0, 5);

  return {
    score,
    grade: gradeForScore(score),
    generatedAt,
    model: "soroban-explainable-audit-v1",
    findings,
    recommendations: topFindings.map((finding) => ({
      id: `${finding.id}-recommendation`,
      title: finding.title,
      category: finding.category,
      priority: recommendationPriority(finding.severity),
      rationale: finding.explanation,
      implementation: finding.recommendation,
    })),
    optimizations: [
      {
        id: "typed-storage",
        title: "Prefer typed storage keys",
        impact: /contracttype|enum\s+[A-Za-z0-9_]*Key/.test(source)
          ? "low"
          : "high",
        explanation:
          "Typed keys reduce collision risk and make state migrations easier to review.",
      },
      {
        id: "event-indexing",
        title: "Emit events for registry indexing",
        impact: /events\(\)\.publish/.test(source) ? "low" : "medium",
        explanation:
          "Event streams improve auditability, search freshness, and downstream analytics.",
      },
      {
        id: "bounded-execution",
        title: "Keep execution budget bounded",
        impact: /\b(loop|while|for)\b/.test(source) ? "medium" : "low",
        explanation:
          "Bounded loops and input limits reduce failed invocations under Soroban budgets.",
      },
    ],
    signals,
    summary:
      findings.length === 0
        ? "No high-confidence issues were detected by the local audit model."
        : `${findings.length} issue${findings.length === 1 ? "" : "s"} detected, led by ${findings[0].title.toLowerCase()}.`,
  };
}
