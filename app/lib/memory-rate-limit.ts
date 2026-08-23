/** Rate-limit IP — Postgres durable (repli mémoire si BDD indisponible). */

import { consumeRateLimit } from "@/app/lib/rate-limit";

export function clientIpFromRequest(req: Request): string {
  return (
    req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ||
    req.headers.get("x-real-ip")?.trim() ||
    "unknown"
  );
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
