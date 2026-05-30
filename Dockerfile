FROM rust:1.85-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY config ./config
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/rusty-mesh /usr/local/bin/rusty-mesh
COPY config ./config

ENV APP__ENV=production
ENV APP__SERVER__HOST=0.0.0.0
ENV APP__SERVER__PORT=3080

EXPOSE 3080

CMD ["rusty-mesh"]
