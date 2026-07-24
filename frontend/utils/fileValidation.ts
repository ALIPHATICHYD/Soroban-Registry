const MB = 1024 * 1024;

export const MAX_TOTAL_UPLOAD_BYTES = 100 * MB;

const ALLOWED_MIME_TYPES = new Set<string>([
  "application/pdf",
  "text/plain",
  "application/msword",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
]);

const ALLOWED_EXTENSIONS = new Set<string>(["pdf", "txt", "doc", "docx"]);

export type FileValidationErrorCode =
  | "type_not_allowed"
  | "total_size_exceeded"
  | "duplicate";

export type FileValidationError = {
  code: FileValidationErrorCode;
  message: string;
  fileName?: string;
};

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"] as const;
  const exp = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / Math.pow(1024, exp);
  return `${value.toFixed(value >= 10 || exp === 0 ? 0 : 1)} ${units[exp]}`;
}

function getExtension(fileName: string): string {
  const parts = fileName.toLowerCase().split(".");
  return parts.length > 1 ? parts[parts.length - 1] : "";
}

function fileFingerprint(file: File): string {
  return `${file.name}::${file.size}::${file.lastModified}`;
}

export function validateFilesToAdd(params: {
  existingFiles: File[];
  newFiles: File[];
  maxTotalBytes?: number;
}): { accepted: File[]; errors: FileValidationError[] } {
  const {
    existingFiles,
    newFiles,
    maxTotalBytes = MAX_TOTAL_UPLOAD_BYTES,
  } = params;

  const errors: FileValidationError[] = [];
  const accepted: File[] = [];

  const existingFingerprints = new Set(existingFiles.map(fileFingerprint));
  const startingTotal = existingFiles.reduce((sum, f) => sum + f.size, 0);
  let runningTotal = startingTotal;

  for (const file of newFiles) {
    const fingerprint = fileFingerprint(file);
    if (existingFingerprints.has(fingerprint)) {
      errors.push({
        code: "duplicate",
        message: `Duplicate file skipped: ${file.name}`,
        fileName: file.name,
      });
      continue;
    }

    const ext = getExtension(file.name);
    const isAllowedByMime = !!file.type && ALLOWED_MIME_TYPES.has(file.type);
    const isAllowedByExt = !!ext && ALLOWED_EXTENSIONS.has(ext);
    if (!isAllowedByMime && !isAllowedByExt) {
      errors.push({
        code: "type_not_allowed",
        message: `Unsupported file type: ${file.name}. Allowed: PDF, DOC, TXT`,
        fileName: file.name,
      });
      continue;
    }

    if (runningTotal + file.size > maxTotalBytes) {
      errors.push({
        code: "total_size_exceeded",
        message: `Total upload size exceeds ${formatBytes(maxTotalBytes)} (current: ${formatBytes(runningTotal)})`,
      });
      break;
    }

    accepted.push(file);
    existingFingerprints.add(fingerprint);
    runningTotal += file.size;
  }

  return { accepted, errors };
}
