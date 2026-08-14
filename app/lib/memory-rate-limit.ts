/** Rate-limit en mémoire (best-effort par instance), aligné chatbot / portail parents. */

export function clientIpFromRequest(req: Request): string {
  return (
    req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ||
    req.headers.get("x-real-ip")?.trim() ||
    "unknown"
  );
}

export function createMemoryRateLimiter(options: { windowMs: number; max: number }) {
  const hits = new Map<string, { count: number; resetAt: number }>();
  return {
    allow(key: string): boolean {
      const now = Date.now();
      const cur = hits.get(key);
      if (!cur || cur.resetAt < now) {
        hits.set(key, { count: 1, resetAt: now + options.windowMs });
        return true;
      }
      if (cur.count >= options.max) return false;
      cur.count += 1;
      return true;
    },
  };
}
