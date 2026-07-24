# Data Encryption (Issue #895)

This document describes how the Soroban Registry backend protects data **at rest**
and **in transit**, the algorithms chosen and why, and the operational runbook for
key and certificate management.

## Summary

| Requirement | Mechanism |
| --- | --- |
| HTTPS/TLS for all API traffic (TLS 1.2+) | TLS termination at proxy/LB + HSTS & HTTPS-redirect middleware |
| Database encryption at rest | Volume/disk encryption + Postgres TDE (infra), see below |
| Sensitive fields encrypted in DB | AES-256-GCM application-level field encryption |
| Key management system | Versioned in-memory keyring with zero-downtime rotation |
| Certificate management & rotation | Documented below (proxy-managed, automated) |
| Encrypted backups | Backup metadata & state snapshots encrypted with field encryption |
| Secure key storage (not in config files) | Keys loaded from environment only, injected by a secrets manager |
| Algorithm choices documented | This document |

## Algorithm choices

### Field encryption — AES-256-GCM

- **Why AES-256-GCM:** it is an AEAD cipher (authenticated encryption with
  associated data), so it provides confidentiality **and** integrity in a single
  pass. It is FIPS 140-approved and hardware-accelerated via AES-NI on modern
  CPUs, giving strong security with negligible overhead.
- **Key size:** 256-bit.
- **Nonce:** 96-bit (12 bytes), drawn from the OS CSPRNG (`OsRng`) for **every**
  encryption. A unique random nonce per message satisfies GCM's (key, nonce)
  uniqueness requirement; nonces are never reused or derived from data.
- **Authentication tag:** 128-bit, verified on decrypt. Any tampering with stored
  ciphertext causes decryption to fail rather than return corrupted plaintext.

### Envelope format

Encrypted values are stored as a self-describing ASCII string:

```
enc:v1:<key_id>:<base64( nonce(12 bytes) || ciphertext || tag )>
```

- The `enc:v1:` prefix lets the application distinguish encrypted values from
  legacy plaintext, enabling lazy migration (old rows keep reading, new writes
  are encrypted).
- The embedded `key_id` lets us decrypt with the correct key after a rotation
  without any external metadata.

Implementation: [`backend/api/src/crypto/`](../backend/api/src/crypto/).

## Key management

Keys are **never stored in config files or committed to the repository**. They are
read from the process environment, which is populated by a secrets manager
(HashiCorp Vault, AWS Secrets Manager, GCP Secret Manager, Kubernetes Secrets,
etc.) at deploy time, and held only in memory. Key bytes are redacted from debug
output and zeroed on drop.

### Environment variables

- `ENCRYPTION_KEYS` — comma-separated `id:base64key` entries; each key must decode
  to exactly 32 bytes. Generate with `openssl rand -base64 32`.
- `ENCRYPTION_ACTIVE_KEY_ID` — which key id signs new encryptions. May be omitted
  when exactly one key is configured.

If `ENCRYPTION_KEYS` is unset, the service runs in **pass-through mode** (no
encryption) and logs a prominent warning. This keeps local development frictionless
while making production misconfiguration obvious. A *present but invalid*
configuration fails fast at startup rather than silently storing plaintext.

### Key rotation (zero downtime)

1. Generate a new key: `openssl rand -base64 32`.
2. **Prepend** it to `ENCRYPTION_KEYS`, keeping the old key in the list:
   `ENCRYPTION_KEYS=new-id:NEWKEY,old-id:OLDKEY`
3. Set `ENCRYPTION_ACTIVE_KEY_ID=new-id`.
4. Deploy. New writes use the new key; existing rows still decrypt with the
   retained old key (selected via the `key_id` embedded in each envelope).
5. (Optional) Re-encrypt old rows in the background, then drop the retired key
   from `ENCRYPTION_KEYS` on the next deploy.

Because the key id travels with each ciphertext, no bulk re-encryption is required
to rotate, and retired keys can be removed once no ciphertext references them.

## Encryption in transit

### Client ↔ API (HTTPS/TLS 1.2+)

The API is deployed behind a TLS-terminating reverse proxy / load balancer
(nginx, Caddy, AWS ALB, GCP LB, Cloudflare). The application enforces transport
security with middleware ([`backend/api/src/security.rs`](../backend/api/src/security.rs)):

- **HSTS** (`Strict-Transport-Security`) is added to every response. Defaults:
  `max-age=63072000; includeSubDomains`. Configurable via `HSTS_ENABLED`,
  `HSTS_MAX_AGE_SECS`, `HSTS_INCLUDE_SUBDOMAINS`.
- **HTTPS redirect:** set `FORCE_HTTPS=true` to 308-redirect plaintext requests
  (detected via `X-Forwarded-Proto: http`) to `https://`.

Configure the terminating proxy to allow **TLS 1.2 and 1.3 only** and a modern
cipher suite list (e.g. Mozilla "intermediate"). TLS &lt; 1.2 must be disabled.

### API ↔ Database (TLS)

SQLx is built with `runtime-tokio-rustls`, so TLS to Postgres is available. In
non-local deployments append an `sslmode` to `DATABASE_URL`
(`...?sslmode=verify-full` is recommended). Startup logs a warning when a
non-local `DATABASE_URL` does not request TLS.

## Database encryption at rest

Application-level field encryption (above) protects the most sensitive columns
even from someone with raw table access. In addition, deploy with:

- **Disk/volume encryption** for the database host (LUKS, AWS EBS encryption, GCP
  CMEK persistent disks) — encrypts the entire data directory and WAL.
- **Managed TDE** where available (e.g. RDS/Cloud SQL "encryption at rest"),
  ideally with a customer-managed key (CMK) in the cloud KMS.

These are infrastructure controls configured outside the application.

## Encrypted backups

Contract backups encrypt their sensitive contents at rest. In
[`backup_handlers.rs`](../backend/api/src/backup_handlers.rs), the `metadata` and
`state_snapshot` JSON blobs are run through the field-encryption service before
being written to `contract_backups`, and decrypted transparently on read. With
keys configured, a dump of the backup tables contains only ciphertext envelopes.

Backups should additionally be written to encrypted object storage (e.g. S3 SSE-KMS).

## Certificate management & rotation

- **Issuance/renewal:** use ACME (Let's Encrypt) via the terminating proxy
  (Caddy auto-HTTPS, nginx + certbot, or cloud-managed certificates). Certificates
  auto-renew well before expiry (typically at ~30 days remaining).
- **Rotation:** automated by the ACME client / cloud provider; no application
  restart is required when the proxy reloads certificates.
- **Monitoring:** alert on certificate expiry (&lt; 14 days) via the existing
  alerting stack.
- For service-to-service mTLS (DB, internal services), rotate CA-issued certs on
  the provider's schedule and reload connection pools.

## Testing

Unit tests for the crypto layer live alongside the modules in
`backend/api/src/crypto/` and cover round-trips, nonce non-determinism, tamper
detection, rotation/retired-key decryption, and pass-through mode. HTTPS/HSTS
middleware tests live in `backend/api/src/security.rs`.

Run with:

```bash
cargo test -p api crypto
cargo test -p api security
```
