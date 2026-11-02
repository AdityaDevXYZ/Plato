# Multi-stage production build for PLATO API & Background DEE Worker
FROM rust:1.77-slim-bookworm AS builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace sources and manifests
COPY . .

# Build release binaries for both the API Gateway and the Asynchronous DEE Worker
RUN cargo build --release --bin api --bin worker

# Production runner image
FROM debian:bookworm-slim AS runner
WORKDIR /app

# Install runtime dependencies and curl for container health checks
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binaries from builder
COPY --from=builder /app/target/release/api /usr/local/bin/plato-api
COPY --from=builder /app/target/release/worker /usr/local/bin/plato-worker

# Create convenience symlinks
RUN ln -s /usr/local/bin/plato-api /usr/local/bin/api && \
    ln -s /usr/local/bin/plato-worker /usr/local/bin/worker

# Copy database migrations for automatic runtime schema migration
COPY migrations ./migrations

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=5s --retries=5 --start-period=5s \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["plato-api"]
