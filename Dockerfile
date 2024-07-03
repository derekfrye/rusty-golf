FROM rust:latest

WORKDIR /usr/src/app

# pre-copy/cache go.mod for pre-downloading dependencies and only redownloading them in subsequent builds if they change
# COPY go.mod go.sum ./
# RUN go mod download && go mod verify

COPY src/ .
# COPY templates/ .
COPY src/* ./src/
# COPY templates/* ./templates/
COPY Cargo.toml .
# RUN go build -v -o /usr/local/bin/app ./...
RUN cargo install --path .

CMD ["rusty-golf"]