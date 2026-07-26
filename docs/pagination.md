# Pagination

The list and search endpoints support two pagination modes: **offset** (default,
simple) and **cursor** (stable under concurrent writes). This document is the
contract for both.

## Endpoints

| Endpoint | Offset | Cursor |
| --- | --- | --- |
| `GET /api/contracts` (list) | ✅ | ✅ |
| `GET /api/search` (full-text) | ✅ | ✅ |
| `GET /api/v1/contracts/search` (advanced) | ✅ | ✅ (served by PostgreSQL) |

## Offset pagination (default)

Pass `page`/`limit` (or `limit`/`offset`, depending on the endpoint):

```
GET /api/search?q=token&limit=20&offset=40
```

Offset pagination is simple and lets a client jump to an arbitrary page, but it
is **not stable under concurrent writes**. If contracts are inserted or removed
between two paginated requests, rows can be **skipped or duplicated**, because
`OFFSET n` is resolved against the table as it exists at the moment each page is
fetched. Example: reading page 1 (`offset=0`), then a new contract is published
that sorts onto page 1, then reading page 2 (`offset=20`) re-returns the row that
was pushed from the end of page 1 to the start of page 2.

Use offset pagination for shallow, human-driven browsing where occasional
drift is acceptable, or when you need relevance-ranked search results.

## Cursor pagination (stable)

Cursor (keyset) pagination is immune to skips and duplicates under concurrent
writes. It walks a stable ordering key — `(created_at DESC, id DESC)` — so a page
boundary always refers to the same logical position regardless of rows added or
removed elsewhere in the result set.

### How to use it

1. **Start** a cursor walk by sending an **empty** `cursor` parameter:

   ```
   GET /api/search?q=token&limit=20&cursor=
   ```

2. The response includes a `next_cursor` when more rows remain:

   ```json
   {
     "total": 137,
     "contracts": [ /* … up to `limit` items … */ ],
     "next_cursor": "eyJ0aW1lc3RhbXAiOiIyMDI2LTA3LTI0VDE2Oj..."
   }
   ```

3. **Continue** by passing that value back as `cursor`:

   ```
   GET /api/search?q=token&limit=20&cursor=eyJ0aW1lc3RhbXAiOiIyMDI2LTA3LTI0VDE2Oj...
   ```

4. **Stop** when the response has no `next_cursor` (the last page).

### Contract / guarantees

- **The cursor is opaque.** It is a URL-safe base64 token; do not parse, modify,
  or construct it. Its internal shape may change without notice.
- **Ordering is `(created_at DESC, id DESC)`.** In cursor mode this ordering is
  fixed and **overrides** any `sort_by`/relevance ordering, because keyset
  pagination requires a stable, unique key (relevance scores are neither).
- **No skips or duplicates.** Every row that existed when the walk began is
  returned exactly once, even if contracts are inserted or updated between page
  requests. Rows inserted *after* the walk passed their position are not
  back-filled into earlier pages (they will appear if you start a fresh walk).
- **`total` is the full match count** for the query/filters and is stable across
  a walk; it is not reduced by the cursor.
- **An invalid, non-empty cursor returns `400`** (`InvalidPaginationCursor` /
  `INVALID_CURSOR`). An empty cursor (`cursor=`) is valid and means "first page".
- **`limit`** is clamped to `1..=100` (default 20) and bounds each page.

### Notes per endpoint

- `GET /api/v1/contracts/search`: cursor requests are served by the PostgreSQL
  keyset path and bypass Elasticsearch (whose `from`/`size` paging is offset
  based). The response `backend` field reads `postgres_fallback` in this mode.
- Filters (`networks`, `categories`, `verified_only`, …) combine with cursor
  pagination exactly as they do with offset pagination.

## Choosing a mode

- **Stable iteration over a full result set** (exports, syncing, "load more"
  infinite scroll): use **cursor** pagination.
- **Relevance-ranked results or jump-to-page UIs**: use **offset** pagination.
