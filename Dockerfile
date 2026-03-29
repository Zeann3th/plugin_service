FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

RUN apt-get update && apt-get install -y libpq-dev && rm -rf /var/lib/apt/lists/*

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin plugin_service && \
    cp target/release/plugin_service /usr/local/bin/plugin-service

FROM ubuntu:24.04 AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/plugin-service /usr/local/bin/plugin-service
COPY --from=builder /app/migrations /app/migrations

RUN useradd -ms /bin/bash appuser && \
    chown -R appuser:appuser /app
USER appuser

ENV RUST_LOG=info
EXPOSE 7554
ENTRYPOINT ["/usr/local/bin/plugin-service"]
