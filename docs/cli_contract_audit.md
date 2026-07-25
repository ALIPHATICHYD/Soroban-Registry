# `soroban-registry contract audit` CLI Specification

Command specification for the local CLI tool to detect bytecode, ABI, and storage schema drift between a local compiled WASM and published registry contracts.

---

## 1. CLI Usage

```bash
soroban-registry contract audit --local target/wasm32-unknown-unknown/release/contract.wasm --remote CABC123...XYZ
```

---

## 2. Drift Checks Implemented

| Check | Pass Condition | Failure Output |
| :--- | :--- | :--- |
| **Bytecode Hash** | Local SHA256 == Remote SHA256 | `[FAIL] WASM byte mismatch` |
| **Interface ABI** | All exported functions & types match | `[FAIL] Missing function: 'withdraw'` |
| **WASM Optimizations** | Optimizations applied via `wasm-opt` | `[WARN] Unoptimized WASM detected` |

---

## References

- Issue reference: Fixes #1060
