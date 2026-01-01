FROM node:20-bullseye

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/root/.cargo/bin:${PATH}"

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    curl \
    build-essential \
    pkg-config \
    python3 \
  && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal
RUN rustup target add wasm32-unknown-unknown
RUN cargo install worker-build

RUN npm install -g wrangler

WORKDIR /work

EXPOSE 8787
