import { NextFunction, Request, Response } from "express";
import { createClient } from "redis";
import { logger } from "../logger.js";

const WINDOW_SECONDS = 60;
const REDIS_TIMEOUT_MS = 5000;
const SLOW_CALL_MS = 25;

type InMemoryBucket = number[];
const inMemoryBuckets = new Map<string, InMemoryBucket>();

function getClientIp(req: Request): string {
  const forwarded = req.headers["x-forwarded-for"];
  if (typeof forwarded === "string" && forwarded.length > 0) {
    return forwarded.split(",")[0].trim();
  }
  return req.ip || req.socket.remoteAddress || "unknown";
}

function getEndpointKey(req: Request): string {
  const endpoint = `${req.baseUrl || ""}${req.path || ""}`;
  return endpoint.replace(/[:\s]+/g, "_");
}

function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error(`Redis timeout after ${timeoutMs}ms`));
    }, timeoutMs);
    promise
      .then((value) => {
        clearTimeout(timeout);
        resolve(value);
      })
      .catch((error) => {
        clearTimeout(timeout);
        reject(error);
      });
  });
}

function inMemoryAllowRequest(key: string, maxRequests: number): boolean {
  const now = Date.now();
  const cutoff = now - WINDOW_SECONDS * 1000;
  const bucket = inMemoryBuckets.get(key) ?? [];
  const active = bucket.filter((ts) => ts >= cutoff);
  if (active.length >= maxRequests) {
    inMemoryBuckets.set(key, active);
    return false;
  }
  active.push(now);
  inMemoryBuckets.set(key, active);
  return true;
}

async function ensureConnected(client: {
  isOpen: boolean;
  connect: () => Promise<unknown>;
}): Promise<void> {
  if (!client.isOpen) {
    await client.connect();
  }
}

export interface RateLimiterOptions {
  maxRequestsPerMinute?: number;
}

/**
 * Creates a distributed sliding-window rate limiter.
 */
export function createRateLimiterMiddleware(options: RateLimiterOptions = {}) {
  const redisUrl = process.env.REDIS_URL;
  const maxRequestsPerMinute =
    options.maxRequestsPerMinute ??
    parseInt(process.env.RATE_LIMIT_MAX_REQUESTS || "120", 10);

  const redisClient = redisUrl
    ? createClient({
        url: redisUrl,
        socket: { connectTimeout: REDIS_TIMEOUT_MS },
      })
    : null;

  if (redisClient) {
    redisClient.on("error", (error) => {
      logger.warn({ error }, "[rate-limiter] redis error, using fallback");
    });
  }

  return async (req: Request, res: Response, next: NextFunction) => {
    const endpoint = getEndpointKey(req);
    const ip = getClientIp(req);
    const now = Date.now();
    const window = Math.floor(now / (WINDOW_SECONDS * 1000));
    const currentWindowKey = `ratelimit:${endpoint}:${ip}:${window}`;
    const previousWindowKey = `ratelimit:${endpoint}:${ip}:${window - 1}`;
    const fallbackKey = `${endpoint}:${ip}`;

    if (!redisClient) {
      if (!inMemoryAllowRequest(fallbackKey, maxRequestsPerMinute)) {
        res.status(429).json({ error: "rate limit exceeded" });
        return;
      }
      next();
      return;
    }

    const startedAt = Date.now();

    try {
      await ensureConnected(redisClient);
      const elapsedIntoWindowMs = now % (WINDOW_SECONDS * 1000);
      const previousWeight = 1 - elapsedIntoWindowMs / (WINDOW_SECONDS * 1000);

      const txResult = await withTimeout(
        redisClient
          .multi()
          .get(currentWindowKey)
          .get(previousWindowKey)
          .incr(currentWindowKey)
          .expire(currentWindowKey, WINDOW_SECONDS + 1)
          .exec(),
        REDIS_TIMEOUT_MS,
      );

      const currentBefore = Number.parseInt((txResult?.[0] as string) || "0", 10);
      const previousCount = Number.parseInt((txResult?.[1] as string) || "0", 10);
      const projectedCount = currentBefore + 1;
      const weightedCount = previousCount * previousWeight + projectedCount;

      const redisLatency = Date.now() - startedAt;
      if (redisLatency > SLOW_CALL_MS) {
        logger.warn({ redisLatency, thresholdMs: SLOW_CALL_MS }, "[rate-limiter] slow redis call");
      }

      if (weightedCount > maxRequestsPerMinute) {
        res.status(429).json({ error: "rate limit exceeded" });
        return;
      }

      next();
    } catch (error) {
      logger.warn({ error }, "[rate-limiter] redis timeout/failure, using fallback");
      if (!inMemoryAllowRequest(fallbackKey, maxRequestsPerMinute)) {
        res.status(429).json({ error: "rate limit exceeded" });
        return;
      }
      next();
    }
  };
}
