FROM rust:latest

WORKDIR /usr/src/app

COPY sqlx-middleware /usr/src/sqlx-middleware
COPY . .
RUN cargo clean
RUN cargo build --release
RUN cargo install --path .

CMD ["rusty-golf"]