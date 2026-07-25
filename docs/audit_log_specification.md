# Structured Audit Log Specification for Registry Write Operations

Technical design specification for structured, immutable audit log events across all contract publish, deprecation, transfer, and deletion operations in Soroban Registry.

---

## 1. Audit Log Schema

```json
{
  "event_id": "audit_84920481029",
  "timestamp": "2026-07-25T04:15:00Z",
  "actor_id": "usr_01HJ8Z",
  "action": "CONTRACT_PUBLISH",
  "target_contract_id": "CABC123...XYZ",
  "ip_address_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "changes": {
    "version": "1.0.0",
    "wasm_hash": "a8f902..."
  }
}
```

---

## 2. Event Dispatch Engine

- Audit events are emitted asynchronously to PostgreSQL `audit_logs` table.
- Indexing triggers alertwebhooks on high-risk actions (e.g. ownership transfer).

---

## References

- Issue reference: Fixes #1063
