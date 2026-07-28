#!/usr/bin/env bash
set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE_NAME="soroban-registry-test:latest"
DOCKERFILE="$REPO_DIR/backend/Dockerfile.test"

echo "Building Docker image $IMAGE_NAME..."
docker build -t $IMAGE_NAME -f "$DOCKERFILE" "$REPO_DIR"

echo "Running backend tests in container (mounted repo)..."
docker run --rm -v "$REPO_DIR":/work -w /work/backend $IMAGE_NAME bash -lc "cargo test -p api onchain_verification --lib -- --nocapture"
