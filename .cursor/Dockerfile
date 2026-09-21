# Image de base Cloud Agent — Node 24 + outillage SQL / PATH.
FROM node:24-bookworm-slim

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    git curl ca-certificates sudo \
    postgresql postgresql-contrib postgresql-client \
    python3 \
  && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.local/bin:/root/bin:/usr/local/bin:${PATH}"
