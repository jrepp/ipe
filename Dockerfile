# Stage 1: Build the IPE policy engine
FROM rust:1.83-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libclang-dev \
    llvm-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary (only ipe-server)
RUN cargo build --release --package ipe-server

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -u 1000 -g nogroup ipe

# Create directories for sockets and data
RUN mkdir -p /var/run/ipe /var/lib/ipe /etc/ipe \
    && chown -R ipe:nogroup /var/run/ipe /var/lib/ipe /etc/ipe

WORKDIR /app

# Copy server binary from builder
COPY --from=builder /app/target/release/ipe-server /usr/local/bin/ipe-server

# Copy default configuration
COPY --chown=ipe:nogroup deploy/config/default.toml /etc/ipe/config.toml

# Switch to non-root user
USER ipe

# Expose ports
# Data plane TCP port
EXPOSE 9001
# Metrics (if enabled via HTTP)
EXPOSE 9090

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD test -S /var/run/ipe/eval.sock || exit 1

# Default command
ENTRYPOINT ["/usr/local/bin/ipe-server"]
CMD ["--config", "/etc/ipe/config.toml"]
