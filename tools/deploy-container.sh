#!/bin/bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE_NAME="msi-dergisi-platform"
CONTAINER_NAME="msi-web"
TAG="${1:-$(git -C "$PROJECT_DIR" describe --tags --always 2>/dev/null || echo latest)}"

echo "==> Building ${IMAGE_NAME}:${TAG} ..."
podman build -t "${IMAGE_NAME}:${TAG}" -f "$PROJECT_DIR/Containerfile" "$PROJECT_DIR"

echo "==> Stopping existing container (if any) ..."
podman stop "$CONTAINER_NAME" 2>/dev/null || true
podman rm "$CONTAINER_NAME" 2>/dev/null || true

echo "==> Starting container with podman-compose ..."
cd "$PROJECT_DIR"
podman-compose up -d

echo "==> Waiting for health check ..."
for i in $(seq 1 30); do
 if curl -sf http://127.0.0.1:8090/api/v1/health >/dev/null 2>&1; then
 echo "==> Container is healthy!"
 exit 0
 fi
 sleep 2
done

echo "ERROR: Container failed health check after 60s"
podman logs "$CONTAINER_NAME"
exit 1