-- Issue #611: analytics aggregation enhancements for time-series endpoint support.

ALTER TABLE analytics_daily_aggregates
    ADD COLUMN IF NOT EXISTS update_count INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_analytics_daily_aggregates_date
    ON analytics_daily_aggregates(date);

CREATE INDEX IF NOT EXISTS idx_analytics_daily_aggregates_date_contract
    ON analytics_daily_aggregates(date, contract_id);

-- Backfill update_count for historical data from raw events.
UPDATE analytics_daily_aggregates a
SET update_count = src.update_count
FROM (
    SELECT contract_id, DATE(created_at) AS event_date, COUNT(*)::INTEGER AS update_count
    FROM analytics_events
    WHERE event_type = 'contract_updated'
    GROUP BY contract_id, DATE(created_at)
) src
WHERE a.contract_id = src.contract_id
  AND a.date = src.event_date;
