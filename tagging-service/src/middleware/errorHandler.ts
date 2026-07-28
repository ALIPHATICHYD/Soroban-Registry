import { NextFunction, Request, Response } from "express";
import { AppError, ErrorCategory, ErrorCode, isAppError, toErrorResponse } from "../errors.js";
import { logger } from "../logger.js";

function getClientIp(req: Request): string {
  const forwarded = req.headers["x-forwarded-for"];
  if (typeof forwarded === "string" && forwarded.length > 0) {
    return forwarded.split(",")[0].trim();
  }
  return req.ip || req.socket.remoteAddress || "unknown";
}

function getErrorCategory(error: AppError): string {
  return error.category;
}

export function errorHandler(
  err: Error,
  req: Request,
  res: Response,
  _next: NextFunction,
): void {
  const requestId =
    (req.headers["x-request-id"] as string) ||
    (req.headers["x-correlation-id"] as string) ||
    "";

  if (isAppError(err)) {
    logger.error(
      {
        errorCode: err.errorCode,
        category: getErrorCategory(err),
        statusCode: err.statusCode,
        message: err.message,
        requestId,
        method: req.method,
        path: req.path,
        clientIp: getClientIp(req),
        details: err.details,
        cause: err.cause instanceof Error ? { message: err.cause.message, stack: err.cause.stack } : undefined,
      },
      `[${err.errorCode}] ${err.message}`,
    );

    const response = toErrorResponse(err);
    res.status(err.statusCode).json({
      ...response,
      request_id: requestId,
    });
    return;
  }

  const isServerError = res.statusCode >= 500 || !res.statusCode;
  const statusCode = isServerError ? 500 : res.statusCode || 500;

  logger.error(
    {
      error: err.message,
      stack: err.stack,
      requestId,
      method: req.method,
      path: req.path,
      clientIp: getClientIp(req),
    },
    `Unhandled error: ${err.message}`,
  );

  res.status(statusCode).json({
    code: ErrorCode.InternalError,
    error_code: ErrorCode.InternalError,
    message: isServerError ? "Internal server error" : err.message,
    request_id: requestId,
    timestamp: new Date().toISOString(),
  });
}

export function notFoundHandler(req: Request, res: Response, _next: NextFunction): void {
  const requestId =
    (req.headers["x-request-id"] as string) ||
    (req.headers["x-correlation-id"] as string) ||
    "";

  logger.warn(
    {
      method: req.method,
      path: req.path,
      requestId,
      clientIp: getClientIp(req),
    },
    `Route not found: ${req.method} ${req.path}`,
  );

  res.status(404).json({
    code: ErrorCode.NotFound,
    error_code: ErrorCode.NotFound,
    message: `Route not found: ${req.method} ${req.path}`,
    request_id: requestId,
    timestamp: new Date().toISOString(),
  });
}

export function asyncHandler(
  fn: (req: Request, res: Response, next: NextFunction) => Promise<void>,
) {
  return (req: Request, res: Response, next: NextFunction): void => {
    Promise.resolve(fn(req, res, next)).catch(next);
  };
}
