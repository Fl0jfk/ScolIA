# Image de base Cloud Agent — Node 24 (remplace l'ancien pin Node 20).
FROM node:24-bookworm-slim

RUN apt-get update \
  && apt-get install -y --no-install-recommends git curl ca-certificates sudo \
  && rm -rf /var/lib/apt/lists/*
