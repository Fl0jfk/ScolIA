# syntax=docker/dockerfile:1
# Image de production du noyau ENT (`ent-api`).
# Build multi-étages : Rust (musl-free, rustls partout) → Debian slim.
#
#   docker build -t ent-api:local ent-rust
#   docker run --rm -p 8088:8088 \
#     -e ENT_DATABASE_URL='postgres://scola:***@db:5432/scola' \
#     ent-api:local
#
# Migrations : embarquées dans le binaire (`include_str!`) — appliquées au
# démarrage. `ENT_ENV=prod` ferme l'inscription libre et le labo (voir plus bas).

FROM rust:1.99-slim-bookworm AS builder
WORKDIR /app
# SHA git injecté dans le binaire (visible via GET /health) :
#   docker build --build-arg GIT_SHA="$(git rev-parse HEAD)" …
ARG GIT_SHA=dev
# `rust:slim` fournit gcc + libc pour les crates à composants natifs (ring).
COPY . .
RUN GIT_SHA="$GIT_SHA" cargo build --release -p ent-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin ent
WORKDIR /app
# Le binaire sert les statiques via un chemin absolu figé à la compilation
# (`concat!(env!("CARGO_MANIFEST_DIR"), "/static")`) : on copie donc au même
# emplacement `/app/crates/ent-api/static`.
COPY --from=builder /app/target/release/ent-api /usr/local/bin/ent-api
COPY --from=builder /app/crates/ent-api/static /app/crates/ent-api/static

ENV ENT_API_BIND=0.0.0.0:8088 \
    ENT_ENV=prod \
    RUST_LOG=info
USER ent
EXPOSE 8088

HEALTHCHECK --interval=30s --timeout=3s --start-period=30s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8088/health || exit 1

ENTRYPOINT ["/usr/local/bin/ent-api"]
