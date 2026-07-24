"use client";

import { ChangeEvent, useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import {
  AlertTriangle,
  ArrowLeft,
  CheckCircle2,
  Clock3,
  Download,
  FileUp,
  RefreshCw,
  UploadCloud,
} from "lucide-react";
import { api, Contract, Network, PublishRequest } from "@/lib/api";
import { useToast } from "@/hooks/useToast";
import { downloadTextFile } from "@/utils/export";

type ImportFormat = "json" | "csv";
type ExportFormat = "json" | "csv";
type HistoryKind = "import" | "export";
type HistoryStatus = "success" | "partial" | "failed";

type ImportPreviewRow = PublishRequest & {
  rowNumber: number;
};

type ImportError = {
  rowNumber: number;
  field: keyof PublishRequest | "row";
  message: string;
};

type ExportFilters = {
  query: string;
  network: "" | Network;
  category: string;
  verifiedOnly: boolean;
};

type ProgressState = {
  active: boolean;
  mode: HistoryKind;
  current: number;
  total: number;
  label: string;
};

type HistoryItem = {
  id: string;
  kind: HistoryKind;
  status: HistoryStatus;
  createdAt: string;
  format: ImportFormat | ExportFormat;
  fileName?: string;
  summary: string;
  total: number;
  successCount: number;
  failureCount: number;
  filters?: ExportFilters;
  errors?: Array<{ rowNumber?: number; message: string }>;
  failedRows?: ImportPreviewRow[];
};

const HISTORY_STORAGE_KEY = "contract_import_export_history_v1";
const MAX_HISTORY_ITEMS = 50;
const MAX_IMPORT_FILE_BYTES = 15 * 1024 * 1024;
const REQUIRED_IMPORT_FIELDS: Array<keyof PublishRequest> = [
  "contract_id",
  "name",
  "network",
  "publisher_address",
  "tags",
];
const PREVIEW_ROWS_LIMIT = 10;

function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let inQuotes = false;

  for (let i = 0; i < text.length; i += 1) {
    const char = text[i];
    const nextChar = text[i + 1];

    if (char === '"') {
      if (inQuotes && nextChar === '"') {
        field += '"';
        i += 1;
      } else {
        inQuotes = !inQuotes;
      }
      continue;
    }

    if (char === "," && !inQuotes) {
      row.push(field.trim());
      field = "";
      continue;
    }

    if ((char === "\n" || char === "\r") && !inQuotes) {
      if (char === "\r" && nextChar === "\n") {
        i += 1;
      }
      row.push(field.trim());
      const hasContent = row.some((value) => value.length > 0);
      if (hasContent) {
        rows.push(row);
      }
      row = [];
      field = "";
      continue;
    }

    field += char;
  }

  if (field.length > 0 || row.length > 0) {
    row.push(field.trim());
    if (row.some((value) => value.length > 0)) {
      rows.push(row);
    }
  }

  return rows;
}

function parseTags(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value.map((tag) => String(tag).trim()).filter(Boolean);
  }

  if (typeof value !== "string") {
    return [];
  }

  return value
    .split(/\||;|,/g)
    .map((tag) => tag.trim())
    .filter(Boolean);
}

function normalizeRow(
  row: Partial<PublishRequest> & Record<string, unknown>,
  rowNumber: number,
): { value?: ImportPreviewRow; errors: ImportError[] } {
  const network = (row.network ?? "").toString().trim().toLowerCase();
  const normalizedNetwork =
    network === "mainnet" || network === "testnet" || network === "futurenet"
      ? network
      : undefined;

  const normalized: ImportPreviewRow = {
    rowNumber,
    contract_id: (row.contract_id ?? "").toString().trim(),
    name: (row.name ?? "").toString().trim(),
    description: (row.description ?? "").toString().trim() || undefined,
    network: (normalizedNetwork ?? "testnet") as Network,
    category: (row.category ?? "").toString().trim() || undefined,
    tags: parseTags(row.tags),
    source_url: (row.source_url ?? "").toString().trim() || undefined,
    publisher_address: (row.publisher_address ?? "").toString().trim(),
  };

  const errors: ImportError[] = [];

  for (const field of REQUIRED_IMPORT_FIELDS) {
    if (field === "tags") {
      if (normalized.tags.length === 0) {
        errors.push({
          rowNumber,
          field,
          message: "At least one tag is required.",
        });
      }
      continue;
    }

    if (!normalized[field]) {
      errors.push({ rowNumber, field, message: `${field} is required.` });
    }
  }

  if (!normalizedNetwork) {
    errors.push({
      rowNumber,
      field: "network",
      message: `Invalid network "${row.network ?? ""}". Use mainnet, testnet, or futurenet.`,
    });
  }

  return {
    value: errors.length === 0 ? normalized : undefined,
    errors,
  };
}

