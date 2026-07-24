"use client";

import { DiffLine } from "@/utils/comparison";
import { SideBySideRow } from "./SideBySideView";

export function calcStats(lines: DiffLine[]) {
  let added = 0;
  let removed = 0;
  for (const l of lines) {
    if (l.type === "add") added++;
    else if (l.type === "remove") removed++;
  }
  return { added, removed, changed: Math.min(added, removed) };
}

export function buildPatch(
  lines: DiffLine[],
  fromLabel: string,
  toLabel: string,
): string {
  const hunkLines: string[] = [];
  const oldStart = 1;
  const newStart = 1;
  let oldCount = 0;
  let newCount = 0;

  for (const l of lines) {
    if (l.type === "context") {
      hunkLines.push(` ${l.value}`);
      oldCount++;
      newCount++;
    } else if (l.type === "remove") {
      hunkLines.push(`-${l.value}`);
      oldCount++;
    } else {
      hunkLines.push(`+${l.value}`);
      newCount++;
    }
  }

  const header = `--- a/${fromLabel}\n+++ b/${toLabel}\n@@ -${oldStart},${oldCount} +${newStart},${newCount} @@\n`;
  return header + hunkLines.join("\n") + "\n";
}

export function downloadPatch(content: string, filename: string) {
  const blob = new Blob([content], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export function toSideBySideRows(lines: DiffLine[]): SideBySideRow[] {
  const rows: SideBySideRow[] = [];
  let leftNo = 1;
  let rightNo = 1;
  let i = 0;

  while (i < lines.length) {
    const l = lines[i];

    if (l.type === "context") {
      rows.push({ kind: "context", lineNo: leftNo, value: l.value });
      leftNo++;
      rightNo++;
      i++;
      continue;
    }

    const removes: string[] = [];
    const adds: string[] = [];

    while (i < lines.length && lines[i].type === "remove") {
      removes.push(lines[i].value);
      i++;
    }
    while (i < lines.length && lines[i].type === "add") {
      adds.push(lines[i].value);
      i++;
    }

    const maxLen = Math.max(removes.length, adds.length);
    for (let j = 0; j < maxLen; j++) {
      const leftVal = removes[j] ?? null;
      const rightVal = adds[j] ?? null;
      rows.push({
        kind: "change",
        leftNo: leftVal !== null ? leftNo : null,
        leftValue: leftVal,
        rightNo: rightVal !== null ? rightNo : null,
        rightValue: rightVal,
      });
      if (leftVal !== null) leftNo++;
      if (rightVal !== null) rightNo++;
    }
  }

  return rows;
}
