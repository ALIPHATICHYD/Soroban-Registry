# Sensitive Action Authorization

Issue: #1123 — Backend: Add policy engine for sensitive action authorization

## Objective

Centralize authorization for sensitive mutations so handlers no longer make
inconsistent decisions from JWT fields, request-supplied publisher IDs, or
locally duplicated owner checks.

The implementation deliberately separates:

1. **Authentication** — validates the bearer JWT and its referenced session.
2. **Identity resolution** — matches the server-signed publisher UUID to the
   same publisher's Stellar address in the database.
3. **Authorization policy** — decides whether that actor is a publisher,
   resource owner, contract owner, transfer participant, or administrator.
4. **Cryptographic verification** — verifies signed action payloads against
   the Stellar address already resolved for the authenticated actor.

## Policy decisions

- Owner-only actions do not receive an automatic administrator bypass.
- For the migrated actions, acting identity never comes from request fields
  such as `publisher_address` or `user_id`.
- A valid publisher actor requires both the JWT `publisher_id` and `sub`
  Stellar address to resolve to the same database row.
- A missing, expired, revoked, or identity-mismatched referenced session
  invalidates the access token.
- Sessionless JWTs remain supported for backward compatibility and externally
  issued administrator tokens.
- Resource-specific SQL predicates remain in place as defense in depth.
- Webhooks owned by another publisher return `404`, preserving resource
  existence confidentiality.

## Shared policy surface

`backend/api/src/policy.rs` provides the database-verified `PolicyActor` and
the explicit checks used by sensitive handlers:

- registered publisher
- submitted publisher/signing address
- contract owner
- generic resource owner
- administrator
- ownership-transfer sender/recipient action
- webhook owner
- webhook-delivery owner

This is intentionally not a generic RBAC framework. The named rules represent
the actions the application currently has and can be audited directly.

## Protected actions

| Action | Policy |
| --- | --- |
| Publish contract | Registered publisher; submitted address must equal authenticated address |
| Deprecate contract | Contract owner |
| Undeprecate contract | Contract owner |
| Purge expired deprecated contracts | Administrator only |
| Declare package dependencies | Contract owner |
| Trigger dependency vulnerability re-scan | Contract owner |
| Trigger manual security scan | Contract owner |
| Update a security issue status | Contract owner |
| Register a security scanner | Administrator only |
| Directly change a contract publisher | Current owner plus the existing multisig approval |
| Initiate ownership transfer | Current owner plus existing Ed25519 signature verification |
| Accept ownership transfer | Intended recipient only |
| Reject ownership transfer | Sender or intended recipient |
| Create/list webhooks | Registered publisher |
| Delete/test/read deliveries for a webhook | Webhook owner |
| Retry a webhook delivery | Owner of the associated webhook |

The expired-deprecation purge route was also moved into the admin router, while
the handler retains its own admin policy check.

## Identity and session corrections

Previously, `AuthenticatedUser` parsed the JWT `sub` field as a UUID. `sub` is
a Stellar `G...` address, so publisher IDs became the nil UUID. The extractor
now uses the server-signed `publisher_id` claim and the shared AppState-backed
JWT/session validation path.

Session-backed tokens are checked against the in-memory session record for:

- session existence
- session expiry
- Stellar subject
- publisher UUID
- role
- scopes
- MFA state

The general JWT claims extractor and `AuthContext` now use the same validation
path.

## Signature identity

Ownership-transfer signatures continue using their existing domain-separated,
nonce-protected payloads. Transfer authorization decisions now come from the
shared policy actor, and signature verification always uses that actor's
database-verified Stellar address.

The CLI deprecation request was aligned with the backend request model. When a
deprecation signature envelope is present, the backend verifies:

- all signature-envelope fields are present
- signing address equals the authenticated actor's address
- action is `deprecate`
- signed contract ID equals the resolved contract
- nonce syntax and length
- timestamp freshness
- Ed25519 signature against the actor's Stellar account

Unsigned deprecation requests remain supported and rely on the authenticated
owner policy. This preserves existing API clients while making signed clients
verifiable instead of silently ignoring their signature data.

## Client compatibility

The frontend API wrapper now preserves caller-provided headers and attaches the
existing `soroban_registry_token` bearer token when the caller did not provide
an explicit authorization header. This gives affected mutation callers one
consistent authentication path instead of duplicating header plumbing.

The CLI request layer already attaches its stored access token. The deprecation
CLI now sends the backend's required retirement and reason fields together with
the signed envelope, and derives the signing identity with Stellar StrKey
encoding so it matches the canonical `G...` account used by authentication.

## Tests

Policy-focused unit tests cover:

- registered and unregistered publishers
- owner allow and non-owner deny
- no automatic admin owner bypass
- admin allow and publisher deny
- case-insensitive admin role claims
- submitted publisher/signature address match and mismatch
- transfer sender reject
- transfer sender accept denial
- transfer recipient accept/reject
- outsider denial

Authentication tests cover:

- valid live session
- expired session
- session identity mismatch
- revoked/missing session
- backward-compatible sessionless JWT

The ownership-transfer live fixture now registers and authenticates a publisher
before publishing. The frontend API test verifies automatic bearer-token
attachment.

## Operational notes

- No dependency or database migration was added.
- Existing compare-and-swap, row-locking, multisig, and resource-scoped SQL
  checks were retained.
- Authorization covers the currently exposed `/api/webhooks` management API.
  The separate lifecycle `webhook_subscriptions` storage has no management
  handlers to migrate in this issue.
- Scheduled callers of the deprecation purge endpoint must supply an
  administrator bearer token.
