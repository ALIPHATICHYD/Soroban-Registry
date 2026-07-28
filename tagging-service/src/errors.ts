export enum ErrorCode {
  BadRequest = "BAD_REQUEST",
  Unauthorized = "UNAUTHORIZED",
  Forbidden = "FORBIDDEN",
  NotFound = "NOT_FOUND",
  Conflict = "CONFLICT",
  UnprocessableEntity = "UNPROCESSABLE_ENTITY",
  PayloadTooLarge = "PAYLOAD_TOO_LARGE",
  RateLimited = "RATE_LIMITED",
  InternalError = "INTERNAL_ERROR",
  DatabaseError = "DATABASE_ERROR",
  ServiceUnavailable = "SERVICE_UNAVAILABLE",
  ValidationError = "VALIDATION_ERROR",
}

export enum ErrorCategory {
  Validation = "validation",
  Authentication = "authentication",
  NotFound = "not_found",
  Conflict = "conflict",
  Database = "database",
  ExternalService = "external_service",
  RateLimit = "rate_limit",
  Internal = "internal",
  Network = "network",
}

export interface ErrorResponse {
  code: string;
  error_code: string;
  message: string;
  details?: Record<string, unknown>;
  request_id?: string;
  timestamp: string;
}

export class AppError extends Error {
  public readonly statusCode: number;
  public readonly errorCode: ErrorCode;
  public readonly category: ErrorCategory;
  public readonly details?: Record<string, unknown>;
  public readonly timestamp: string;
  public readonly cause?: Error;

  constructor(
    statusCode: number,
    errorCode: ErrorCode,
    category: ErrorCategory,
    message: string,
    options?: {
      details?: Record<string, unknown>;
      cause?: Error;
    },
  ) {
    super(message);
    this.name = "AppError";
    this.statusCode = statusCode;
    this.errorCode = errorCode;
    this.category = category;
    this.details = options?.details;
    this.timestamp = new Date().toISOString();
    this.cause = options?.cause;
  }

  toJSON(): ErrorResponse {
    return {
      code: this.errorCode,
      error_code: this.errorCode,
      message: this.message,
      details: this.details,
      timestamp: this.timestamp,
    };
  }
}

export class BadRequestError extends AppError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(400, ErrorCode.BadRequest, ErrorCategory.Validation, message, { details, cause });
  }
}

export class NotFoundError extends AppError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(404, ErrorCode.NotFound, ErrorCategory.NotFound, message, { details });
  }
}

export class ConflictError extends AppError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(409, ErrorCode.Conflict, ErrorCategory.Conflict, message, { details });
  }
}

export class DatabaseError extends AppError {
  constructor(message: string, cause?: Error) {
    super(500, ErrorCode.DatabaseError, ErrorCategory.Database, message, { cause });
  }
}

export class InternalError extends AppError {
  constructor(message = "Internal server error", cause?: Error) {
    super(500, ErrorCode.InternalError, ErrorCategory.Internal, message, { cause });
  }
}

export class RateLimitError extends AppError {
  constructor(message = "Rate limit exceeded") {
    super(429, ErrorCode.RateLimited, ErrorCategory.RateLimit, message);
  }
}

export class ValidationError extends AppError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(400, ErrorCode.ValidationError, ErrorCategory.Validation, message, { details });
  }
}

export class ServiceUnavailableError extends AppError {
  constructor(message = "Service temporarily unavailable", cause?: Error) {
    super(503, ErrorCode.ServiceUnavailable, ErrorCategory.ExternalService, message, { cause });
  }
}

export function isAppError(error: unknown): error is AppError {
  return error instanceof AppError;
}

export function toErrorResponse(error: unknown): ErrorResponse {
  if (isAppError(error)) {
    return error.toJSON();
  }

  if (error instanceof Error) {
    return {
      code: ErrorCode.InternalError,
      error_code: ErrorCode.InternalError,
      message: "Internal server error",
      timestamp: new Date().toISOString(),
    };
  }

  return {
    code: ErrorCode.InternalError,
    error_code: ErrorCode.InternalError,
    message: "Unknown error",
    timestamp: new Date().toISOString(),
  };
}

export function normalizeError(error: unknown, defaultMessage = "Internal server error"): AppError {
  if (isAppError(error)) {
    return error;
  }

  if (error instanceof Error) {
    return new InternalError(defaultMessage, error);
  }

  return new InternalError(defaultMessage);
}