function detectImportFormat(fileName: string): ImportFormat | null {
  const lower = fileName.toLowerCase();
  if (lower.endsWith(".json")) return "json";
  if (lower.endsWith(".csv")) return "csv";
  return null;
}

function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  return "Unknown error";
}

function escapeCsvCell(value: string): string {
  if (/[,"\n\r]/.test(value)) {
    return `"${value.replaceAll('"', '""')}"`;
  }
  return value;
}

function formatDateTime(value: string): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function buildExportRows(contracts: Contract[]) {
  return contracts.map((contract) => ({
    id: contract.id,
    contract_id: contract.contract_id,
    wasm_hash: contract.wasm_hash,
    name: contract.name,
    description: contract.description ?? "",
    publisher_id: contract.publisher_id,
    network: contract.network,
    is_verified: contract.is_verified,
    category: contract.category ?? "",
    tags: contract.tags.join("|"),
    popularity_score: contract.popularity_score ?? "",
    downloads: contract.downloads ?? "",
    average_rating: contract.average_rating ?? contract.avg_rating ?? "",
    review_count: contract.review_count ?? "",
    deployment_count: contract.deployment_count ?? "",
    interaction_count: contract.interaction_count ?? "",
    relevance_score: contract.relevance_score ?? "",
    logo_url: contract.logo_url ?? "",
    created_at: contract.created_at,
    updated_at: contract.updated_at,
    verified_at: contract.verified_at ?? "",
    last_accessed_at: contract.last_accessed_at ?? "",
    is_maintenance: contract.is_maintenance ?? false,
    logical_id: contract.logical_id ?? "",
    network_configs: contract.network_configs
      ? JSON.stringify(contract.network_configs)
      : "",
  }));
}

export default function ContractImportExportPanel() {
  const { showError, showInfo, showSuccess, showWarning } = useToast();

  const [importFileName, setImportFileName] = useState<string>("");
  const [importFormat, setImportFormat] = useState<ImportFormat>("json");
  const [importPreviewRows, setImportPreviewRows] = useState<
    ImportPreviewRow[]
  >([]);
  const [importErrors, setImportErrors] = useState<ImportError[]>([]);
  const [importing, setImporting] = useState(false);

  const [exportFormat, setExportFormat] = useState<ExportFormat>("json");
  const [exportFilters, setExportFilters] = useState<ExportFilters>({
    query: "",
    network: "",
    category: "",
    verifiedOnly: false,
  });
  const [exporting, setExporting] = useState(false);

  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [progress, setProgress] = useState<ProgressState>({
    active: false,
    mode: "import",
    current: 0,
    total: 0,
    label: "",
  });

  useEffect(() => {
    if (typeof window === "undefined") return;
    const raw = window.localStorage.getItem(HISTORY_STORAGE_KEY);
    if (!raw) return;

    try {
      const parsed = JSON.parse(raw) as HistoryItem[];
      if (Array.isArray(parsed)) {
        setHistory(parsed);
      }
    } catch {
      window.localStorage.removeItem(HISTORY_STORAGE_KEY);
    }
  }, []);

  const persistHistory = useCallback((next: HistoryItem[]) => {
    setHistory(next);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify(next));
    }
  }, []);

  const addHistoryItem = useCallback(
    (item: HistoryItem) => {
      const next = [item, ...history].slice(0, MAX_HISTORY_ITEMS);
      persistHistory(next);
    },
    [history, persistHistory],
  );

  const progressPercent = useMemo(() => {
    if (!progress.active || progress.total <= 0) return 0;
    return Math.min(100, Math.round((progress.current / progress.total) * 100));
  }, [progress.active, progress.current, progress.total]);

  const onSelectImportFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const selectedFile = event.target.files?.[0];
    if (!selectedFile) return;

    const detected = detectImportFormat(selectedFile.name);
    if (!detected) {
      showError("Unsupported file type. Upload a .json or .csv file.");
      return;
    }

    if (selectedFile.size > MAX_IMPORT_FILE_BYTES) {
      showError("File is too large. Maximum supported size is 15MB.");
      return;
    }

    setImportFileName(selectedFile.name);
    setImportFormat(detected);
    setImportPreviewRows([]);
    setImportErrors([]);

    try {
      const text = await selectedFile.text();
      const parsedRows: Array<Record<string, unknown>> = [];

      if (detected === "json") {
        const payload = JSON.parse(text) as unknown;
        const candidates = Array.isArray(payload)
          ? payload
          : typeof payload === "object" &&
              payload !== null &&
              Array.isArray((payload as { items?: unknown[] }).items)
            ? (payload as { items: unknown[] }).items
            : typeof payload === "object" &&
                payload !== null &&
                Array.isArray((payload as { contracts?: unknown[] }).contracts)
              ? (payload as { contracts: unknown[] }).contracts
              : [];

        if (candidates.length === 0) {
          throw new Error(
            "No rows found. Expected a JSON array or an object with items/contracts.",
          );
        }

        candidates.forEach((candidate) => {
          if (candidate && typeof candidate === "object") {
            parsedRows.push(candidate as Record<string, unknown>);
          }
        });
      } else {
        const rows = parseCsv(text);
        if (rows.length < 2) {
          throw new Error(
            "CSV must include a header row and at least one data row.",
          );
        }

        const headers = rows[0].map((value) => value.trim());
        rows.slice(1).forEach((line) => {
          const row: Record<string, unknown> = {};
          headers.forEach((header, index) => {
            row[header] = line[index] ?? "";
          });
          parsedRows.push(row);
        });
      }

      const validRows: ImportPreviewRow[] = [];
      const rowErrors: ImportError[] = [];

      parsedRows.forEach((row, index) => {
        const normalized = normalizeRow(
          row as Partial<PublishRequest> & Record<string, unknown>,
          index + 1,
        );
        if (normalized.value) {
          validRows.push(normalized.value);
        }
        if (normalized.errors.length > 0) {
          rowErrors.push(...normalized.errors);
        }
      });

      setImportPreviewRows(validRows);
      setImportErrors(rowErrors);

      if (validRows.length === 0) {
        showError("No valid rows found. Fix the file and upload again.");
      } else if (rowErrors.length > 0) {
        showWarning(
          `Preview ready with ${rowErrors.length} validation issue(s). Only valid rows can be imported.`,
        );
      } else {
        showSuccess(`Preview ready for ${validRows.length} contract row(s).`);
      }
    } catch (error) {
      setImportPreviewRows([]);
      setImportErrors([
        {
          rowNumber: 0,
          field: "row",
          message: toErrorMessage(error),
        },
      ]);
      showError(`Unable to parse file: ${toErrorMessage(error)}`);
    }
  };

  const runImport = useCallback(
    async (
      rows: ImportPreviewRow[],
      source: { fileName?: string; format: ImportFormat; reason: string },
    ) => {
      if (rows.length === 0) {
        showWarning("Nothing to import.");
        return;
      }

      const confirmed = window.confirm(
        `Confirm import of ${rows.length} row(s). This action cannot be undone.`,
      );
      if (!confirmed) return;

      setImporting(true);
      setProgress({
        active: true,
        mode: "import",
        current: 0,
        total: rows.length,
        label: source.reason,
      });
      showInfo(`Import started for ${rows.length} row(s).`);

      let successCount = 0;
      const failedRows: ImportPreviewRow[] = [];
      const failedErrors: Array<{ rowNumber?: number; message: string }> = [];

      for (let index = 0; index < rows.length; index += 1) {
        const row = rows[index];

        try {
          const request: PublishRequest = {
            contract_id: row.contract_id,
            name: row.name,
            description: row.description,
            network: row.network,
            category: row.category,
            tags: row.tags,
            source_url: row.source_url,
            publisher_address: row.publisher_address,
          };
          await api.publishContract(request);
          successCount += 1;
        } catch (error) {
          failedRows.push(row);
          failedErrors.push({
            rowNumber: row.rowNumber,
            message: toErrorMessage(error),
          });
        }

        setProgress((current: ProgressState) => ({
          ...current,
          current: index + 1,
        }));

        if ((index + 1) % 20 === 0) {
          await new Promise<void>((resolve) => {
            window.setTimeout(() => resolve(), 0);
          });
        }
      }

      const failureCount = failedRows.length;
      const status: HistoryStatus =
        failureCount === 0
          ? "success"
          : successCount > 0
            ? "partial"
            : "failed";

      addHistoryItem({
        id: `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
        kind: "import",
        status,
        createdAt: new Date().toISOString(),
        format: source.format,
        fileName: source.fileName,
        summary:
          failureCount === 0
            ? `Imported ${successCount} contracts.`
            : `Imported ${successCount}/${rows.length} contracts (${failureCount} failed).`,
        total: rows.length,
        successCount,
        failureCount,
        errors: failedErrors.slice(0, 20),
        failedRows,
      });

      setProgress({
        active: false,
        mode: "import",
        current: rows.length,
        total: rows.length,
        label: "",
      });
      setImporting(false);

      if (failureCount === 0) {
        showSuccess(`Import finished. ${successCount} contract(s) published.`);
      } else {
        showWarning(
          `Import finished with ${failureCount} failed row(s). Retry is available in history.`,
        );
      }
    },
    [addHistoryItem, showInfo, showSuccess, showWarning],
  );

  const startPreviewImport = () => {
    runImport(importPreviewRows, {
      fileName: importFileName,
      format: importFormat,
      reason: `Importing ${importFileName || "uploaded data"}`,
    });
  };

  const retryFailedImport = (entry: HistoryItem) => {
    if (importing || exporting) return;
    runImport(entry.failedRows ?? [], {
      fileName: entry.fileName,
      format: entry.format as ImportFormat,
      reason: `Retrying failed rows from ${entry.fileName || "history entry"}`,
    });
  };

  const startExport = async () => {
    if (importing || exporting) return;

    const confirmed = window.confirm(
      "Confirm export. This will download a file to your device.",
    );
    if (!confirmed) return;

    try {
      setExporting(true);
      showInfo("Export started. Fetching contracts...");

      const firstPage = await api.getContracts({
        query: exportFilters.query || undefined,
        network: exportFilters.network || undefined,
        category: exportFilters.category || undefined,
        verified_only: exportFilters.verifiedOnly || undefined,
        page: 1,
        page_size: 100,
      });

      const allItems = [...firstPage.items];
      const totalPages = Math.max(1, firstPage.total_pages);

      setProgress({
        active: true,
        mode: "export",
        current: 1,
        total: totalPages,
        label: "Fetching export pages",
      });

      for (let page = 2; page <= totalPages; page += 1) {
        const next = await api.getContracts({
          query: exportFilters.query || undefined,
          network: exportFilters.network || undefined,
          category: exportFilters.category || undefined,
          verified_only: exportFilters.verifiedOnly || undefined,
          page,
          page_size: 100,
        });
        allItems.push(...next.items);
        setProgress((current: ProgressState) => ({
          ...current,
          current: page,
        }));
      }

      const exportRows = buildExportRows(allItems);
      const timestamp = new Date()
        .toISOString()
        .slice(0, 19)
        .replaceAll(":", "-");

      if (exportFormat === "json") {
        const content = JSON.stringify(exportRows, null, 2);
        downloadTextFile(
          `contracts-export-${timestamp}.json`,
          content,
          "application/json;charset=utf-8",
        );
      } else {
        const headers = Object.keys(
          exportRows[0] ?? {
            id: "",
            contract_id: "",
            wasm_hash: "",
            name: "",
            description: "",
            publisher_id: "",
            network: "",
            is_verified: "",
            category: "",
            tags: "",
            popularity_score: "",
            downloads: "",
            average_rating: "",
            review_count: "",
            deployment_count: "",
            interaction_count: "",
            relevance_score: "",
            logo_url: "",
            created_at: "",
            updated_at: "",
            verified_at: "",
            last_accessed_at: "",
            is_maintenance: "",
            logical_id: "",
            network_configs: "",
          },
        );
        const csvRows = [
          headers.map(escapeCsvCell).join(","),
          ...exportRows.map((row) =>
            headers
              .map((header) =>
                escapeCsvCell(String(row[header as keyof typeof row] ?? "")),
              )
              .join(","),
          ),
        ];
        downloadTextFile(
          `contracts-export-${timestamp}.csv`,
          csvRows.join("\n"),
          "text/csv;charset=utf-8",
        );
      }

      addHistoryItem({
        id: `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
        kind: "export",
        status: "success",
        createdAt: new Date().toISOString(),
        format: exportFormat,
        summary: `Exported ${allItems.length} contract(s).`,
        total: allItems.length,
        successCount: allItems.length,
        failureCount: 0,
        filters: exportFilters,
      });

      setProgress({
        active: false,
        mode: "export",
        current: totalPages,
        total: totalPages,
        label: "",
      });
      setExporting(false);

      showSuccess(
        `Export complete. Downloaded ${allItems.length} contract(s).`,
      );
    } catch (error) {
      addHistoryItem({
        id: `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
        kind: "export",
        status: "failed",
        createdAt: new Date().toISOString(),
        format: exportFormat,
        summary: "Export failed.",
        total: 0,
        successCount: 0,
        failureCount: 1,
        filters: exportFilters,
        errors: [{ message: toErrorMessage(error) }],
      });

      setProgress((current: ProgressState) => ({
        ...current,
        active: false,
        label: "",
      }));
      setExporting(false);
      showError(`Export failed: ${toErrorMessage(error)}`);
    }
  };

  return (
    <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 sm:py-10">
      <div className="flex items-center justify-between gap-4 mb-6">
        <div>
          <h1 className="text-2xl sm:text-3xl font-bold text-foreground">
            Contract Import and Export
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            Bulk import contracts from JSON/CSV or export filtered contract
            datasets.
          </p>
        </div>
        <Link
          href="/contracts"
          className="inline-flex items-center gap-2 px-3 py-2 rounded-lg border border-border text-sm text-foreground hover:bg-accent transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          Back to contracts
        </Link>
      </div>

      {progress.active && (
        <section
          className="mb-6 rounded-xl border border-border bg-card p-4"
          aria-live="polite"
        >
          <div className="flex items-center justify-between text-sm mb-2">
            <div className="flex items-center gap-2 text-foreground">
              <Clock3 className="w-4 h-4 text-primary" />
              <span>{progress.label}</span>
            </div>
            <span className="text-muted-foreground">
              {progress.current}/{progress.total} ({progressPercent}%)
            </span>
          </div>
          <div className="w-full h-2 bg-muted rounded-full overflow-hidden">
            <div
              className="h-full bg-primary transition-all duration-300"
              style={{ width: `${progressPercent}%` }}
            />
          </div>
        </section>
      )}

      <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
        <section className="rounded-2xl border border-border bg-card p-5 sm:p-6">
          <div className="flex items-start justify-between gap-4 mb-5">
            <div>
              <h2 className="text-lg font-semibold text-foreground flex items-center gap-2">
                <UploadCloud className="w-5 h-5 text-primary" />
                Import Contracts
              </h2>
              <p className="text-sm text-muted-foreground mt-1">
                Upload a JSON or CSV file, verify preview rows, then confirm
                import.
              </p>
            </div>
          </div>

          <label
            className="block text-sm font-medium text-foreground mb-2"
            htmlFor="contract-import-file"
          >
            Source file
          </label>
          <input
            id="contract-import-file"
            type="file"
            accept=".json,.csv,application/json,text/csv"
            onChange={onSelectImportFile}
            className="block w-full text-sm file:mr-4 file:rounded-md file:border-0 file:bg-primary file:px-3 file:py-2 file:text-primary-foreground file:cursor-pointer border border-border rounded-lg p-2"
          />
          <p className="text-xs text-muted-foreground mt-2">
            Maximum file size: 15MB. Required fields: contract_id, name,
            network, publisher_address, tags.
          </p>

          {importFileName && (
            <div className="mt-4 rounded-lg border border-border bg-background p-3">
              <p className="text-sm text-foreground">
                <strong>Loaded file:</strong> {importFileName} (
                {importFormat.toUpperCase()})
              </p>
              <p className="text-sm text-muted-foreground mt-1">
                Valid rows: {importPreviewRows.length} | Validation issues:{" "}
                {importErrors.length}
              </p>
            </div>
          )}

          {importPreviewRows.length > 0 && (
            <div className="mt-5">
              <h3 className="text-sm font-semibold text-foreground mb-2">
                Preview (first {PREVIEW_ROWS_LIMIT})
              </h3>
              <div className="overflow-x-auto rounded-lg border border-border">
                <table className="w-full text-sm">
                  <thead className="bg-muted/40 text-left text-muted-foreground">
                    <tr>
                      <th className="px-3 py-2">Row</th>
                      <th className="px-3 py-2">Contract ID</th>
                      <th className="px-3 py-2">Name</th>
                      <th className="px-3 py-2">Network</th>
                      <th className="px-3 py-2">Publisher</th>
                    </tr>
                  </thead>
                  <tbody>
                    {importPreviewRows
                      .slice(0, PREVIEW_ROWS_LIMIT)
                      .map((row) => (
                        <tr
                          key={`${row.contract_id}-${row.rowNumber}`}
                          className="border-t border-border"
                        >
                          <td className="px-3 py-2">{row.rowNumber}</td>
                          <td className="px-3 py-2 font-mono text-xs">
                            {row.contract_id}
                          </td>
                          <td className="px-3 py-2">{row.name}</td>
                          <td className="px-3 py-2">{row.network}</td>
                          <td className="px-3 py-2 font-mono text-xs">
                            {row.publisher_address}
                          </td>
                        </tr>
                      ))}
                  </tbody>
                </table>
              </div>
              {importPreviewRows.length > PREVIEW_ROWS_LIMIT && (
                <p className="text-xs text-muted-foreground mt-2">
                  Showing first {PREVIEW_ROWS_LIMIT} of{" "}
                  {importPreviewRows.length} row(s).
                </p>
              )}
            </div>
          )}

          {importErrors.length > 0 && (
            <div className="mt-5 rounded-lg border border-amber-300 bg-amber-50/70 dark:bg-amber-900/20 p-3">
              <h3 className="text-sm font-semibold text-amber-800 dark:text-amber-300 flex items-center gap-2">
                <AlertTriangle className="w-4 h-4" />
                Validation errors ({importErrors.length})
              </h3>
              <ul className="mt-2 text-xs sm:text-sm text-amber-900 dark:text-amber-200 space-y-1 max-h-32 overflow-auto">
                {importErrors.slice(0, 20).map((error, index) => (
                  <li key={`${error.rowNumber}-${error.field}-${index}`}>
                    Row {error.rowNumber || "-"} [{error.field}]:{" "}
                    {error.message}
                  </li>
                ))}
              </ul>
              {importErrors.length > 20 && (
                <p className="text-xs text-amber-900 dark:text-amber-200 mt-2">
                  Showing first 20 validation errors.
                </p>
              )}
            </div>
          )}

          <button
            type="button"
            onClick={startPreviewImport}
            disabled={importing || exporting || importPreviewRows.length === 0}
            className="mt-6 inline-flex items-center gap-2 px-4 py-2.5 rounded-lg bg-primary text-primary-foreground text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed hover:opacity-90 transition-opacity"
          >
            {importing ? (
              <RefreshCw className="w-4 h-4 animate-spin" />
            ) : (
              <FileUp className="w-4 h-4" />
            )}
            {importing ? "Importing..." : "Confirm and import"}
          </button>
        </section>

        <section className="rounded-2xl border border-border bg-card p-5 sm:p-6">
          <h2 className="text-lg font-semibold text-foreground flex items-center gap-2">
            <Download className="w-5 h-5 text-primary" />
            Export Contracts
          </h2>
          <p className="text-sm text-muted-foreground mt-1 mb-5">
            Select format and filters, then confirm export to download your
            dataset.
          </p>

          <div className="space-y-4">
            <div>
              <label
                className="block text-sm font-medium text-foreground mb-1"
                htmlFor="export-format"
              >
                Format
              </label>
              <select
                id="export-format"
                value={exportFormat}
                onChange={(event) =>
                  setExportFormat(event.target.value as ExportFormat)
                }
                className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
              >
                <option value="json">JSON</option>
                <option value="csv">CSV</option>
              </select>
            </div>

            <div>
              <label
                className="block text-sm font-medium text-foreground mb-1"
                htmlFor="export-query"
              >
                Search query
              </label>
              <input
                id="export-query"
                type="text"
                value={exportFilters.query}
                onChange={(event) =>
                  setExportFilters((current) => ({
                    ...current,
                    query: event.target.value,
                  }))
                }
                placeholder="token, defi, bridge..."
                className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
              />
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label
                  className="block text-sm font-medium text-foreground mb-1"
                  htmlFor="export-network"
                >
                  Network
                </label>
                <select
                  id="export-network"
                  value={exportFilters.network}
                  onChange={(event) =>
                    setExportFilters((current) => ({
                      ...current,
                      network: event.target.value as "" | Network,
                    }))
                  }
                  className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
                >
                  <option value="">All networks</option>
                  <option value="mainnet">Mainnet</option>
                  <option value="testnet">Testnet</option>
                  <option value="futurenet">Futurenet</option>
                </select>
              </div>

              <div>
                <label
                  className="block text-sm font-medium text-foreground mb-1"
                  htmlFor="export-category"
                >
                  Category
                </label>
                <input
                  id="export-category"
                  type="text"
                  value={exportFilters.category}
                  onChange={(event) =>
                    setExportFilters((current) => ({
                      ...current,
                      category: event.target.value,
                    }))
                  }
                  placeholder="DeFi, NFT..."
                  className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
                />
              </div>
            </div>

            <label className="inline-flex items-center gap-2 text-sm text-foreground">
              <input
                type="checkbox"
                checked={exportFilters.verifiedOnly}
                onChange={(event) =>
                  setExportFilters((current) => ({
                    ...current,
                    verifiedOnly: event.target.checked,
                  }))
                }
                className="rounded border-border"
              />
              Verified only
            </label>

            <button
              type="button"
              onClick={startExport}
              disabled={importing || exporting}
              className="inline-flex items-center gap-2 px-4 py-2.5 rounded-lg bg-primary text-primary-foreground text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed hover:opacity-90 transition-opacity"
            >
              {exporting ? (
                <RefreshCw className="w-4 h-4 animate-spin" />
              ) : (
                <Download className="w-4 h-4" />
              )}
              {exporting ? "Exporting..." : "Confirm and export"}
            </button>
          </div>
        </section>
      </div>

      <section className="mt-8 rounded-2xl border border-border bg-card p-5 sm:p-6">
        <h2 className="text-lg font-semibold text-foreground mb-4">
          Import and export history
        </h2>
        {history.length === 0 ? (
          <p className="text-sm text-muted-foreground">No history yet.</p>
        ) : (
          <div className="space-y-3">
            {history.map((entry) => (
              <article
                key={entry.id}
                className="rounded-lg border border-border bg-background p-4"
              >
                <div className="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3">
                  <div>
                    <p className="text-sm font-semibold text-foreground flex items-center gap-2">
                      {entry.status === "success" ? (
                        <CheckCircle2 className="w-4 h-4 text-green-500" />
                      ) : (
                        <AlertTriangle className="w-4 h-4 text-amber-500" />
                      )}
                      {entry.kind.toUpperCase()} · {entry.format.toUpperCase()}{" "}
                      · {entry.status}
                    </p>
                    <p className="text-sm text-muted-foreground mt-1">
                      {entry.summary}
                    </p>
                    <p className="text-xs text-muted-foreground mt-1">
                      {formatDateTime(entry.createdAt)}
                    </p>
                    {entry.fileName && (
                      <p className="text-xs text-muted-foreground mt-1">
                        File: {entry.fileName}
                      </p>
                    )}
                  </div>

                  <div className="text-xs text-muted-foreground">
                    <p>Total: {entry.total}</p>
                    <p>Success: {entry.successCount}</p>
                    <p>Failed: {entry.failureCount}</p>
                  </div>
                </div>

                {entry.errors && entry.errors.length > 0 && (
                  <ul className="mt-3 text-xs text-rose-700 dark:text-rose-300 space-y-1 max-h-28 overflow-auto">
                    {entry.errors.slice(0, 8).map((error, index) => (
                      <li key={`${entry.id}-error-${index}`}>
                        {error.rowNumber ? `Row ${error.rowNumber}: ` : ""}
                        {error.message}
                      </li>
                    ))}
                  </ul>
                )}

                {entry.kind === "import" &&
                  entry.failureCount > 0 &&
                  (entry.failedRows?.length ?? 0) > 0 && (
                    <button
                      type="button"
                      onClick={() => retryFailedImport(entry)}
                      disabled={importing || exporting}
                      className="mt-3 inline-flex items-center gap-2 px-3 py-2 rounded-md border border-border text-sm text-foreground hover:bg-accent disabled:opacity-50"
                    >
                      <RefreshCw className="w-4 h-4" />
                      Retry failed imports ({entry.failedRows?.length ?? 0})
                    </button>
                  )}
              </article>
            ))}
          </div>
        )}
      </section>
    </main>
  );
}
