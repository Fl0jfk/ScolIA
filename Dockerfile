# ============================================================
# Dockerfile — DocsLaPro Next.js (Scaleway Serverless Containers)
# Build : obligatoirement --platform=linux/amd64 depuis un Mac ARM
# ============================================================

FROM node:24-bookworm-slim AS builder
WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .

# NEXT_PUBLIC_* sont figées au BUILD Next.js — à passer en --build-arg
ARG NEXT_PUBLIC_APP_URL
ARG NEXT_PUBLIC_PLATFORM_APP_URL
ARG NEXT_PUBLIC_AFTER_SIGN_IN_URL=/dashboard
ARG NEXT_PUBLIC_AFTER_SIGN_UP_URL=/dashboard
ARG NEXT_PUBLIC_SCOLA_IMAGE_CDN_HOST
ARG PLATFORM_APP_URL
ARG PLATFORM_HOSTNAMES
ARG GIT_SHA=unknown

ENV NEXT_PUBLIC_APP_URL=$NEXT_PUBLIC_APP_URL
ENV NEXT_PUBLIC_PLATFORM_APP_URL=$NEXT_PUBLIC_PLATFORM_APP_URL
ENV NEXT_PUBLIC_AFTER_SIGN_IN_URL=$NEXT_PUBLIC_AFTER_SIGN_IN_URL
ENV NEXT_PUBLIC_AFTER_SIGN_UP_URL=$NEXT_PUBLIC_AFTER_SIGN_UP_URL
ENV NEXT_PUBLIC_SCOLA_IMAGE_CDN_HOST=$NEXT_PUBLIC_SCOLA_IMAGE_CDN_HOST
ENV PLATFORM_APP_URL=$PLATFORM_APP_URL
ENV PLATFORM_HOSTNAMES=$PLATFORM_HOSTNAMES
ENV GIT_SHA=$GIT_SHA
ENV NEXT_TELEMETRY_DISABLED=1

RUN npm run build


FROM node:24-bookworm-slim AS runner
WORKDIR /app

ARG GIT_SHA=unknown
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV GIT_SHA=$GIT_SHA
ENV NODE_OPTIONS=--disable-warning=DEP0040
ENV PORT=8080
ENV HOSTNAME=0.0.0.0

RUN apt-get update && apt-get install -y --no-install-recommends \
    fontconfig \
    fonts-dejavu-core \
    ca-certificates \
  && rm -rf /var/lib/apt/lists/*

RUN groupadd --system --gid 1001 nodejs && \
    useradd --system --uid 1001 --gid nodejs nextjs

COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static
COPY --from=builder --chown=nextjs:nodejs /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/assets ./assets
COPY --from=builder --chown=nextjs:nodejs /app/node_modules/@napi-rs ./node_modules/@napi-rs
COPY --from=builder --chown=nextjs:nodejs /app/node_modules/pdfjs-dist ./node_modules/pdfjs-dist
COPY --from=builder --chown=nextjs:nodejs /app/drizzle ./drizzle
COPY --from=builder --chown=nextjs:nodejs /app/scripts/apply-migrations-direct.mjs ./scripts/apply-migrations-direct.mjs
COPY --from=builder --chown=nextjs:nodejs /app/scripts/assert-local-database.mjs ./scripts/assert-local-database.mjs
COPY --from=builder --chown=nextjs:nodejs --chmod=755 /app/scripts/docker-entrypoint.sh ./scripts/docker-entrypoint.sh
COPY --from=builder --chown=nextjs:nodejs /app/node_modules/postgres ./node_modules/postgres

USER nextjs

EXPOSE 8080

ENTRYPOINT ["/app/scripts/docker-entrypoint.sh"]
