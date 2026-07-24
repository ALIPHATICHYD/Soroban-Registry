# Pull Request: Add Version History and Changelog Diff Endpoints

**Title:** `feat: Add version history compare and changelog diff endpoints (#955)`

**Branch:** `feature/version-pagination-diff`

**Related Issue:** #955 (Add changelog and version history diff endpoints)

---

### 📝 Summary

This pull request completes and verifies the implementation of the **Version History Compare & Changelog Diff** functionality (Issue #955) inside the Soroban Registry. It enables consumers to track semantic version history, explicitly analyze version-to-version metadata differences (such as WASM hashes, source URLs, commit hashes, release notes, and change notes), and ensures reliable backend builds with updated lockfile constraints.

### 🚀 Changes Made

| Component | Files Added/Modified | Description |
|-----------|----------------------|-------------|
| **API Routes** | `backend/api/src/routes.rs` | Exposes the version comparison route: `GET /api/contracts/:id/versions/compare` mapping to the compare handler. |
| **API Handlers** | `backend/api/src/handlers.rs` | Refines and exports `compare_contract_versions` to parse version parameters, query database version records, perform semantic field comparisons, and return a structured difference. |
| **Cargo Build** | `backend/Cargo.lock` | Restores and updates dependency locking (`redox_syscall`, telemetry, etc.) to ensure build stability and resolve local/remote mismatch conflicts. |

---

### 🔍 Endpoint Details

#### `GET /api/contracts/:id/versions/compare?from={version_a}&to={version_b}`

Allows clients to retrieve a comprehensive diff between two version tags of a specific contract.

**Sample Request:**
```bash
curl "https://registry.soroban.org/api/contracts/550e8400-e29b-41d4-a716-446655440000/versions/compare?from=1.0.0&to=1.1.0"
```

**Sample Response Schema:**
```json
{
  "contract_id": "550e8400-e29b-41d4-a716-446655440000",
  "from_version": {
    "version": "1.0.0",
    "wasm_hash": "a1b2c3d4...",
    "source_url": "https://github.com/example/contract",
    "commit_hash": "f623ad...",
    "release_notes": "Initial release",
    "change_notes": null
  },
  "to_version": {
    "version": "1.1.0",
    "wasm_hash": "e5f6g7h8...",
    "source_url": "https://github.com/example/contract",
    "commit_hash": "c71a30...",
    "release_notes": "Added performance improvements",
    "change_notes": "Optimized execution cost"
  },
  "differences": [
    {
      "field": "wasm_hash",
      "from_value": "a1b2c3d4...",
      "to_value": "e5f6g7h8..."
    },
    {
      "field": "commit_hash",
      "from_value": "f623ad...",
      "to_value": "c71a30..."
    },
    {
      "field": "release_notes",
      "from_value": "Initial release",
      "to_value": "Added performance improvements"
    },
    {
      "field": "change_notes",
      "from_value": null,
      "to_value": "Optimized execution cost"
    }
  ],
  "wasm_changed": true
}
```

---

### ✅ Checklist

- [x] Version comparison endpoint `GET /api/contracts/:id/versions/compare` registered and verified.
- [x] Pull request synchronized with the latest commits from the upstream `main` branch.
- [x] Resolved lockfile integration conflicts (`backend/Cargo.lock`) cleanly with the latest telemetry and observability updates.
- [x] Verified that the workspace compiles successfully.
- [x] Pushed and updated the remote branch `feature/version-pagination-diff`.

---

**PR Link:** https://github.com/Robinsonchiziterem/Soroban-Registry/pull/new/feature/version-pagination-diff
