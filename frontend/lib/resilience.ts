// Simple circuit breaker + exponential backoff utility for frontend API calls
import { logError } from "./errors";

type BreakerState = "closed" | "open" | "half-open";

interface CircuitOptions {
  failureThreshold?: number; // failures before opening
  cooldownPeriodMs?: number; // how long to stay open
  halfOpenMaxTrial?: number; // allowed trial requests in half-open
  maxRetries?: number;
  retryBaseMs?: number;
}

const DEFAULTS: Required<CircuitOptions> = {
  failureThreshold: 5,
  cooldownPeriodMs: 30000, // 30s
  halfOpenMaxTrial: 1,
  maxRetries: 2,
  retryBaseMs: 300,
};

class CircuitBreaker {
  private failures = 0;
  private state: BreakerState = "closed";
  private openedAt: number | null = null;
  private trials = 0;
  private opts: Required<CircuitOptions>;

  constructor(opts?: CircuitOptions) {
    this.opts = { ...DEFAULTS, ...(opts || {}) };
  }

  public recordSuccess() {
    this.failures = 0;
    this.state = "closed";
    this.openedAt = null;
    this.trials = 0;
    try {
      if (typeof window !== "undefined" && process.env.NEXT_PUBLIC_API_URL) {
        void fetch(
          `${process.env.NEXT_PUBLIC_API_URL.replace(/\/$/, "")}/api/observability/client_breaker`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              endpoint: "",
              state: "closed",
              failures: this.failures,
              opened_at: null,
            }),
            keepalive: true,
          },
        ).catch(() => {});
      }
    } catch {}
  }

  public reset() {
    this.failures = 0;
    this.state = "closed";
    this.openedAt = null;
    this.trials = 0;
  }

  public recordFailure() {
    this.failures += 1;
    if (this.failures >= this.opts.failureThreshold) {
      this.open();
    }
  }

  private open() {
    this.state = "open";
    this.openedAt = Date.now();
    this.trials = 0;
    try {
      logError(new Error("Circuit opened"), { endpoint: undefined });
    } catch {}
    try {
      if (typeof window !== "undefined" && process.env.NEXT_PUBLIC_API_URL) {
        void fetch(
          `${process.env.NEXT_PUBLIC_API_URL.replace(/\/$/, "")}/api/observability/client_breaker`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              endpoint: "",
              state: "open",
              failures: this.failures,
              opened_at: this.openedAt,
            }),
            keepalive: true,
          },
        ).catch(() => {});
      }
    } catch {}
  }

  private tryHalfOpen() {
    this.state = "half-open";
    this.trials = 0;
  }

  public isClosed() {
    if (this.state === "open" && this.openedAt) {
      const since = Date.now() - this.openedAt;
      if (since > this.opts.cooldownPeriodMs) {
        this.tryHalfOpen();
        return true;
      }
      return false;
    }
    return true;
  }

  public allowRequest(): boolean {
    if (this.state === "closed") return true;
    if (this.state === "open") {
      if (
        this.openedAt &&
        Date.now() - this.openedAt > this.opts.cooldownPeriodMs
      ) {
        this.tryHalfOpen();
        return true;
      }
      return false;
    }
    // half-open
    if (this.trials < this.opts.halfOpenMaxTrial) {
      this.trials += 1;
      return true;
    }
    return false;
  }

  public shouldOpen() {
    return this.failures >= this.opts.failureThreshold;
  }

  public getState() {
    return this.state;
  }

  public getDebugState() {
    return {
      state: this.state,
      failures: this.failures,
      openedAt: this.openedAt,
      trials: this.trials,
    };
  }
}

// Keep breakers per-endpoint in-memory
const breakers = new Map<string, CircuitBreaker>();

export function getBreaker(key: string, opts?: CircuitOptions) {
  if (!breakers.has(key)) {
    breakers.set(key, new CircuitBreaker(opts));
  }
  return breakers.get(key)!;
}

export function getAllBreakerStates() {
  const out: Record<
    string,
    {
      state: BreakerState;
      failures: number;
      openedAt: number | null;
      trials: number;
    }
  > = {};
  for (const [k, b] of breakers.entries()) {
    out[k] = b.getDebugState();
  }
  return out;
}

export function sleep(ms: number) {
  return new Promise((res) => setTimeout(res, ms));
}

export function backoffDelay(base: number, attempt: number) {
  const jitter = Math.round(Math.random() * base);
  return base * Math.pow(2, attempt) + jitter;
}

export async function resilientCall<T>(
  key: string,
  fn: () => Promise<T>,
  opts?: CircuitOptions & { endpoint?: string },
): Promise<T> {
  const breaker = getBreaker(key, opts);
  const maxRetries = opts?.maxRetries ?? DEFAULTS.maxRetries;
  const retryBase = opts?.retryBaseMs ?? DEFAULTS.retryBaseMs;

  if (!breaker.allowRequest()) {
    const err = new Error("Service temporarily unavailable (circuit open)");
    err.name = "CircuitOpenError";
    throw err;
  }

  let lastErr: unknown = null;
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      const res = await fn();
      breaker.recordSuccess();
      try {
        const endpointLabel = opts?.endpoint ?? key;
        if (typeof window !== "undefined" && process.env.NEXT_PUBLIC_API_URL) {
          void fetch(
            `${process.env.NEXT_PUBLIC_API_URL.replace(/\/$/, "")}/api/observability/client_breaker`,
            {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                endpoint: endpointLabel,
                state: "closed",
                failures: breaker.getDebugState().failures,
                opened_at: breaker.getDebugState().openedAt,
              }),
              keepalive: true,
            },
          ).catch(() => {});
        }
      } catch {}
      return res;
    } catch (e) {
      lastErr = e;
      breaker.recordFailure();
      try {
        logError(e as Error, { endpoint: opts?.endpoint ?? key, attempt });
      } catch {}
      try {
        const endpointLabel = opts?.endpoint ?? key;
        if (
          !breaker.allowRequest() &&
          typeof window !== "undefined" &&
          process.env.NEXT_PUBLIC_API_URL
        ) {
          void fetch(
            `${process.env.NEXT_PUBLIC_API_URL.replace(/\/$/, "")}/api/observability/client_breaker`,
            {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                endpoint: endpointLabel,
                state: "open",
                failures: breaker.getDebugState().failures,
                opened_at: breaker.getDebugState().openedAt,
              }),
              keepalive: true,
            },
          ).catch(() => {});
        }
      } catch {}
      // If we've opened the circuit, stop immediately
      if (!breaker.allowRequest()) break;

      if (attempt < maxRetries) {
        const delay = backoffDelay(retryBase, attempt);
        await sleep(delay);
        continue;
      }
    }
  }

  throw lastErr;
}
