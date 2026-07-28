"use client";

import type { CellTone } from "@/utils/comparison";

type Props = {
  label: string;
  values: Array<{ contractId: string; display: string; tone: CellTone }>;
};

function toneClass(tone: CellTone) {
  if (tone === "best")
    return "bg-green-500/10 text-green-700 dark:text-green-300 border-green-500/20";
  if (tone === "worst")
    return "bg-red-500/10 text-red-700 dark:text-red-300 border-red-500/20";
  if (tone === "different")
    return "bg-amber-500/10 text-amber-700 dark:text-amber-300 border-amber-500/20";
  return "bg-transparent text-foreground border-border";
}

export default function ComparisonRow({ label, values }: Props) {
  return (
    <tr className="border-t border-border">
      <th
        scope="row"
        className="text-left text-xs font-semibold text-muted-foreground px-3 py-2 bg-accent/40"
      >
        {label}
      </th>
      {values.map((v) => (
        <td key={v.contractId} className="px-3 py-2">
          <div
            className={`rounded-lg border px-2 py-1.5 text-xs ${toneClass(v.tone)}`}
          >
            {v.display}
          </div>
        </td>
      ))}
    </tr>
  );
}
