# Stage 1: Chef planner — cache dependency graph
FROM lukemathwalker/cargo-chef:latest-rust-1.85 AS chef
WORKDIR /app

# Stage 2: Plan dependencies
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build dependencies (cached layer — only rebuilds when deps change)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 4: Build application
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin rustbill-server

# Stage 5: Minimal runtime image
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl && \
    rm -rf /var/lib/apt/lists/*

RUN groupadd --system --gid 1001 billing && \
    useradd --system --uid 1001 --gid billing billing

COPY --from=builder /app/target/release/rustbill-server /usr/local/bin/
COPY config/ /etc/rustbill/config/

USER billing

EXPOSE 3001
ENV RUST_LOG=rustbill_server=info,rustbill_core=info,tower_http=info
ENV RUN_MODE=production

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

CMD ["rustbill-server"]
