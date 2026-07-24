"use client";

import React, { useState } from "react";
import { Download, ChevronDown, FileText, FileJson } from "lucide-react";
import { AnalyticsResponse, TimePeriod } from "@/types/analytics";

interface AnalyticsExportProps {
  data: AnalyticsResponse;
  period: TimePeriod;
}

function csvEscape(value: string | number | null | undefined): string {
  if (value === null || value === undefined) {
    return "";
  }
  const str = String(value);
  if (/[",\r\n]/.test(str)) {
    const escaped = str.replace(/"/g, '""');
    return `"${escaped}"`;
  }
  return str;
}

function toCSV(data: AnalyticsResponse): string {
  const sections: string[] = [];

  sections.push("=== Search Trends ===");
  sections.push("Date,Searches,Unique Terms");
  data.searchTrends.forEach((row) => {
    sections.push(
      `${csvEscape(row.date)},${csvEscape(row.searches)},${csvEscape(row.uniqueTerms)}`,
    );
  });

  sections.push("");
  sections.push("=== Top Search Terms ===");
  sections.push("Term,Count,Growth(%)");
  data.topSearchTerms.forEach((row) => {
    sections.push(
      `${csvEscape(row.term)},${csvEscape(row.count)},${csvEscape(row.growth)}`,
    );
  });

  sections.push("");
  sections.push("=== Engagement Funnel ===");
  sections.push("Stage,Users,Percentage(%)");
  data.engagementFunnel.forEach((row) => {
    sections.push(
      `${csvEscape(row.stage)},${csvEscape(row.users)},${csvEscape(row.percentage)}`,
    );
  });

  sections.push("");
  sections.push("=== Category Popularity ===");
  sections.push("Category,Searches,Views,Deployments");
  data.categoryPopularity.forEach((row) => {
    sections.push(
      `${csvEscape(row.category)},${csvEscape(row.searches)},${csvEscape(row.views)},${csvEscape(row.deployments)}`,
    );
  });

  sections.push("");
  sections.push("=== Network Distribution ===");
  sections.push("Network,Region,Count,Percentage(%)");
  data.networkDistribution.forEach((row) => {
    sections.push(
      `${csvEscape(row.network)},${csvEscape(row.region)},${csvEscape(row.count)},${csvEscape(row.percentage)}`,
    );
  });

  return sections.join("\n");
}

function downloadFile(content: string, filename: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

const AnalyticsExport: React.FC<AnalyticsExportProps> = ({ data, period }) => {
  const [open, setOpen] = useState(false);
  const [exporting, setExporting] = useState(false);

  const timestamp = new Date().toISOString().split("T")[0];
  const baseFilename = `soroban-analytics-${period}-${timestamp}`;

  const exportJSON = async () => {
    setExporting(true);
    try {
      const content = JSON.stringify(data, null, 2);
      downloadFile(content, `${baseFilename}.json`, "application/json");
    } finally {
      setExporting(false);
      setOpen(false);
    }
  };

  const exportCSV = async () => {
    setExporting(true);
    try {
      const content = toCSV(data);
      downloadFile(content, `${baseFilename}.csv`, "text/csv");
    } finally {
      setExporting(false);
      setOpen(false);
    }
  };

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        disabled={exporting}
        className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg border border-border bg-card text-sm font-medium text-foreground hover:bg-accent transition-colors disabled:opacity-50"
      >
        <Download className="w-4 h-4" />
        Export
        <ChevronDown
          className={`w-3.5 h-3.5 transition-transform ${open ? "rotate-180" : ""}`}
        />
      </button>

      {open && (
        <>
          <div className="fixed inset-0 z-10" onClick={() => setOpen(false)} />
          <div className="absolute right-0 top-full mt-1.5 z-20 w-44 rounded-xl border border-border bg-card shadow-lg shadow-black/10 overflow-hidden">
            <div className="py-1">
              <button
                onClick={exportCSV}
                className="flex items-center gap-2.5 w-full px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
              >
                <FileText className="w-4 h-4 text-primary/70" />
                Export as CSV
              </button>
              <button
                onClick={exportJSON}
                className="flex items-center gap-2.5 w-full px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
              >
                <FileJson className="w-4 h-4 text-primary/70" />
                Export as JSON
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
};

export default AnalyticsExport;
