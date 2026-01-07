FROM fedora:43
ENV PATH="/root/.cargo/bin:${PATH}"

RUN dnf -y update \
  && dnf -y install \
    curl \
    gcc \
    gcc-c++ \
    make \
    nodejs \
    npm \
    pkgconf \
    python3 \
  && dnf clean all

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal
RUN rustup target add wasm32-unknown-unknown
RUN cargo install worker-build

RUN npm install -g wrangler

WORKDIR /work

EXPOSE 8787
