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

## Client-side pagination (`registry_client`)

Everything above is the HTTP contract. Consumers should not implement it by
hand: the `registry_client` crate (`backend/registry_client`) wraps both modes
in one typed abstraction, and the CLI's `contract search` command uses it.

```rust
use registry_client::{ContractSearchRequest, PageLimits, RegistryClient};

let client = RegistryClient::new("http://localhost:3001")?;
let request = ContractSearchRequest::cursor("swap").with_networks(["testnet"]);

let mut walk = client.search_paginator(
    request,
    PageLimits::default().with_page_size(50).with_max_items(Some(1_000)),
)?;

while let Some(page) = walk.next_page().await? {
    for hit in page.items {
        println!("{} ({:?} of {:?})", hit.name, page.total, walk.total());
    }
}
```

A walk is also available as a stream: `Paginator::pages()` yields
`RegistryPage<T>` values and `Paginator::items()` flattens them into items (pin
the stream, e.g. with `Box::pin`, before polling it). `Paginator::collect_all()`
returns a `PageCollection` with the items, the server total, the pages fetched,
and why the walk stopped.

### What the abstraction guarantees

| Guarantee | Behaviour |
| --- | --- |
| Cursors stay opaque | Tokens are compared and echoed back byte-for-byte; the client never decodes or constructs one. |
| Modes never mix | `PaginationMode` is explicit. A cursor combined with an offset is rejected before a request is sent. |
| No infinite loops | A repeated cursor, an offset that fails to advance, or repeated empty-but-continuable pages all end the walk with a specific error. |
| No silent wraparound | `offset + page_len` is overflow-checked on every page. |
| Retries never duplicate | Transport retries happen inside one page fetch, and a failed fetch leaves the walk untouched, so retrying re-requests the same page. |
| Bounds by default | `max_pages` defaults to a finite value, and `max_items` caps what a walk emits. Stopping at a bound reports the continuation to resume from. |
| Cancellation | A `CancelToken` stops a walk between pages or mid-request, keeping the pages already emitted. |
| Totals preserved | `total` is surfaced verbatim whenever the endpoint reports it. |
| Ordering preserved | Items are never re-sorted, so the endpoint's ordering guarantee is what you observe. |

Cursor mode talks to `GET /api/search`; offset mode talks to
`GET /api/v1/contracts/search` (tag filters force `GET /api/search`, the only
search endpoint that supports them).

### CLI

```bash
# One page, relevance ordered (unchanged single-page behaviour)
soroban-registry contract search swap --limit 20

# Every page, cursor paginated, bounded at 1000 items
soroban-registry contract search swap --all --max-items 1000

# Explicit mode, resuming from a token a previous run printed
soroban-registry contract search swap --pagination cursor --cursor eyJ0aW1lc3Rh…

# Machine-readable output, including pagination metadata
soroban-registry contract search swap --all --max-items 500 --json
```

`--all` is always bounded (defaults: `--max-items 1000`, `--max-pages 100`) and
Ctrl-C stops the walk and prints what was already fetched. `--cursor` and
`--offset` are mutually exclusive. Without `--pagination`, `--all` uses cursor
pagination and a single page uses offset pagination.

`--json` output carries a `pagination` object:

```json
{
  "query": "swap",
  "contracts": [],
  "count": 1000,
  "pagination": {
    "mode": "cursor",
    "page_size": 50,
    "pages_fetched": 20,
    "total": 4213,
    "complete": false,
    "stop_reason": "max_items",
    "cancelled": false,
    "next_cursor": "eyJ0aW1lc3Rh…",
    "next_offset": null,
    "max_items": 1000,
    "max_pages": 100
  }
}
```

`stop_reason` is one of `exhausted`, `max_items`, `max_pages`, `cancelled`, or
`single_page` (a search run without `--all`). When it is not `exhausted`,
`next_cursor`/`next_offset` say where a follow-up run should resume.
