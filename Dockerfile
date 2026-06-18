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

# Build the application and operational helper binaries.
RUN cargo build --release --package web-server --package terminology-loader

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
COPY --from=builder /app/target/release/web-server /app/web-server
COPY --from=builder /app/target/release/terminology-loader /app/terminology-loader

# Copy web-folder if it exists (static files)
COPY --chown=appuser:appuser web-folder/ /app/web-folder/

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
