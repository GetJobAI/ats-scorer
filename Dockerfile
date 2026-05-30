FROM lukemathwalker/cargo-chef:latest-rust-slim-trixie AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN apt-get update && apt-get install -y pkg-config libssl-dev cmake lld g++
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin ats-scorer

FROM debian:trixie-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ats-scorer /usr/local/bin/
COPY .env.example .env

ENTRYPOINT ["ats-scorer"]
CMD ["serve"]
