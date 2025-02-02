FROM rust:1.84.0

# Install tini
# trunk-ignore(hadolint/DL3008)
# trunk-ignore(hadolint/DL3015)
RUN apt-get update && apt-get install -y tini && rm -rf /var/lib/apt/lists/*

# Create a non-root user and switch to it
# RUN useradd -m appuser
# USER appuser

WORKDIR /usr/src/app

COPY sql-middleware /usr/src/sql-middleware
COPY . .
RUN cargo clean
# trunk-ignore(hadolint/DL3059)
RUN cargo build --release
# trunk-ignore(hadolint/DL3059)
RUN cargo install --path .

# Use tini as the init system
ENTRYPOINT ["/usr/bin/tini", "--"]

# Add HEALTHCHECK instruction
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 CMD curl -f http://localhost:8081/health || exit 1

CMD ["rusty-golf"]