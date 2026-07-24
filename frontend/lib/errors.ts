/**
 * Custom error classes for API and network errors
 */

export class ApiError extends Error {
  constructor(
    message: string,
    public statusCode?: number,
    public originalError?: unknown,
    public endpoint?: string,
  ) {
    super(message);
    this.name = "ApiError";
    Object.setPrototypeOf(this, ApiError.prototype);
  }
}

export class NetworkError extends ApiError {
  constructor(message: string, endpoint?: string) {
    super(message, undefined, undefined, endpoint);
    this.name = "NetworkError";
    Object.setPrototypeOf(this, NetworkError.prototype);
  }
}

export class ValidationError extends ApiError {
  constructor(
    message: string,
    public fields?: Record<string, string[]>,
  ) {
    super(message, 400);
    this.name = "ValidationError";
    Object.setPrototypeOf(this, ValidationError.prototype);
  }
}

/**
 * Error message mapping for HTTP status codes
 */
export function getErrorMessage(
  statusCode: number,
  serverMessage?: string,
): string {
  if (serverMessage) return serverMessage;

  const messages: Record<number, string> = {
    400: "Invalid request. Please check your input.",
    401: "Authentication required. Please log in.",
    403: "You do not have permission to perform this action.",
    404: "The requested resource was not found.",
    409: "This action conflicts with existing data.",
    422: "The provided data is invalid.",
    429: "Too many requests. Please try again later.",
    500: "A server error occurred. Please try again.",
    502: "The server is temporarily unavailable.",
    503: "The service is temporarily unavailable.",
    504: "The request timed out. Please try again.",
  };

  return messages[statusCode] || "An unexpected error occurred.";
}

/**
 * Extract error data from API response
 */
export async function extractErrorData(
  response: Response,
): Promise<{ message?: string; details?: unknown }> {
  try {
    const contentType = response.headers.get("content-type");
    if (contentType?.includes("application/json")) {
      const data = await response.json();
      return {
        message: data.message || data.error || data.detail,
        details: data,
      };
    }
    const text = await response.text();
    return { message: text };
  } catch {
    return {};
  }
}

/**
 * Create appropriate error based on status code and error data
 */
export function createApiError(
  statusCode: number,
  errorData: { message?: string; details?: unknown },
  endpoint: string,
): ApiError {
  const message = getErrorMessage(statusCode, errorData.message);

  if (
    statusCode === 422 &&
    errorData.details &&
    typeof errorData.details === "object"
  ) {
    const details = errorData.details as Record<string, unknown>;
    if (details.fields) {
      return new ValidationError(
        message,
        details.fields as Record<string, string[]>,
      );
    }
  }

  return new ApiError(message, statusCode, errorData.details, endpoint);
}

/**
 * Normalize any error into a consistent structure
 */
export interface NormalizedError {
  message: string;
  statusCode?: number;
  type: "network" | "api" | "validation" | "unknown";
  category: "user" | "system" | "network" | "validation" | "unknown";
  severity: "info" | "warning" | "error" | "critical";
  endpoint?: string;
  timestamp: string;
  name?: string;
  stack?: string;
  details?: unknown;
}

export function normalizeError(
  error: unknown,
  endpoint?: string,
): NormalizedError {
  const timestamp = new Date().toISOString();

  if (error instanceof NetworkError) {
    return {
      message: error.message,
      type: "network",
      category: "network",
      severity: "error",
      endpoint: error.endpoint || endpoint,
      timestamp,
      name: error.name,
      stack: error.stack,
    };
  }

  if (error instanceof ValidationError) {
    return {
      message: error.message,
      statusCode: error.statusCode,
      type: "validation",
      category: "validation",
      severity: "warning",
      endpoint: error.endpoint || endpoint,
      timestamp,
      name: error.name,
      stack: error.stack,
      details: sanitizeForLogging(error.fields),
    };
  }

  if (error instanceof ApiError) {
    const isServerError = (error.statusCode ?? 0) >= 500;
    return {
      message: error.message,
      statusCode: error.statusCode,
      type: "api",
      category: isServerError ? "system" : "user",
      severity: isServerError ? "critical" : "warning",
      endpoint: error.endpoint || endpoint,
      timestamp,
      name: error.name,
      stack: error.stack,
      details: sanitizeForLogging(error.originalError),
    };
  }

  if (error instanceof Error) {
    return {
      message: error.message,
      type: "unknown",
      category: "unknown",
      severity: isCriticalMessage(error.message) ? "critical" : "error",
      endpoint,
      timestamp,
      name: error.name,
      stack: error.stack,
      details: sanitizeForLogging({ name: error.name }),
    };
  }

  return {
    message: "An unexpected error occurred",
    type: "unknown",
    category: "unknown",
    severity: "error",
    endpoint,
    timestamp,
    details: sanitizeForLogging(error),
  };
}

