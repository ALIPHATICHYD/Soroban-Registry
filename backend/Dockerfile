FROM rust:1.85-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev build-essential ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests for layer caching
COPY backend/Cargo.toml backend/Cargo.lock* backend/
COPY backend/api/Cargo.toml backend/api/
COPY backend/shared/Cargo.toml backend/shared/
COPY backend/indexer/Cargo.toml backend/indexer/
COPY backend/verifier/Cargo.toml backend/verifier/
COPY backend/seeder/Cargo.toml backend/seeder/
COPY backend/contract_abi/Cargo.toml backend/contract_abi/
COPY soroban-registry/Cargo.toml soroban-registry/
COPY soroban-registry/crates/ soroban-registry/crates/

# Copy full source
COPY backend/ backend/

RUN cd backend && cargo build --release --bin api --bin indexer

# ── Runtime ────────────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder --chown=nonroot:nonroot /app/backend/target/release/api /app/api
COPY --from=builder --chown=nonroot:nonroot /app/backend/target/release/indexer /app/indexer

USER nonroot:nonroot
EXPOSE 3001

CMD ["/app/api"]
