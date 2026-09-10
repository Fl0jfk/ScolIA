import "server-only";

import Redis from "ioredis";

/**
 * Client Valkey (protocole Redis-compatible).
 * URL : VALKEY_URL (préféré) ou REDIS_URL (Scaleway Managed Redis / compat).
 * Absent → l’app fonctionne sans cache partagé (repli mémoire / Postgres).
 */

let client: Redis | null | undefined;
let loggedMissing = false;
let loggedReady = false;

export function isValkeyConfigured(): boolean {
  return Boolean(resolveValkeyUrl());
}

function resolveValkeyUrl(): string | null {
  const url =
    process.env.VALKEY_URL?.trim() ||
    process.env.REDIS_URL?.trim() ||
    "";
  return url || null;
}

/** Client partagé, ou null si non configuré / indisponible. */
export function getValkey(): Redis | null {
  if (client !== undefined) return client;

  const url = resolveValkeyUrl();
  if (!url) {
    if (!loggedMissing) {
      loggedMissing = true;
      console.info(
        "[valkey] non configuré (VALKEY_URL / REDIS_URL) — caches partagés désactivés",
      );
    }
    client = null;
    return null;
  }

  try {
    const redis = new Redis(url, {
      maxRetriesPerRequest: 1,
      enableReadyCheck: true,
      lazyConnect: true,
      connectTimeout: 5_000,
      // Ne pas faire planter Next si Valkey down.
      retryStrategy(times) {
        if (times > 8) return null;
        return Math.min(times * 200, 2_000);
      },
    });

    redis.on("error", (err) => {
      console.error("[valkey]", err instanceof Error ? err.message : err);
    });
    redis.on("ready", () => {
      if (!loggedReady) {
        loggedReady = true;
        console.info("[valkey] connecté");
      }
    });

    void redis.connect().catch((err) => {
      console.error(
        "[valkey] connexion échouée — repli sans cache partagé",
        err instanceof Error ? err.message : err,
      );
    });

    client = redis;
    return client;
  } catch (error) {
    console.error("[valkey] init", error);
    client = null;
    return null;
  }
}

export async function valkeyGet(key: string): Promise<string | null> {
  const v = getValkey();
  if (!v) return null;
  try {
    return await v.get(key);
  } catch (error) {
    console.error("[valkey] get", key, error);
    return null;
  }
}

export async function valkeySet(
  key: string,
  value: string,
  ttlSeconds?: number,
): Promise<boolean> {
  const v = getValkey();
  if (!v) return false;
  try {
    if (ttlSeconds && ttlSeconds > 0) {
      await v.set(key, value, "EX", Math.floor(ttlSeconds));
    } else {
      await v.set(key, value);
    }
    return true;
  } catch (error) {
    console.error("[valkey] set", key, error);
    return false;
  }
}

export async function valkeyDel(...keys: string[]): Promise<void> {
  if (keys.length === 0) return;
  const v = getValkey();
  if (!v) return;
  try {
    await v.del(...keys);
  } catch (error) {
    console.error("[valkey] del", error);
  }
}

export async function valkeyGetJson<T>(key: string): Promise<T | null> {
  const raw = await valkeyGet(key);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

export async function valkeySetJson(
  key: string,
  value: unknown,
  ttlSeconds?: number,
): Promise<boolean> {
  try {
    return await valkeySet(key, JSON.stringify(value), ttlSeconds);
  } catch (error) {
    console.error("[valkey] setJson", key, error);
    return false;
  }
}

/**
 * Cache-aside : lit Valkey, sinon charge via `loader`, stocke le résultat.
 * Si Valkey down → appelle toujours `loader` (pas de panne).
 */
export async function valkeyCached<T>(opts: {
  key: string;
  ttlSeconds: number;
  loader: () => Promise<T>;
}): Promise<T> {
  const hit = await valkeyGetJson<T>(opts.key);
  if (hit !== null && hit !== undefined) return hit;
  const fresh = await opts.loader();
  void valkeySetJson(opts.key, fresh, opts.ttlSeconds);
  return fresh;
}

export async function valkeyIncr(
  key: string,
  ttlSeconds: number,
): Promise<number | null> {
  const v = getValkey();
  if (!v) return null;
  try {
    const count = await v.incr(key);
    if (count === 1) {
      await v.expire(key, Math.max(1, Math.floor(ttlSeconds)));
    }
    return count;
  } catch (error) {
    console.error("[valkey] incr", key, error);
    return null;
  }
}

/** Secondary storage Better-Auth (sessions / rate-limit auth). */
export function createValkeySecondaryStorage(keyPrefix = "ba:"): {
  get: (key: string) => Promise<string | null>;
  getAndDelete: (key: string) => Promise<string | null>;
  set: (key: string, value: string, ttl?: number) => Promise<void>;
  delete: (key: string) => Promise<void>;
  increment: (key: string, ttl: number) => Promise<number>;
} {
  const p = (key: string) => `${keyPrefix}${key}`;
  return {
    async get(key) {
      return valkeyGet(p(key));
    },
    async getAndDelete(key) {
      const v = getValkey();
      if (!v) return null;
      try {
        const full = p(key);
        const value = await v.get(full);
        if (value !== null) await v.del(full);
        return value;
      } catch (error) {
        console.error("[valkey] getAndDelete", error);
        return null;
      }
    },
    async set(key, value, ttl) {
      await valkeySet(p(key), value, ttl);
    },
    async delete(key) {
      await valkeyDel(p(key));
    },
    async increment(key, ttl) {
      const n = await valkeyIncr(p(key), ttl);
      if (n === null) {
        throw new Error("Valkey indisponible pour increment");
      }
      return n;
    },
  };
}

/** Supprime les clés matching `prefix*` (SCAN — safe prod). */
export async function valkeyDeleteByPrefix(prefix: string): Promise<number> {
  const v = getValkey();
  if (!v || !prefix) return 0;
  let deleted = 0;
  try {
    let cursor = "0";
    do {
      const [next, keys] = await v.scan(cursor, "MATCH", `${prefix}*`, "COUNT", 100);
      cursor = next;
      if (keys.length > 0) {
        deleted += await v.del(...keys);
      }
    } while (cursor !== "0");
  } catch (error) {
    console.error("[valkey] deleteByPrefix", prefix, error);
  }
  return deleted;
}

export async function closeValkey(): Promise<void> {
  if (client) {
    try {
      await client.quit();
    } catch {
      client.disconnect();
    }
  }
  client = undefined;
  loggedReady = false;
}
