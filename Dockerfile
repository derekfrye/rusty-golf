# --- Builder stage ---
FROM fedora:43 AS builder

# Install Rust, Cargo, and build dependencies
RUN dnf -y update && \
    dnf -y install openssl-devel pkgconf gcc sqlite-devel sqlite && \
    dnf clean all

WORKDIR /usr/src/app

RUN curl --proto '=https' --tlsv1.3 -sSf https://sh.rustup.rs -o rust.sh \
 && chmod +x rust.sh \
 && ./rust.sh -y --profile minimal --default-toolchain stable --no-modify-path \
 && rm rust.sh

ENV PATH=/root/.cargo/bin:${PATH}

COPY Cargo.toml Cargo.lock ./
COPY actix ./actix
COPY core ./core
COPY static ./static

# Optional clean to reduce caching issues
# RUN cargo clean

# Build the release binary
RUN cargo build -p rusty-golf-actix --release

# --- Runtime stage ---
FROM fedora:43

# Install tini and runtime dependencies only
RUN dnf -y install tini openssl curl && \
    dnf clean all

# Create a non-root user
# RUN useradd -m -U appuser

# WORKDIR /home/appuser
WORKDIR /usr/src/app

# Copy the binary from the builder
COPY --from=builder /usr/src/app/target/release/rusty-golf-actix /usr/src/app/rusty-golf-actix
COPY --from=builder /usr/src/app/static /usr/src/app/static

# Set permissions
# RUN chown appuser:appuser /usr/local/bin/rusty-golf

# Drop privileges
# USER appuser

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:5201/health || exit 1

WORKDIR /usr/src/app

# Use tini as the init system
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/src/app/rusty-golf-actix"]

# Default command
CMD ["--db-type=sqlite"]
