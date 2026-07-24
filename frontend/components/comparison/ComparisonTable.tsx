"use client";

import type {
  ComparableContract,
  ComparisonMetricKey,
  CellTone,
} from "@/utils/comparison";
import ComparisonRow from "./ComparisonRow";

type Metric = {
  key: ComparisonMetricKey;
  label: string;
  getDisplayValue: (c: ComparableContract) => string;
};

type Props = {
  contracts: ComparableContract[];
  metrics: Metric[];
  tones: Record<ComparisonMetricKey, Record<string, CellTone>>;
};

export default function ComparisonTable({ contracts, metrics, tones }: Props) {
  return (
    <div className="hidden lg:block overflow-x-auto rounded-2xl border border-border bg-card">
      <table className="min-w-full">
        <thead className="bg-accent/60">
          <tr>
            <th className="px-3 py-3 text-left text-xs font-semibold text-muted-foreground">
              Attribute
            </th>
            {contracts.map((c) => (
              <th
                key={c.id}
                className="px-3 py-3 text-left text-xs font-semibold text-foreground"
              >
                <div className="min-w-[220px]">
                  <div className="font-semibold">{c.name}</div>
                  {c.base?.contract_id && (
                    <div className="mt-1 text-[11px] text-muted-foreground font-mono">
                      {c.base.contract_id}
                    </div>
                  )}
                </div>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {metrics.map((m) => (
            <ComparisonRow
              key={m.key}
              label={m.label}
              values={contracts.map((c) => ({
                contractId: c.id,
                display: m.getDisplayValue(c),
                tone: tones[m.key]?.[c.id] ?? "neutral",
              }))}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}
