FROM rust:1.84.0

# Install tini
RUN apt-get update && apt-get install -y tini

# Create a non-root user and switch to it
RUN useradd -m appuser
USER appuser

WORKDIR /usr/src/app

COPY sqlx-middleware /usr/src/sqlx-middleware
COPY . .
RUN cargo clean
RUN cargo build --release
RUN cargo install --path .

# Use tini as the init system
ENTRYPOINT ["/usr/bin/tini", "--"]

# Add HEALTHCHECK instruction
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 CMD ["curl", "-f", "http://localhost:8081/health"] || exit 1

CMD ["rusty-golf"]