"use client";

import { CompatibilityMatrix, CompatibilityEntry, api } from "@/lib/api";
import {
  AlertTriangle,
  CheckCircle,
  XCircle,
  Download,
  FileJson,
} from "lucide-react";

interface CompatibilityMatrixDisplayProps {
  data: CompatibilityMatrix;
  contractId: string;
}

function CompatibilityBadge({ entry }: { entry: CompatibilityEntry }) {
  if (entry.is_compatible) {
    return (
      <span className="inline-flex items-center gap-1 rounded-full bg-green-100 px-2 py-0.5 text-xs font-semibold text-green-800 dark:bg-green-900/40 dark:text-green-300">
        <CheckCircle className="h-3 w-3" />
        Compatible
      </span>
    );
  }

  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-red-100 px-2 py-0.5 text-xs font-semibold text-red-800 dark:bg-red-900/40 dark:text-red-300">
      <XCircle className="h-3 w-3" />
      Breaking
    </span>
  );
}

export function CompatibilityMatrixDisplay({
  data,
  contractId,
}: CompatibilityMatrixDisplayProps) {
  const csvUrl = api.getCompatibilityExportUrl(contractId, "csv");
  const jsonUrl = api.getCompatibilityExportUrl(contractId, "json");
  const targetVersions = data.version_order;

  return (
    <div className="space-y-6">
      {data.warnings.length > 0 && (
        <div className="rounded-xl border border-amber-300 bg-amber-50 p-4 dark:border-amber-700 dark:bg-amber-900/20">
          <div className="mb-2 flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 flex-shrink-0 text-amber-600 dark:text-amber-400" />
            <span className="text-sm font-semibold text-amber-800 dark:text-amber-300">
              Breaking changes detected
            </span>
          </div>
          <ul className="ml-7 space-y-1">
            {data.warnings.map((warning, index) => (
              <li
                key={`${warning}-${index}`}
                className="text-sm text-amber-700 dark:text-amber-300"
              >
                {warning}
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="flex flex-col justify-between gap-3 sm:flex-row sm:items-center">
        <p className="text-sm text-muted-foreground">
          {data.total_pairs} upgrade paths across {data.version_order.length}{" "}
          contract versions.
        </p>
        <div className="flex gap-2">
          <a
            href={csvUrl}
            download="compatibility.csv"
            className="inline-flex items-center gap-1.5 rounded-lg border border-border bg-card px-3 py-1.5 text-sm text-foreground transition-colors hover:bg-accent"
          >
            <Download className="h-3.5 w-3.5" />
            CSV
          </a>
          <a
            href={jsonUrl}
            download="compatibility.json"
            className="inline-flex items-center gap-1.5 rounded-lg border border-border bg-card px-3 py-1.5 text-sm text-foreground transition-colors hover:bg-accent"
          >
            <FileJson className="h-3.5 w-3.5" />
            JSON
          </a>
        </div>
      </div>

      {data.rows.length === 0 ? (
        <div className="rounded-xl border border-border bg-card py-12 text-center">
          <CheckCircle className="mx-auto mb-3 h-10 w-10 text-muted-foreground" />
          <p className="text-sm text-muted-foreground">
            No versions available to compare yet.
          </p>
        </div>
      ) : (
        <div className="overflow-x-auto rounded-xl border border-border">
          <table className="min-w-full divide-y divide-border text-sm">
            <thead className="bg-accent">
              <tr>
                <th className="sticky left-0 z-10 whitespace-nowrap border-r border-border bg-accent px-4 py-3 text-left font-semibold text-foreground">
                  From \ To
                </th>
                {targetVersions.map((version) => (
                  <th
                    key={version}
                    className="px-4 py-3 text-center font-semibold text-foreground whitespace-nowrap"
                  >
                    <span className="inline-block rounded-md bg-primary/10 px-2 py-0.5 font-mono text-xs text-primary">
                      v{version}
                    </span>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-border bg-card">
              {data.rows.map((row) => (
                <tr
                  key={row.source_version}
                  className="hover:bg-accent transition-colors"
                >
                  <td className="sticky left-0 z-10 whitespace-nowrap border-r border-border bg-card px-4 py-3 font-medium text-foreground">
                    <span className="inline-block rounded-md bg-primary/10 px-2 py-0.5 font-mono text-xs text-primary">
                      v{row.source_version}
                    </span>
                  </td>
                  {targetVersions.map((targetVersion) => {
                    const entry = row.targets.find(
                      (candidate) => candidate.target_version === targetVersion,
                    );
                    const isDiagonalOrPast =
                      targetVersion === row.source_version ||
                      data.version_order.indexOf(targetVersion) <=
                        data.version_order.indexOf(row.source_version);

                    return (
                      <td
                        key={`${row.source_version}-${targetVersion}`}
                        className={`px-4 py-3 text-center ${
                          entry
                            ? entry.is_compatible
                              ? "bg-green-50/40 dark:bg-green-900/10"
                              : "bg-red-50/40 dark:bg-red-900/10"
                            : ""
                        }`}
                      >
                        {entry ? (
                          <div className="space-y-2">
                            <div className="flex justify-center">
                              <CompatibilityBadge entry={entry} />
                            </div>
                            <div className="text-xs text-muted-foreground">
                              {entry.breaking_change_count > 0
                                ? `${entry.breaking_change_count} breaking change${entry.breaking_change_count === 1 ? "" : "s"}`
                                : "No breaking changes"}
                            </div>
                            {entry.breaking_changes.length > 0 && (
                              <details className="text-left">
                                <summary className="cursor-pointer text-xs text-muted-foreground hover:text-foreground">
                                  View differences
                                </summary>
                                <ul className="mt-2 space-y-1 text-xs text-muted-foreground">
                                  {entry.breaking_changes.map((change) => (
                                    <li key={change}>{change}</li>
                                  ))}
                                </ul>
                              </details>
                            )}
                          </div>
                        ) : isDiagonalOrPast ? (
                          <span className="text-muted-foreground">-</span>
                        ) : (
                          <span className="text-muted-foreground">
                            Unavailable
                          </span>
                        )}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
