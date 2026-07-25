# Contract Deprecation Soft-Delete & Grace Period Specification

Technical design document for implementing soft-delete lifecycle states and a 30-day deprecation grace period for registered Soroban smart contracts in the Soroban Registry backend.

---

## 1. Deprecation Lifecycle States

```
[ Active ] ───> [ Deprecated (Soft-Deleted) ] ───> [ Archived (Hard-Deleted) ]
                       │
                       ▼ (30-Day Grace Period)
        [ Grace Period: Read-only API access ]
```

1. **Active:** Fully searchable, indexable, and installable via CLI.
2. **Deprecated (Soft-Deleted):** Marked as `is_deprecated = true`, hidden from primary search results, but API endpoints serve `X-Deprecation-Warning` headers.
3. **Archived:** Expired after 30-day grace period; binary assets moved to cold storage.

---

## 2. API Header Contracts

Deprecation endpoints return standard HTTP headers:

```http
HTTP/1.1 200 OK
Deprecation: @1753401600
Sunset: Mon, 24 Aug 2026 00:00:00 GMT
Link: <https://registry.soroban.org/docs/deprecation>; rel="deprecation"
```

---

## References

- Registry API: [`backend/src/api/contracts.rs`](../backend/src/api/contracts.rs)
- Issue reference: Fixes #1061
