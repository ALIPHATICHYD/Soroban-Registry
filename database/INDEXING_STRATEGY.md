# Database Indexing Strategy

This document describes the contract indexing strategy implemented for the Soroban Registry backend.

## Goals

- Improve contract listing and export query performance by at least 50%
- Keep index size reasonable relative to stored contract metadata
- Support common filter combinations used by the API
- Use full-text search for name and description queries
- Enable automated monitoring and periodic maintenance

## Added indexes

The following indexes were added in `database/migrations/20260530020000_issue874_indexing_strategy.sql`:

- `idx_contracts_created_at` on `contracts(created_at DESC)`
- `idx_contracts_updated_at` on `contracts(updated_at DESC)`
- `idx_contracts_verified_at` on `contracts(verified_at DESC)`
- `idx_contracts_last_accessed_at` on `contracts(last_accessed_at DESC)`
- `idx_contracts_verification_status_created_at` on `(verification_status, created_at DESC)`
- `idx_contracts_network_category_verification_created_at` on `(network, category, verification_status, created_at DESC)`
- `idx_contracts_verified_public_created_at` on `(created_at DESC)` where `verification_status = 'verified' AND visibility = 'public'`
- `idx_contracts_search_vector_fts` on `search_vector` using GIN

## Maintenance

Run the following in a low-traffic window to rebuild indexes and refresh planner statistics:

```sql
VACUUM ANALYZE contracts;
REINDEX TABLE contracts;
```

For large datasets, use:

```sql
REINDEX INDEX CONCURRENTLY idx_contracts_search_vector_fts;
VACUUM ANALYZE contracts;
```

## Monitoring

Use the existing API monitoring endpoints to track index usage and query performance:

- `GET /api/admin/db/index-stats`
- `GET /api/admin/db/performance-report`
- `GET /api/admin/db/slow-queries`

The performance report already includes recommendations for indexes with zero scans.

## Benchmark guidance

Compare before/after query plans with:

```sql
EXPLAIN ANALYZE SELECT ... FROM contracts WHERE ...;
```

Look for significant reductions in execution time and heap scans on contract filter queries.
