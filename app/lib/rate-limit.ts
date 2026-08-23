import "server-only";

import { eq, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { appRateLimit } from "@/db/schema";

type Bucket = { count: number; resetAt: number };

/** Repli mémoire si BDD indisponible (dev / bootstrap). */
const memoryBuckets = new Map<string, Bucket>();

function consumeMemory(opts: {
  key: string;
  limit: number;
  windowMs: number;
}): { ok: true } | { ok: false; retryAfterSec: number } {
  const now = Date.now();
  const current = memoryBuckets.get(opts.key);
  if (!current || current.resetAt <= now) {
    memoryBuckets.set(opts.key, { count: 1, resetAt: now + opts.windowMs });
    return { ok: true };
  }
  if (current.count >= opts.limit) {
    return { ok: false, retryAfterSec: Math.max(1, Math.ceil((current.resetAt - now) / 1000)) };
  }
  current.count += 1;
  return { ok: true };
}

/**
 * Rate-limit durable (Postgres) — partagé entre réplicas Scaleway.
 */
export async function consumeRateLimit(opts: {
  key: string;
  limit: number;
  windowMs: number;
}): Promise<{ ok: true } | { ok: false; retryAfterSec: number }> {
  if (!isDatabaseConfigured()) {
    return consumeMemory(opts);
  }

  const now = Date.now();
  const newResetAt = new Date(now + opts.windowMs);

  try {
    const db = getDb();
    const [existing] = await db
      .select()
      .from(appRateLimit)
      .where(eq(appRateLimit.key, opts.key))
      .limit(1);

    if (!existing || existing.resetAt.getTime() <= now) {
      await db
        .insert(appRateLimit)
        .values({ key: opts.key, count: 1, resetAt: newResetAt })
        .onConflictDoUpdate({
          target: appRateLimit.key,
          set: { count: 1, resetAt: newResetAt },
        });
      return { ok: true };
    }

    if (existing.count >= opts.limit) {
      return {
        ok: false,
        retryAfterSec: Math.max(1, Math.ceil((existing.resetAt.getTime() - now) / 1000)),
      };
    }

    const [updated] = await db
      .update(appRateLimit)
      .set({ count: sql`${appRateLimit.count} + 1` })
      .where(eq(appRateLimit.key, opts.key))
      .returning({ count: appRateLimit.count, resetAt: appRateLimit.resetAt });

    const count = updated?.count ?? existing.count + 1;
    if (count > opts.limit) {
      return {
        ok: false,
        retryAfterSec: Math.max(
          1,
          Math.ceil(((updated?.resetAt ?? existing.resetAt).getTime() - now) / 1000),
        ),
      };
    }
    return { ok: true };
  } catch (error) {
    console.error("[rate-limit] postgres fallback mémoire", error);
    return consumeMemory(opts);
  }
}
