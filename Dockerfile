# syntax=docker/dockerfile:1.7

# ============================================
# Stage 1: Build the application
# ============================================
FROM rust:1.85-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    clang \
    pkg-config \
    libclang-dev \
    libxml2-dev \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy everything (simpler approach)
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY registry/ registry/
COPY assets/ assets/

# Build the application and operational helper binaries.
# Keep Cargo's downloads and compiled target between source-only rebuilds.
RUN --mount=type=cache,id=e2br3-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=e2br3-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=e2br3-cargo-target,target=/app/target,sharing=locked \
    cargo build --release --jobs 4 --package web-server --package terminology-loader \
    && mkdir -p /app/build-artifacts \
    && cp target/release/web-server target/release/terminology-loader /app/build-artifacts/

# ============================================
# Stage 2: Create minimal runtime image
# ============================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libxml2 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd --create-home --shell /bin/bash appuser

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/build-artifacts/web-server /app/web-server
COPY --from=builder /app/build-artifacts/terminology-loader /app/terminology-loader

# Copy web-folder if it exists (static files)
COPY --chown=appuser:appuser web-folder/ /app/web-folder/
COPY db/migrations/ /app/db/migrations/
COPY deploy/ec2/migrate-rds.sh /app/deploy/ec2/migrate-rds.sh

# Set ownership
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose the port
EXPOSE 8080

# Set environment variables (override in deployment)
ENV RUST_LOG="web_server=info,lib_core=info,lib_web=info"
ENV SERVICE_WEB_FOLDER="/app/web-folder/"

# Run the application
CMD ["./web-server"]
