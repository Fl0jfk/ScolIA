import "server-only";

type Bucket = { count: number; resetAt: number };

const buckets = new Map<string, Bucket>();

/** Rate-limit mémoire (par instance conteneur). Suffisant pour freiner le brute-force local. */
export function consumeRateLimit(opts: {
  key: string;
  limit: number;
  windowMs: number;
}): { ok: true } | { ok: false; retryAfterSec: number } {
  const now = Date.now();
  const current = buckets.get(opts.key);
  if (!current || current.resetAt <= now) {
    buckets.set(opts.key, { count: 1, resetAt: now + opts.windowMs });
    return { ok: true };
  }
  if (current.count >= opts.limit) {
    return { ok: false, retryAfterSec: Math.max(1, Math.ceil((current.resetAt - now) / 1000)) };
  }
  current.count += 1;
  return { ok: true };
}
