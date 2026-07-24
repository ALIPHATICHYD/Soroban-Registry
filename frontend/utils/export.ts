import type { ComparableContract } from "@/utils/comparison";

function escapeCsvCell(value: string) {
  if (
    value.includes('"') ||
    value.includes(",") ||
    value.includes("\n") ||
    value.includes("\r")
  ) {
    return `"${value.replaceAll('"', '""')}"`;
  }
  return value;
}

export function buildComparisonCsv(
  contracts: ComparableContract[],
  metrics: Array<{
    key: string;
    label: string;
    getValue: (c: ComparableContract) => string | number | boolean;
  }>,
) {
  const header = ["Attribute", ...contracts.map((c) => c.name)];
  const rows: string[][] = [header];

  for (const metric of metrics) {
    const row = [
      metric.label,
      ...contracts.map((c) => String(metric.getValue(c))),
    ];
    rows.push(row);
  }

  return rows.map((r) => r.map(escapeCsvCell).join(",")).join("\n");
}

export function downloadTextFile(
  filename: string,
  content: string,
  mimeType: string,
) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export function exportComparisonToCsv(
  contracts: ComparableContract[],
  metrics: Array<{
    key: string;
    label: string;
    getValue: (c: ComparableContract) => string | number | boolean;
  }>,
) {
  const csv = buildComparisonCsv(contracts, metrics);
  const filename = `contracts-comparison-${new Date().toISOString().slice(0, 10)}.csv`;
  downloadTextFile(filename, csv, "text/csv;charset=utf-8");
}

export function exportComparisonToPdf(
  contracts: ComparableContract[],
  rows: Array<{ label: string; values: string[] }>,
) {
  const title = "Soroban Registry - Contract Comparison";
  const html = `<!doctype html>
  <html>
    <head>
      <meta charset="utf-8" />
      <title>${title}</title>
      <style>
        body { font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Arial; color: #0f172a; padding: 24px; }
        h1 { font-size: 18px; margin: 0 0 8px; }
        p { font-size: 12px; margin: 0 0 16px; color: #475569; }
        table { width: 100%; border-collapse: collapse; font-size: 12px; }
        th, td { border: 1px solid #cbd5e1; padding: 8px; vertical-align: top; }
        th { background: #f1f5f9; text-align: left; }
        code { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace; }
      </style>
    </head>
    <body>
      <h1>${title}</h1>
      <p>Compared: ${contracts.map((c) => c.name).join(" | ")}</p>
      <table>
        <thead>
          <tr>
            <th>Attribute</th>
            ${contracts.map((c) => `<th>${c.name}</th>`).join("")}
          </tr>
        </thead>
        <tbody>
          ${rows
            .map(
              (r) => `<tr>
                <td>${r.label}</td>
                ${r.values.map((v) => `<td>${escapeHtml(String(v))}</td>`).join("")}
              </tr>`,
            )
            .join("")}
        </tbody>
      </table>
    </body>
  </html>`;

  const win = window.open("", "_blank", "noopener,noreferrer");
  if (!win) return false;
  win.document.open();
  win.document.write(html);
  win.document.close();

  const finalize = () => {
    win.focus();
    win.print();
  };

  if (win.document.readyState === "complete") {
    setTimeout(finalize, 50);
  } else {
    win.addEventListener("load", () => setTimeout(finalize, 50), {
      once: true,
    });
  }
  return true;
}

function escapeHtml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}
