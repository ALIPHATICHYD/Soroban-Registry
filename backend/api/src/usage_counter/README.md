# Contract Usage Counter

## Overview

The contract usage counter tracks how many times contracts are accessed via the API. This feature provides valuable analytics for contract popularity and usage patterns.

## Features

- **Atomic Increments**: Thread-safe counter updates using SQL `UPDATE` with `usage_count = usage_count + 1`
- **Timeout Protection**: 10ms timeout ensures counter updates don't block API requests
- **Retry Logic**: Exponential backoff retry for transient database failures
- **Fire-and-Forget**: Counter updates run asynchronously, never blocking main requests
- **Statistics Endpoint**: `/api/contracts/:id/stats` endpoint to retrieve usage statistics

## Database Schema

```sql
-- Added to contracts table
ALTER TABLE contracts
    ADD COLUMN usage_count BIGINT NOT NULL DEFAULT 0;

-- Constraint ensures non-negative values
ALTER TABLE contracts
    ADD CONSTRAINT chk_contracts_usage_count_non_negative
    CHECK (usage_count >= 0);

-- Index for efficient statistics queries
CREATE INDEX idx_contracts_usage_count ON contracts(usage_count DESC);
```

## API Integration

The usage counter is automatically incremented in the following handlers:

1. **`GET /api/contracts/:id`** - Viewing a contract
2. **`POST /api/contracts/:id/versions`** - Creating a new version
3. **`POST /api/contracts/:id/publish`** - Publishing a contract
4. **`PUT /api/contracts/:id/metadata`** - Updating contract metadata
5. **`PUT /api/contracts/:id/status`** - Updating contract status

## Usage Statistics Endpoint

```
GET /api/contracts/:id/stats
```

**Response:**

```json
{
  "contract_id": "uuid",
  "usage_count": 42,
  "last_accessed_at": "2023-01-01T00:00:00Z"
}
```

## Configuration

Default configuration values:

- **Timeout**: 10ms for individual operations, 100ms for retry operations
- **Max Retries**: 3 attempts with exponential backoff
- **Base Delay**: 10ms for retry backoff

## Error Handling

- Counter failures are logged but never propagated to API responses
- Timeouts are logged as warnings
- Database errors are logged as errors with full context

## Testing

- **Unit Tests**: `usage_counter.rs` contains comprehensive unit tests
- **Migration Tests**: `usage_count_migration_tests.rs` validates SQL migration
- **Integration Tests**: `usage_counter_integration_tests.rs` (requires test database)

## Performance Considerations

1. **Index Usage**: The `idx_contracts_usage_count` index optimizes statistics queries
2. **Atomic Operations**: SQL `UPDATE` ensures consistency under concurrent access
3. **Async Execution**: Counter updates run in background threads
4. **Timeout Protection**: Prevents database issues from affecting API performance

## Monitoring

Monitor the following logs:

- `"usage counter incremented"` - Successful increments
- `"usage counter update timed out"` - Timeout warnings
- `"Failed to increment usage counter"` - Error conditions

## Backward Compatibility

- The `usage_count` field has `#[serde(default)]` annotation
- Existing API responses without `usage_count` will deserialize with `usage_count: 0`
- No breaking changes to existing endpoints
