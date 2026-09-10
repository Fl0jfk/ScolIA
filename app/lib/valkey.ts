import "server-only";

import Redis from "ioredis";

/**
 * Client Valkey / Managed Redis (protocole Redis).
 * URL : VALKEY_URL (préféré) ou REDIS_URL.
 * Absent / hors service → repli immédiat (pas d’attente longue).
 */

let client: Redis | null | undefined;
let loggedMissing = false;
let loggedReady = false;

/** Coupe-circuit : après échecs, on ignore Valkey un moment (évite +5s par clic). */
let circuitOpenUntil = 0;
let consecutiveFailures = 0;
const CIRCUIT_AFTER_FAILURES = 3;
const CIRCUIT_COOLDOWN_MS = 30_000;
const CMD_TIMEOUT_MS = 1_200;

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

function tripCircuit(reason: string): void {
  consecutiveFailures += 1;
  if (consecutiveFailures < CIRCUIT_AFTER_FAILURES) return;
  circuitOpenUntil = Date.now() + CIRCUIT_COOLDOWN_MS;
  console.warn(
    `[valkey] coupe-circuit ${CIRCUIT_COOLDOWN_MS / 1000}s — ${reason}`,
  );
}

function resetCircuit(): void {
  consecutiveFailures = 0;
  circuitOpenUntil = 0;
}

function circuitOpen(): boolean {
  return Date.now() < circuitOpenUntil;
}

async function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        timer = setTimeout(() => reject(new Error(`valkey timeout ${ms}ms`)), ms);
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

/** Client partagé, ou null si non configuré / coupe-circuit. */
export function getValkey(): Redis | null {
  if (circuitOpen()) return null;
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
    const useTls = url.startsWith("rediss://");
    const redis = new Redis(url, {
      maxRetriesPerRequest: 1,
      enableReadyCheck: true,
      lazyConnect: true,
      connectTimeout: 1_500,
      enableOfflineQueue: false,
      keepAlive: 10_000,
      ...(useTls
        ? {
            tls: {
              rejectUnauthorized:
                process.env.VALKEY_TLS_REJECT_UNAUTHORIZED !== "0",
            },
          }
        : {}),
      retryStrategy(times) {
        if (times > 4) {
          tripCircuit("retry épuisé");
          return null;
        }
        return Math.min(times * 150, 800);
      },
      reconnectOnError() {
        return true;
      },
    });

    redis.on("error", (err) => {
      const msg = err instanceof Error ? err.message : String(err);
      console.error("[valkey]", msg);
      if (/closed|ECONNRESET|ETIMEDOUT|ECONNREFUSED/i.test(msg)) {
        tripCircuit(msg);
      }
    });
    redis.on("ready", () => {
      resetCircuit();
      if (!loggedReady) {
        loggedReady = true;
        console.info("[valkey] connecté");
      }
    });
    redis.on("end", () => {
      tripCircuit("connexion fermée");
    });

    void redis.connect().catch((err) => {
      tripCircuit(err instanceof Error ? err.message : "connect failed");
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
    tripCircuit("init");
    return null;
  }
}

async function ensureConnected(v: Redis): Promise<boolean> {
  if (v.status === "ready") return true;
  if (
    v.status === "connecting" ||
    v.status === "connect" ||
    v.status === "wait" ||
    v.status === "end" ||
    v.status === "close"
  ) {
    try {
      await withTimeout(v.connect(), CMD_TIMEOUT_MS);
      // Relecture après await : TS ne ré-élargit pas le littéral de status.
      const status = v.status as string;
      return status === "ready";
    } catch {
      return false;
    }
  }
  return false;
}

async function runCommand<T>(fn: (v: Redis) => Promise<T>): Promise<T | null> {
  if (circuitOpen()) return null;
  const v = getValkey();
  if (!v) return null;
  try {
    const ok = await ensureConnected(v);
    if (!ok) {
      tripCircuit("not ready");
      return null;
    }
    const result = await withTimeout(fn(v), CMD_TIMEOUT_MS);
    resetCircuit();
    return result;
  } catch (error) {
    tripCircuit(error instanceof Error ? error.message : "cmd");
    return null;
  }
}

export async function valkeyGet(key: string): Promise<string | null> {
  return runCommand((v) => v.get(key));
}

export async function valkeySet(
  key: string,
  value: string,
  ttlSeconds?: number,
): Promise<boolean> {
  const ok = await runCommand(async (v) => {
    if (ttlSeconds && ttlSeconds > 0) {
      await v.set(key, value, "EX", Math.floor(ttlSeconds));
    } else {
      await v.set(key, value);
    }
    return true;
  });
  return ok === true;
}

export async function valkeyDel(...keys: string[]): Promise<void> {
  if (keys.length === 0) return;
  await runCommand((v) => v.del(...keys));
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
  return runCommand(async (v) => {
    const count = await v.incr(key);
    if (count === 1) {
      await v.expire(key, Math.max(1, Math.floor(ttlSeconds)));
    }
    return count;
  });
}

/**
 * Secondary storage Better-Auth.
 * Ne jette jamais : un Valkey down ne doit pas casser login / rate-limit.
 * Preferer rateLimit.storage = "database" côté auth.
 */
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
      const full = p(key);
      const value = await valkeyGet(full);
      if (value !== null) await valkeyDel(full);
      return value;
    },
    async set(key, value, ttl) {
      await valkeySet(p(key), value, ttl);
    },
    async delete(key) {
      await valkeyDel(p(key));
    },
    async increment(key, ttl) {
      const n = await valkeyIncr(p(key), ttl);
      // Better-Auth exige un number : repli local process si Valkey down.
      if (n === null) {
        return 1;
      }
      return n;
    },
  };
}

export async function valkeyDeleteByPrefix(prefix: string): Promise<number> {
  if (!prefix || circuitOpen()) return 0;
  const v = getValkey();
  if (!v) return 0;
  let deleted = 0;
  try {
    const ok = await ensureConnected(v);
    if (!ok) return 0;
    let cursor = "0";
    do {
      const [next, keys] = await withTimeout(
        v.scan(cursor, "MATCH", `${prefix}*`, "COUNT", 100),
        CMD_TIMEOUT_MS,
      );
      cursor = next;
      if (keys.length > 0) {
        deleted += await withTimeout(v.del(...keys), CMD_TIMEOUT_MS);
      }
    } while (cursor !== "0");
    resetCircuit();
  } catch (error) {
    tripCircuit(error instanceof Error ? error.message : "scan");
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
