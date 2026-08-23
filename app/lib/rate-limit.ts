import "server-only";

import { eq, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { appRateLimit } from "@/db/schema";

type Bucket = { count: number; resetAt: number };

/** Repli mémoire si BDD indisponible (dev / bootstrap). */
const memoryBuckets = new Map<string, Bucket>();

function asRetryAfterSec(resetAtMs: number, now = Date.now()): number {
  if (!Number.isFinite(resetAtMs)) return 60;
  return Math.max(1, Math.ceil((resetAtMs - now) / 1000));
}

function toResetMs(value: unknown): number {
  if (value instanceof Date) {
    const ms = value.getTime();
    return Number.isFinite(ms) ? ms : NaN;
  }
  if (typeof value === "string" || typeof value === "number") {
    const ms = new Date(value).getTime();
    return Number.isFinite(ms) ? ms : NaN;
  }
  return NaN;
}

function consumeMemory(opts: {
  key: string;
  limit: number;
  windowMs: number;
}): { ok: true; remaining: number } | { ok: false; retryAfterSec: number } {
  const now = Date.now();
  const current = memoryBuckets.get(opts.key);
  if (!current || current.resetAt <= now) {
    memoryBuckets.set(opts.key, { count: 1, resetAt: now + opts.windowMs });
    return { ok: true, remaining: opts.limit - 1 };
  }
  if (current.count >= opts.limit) {
    return { ok: false, retryAfterSec: asRetryAfterSec(current.resetAt, now) };
  }
  current.count += 1;
  return { ok: true, remaining: opts.limit - current.count };
}

/**
 * Rate-limit durable (Postgres) — partagé entre réplicas Scaleway.
 */
export async function consumeRateLimit(opts: {
  key: string;
  limit: number;
  windowMs: number;
}): Promise<{ ok: true; remaining: number } | { ok: false; retryAfterSec: number }> {
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

    const existingResetMs = existing ? toResetMs(existing.resetAt) : NaN;
    const windowExpired = !existing || !Number.isFinite(existingResetMs) || existingResetMs <= now;

    if (windowExpired) {
      await db
        .insert(appRateLimit)
        .values({ key: opts.key, count: 1, resetAt: newResetAt })
        .onConflictDoUpdate({
          target: appRateLimit.key,
          set: { count: 1, resetAt: newResetAt },
        });
      return { ok: true, remaining: opts.limit - 1 };
    }

    const currentCount = Number(existing.count);
    if (Number.isFinite(currentCount) && currentCount >= opts.limit) {
      return { ok: false, retryAfterSec: asRetryAfterSec(existingResetMs, now) };
    }

    const [updated] = await db
      .update(appRateLimit)
      .set({ count: sql`${appRateLimit.count} + 1` })
      .where(eq(appRateLimit.key, opts.key))
      .returning({ count: appRateLimit.count, resetAt: appRateLimit.resetAt });

    const count = Number(updated?.count ?? currentCount + 1);
    const resetMs = toResetMs(updated?.resetAt ?? existing.resetAt);
    if (count > opts.limit) {
      return { ok: false, retryAfterSec: asRetryAfterSec(resetMs, now) };
    }
    return { ok: true, remaining: Math.max(0, opts.limit - count) };
  } catch (error) {
    console.error("[rate-limit] postgres fallback mémoire", error);
    return consumeMemory(opts);
  }
}

/** Efface un bucket (ex. après succès légitime). */
export async function clearRateLimit(key: string): Promise<void> {
  memoryBuckets.delete(key);
  if (!isDatabaseConfigured()) return;
  try {
    const db = getDb();
    await db.delete(appRateLimit).where(eq(appRateLimit.key, key));
  } catch (error) {
    console.error("[rate-limit] clear", error);
  }
}
