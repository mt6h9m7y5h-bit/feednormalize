# syntax=docker/dockerfile:1

# --- cargo-chef: cache dependency builds across source changes ---
FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ENV RUSTFLAGS="-D warnings"
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
# migrations/ must be present at compile time: sqlx::migrate!() embeds SQL in the binary.
COPY . .
RUN cargo build --release --bin feednormalize --locked

# --- minimal runtime ---
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

RUN useradd --system --no-create-home --uid 10001 appuser

COPY --from=builder /app/target/release/feednormalize /usr/local/bin/feednormalize
# Copied for visibility/tooling; runtime migrations use the compile-time embed.
COPY --from=builder /app/migrations ./migrations

RUN mkdir -p uploads && chown -R appuser:appuser /app

USER appuser

ENV HOST=0.0.0.0
ENV PORT=3000
EXPOSE 3000

CMD ["feednormalize"]
