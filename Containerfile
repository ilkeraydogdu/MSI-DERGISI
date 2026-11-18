# MSI Dergisi Platform — Multi-stage Podman/Docker build
# Stage 1: Build Rust binary
FROM rust:1.99-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release || true
RUN rm -rf src

COPY src/ src/
RUN touch src/main.rs && cargo build --release

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /usr/sbin/nologin -m appuser

WORKDIR /app

COPY --from=builder /build/target/release/msi-dergisi-platform /usr/local/bin/msi-dergisi-platform
COPY public/ ./public/

RUN mkdir -p /app/storage/db /app/storage/edergi /app/storage/artifacts /app/storage/seed \
    && chown -R appuser:appuser /app

USER appuser

EXPOSE 8090

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:8090/api/v1/health || exit 1

ENTRYPOINT ["/usr/local/bin/msi-dergisi-platform"]