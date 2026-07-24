# API CORS and CSRF Policy

The API reads allowed browser origins from `CORS_ALLOWED_ORIGINS`, falling back
to `ALLOWED_ORIGINS`, then to:

```text
http://localhost:3000,https://soroban-registry.vercel.app
```

Allowed origins receive credentials-enabled CORS responses, preflight support
for `GET`, `HEAD`, `POST`, `PUT`, `PATCH`, `DELETE`, and `OPTIONS`, and exposed
request/rate-limit/CSRF headers.

Browser mutation requests are rejected when:

- `Origin` is not in the configured allow-list.
- `Sec-Fetch-Site` reports `cross-site`.
- A cookie or browser origin is present but `X-CSRF-Token` does not match the
  `sr_csrf` same-site cookie.

Clients can fetch a CSRF token from:

```text
GET /api/auth/csrf
```

The response sets `sr_csrf` with `SameSite=Lax`, `HttpOnly`, `Path=/`, and
`Secure` by default. Set `CSRF_COOKIE_SECURE=false` for local non-HTTPS
development only. Set `CSRF_COOKIE_SAMESITE=strict|lax|none` to tune cookie
same-site behavior.

---

## Rate Limiting (Issue #1045)

All endpoints are protected by a **sliding-window rate limiter**. The limiter
is per-instance; for distributed setups see the horizontal scaling note in
`rate_limit.rs`.

Every response includes standard rate-limit headers:

| Header                  | Meaning                                    |
|-------------------------|--------------------------------------------|
| `X-RateLimit-Limit`     | Max requests allowed in the current window |
| `X-RateLimit-Remaining` | Requests left in the current window        |
| `X-RateLimit-Reset`     | Seconds until the window resets            |
| `Retry-After`           | Seconds to wait before retrying (429 only) |

When a limit is exceeded the API returns **HTTP 429 Too Many Requests** with a
JSON body:

```json
{
  "error_code": "RATE_LIMITED",
  "message": "Too many requests. Please retry after the indicated time.",
  "details": { "retry_after_seconds": 42 }
}
```

### General limits (per window, configurable via env)

| Client type   | Read (GET/HEAD) | Write (POST/PUT/PATCH/DELETE) |
|---------------|-----------------|-------------------------------|
| Anonymous IP  | 1 000 req/min   | 100 req/window                |
| Authenticated | 1 000 req/min   | 300 req/window                |
| Enterprise    | 100 000 req/win | 100 000 req/window            |

### Publish endpoint — `POST /api/contracts`

Publishing a contract is the most resource-intensive write operation. It gets a
dedicated, tighter bucket so a single IP cannot flood the registry with junk
entries.

| Client type   | Default limit     | Env var                              |
|---------------|-------------------|--------------------------------------|
| Anonymous IP  | **5** per window  | `RATE_LIMIT_PUBLISH_ANON_PER_WINDOW` |
| Authenticated | **30** per window | `RATE_LIMIT_PUBLISH_AUTH_PER_WINDOW` |

### Search / list endpoints

Aggressive scraping of search is rate-limited independently of the general read
quota. The following paths share the same search bucket per client:

- `GET /api/contracts`
- `GET /api/v1/contracts/search`
- `GET /api/contracts/suggestions`
- `GET /api/v1/contracts/trending`

| Client type   | Default limit      | Env var                              |
|---------------|--------------------|--------------------------------------|
| Anonymous IP  | **100** per window | `RATE_LIMIT_SEARCH_ANON_PER_WINDOW`  |
| Authenticated | **500** per window | `RATE_LIMIT_SEARCH_AUTH_PER_WINDOW`  |

### Exempt paths

The following paths bypass rate limiting entirely:

- `/health*` — load-balancer health probes
- `/metrics` — Prometheus scrape
- `/api/admin/*` — internal operator endpoints

### Trusted clients

Specific IPs or API keys can be whitelisted to bypass all limits:

```
RATE_LIMIT_TRUSTED_IPS=10.0.0.1,10.0.0.2
RATE_LIMIT_TRUSTED_API_KEYS=my-internal-service-key
```

See `.env.example` for the full list of configurable variables.
