'use client';

import FileUploader from '@/components/verification/FileUploader';

export default function DocumentUploadStep(props: {
  files: File[];
  progress: Record<string, number>;
  errors: string[];
  totalBytes: number;
  maxBytes: number;
  onAddFiles: (files: File[]) => void;
  onRemoveFile: (key: string) => void;
}) {
  return (
    <div className="space-y-3">
      <div className="rounded-2xl border border-border bg-card p-4">
        <p className="text-sm text-muted-foreground">
          Upload audit reports, architecture docs, threat models, or any supporting materials. At least one document is required.
        </p>
      </div>

      <FileUploader
        files={props.files}
        progress={props.progress}
        errors={props.errors}
        totalBytes={props.totalBytes}
        maxBytes={props.maxBytes}
        onAddFiles={props.onAddFiles}
        onRemoveFile={props.onRemoveFile}
      />
    </div>
  );
}

