import cron from "node-cron";
import { pool } from "./db.js";
import { logger } from "./logger.js";

const TRENDING_SPIKE_FACTOR = 2;

async function updateTrendingAnalytics(): Promise<void> {
  const client = await pool.connect();

  try {
    await client.query("BEGIN");

    await client.query(
      `INSERT INTO tag_usage_log (tag_id, usage_count)
       SELECT id, usage_count FROM tags`,
    );

    const { rows: spikes } = await client.query<{ tag_id: string }>(
      `WITH current_snapshot AS (
        SELECT DISTINCT ON (tag_id) tag_id, usage_count
        FROM tag_usage_log
        ORDER BY tag_id, recorded_at DESC
      ),
      previous_snapshot AS (
        SELECT DISTINCT ON (tag_id) tag_id, usage_count
        FROM tag_usage_log
        WHERE recorded_at < (SELECT MIN(recorded_at) FROM current_snapshot)
        ORDER BY tag_id, recorded_at DESC
      )
      SELECT c.tag_id
      FROM current_snapshot c
      JOIN previous_snapshot p ON c.tag_id = p.tag_id
      WHERE p.usage_count > 0
        AND c.usage_count >= p.usage_count * $1`,
      [TRENDING_SPIKE_FACTOR],
    );

    await client.query(
      `UPDATE tags SET is_trending = FALSE WHERE is_trending = TRUE`,
    );

    if (spikes.length > 0) {
      const ids = spikes.map((s) => s.tag_id);
      await client.query(
        `UPDATE tags SET is_trending = TRUE WHERE id = ANY($1::uuid[])`,
        [ids],
      );
    }

    await client.query(
      `DELETE FROM tag_usage_log
       WHERE recorded_at < NOW() - INTERVAL '7 days'`,
    );

    await client.query("COMMIT");
  } catch (err) {
    await client.query("ROLLBACK");
    logger.error({ err }, "trending analytics update failed");
  } finally {
    client.release();
  }
}

export function startCronJobs(): void {
  cron.schedule("0 * * * *", () => {
    updateTrendingAnalytics().catch((err) => {
      logger.error({ err }, "cron job updateTrendingAnalytics failed");
    });
  });
}