/**
 * Error logging utility
 */
export interface ErrorLogger {
  logError(error: NormalizedError): void;
}

let errorLogger: ErrorLogger | null = null;

export function setErrorLogger(logger: ErrorLogger | null) {
  errorLogger = logger;
}

export function logError(error: Error, context?: Record<string, unknown>) {
  const normalized = normalizeError(error, context?.endpoint as string);
  const safeContext = sanitizeForLogging(context ?? {});

  console.error("[Error]", {
    timestamp: new Date().toISOString(),
    message: error.message,
    name: error.name,
    stack: error.stack,
    category: normalized.category,
    severity: normalized.severity,
    ...(typeof safeContext === "object" && safeContext !== null
      ? safeContext
      : {}),
  });

  if (errorLogger) {
    errorLogger.logError(normalized);
  }

  reportError(normalized, safeContext);
  return normalized;
}

function reportError(error: NormalizedError, context: unknown) {
  if (typeof window === "undefined") return;
  if (process.env.NEXT_PUBLIC_ERROR_REPORTING === "false") return;

  const apiUrl = process.env.NEXT_PUBLIC_API_URL;
  if (!apiUrl) return;

  const payload = {
    source: "frontend",
    category: error.category,
    severity: error.severity,
    message: error.message,
    stack_trace: error.stack,
    route:
      typeof window !== "undefined" ? window.location.pathname : error.endpoint,
    request_id: extractRequestId(error.details),
    user_agent:
      typeof navigator !== "undefined" ? navigator.userAgent : undefined,
    metadata: sanitizeForLogging({
      type: error.type,
      statusCode: error.statusCode,
      endpoint: error.endpoint,
      name: error.name,
      context,
      details: error.details,
    }),
  };

  const body = JSON.stringify(payload);
  const url = `${apiUrl.replace(/\/$/, "")}/api/errors/report`;

  try {
    if (navigator.sendBeacon) {
      const blob = new Blob([body], { type: "application/json" });
      if (navigator.sendBeacon(url, blob)) return;
    }
  } catch {
    // Fall through to fetch below.
  }

  void fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
    keepalive: true,
  }).catch(() => {
    // Avoid recursive reporting if the report endpoint is unavailable.
  });
}

function extractRequestId(details: unknown): string | undefined {
  if (!details || typeof details !== "object") return undefined;
  const value = details as Record<string, unknown>;
  const direct =
    value.request_id ||
    value.requestId ||
    value.correlation_id ||
    value.correlationId;
  return typeof direct === "string" ? direct : undefined;
}

function isCriticalMessage(message: string) {
  const lower = message.toLowerCase();
  return (
    lower.includes("panic") ||
    lower.includes("invariant") ||
    lower.includes("data loss")
  );
}

function sanitizeForLogging(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map((item) => sanitizeForLogging(item));
  }
  if (!value || typeof value !== "object") {
    return value;
  }

  const output: Record<string, unknown> = {};
  for (const [key, nested] of Object.entries(
    value as Record<string, unknown>,
  )) {
    output[key] = isSensitiveKey(key)
      ? "[REDACTED]"
      : sanitizeForLogging(nested);
  }
  return output;
}

function isSensitiveKey(key: string) {
  const lower = key.toLowerCase();
  return [
    "password",
    "secret",
    "token",
    "api_key",
    "authorization",
    "cookie",
  ].some((needle) => lower.includes(needle));
}
