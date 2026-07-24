"use client";

import type {
  ComparableContract,
  ComparisonMetricKey,
  CellTone,
} from "@/utils/comparison";

type Metric = {
  key: ComparisonMetricKey;
  label: string;
  getDisplayValue: (c: ComparableContract) => string;
};

type Props = {
  contract: ComparableContract;
  metrics: Metric[];
  tones: Record<ComparisonMetricKey, Record<string, CellTone>>;
};

function toneClass(tone: CellTone) {
  if (tone === "best") return "text-green-700 dark:text-green-300";
  if (tone === "worst") return "text-red-700 dark:text-red-300";
  if (tone === "different") return "text-amber-700 dark:text-amber-300";
  return "text-foreground";
}

export default function MobileComparisonCard({
  contract,
  metrics,
  tones,
}: Props) {
  return (
    <div className="rounded-2xl border border-border bg-card p-5">
      <div className="min-w-0">
        <div className="text-base font-semibold text-foreground truncate">
          {contract.name}
        </div>
        {contract.base?.contract_id && (
          <div className="mt-1 text-xs text-muted-foreground font-mono truncate">
            {contract.base.contract_id}
          </div>
        )}
      </div>
      <div className="mt-4 grid grid-cols-2 gap-3">
        {metrics.map((m) => {
          const tone = tones[m.key]?.[contract.id] ?? "neutral";
          return (
            <div
              key={m.key}
              className="rounded-xl border border-border bg-accent/40 p-3"
            >
              <div className="text-[11px] font-semibold text-muted-foreground">
                {m.label}
              </div>
              <div className={`mt-1 text-sm font-semibold ${toneClass(tone)}`}>
                {m.getDisplayValue(contract)}
              </div>
            </div>
          );
        })}
      </div>
      <div className="mt-4 grid grid-cols-1 gap-3">
        <div className="rounded-xl border border-border bg-accent/40 p-3">
          <div className="text-[11px] font-semibold text-muted-foreground">
            Latest version
          </div>
          <div className="mt-1 text-sm font-semibold text-foreground">
            {contract.latestVersion}
          </div>
        </div>
        <div className="rounded-xl border border-border bg-accent/40 p-3">
          <div className="text-[11px] font-semibold text-muted-foreground">
            ABI methods
          </div>
          <div className="mt-1 text-sm font-semibold text-foreground">
            {contract.abiMethods.length}
          </div>
        </div>
      </div>
    </div>
  );
}
