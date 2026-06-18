# syntax=docker/dockerfile:1.7

FROM rust:1.85-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY config ./config
RUN --mount=type=cache,id=rusty-mesh-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=rusty-mesh-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=rusty-mesh-target,target=/app/target \
    mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && printf '\n' > src/lib.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src

RUN --mount=type=cache,id=rusty-mesh-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=rusty-mesh-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=rusty-mesh-target,target=/app/target \
    touch src/main.rs src/lib.rs \
    && \
    cargo build --release \
    && cp /app/target/release/rusty-mesh /usr/local/bin/rusty-mesh

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/local/bin/rusty-mesh /usr/local/bin/rusty-mesh
COPY config ./config

ENV APP__ENV=production
ENV APP__SERVER__HOST=0.0.0.0
ENV APP__SERVER__PORT=3080

EXPOSE 3080

CMD ["rusty-mesh"]
