"use client";

import React, { useId, useMemo } from "react";
import { FileText, Trash2, UploadCloud } from "lucide-react";
import { formatBytes } from "@/utils/fileValidation";

function fileKey(file: File): string {
  return `${file.name}::${file.size}::${file.lastModified}`;
}

export default function FileUploader(props: {
  files: File[];
  progress: Record<string, number>;
  errors?: string[];
  totalBytes: number;
  maxBytes: number;
  onAddFiles: (files: File[]) => void;
  onRemoveFile: (key: string) => void;
}) {
  const {
    files,
    progress,
    errors,
    totalBytes,
    maxBytes,
    onAddFiles,
    onRemoveFile,
  } = props;
  const inputId = useId();

  const accept = useMemo(() => [".pdf", ".txt", ".doc", ".docx"].join(","), []);

  return (
    <div className="space-y-3">
      <div className="rounded-2xl border border-border bg-card p-4">
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
          <div>
            <div className="flex items-center gap-2">
              <UploadCloud className="w-4 h-4 text-primary" />
              <p className="font-semibold text-foreground">
                Upload supporting documents
              </p>
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              Accepted: PDF, DOC, TXT · Total limit: {formatBytes(maxBytes)} ·
              Current: {formatBytes(totalBytes)}
            </p>
          </div>

          <div className="flex items-center gap-2">
            <input
              id={inputId}
              type="file"
              multiple
              accept={accept}
              className="hidden"
              onChange={(e) => {
                const next = Array.from(e.target.files || []);
                if (next.length > 0) onAddFiles(next);
                e.currentTarget.value = "";
              }}
            />
            <label
              htmlFor={inputId}
              className="inline-flex items-center justify-center px-4 py-2 rounded-lg bg-primary text-primary-foreground font-medium btn-glow cursor-pointer"
            >
              Choose files
            </label>
          </div>
        </div>

        {errors && errors.length > 0 && (
          <div className="mt-3 rounded-lg border border-red-500/20 bg-red-500/10 p-3">
            <ul className="text-sm text-red-600 space-y-1">
              {errors.map((err, idx) => (
                <li key={`${idx}-${err}`}>{err}</li>
              ))}
            </ul>
          </div>
        )}

        {files.length === 0 ? (
          <div className="mt-4 border border-dashed border-border rounded-xl p-6 text-center">
            <p className="text-sm text-muted-foreground">
              No documents uploaded yet.
            </p>
          </div>
        ) : (
          <div className="mt-4 space-y-2">
            {files.map((file) => {
              const key = fileKey(file);
              const pct = progress[key] ?? 0;

              return (
                <div
                  key={key}
                  className="flex items-start gap-3 rounded-xl border border-border bg-background p-3"
                >
                  <div className="w-9 h-9 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0">
                    <FileText className="w-4 h-4 text-primary" />
                  </div>

                  <div className="flex-1 min-w-0">
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0">
                        <p className="text-sm font-medium text-foreground truncate">
                          {file.name}
                        </p>
                        <p className="text-xs text-muted-foreground">
                          {formatBytes(file.size)}
                        </p>
                      </div>
                      <button
                        type="button"
                        onClick={() => onRemoveFile(key)}
                        className="p-2 rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                        aria-label={`Remove ${file.name}`}
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>

                    <div className="mt-2">
                      <div className="h-2 rounded-full bg-border overflow-hidden">
                        <div
                          className="h-full bg-primary transition-[width] duration-300"
                          style={{ width: `${pct}%` }}
                        />
                      </div>
                      <div className="mt-1 text-[11px] text-muted-foreground">
                        {pct >= 100 ? "Uploaded" : `Uploading… ${pct}%`}
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
