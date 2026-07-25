# Two-Step Contract Ownership Transfer Specification

Security specification for a two-step transfer workflow (`propose_transfer` -> `accept_transfer`) to prevent accidental loss of registered contract administrative rights.

---

## 1. Two-Step Transfer Protocol

```
[ Current Owner ] ───> `propose_transfer(new_owner)` ───> [ Pending Owner Field Set ]
                                                                     │
                                                                     ▼
[ Transfer Complete ] <─── `accept_transfer()` <─── [ New Owner Accepts ]
```

---

## References

- Issue reference: Fixes #1058
