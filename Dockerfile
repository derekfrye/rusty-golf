FROM rust:latest

# Install tini
RUN apt-get update && apt-get install -y tini

WORKDIR /usr/src/app

COPY sqlx-middleware /usr/src/sqlx-middleware
COPY . .
RUN cargo clean
RUN cargo build --release
RUN cargo install --path .

# Use tini as the init system
ENTRYPOINT ["/usr/bin/tini", "--"]

CMD ["rusty-golf"]