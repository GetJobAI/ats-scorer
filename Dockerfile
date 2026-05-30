FROM lukemathwalker/cargo-chef:latest-rust-slim-trixie AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Install before any COPY so this layer is never invalidated by source changes
RUN apt-get update && apt-get install -y pkg-config libssl-dev cmake mold g++ \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cargo-registry \
    --mount=type=cache,target=/app/target,id=ats-scorer-target \
    RUSTFLAGS="-C link-arg=-fuse-ld=mold" \
    cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cargo-registry \
    --mount=type=cache,target=/app/target,id=ats-scorer-target \
    RUSTFLAGS="-C link-arg=-fuse-ld=mold" \
    cargo build --release --bin ats-scorer \
    && cp /app/target/release/ats-scorer /app/ats-scorer-bin

FROM debian:trixie-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/ats-scorer-bin /usr/local/bin/ats-scorer
COPY .env.example .env

ENTRYPOINT ["ats-scorer"]
CMD ["serve"]
