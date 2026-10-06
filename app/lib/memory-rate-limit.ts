/** Rate-limit IP — Postgres durable (repli mémoire si BDD indisponible). */

import { consumeRateLimit } from "@/app/lib/rate-limit";

/**
 * IP client derrière reverse proxy (Scaleway / ingress).
 * On prend la **dernière** entrée de `X-Forwarded-For` : c’est celle ajoutée par notre proxy de
 * confiance, pas la première (spoofable par le client). Repli `X-Real-IP` puis « unknown ».
 */
export function clientIpFromRequest(req: Request): string {
  const xff = req.headers.get("x-forwarded-for");
  if (xff) {
    const parts = xff.split(",").map((p) => p.trim()).filter(Boolean);
    if (parts.length > 0) {
      return parts[parts.length - 1]!;
    }
  }
  return req.headers.get("x-real-ip")?.trim() || "unknown";
}

export function createMemoryRateLimiter(options: { windowMs: number; max: number }) {
  return {
    async allow(key: string): Promise<boolean> {
      const result = await consumeRateLimit({
        key: `rl:${options.windowMs}:${options.max}:${key}`,
        limit: options.max,
        windowMs: options.windowMs,
      });
      return result.ok;
    },
  };
}

export function createSlidingWindowRateLimiter(options: { windowMs: number; max: number }) {
  return createMemoryRateLimiter(options);
}
