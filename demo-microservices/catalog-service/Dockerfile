# syntax=docker/dockerfile:1.7

FROM rust:1.85-bookworm AS builder

WORKDIR /app

COPY Cargo.toml ./
RUN --mount=type=cache,id=demo-catalog-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=demo-catalog-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=demo-catalog-target,target=/app/target \
    mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src

RUN --mount=type=cache,id=demo-catalog-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=demo-catalog-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=demo-catalog-target,target=/app/target \
    touch src/main.rs \
    && \
    cargo build --release \
    && cp /app/target/release/demo-catalog-service /usr/local/bin/demo-catalog-service

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/local/bin/demo-catalog-service /usr/local/bin/demo-catalog-service

ENV SERVICE_NAME=catalog-service
ENV SERVICE_VERSION=1.0.0
ENV SERVICE_BIND_HOST=0.0.0.0
ENV SERVICE_PORT=0
ENV MESH_URL=http://rusty-mesh:3080
ENV HEARTBEAT_INTERVAL_SECS=5

CMD ["demo-catalog-service"]
