# WASM artifact canonicalization

Contract verification uses two separate identities for two separate purposes:

1. **Authoritative artifact identity:** SHA-256 of the exact deployed WASM
   bytes. This remains the registry and on-chain trust boundary.
2. **Reproducibility comparison:** a versioned, metadata-tolerant SHA-256 used
   only after the fetched deployed bytes have been proven to match the
   authoritative artifact identity.

This separation prevents harmless compiler metadata from causing false
reproducibility failures without allowing different executable programs to be
treated as the same deployment.

## V1 algorithm

Identifier: `soroban-registry-wasm-canonical-v1`

V1 first validates that the input is a core WebAssembly v1 module. It then
preserves the original module header, section order, section encodings, and
payload bytes, except that it removes these complete custom sections:

| Custom section | Reason excluded |
| --- | --- |
| `name` | Standard debug symbols; not executed by the WebAssembly runtime |
| `producers` | Toolchain provenance/version metadata; not executed by the runtime |

The canonical hash is SHA-256 of those canonical bytes. The algorithm
identifier must always accompany the hash; a future allowlist change requires
a new identifier rather than silently reinterpreting existing values.

## Bytes that remain inside the trust boundary

V1 does **not** alter or remove:

- any standard section, including types, imports, functions, tables, memory,
  globals, exports, elements, code, or data;
- the Soroban `contractspecv0` ABI section;
- the Soroban `contractenvmetav0` environment/SDK interface section;
- any unknown or vendor custom section;
- instruction operands, code-generation-unit output, function ordering, or
  monomorphized executable bodies.

Consequently, a change such as the executable instruction-immediate drift
observed in `rs-soroban-sdk#1975` remains a mismatch. Ignoring it would weaken
artifact verification because the verifier cannot prove that such a byte is
semantically irrelevant.

## Verification sequence

The source verifier applies these checks in order:

1. Normalize and validate the recorded 64-character deployed hash.
2. SHA-256 the fetched deployed bytes and require an exact match with that
   recorded hash. Failure stops verification as `artifact_hash_mismatch`;
   canonical comparison is refused.
3. Validate the authoritative deployed artifact as canonicalizable WASM and
   prepare its V1 hash. Invalid bytes stop verification as
   `invalid_wasm_artifact` for the `deployed` artifact.
4. Compile the submitted source and compare its raw SHA-256 with the deployed
   raw SHA-256. An exact match succeeds as `exact`; no canonical equality
   comparison is used for this path.
5. Only if the raw hashes differ, canonicalize the compiled artifact with the
   same V1 algorithm and compare it with the prepared deployed canonical hash.
   Invalid compiled bytes fail as `invalid_wasm_artifact` for the `compiled`
   artifact. Equality succeeds as `canonical_metadata_only` and reports the
   algorithm and canonical hash.
6. Valid WASM with executable drift, Soroban metadata drift, or any other
   trust-boundary change fails closed as `source_mismatch`.

The original raw hashes are retained in every successful result. A canonical
hash is never compared with a raw on-chain hash.

## Regression and CI coverage

The focused CI job builds the repository's Soroban fixture twice into separate
target directories, compares the two complete artifacts with the V1 tool, and
then runs the shared WASM and verifier test suites. The synthetic drift
fixtures cover both sides of the boundary:

- different `name`/`producers` payloads produce different raw bytes but the
  same canonical V1 hash;
- interface fingerprints remain stable across toolchain-only metadata drift;
- code instruction changes, `contractspecv0`, `contractenvmetav0`, and unknown
  custom-section changes remain mismatches;
- malformed WASM and deployed-byte/raw-hash disagreement fail closed.
