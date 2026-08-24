# Contract dependency graphs and transitive risk

Issue #1147.

Consumers can see individual contract metadata and vulnerability findings, but
not how contracts depend on one another, or how a vulnerable dependency affects
downstream deployments. This document is the contract for the four endpoints
that answer those questions: their JSON shapes, the rules that decide what
counts as a risk, and the guarantees you can rely on.

## Endpoints

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/api/contracts/:id/dependencies` | What this contract depends on, as a tree |
| GET | `/api/contracts/:id/dependents` | What depends on this contract |
| GET | `/api/contracts/:id/dependency-graph` | Flat counts, cycles, diagnostics, truncation |
| GET | `/api/contracts/:id/dependency-risk` | Direct vs inherited findings, with paths |

`:id` accepts a registry UUID **or** a Stellar contract address (`C...`).

### Query parameters

| Parameter | Applies to | Default | Meaning |
| --- | --- | --- | --- |
| `network` | all | none | `mainnet`, `testnet`, or `futurenet`. Required to disambiguate an address registered on more than one network. |
| `transitive` | dependencies, dependents, graph | `true` | `false` walks direct edges only. |
| `depth` | all | 10 | Maximum traversal depth. Capped at 32. |
| `max_nodes` | dependencies, dependents, graph | 1000 | Maximum nodes returned. Capped at 10000. |
| `as_of` | dependencies, dependents, graph | now | RFC3339 instant; replays the graph as it stood then. |
| `include_telemetry` | dependencies, dependents, graph | `false` | Include on-chain call edges alongside declared ones. |
| `page` | graph | 1 | 1-based page of the flat `nodes` list. |
| `per_page` | graph | 50 | Nodes per page. Capped at 200. |

### Resolving `:id`

Because `contracts` is `UNIQUE(contract_id, network)`, a bare address is not a
unique identifier.

| Input | Result |
| --- | --- |
| UUID that exists | resolved |
| UUID that does not exist | `404 ContractNotFound` |
| UUID plus a `network` naming a different network | `400 NetworkMismatch` |
| Address registered on exactly one network | resolved |
| Address registered on several, with `network` | resolved |
| Address registered on several, without `network` | **`409 AmbiguousContractRef`**, with the candidates in `details.candidates` |
| Address that is not registered | `404 ContractNotFound` |
| Neither a UUID nor a well-formed address | `400 InvalidContractRef` |

The 409 is deliberate. Picking a network silently would answer a question about
a *different contract* than the caller meant, and nothing in the response would
reveal it.

```json
{
  "code": "AMBIGUOUSCONTRACTREF",
  "message": "Contract address CB... is registered on 2 networks. Retry with ?network=",
  "details": {
    "candidates": [
      { "id": "…", "contract_id": "CB…", "network": "mainnet" },
      { "id": "…", "contract_id": "CB…", "network": "testnet" }
    ]
  }
}
```

## The edge model

Edges live in `contract_dependency_edges`, one canonical, network-scoped,
bitemporal table.

| Column | Meaning |
| --- | --- |
| `source_contract_id` | The dependent. |
| `target_contract_id` | The dependency. `NULL` when the reference did not resolve. |
| `target_ref` | Exactly what was declared, kept verbatim. |
| `network` | The network both ends belong to. |
| `edge_source` | `declared` or `telemetry`. |
| `edge_state` | `resolved`, `unresolved`, or `network_mismatch`. |
| `version_constraint` | As declared. Opaque; defaults to `*`. |
| `expected_interface_id` | The target's interface id **when the edge was recorded**. |
| `recorded_at` / `superseded_at` | `superseded_at IS NULL` selects current state. |

A read-only view named `contract_dependencies` projects current declared edges
into the legacy `(id, contract_id, dependency_name, dependency_contract_id,
version_constraint, created_at)` shape. Write through
`contract_dependency_edges`, never the view.

### Declared vs telemetry edges

**Declared** edges come from `POST /api/contracts/:id/dependencies`. They are
versioned: changing a declaration supersedes the old row rather than deleting
it, so `as_of` can replay history exactly. Only genuine changes are superseded —
re-saving an unchanged declaration leaves its `recorded_at` alone, so history
does not fill with no-op churn.

**Telemetry** edges are derived live from
`contract_call_edge_daily_aggregates` and are never materialized. That table
stores both endpoints as UUID foreign keys, is already network-scoped, and is
already day-bucketed — it *is* the historical record. Copying it into the
bitemporal table would add a supersede-and-insert to the hottest write path in
the system for no information gain. The consequence for `as_of`: declared edges
replay to the instant, telemetry edges replay to the day.

### References are never inferred

Dependencies are only ever recorded from an explicit declaration. A reference
must be a Stellar contract address and is resolved with
`SELECT id FROM contracts WHERE contract_id = $1 AND network = $2`.

- No match on any network → `edge_state = 'unresolved'`.
- Match on a different network → `edge_state = 'network_mismatch'`.

Neither is bound to a contract, and neither is walked. Both are retained and
reported: the operator declared something real, and dropping it would silently
shrink the graph.

Resolution by `contracts.name` was removed. `name` has no UNIQUE constraint, so
a name lookup bound a dependency to whichever row happened to share a name.
**Legacy rows that resolved by name are re-recorded as `unresolved`** rather
than silently rebound — a behaviour change, and an intentional one.

## Traversal guarantees

**Bounded three ways.** A cycle guard (the walk carries its root-to-here path and
stops on a repeat), a depth bound, and a node budget plus a
`SET LOCAL statement_timeout`. The timeout is not redundant: dropping an axum
future does **not** cancel an in-flight sqlx query, so without it a client that
disconnects mid-traversal would hold a pooled connection for the full runtime of
the query. Exceeding it returns `503 DependencyTraversalTimeout`.

**Truncation is always reported.** `truncated` plus a `truncation_reason` of
`depth_limit`, `node_limit`, or `time_limit`, and a `truncated` diagnostic. A
graph that fits exactly within `depth` and ends in terminal rows is **not**
reported as truncated.

**Deterministic.** Rows come back ordered by
`(depth, network, target_contract_id, target_ref, edge_source)` — total, and with
no floats in any sort key. Two identical requests against an unchanged graph
return byte-identical JSON.

**Cycles terminate and are reported.** The guard emits the loop-closing edge once
and then stops. Without that the cycle would be invisible and would look like
depth truncation.

**Tenancy is enforced inside the walk.** A node the caller may not see is
returned as `[redacted]` — counted, so totals stay honest, but with no address,
name, or interface id — and the walk **stops there**. A private contract is never
a stepping stone to enumerating what it depends on.

## Response shapes

### `/dependencies` and `/dependents`

Field-stable: fields may be added with serde defaults, never renamed or removed
(GraphQL's `Contract.dependencies` resolves through this type).

```json
{
  "root": {
    "contract_id": "CA…",
    "resolved_id": "…uuid…",
    "name": "A",
    "call_volume": 0,
    "status": "unverified",
    "is_circular": false,
    "dependencies": [
      {
        "contract_id": "CB…",
        "resolved_id": "…uuid…",
        "name": "B",
        "status": "resolved",
        "is_circular": false,
        "dependencies": [],
        "visualization_hints": {
          "depth": 1,
          "node_type": "standard",
          "edge_source": "declared",
          "edge_state": "resolved",
          "redacted": false
        }
      }
    ],
    "visualization_hints": {
      "node_type": "root",
      "depth": 0,
      "truncated": false,
      "truncation_reason": null,
      "diagnostics": []
    }
  },
  "total_dependencies": 3,
  "max_depth": 2,
  "has_circular": false
}
```

`status` is one of `resolved`, `unresolved`, `network_mismatch`, or `redacted`.

### `/dependency-graph`

```json
{
  "contract_id": "…uuid…",
  "network": "testnet",
  "total_dependencies": 6,
  "resolved": 4,
  "unresolved": 2,
  "redacted": 0,
  "max_depth": 4,
  "has_circular": true,
  "truncated": false,
  "diagnostics": [],
  "nodes": {
    "items": [
      {
        "contract_id": "CB…",
        "resolved_id": "…uuid…",
        "name": "B",
        "depth": 1,
        "edge_source": "declared",
        "edge_state": "resolved",
        "path": ["…root…", "…b…"],
        "redacted": false
      }
    ],
    "total": 6,
    "page": 1,
    "per_page": 50,
    "pages": 1
  }
}
```

### `/dependency-risk`

```json
{
  "contract_id": "…uuid…",
  "network": "testnet",
  "direct_findings": [ … ],
  "inherited_findings": [ … ],
  "diagnostics": [ … ],
  "direct_risk":  { "effective_severity": "Medium",   "counts": { "critical": 0, "high": 0, "medium": 1, "low": 0 } },
  "overall_risk": { "effective_severity": "Critical", "counts": { "critical": 1, "high": 0, "medium": 1, "low": 3 } },
  "total_dependencies": 3,
  "max_depth": 3,
  "truncated": false
}
```

A finding:

```json
{
  "rule_id": "open_vulnerability",
  "severity": "Critical",
  "origin_contract_id": "…uuid…",
  "finding_id": "…uuid…",
  "path": ["…root…", "…dependency…"],
  "inherited_via_depth": 1,
  "detail": "Reentrancy in withdraw"
}
```

`severity` is capitalized (`Low`, `Medium`, `High`, `Critical`) because it
reuses `IssueSeverity`, whose wire form predates this feature and is shared with
the security endpoints.

## Findings and diagnostics are separate arrays

**Findings carry a severity. Diagnostics do not.**

This is the single most important rule in the design. A cycle, an unresolved
edge, or a truncated walk is a fact about the *graph*, not a security condition.
If they carried a severity, `max()` would drag every cyclic graph to at least
Low, "zero findings" would be unreachable, and the report would be useless as a
CI gate.

### Findings

| Condition | `rule_id` | Source | Direct | Inherited |
| --- | --- | --- | --- | --- |
| Open vulnerability | `open_vulnerability` | `security_issues` (status `open`) | as recorded | **same severity** |
| Signature revoked | `signature_revoked` | `package_signatures.status='revoked'` for the current wasm hash | High | High |
| Artifact quarantined | `artifact_quarantined` | `contracts.artifact_scan_status='quarantined'` | High | Medium |
| Artifact unsigned | `artifact_unsigned` | no `valid` signature for the current wasm hash | Medium | Low |
| Deprecated, no replacement or superseded | `deprecated_no_replacement` | `contracts.deprecation_status` | Medium | Low |
| Deprecated with replacement, in grace period | `deprecated_with_replacement` | `contract_deprecations.grace_period_days` | Low | *not inherited* |
| Interface incompatibility | `interface_incompatibility` | `expected_interface_id != contracts.interface_id` | High | *not inherited* |

An open vulnerability keeps its recorded severity when inherited: a vulnerable
dependency is exactly as exploitable through its caller. Everything else
attenuates, because a problem you can only reach through someone else's code is
a planning problem rather than an immediate one.

Two rules do not propagate at all. An interface incompatibility is a fact about
one specific edge — the expectation recorded on it versus the target now — so
re-reporting it two hops away would be meaningless. A deprecation that has a
named replacement and is still inside its grace period is a scheduled migration.

### Diagnostics

| `kind` | Meaning |
| --- | --- |
| `cycle` | The walk re-encountered a contract already on its path. `path` names the loop, not the lead-in. |
| `unresolved_edge` | A declared reference that binds to nothing. |
| `network_mismatch` | A reference that names a contract on a different network. |
| `truncated` | A depth, node, or time budget was hit. |
| `version_conflict` | Two edges target one contract under constraints no published version satisfies. |
| `redacted_node` | A node hidden by tenancy, counted but not named. |

**Unresolved edges are split by shape.** A well-formed contract address that is
not registered is a genuine gap in the graph. A free-form string is an
undeclared library and merely informational. Treating both alike would bury the
real signal, since most unresolved references are the second kind.

## Interface incompatibility

An edge records the target's `interface_id` at declaration time. An
incompatibility is drift between that recorded value and the target's current
one.

`interface_id` is the existing `soroban-interface-v1` fingerprint
(`shared::interface_fingerprint`), derived at publish time from the
`contractspecv0` custom section of the submitted artifact. It normalizes away doc
comments, library names, and entry ordering, so cosmetic changes do not register
as drift.

**No WASM is ever executed.** The module is walked for custom sections and the
section is read by a dependency-free XDR reader.

`NULL` on either side means *unknown*, not *different*. A contract published
without an artifact, or whose artifact embeds no spec section, has no interface
id, and no incompatibility is reported against it. Reporting drift against an
unknown would flag most of the registry.

## Version conflicts

`version_constraint` is opaque and defaults to `*`, which does not parse as a
constraint. The target carries no version on `contracts` either — versions live
in `contract_versions`. So a conflict cannot be decided from the constraints
alone.

**Definition:** two edges target the same contract under constraints for which no
row in `contract_versions.version` satisfies both.

- An unparseable constraint (including `*`) never creates a conflict. Reporting
  one would flag a parser gap as a dependency problem.
- A target with no published versions never conflicts: there is nothing to
  satisfy either constraint, so any claim would be speculation.

## Combining severities

Risk is `(effective_severity, counts_by_severity)`, compared lexicographically —
not a bare `max()`.

- `max()` alone ranks one High identically to forty Highs, which is exactly the
  distinction someone triaging an upgrade needs.
- Severity is ordinal, so summing or averaging it is meaningless. Forty Highs
  must never add up to a Critical.
- The tuple is float-free and total, so the ordering is deterministic and
  reproducible.

`effective_severity` is `null` when there are no findings. That state is
reachable precisely because diagnostics are excluded from `findings`.

### Deduplication

A finding reachable by several paths is reported once, keyed by
`(rule_id, origin_contract_id, finding_id)`. The **shortest** path is kept —
it is the most actionable route from the contract you asked about to the
problem — with ties broken lexicographically so the result is a function of the
graph rather than of row arrival order.

Findings are ordered by `(severity DESC, rule_id ASC, path ASC)`.

## Pagination

Pagination applies to **`/dependency-graph` only**, over its flat `nodes` list:

```
GET /api/contracts/:id/dependency-graph?page=2&per_page=50
```

`nodes` is a standard `PaginatedResponse` (`items`, `total`, `page`, `per_page`,
`pages`). `per_page` defaults to 50 and is capped at 200; `page` is 1-based, and
out-of-range values are clamped rather than rejected. A page past the end
returns an empty `items` with `total` unchanged.

The tree endpoints `/dependencies` and `/dependents` are **not** paginated,
because a tree cannot be coherently paged — page 2 of a tree is not a tree.
They return the whole traversal, which is already bounded by `max_nodes`. Use
`/dependency-graph` when you want to walk the reachable set incrementally.

Offset (rather than cursor) pagination is defensible here only because the
traversal hard-caps its result set at 10000 rows. It is **not** justified by the
total ordering: the graph mutates between requests, and offsetting into a full
CTE costs the whole graph per page regardless. Cursor pagination is unavailable
anyway — `shared::pagination::Cursor` is hardcoded to `(DateTime<Utc>, Uuid)`
and cannot express graph order.

## Snapshots

`GET /api/contracts/:id/snapshot?include_dependency_graph=true` embeds the risk
report and raises the payload to **schema 1.1**. The default remains 1.0 with no
`dependency_graph` field, byte-identical to before this feature.

The version bump is not cosmetic. `SnapshotPayload` has no
`deny_unknown_fields`, so an older CLI handed a graph-bearing payload that
claimed to be 1.0 would drop the field, recanonicalize without it, and report
**"signature invalid"** — telling the operator their snapshot was tampered with.
With a distinct version it returns `UnsupportedSchema("1.1")`: true, actionable,
and impossible to mistake for tampering. The version is derived from the payload
content, so a graph-bearing snapshot cannot claim to be 1.0.

## CLI

```bash
soroban-registry contract dependencies <ADDRESS> [--network N] [--transitive] [--depth N] [--json]
soroban-registry contract dependents   <ADDRESS> [--network N] [--transitive] [--depth N] [--json]
soroban-registry contract dependency-risk <ADDRESS> [--network N] [--depth N] [--fail-on LEVEL] [--json]
```

`--json` prints the API response unmodified, so scripts get exactly the shapes
documented above.

`--fail-on low|medium|high|critical` exits 1 when
`overall_risk.effective_severity` meets or exceeds the level, and 0 otherwise —
including when there are no findings at all. The comparison uses the severity
the server computed; recomputing it client-side would let the CLI and the
registry disagree about whether a deploy should be blocked. A failed request
never exits 0, so a gate cannot pass silently because the registry was down.

## Testing

- `cargo test -p shared` — the pure rules: severity table, combinator, dedup and
  shortest-path, ordering, version conflicts, budget clamping, cycle segments.
  No database.
- `cargo test -p api --test dependency_graph_tests -- --ignored --test-threads=1`
  — the traversal against real Postgres: cycles, diamonds, truncation, tenancy,
  propagation, determinism, and the compatibility view. See the file header for
  the required environment.
- `cd cli && cargo test --test contract_dependency_graph_tests` — CLI surface,
  argument validation, and failure paths. No registry needed.
