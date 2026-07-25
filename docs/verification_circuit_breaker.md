# Circuit Breaker Specification for External Verification Services

Resilience engineering design pattern for wrapping external Soroban verification RPC and WASM compiler calls in a circuit breaker state machine.

---

## 1. Circuit Breaker State Transitions

```
    ┌──────────┐   Consecutive Failures >= 5   ┌──────────┐
    │  Closed  │ ────────────────────────────> │   Open   │
    └──────────┘                               └──────────┘
         ▲                                           │
         │         Half-Open Success >= 2            │ Reset Timeout (60s)
         └───────────────────────────────────────────┘
```

- **Closed:** All external verification requests pass through normally.
- **Open:** Requests fail fast without calling remote RPC (`Error::VerificationServiceUnavailable`).
- **Half-Open:** Allows 2 test requests to pass after 60s cooldown.

---

## 2. Configuration Parameters

- **Failure Threshold:** 5 consecutive timeouts/errors (HTTP 5xx or RPC timeout > 5000ms).
- **Reset Timeout:** 60,000 ms.
- **Fallback Behavior:** Queue compilation request into async worker retry queue.

---

## References

- Issue reference: Fixes #1059
