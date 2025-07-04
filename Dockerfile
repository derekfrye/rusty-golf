# --- Builder stage ---
FROM fedora:42 AS builder

# Install Rust, Cargo, and build dependencies
RUN dnf -y update && \
    dnf -y install rust cargo openssl-devel pkgconf gcc sqlite-devel sqlite && \
    dnf clean all

WORKDIR /usr/src/app

COPY . .

# Optional clean to reduce caching issues
RUN cargo clean

# Build the release binary
RUN cargo build --release

# --- Runtime stage ---
FROM fedora:42

# Install tini and runtime dependencies only
RUN dnf -y install tini openssl curl && \
    dnf clean all

# Create a non-root user
# RUN useradd -m -U appuser

# WORKDIR /home/appuser

# Copy the binary from the builder
COPY --from=builder /usr/src/app/target/release/rusty-golf /usr/src/app/rusty-golf
COPY --from=builder /usr/src/app/static /usr/src/app/static

# Set permissions
# RUN chown appuser:appuser /usr/local/bin/rusty-golf

# Drop privileges
# USER appuser

WORKDIR /usr/src/app

# Use tini as the init system
ENTRYPOINT ["/usr/bin/tini", "--"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 CMD curl -f http://localhost:8081/health || exit 1

# Default command
CMD ["/usr/src/app/rusty-golf"]
