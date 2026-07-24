import { Pool, QueryConfig, QueryResult, QueryResultRow } from "pg";
import { logger } from "./logger.js";

// Issue #876: Connection pool configuration via environment variables
const MIN_POOL_SIZE = parseInt(process.env.DB_MIN_POOL_SIZE ?? "2", 10);
const MAX_POOL_SIZE = Math.min(
  parseInt(process.env.DB_MAX_POOL_SIZE ?? "50", 10),
  50
);
const IDLE_TIMEOUT_MS = parseInt(process.env.DB_IDLE_TIMEOUT_MS ?? "600000", 10); // 10 min
const CONNECTION_TIMEOUT_MS = parseInt(process.env.DB_ACQUIRE_TIMEOUT_MS ?? "30000", 10);
const QUERY_TIMEOUT_MS = parseInt(process.env.DB_QUERY_TIMEOUT_MS ?? "30000", 10);
const SLOW_QUERY_THRESHOLD_MS = parseInt(process.env.DB_SLOW_QUERY_THRESHOLD_MS ?? "100", 10);

/**
 * Extends pg.Pool to add slow query logging at the application layer.
 * All existing callers using `pool.query()` get instrumented automatically.
 */
class InstrumentedPool extends Pool {
  // Override query to add slow-query detection (Issue #876).
  async query<T extends QueryResultRow = QueryResultRow>(
    textOrConfig: string | QueryConfig,
    values?: unknown[]
  ): Promise<QueryResult<T>> {
    const start = Date.now();
    const result: QueryResult<T> =
      values !== undefined
        ? await super.query<T>(textOrConfig as string, values)
        : await super.query<T>(textOrConfig as QueryConfig);
    const elapsedMs = Date.now() - start;

    if (elapsedMs > SLOW_QUERY_THRESHOLD_MS) {
      const text =
        typeof textOrConfig === "string"
          ? textOrConfig
          : textOrConfig.text ?? "(prepared)";
      logger.warn(
        { elapsedMs, thresholdMs: SLOW_QUERY_THRESHOLD_MS, query: text.substring(0, 200) },
        `slow query detected: ${elapsedMs}ms`
      );
    }

    return result;
  }

  /** Returns a snapshot of the current pool utilisation. */
  getStats() {
    return {
      total: this.totalCount,
      idle: this.idleCount,
      waiting: this.waitingCount,
      max: MAX_POOL_SIZE,
      min: MIN_POOL_SIZE,
      utilizationPct:
        MAX_POOL_SIZE > 0
          ? ((this.totalCount - this.idleCount) / MAX_POOL_SIZE) * 100
          : 0,
    };
  }
}

export const pool = new InstrumentedPool({
  connectionString:
    process.env.DATABASE_URL ||
    "postgresql://postgres:postgres@localhost:5432/soroban_registry",
  min: MIN_POOL_SIZE,
  max: MAX_POOL_SIZE,
  idleTimeoutMillis: IDLE_TIMEOUT_MS,
  connectionTimeoutMillis: CONNECTION_TIMEOUT_MS,
  query_timeout: QUERY_TIMEOUT_MS,
  statement_timeout: QUERY_TIMEOUT_MS,
});

/** Convenience export for callers that want pool stats in health checks. */
export const getPoolStats = () => pool.getStats();
