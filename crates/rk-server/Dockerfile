# Multi-stage build for rk-server
# Build context must be the workspace root (so rk-shared is accessible).
# Example: docker build -f crates/rk-server/Dockerfile .

# -- Stage 1: Build ----------------------------------------------------------
FROM rust:1-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace manifests first to cache dependency downloads.
COPY Cargo.toml Cargo.lock ./
COPY crates/rk-shared/Cargo.toml crates/rk-shared/Cargo.toml
COPY crates/rk-server/Cargo.toml crates/rk-server/Cargo.toml
COPY crates/rk-client/Cargo.toml crates/rk-client/Cargo.toml

# Create stub source files so cargo can resolve the workspace and fetch deps.
RUN mkdir -p crates/rk-shared/src && echo "" > crates/rk-shared/src/lib.rs \
    && mkdir -p crates/rk-server/src && echo "fn main() {}" > crates/rk-server/src/main.rs \
    && mkdir -p crates/rk-client/src && echo "fn main() {}" > crates/rk-client/src/main.rs

RUN cargo build --release --package rk-server 2>/dev/null || true

# Copy real source code and rebuild.
COPY crates/rk-shared crates/rk-shared
COPY crates/rk-server crates/rk-server
COPY crates/rk-client crates/rk-client

# Touch main.rs so cargo detects the change and recompiles.
RUN touch crates/rk-server/src/main.rs

RUN cargo build --release --package rk-server

# -- Stage 2: Runtime --------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary.
COPY --from=builder /build/target/release/rk-server /app/rk-server

# Copy migrations -- the server loads them from ./migrations at startup.
COPY crates/rk-server/migrations /app/migrations

EXPOSE 3000

CMD ["/app/rk-server"]
