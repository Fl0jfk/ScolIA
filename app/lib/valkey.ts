import "server-only";

import Redis from "ioredis";

/**
 * Client Valkey / Managed Redis.
 * Règle d’or : ne JAMAIS ralentir le hot path.
 * - timeout commande court
 * - si pas ready → skip immédiat
 * - half-open : une seule sonde après échec (évite N × timeout)
 * - ready TCP ne rouvre PAS le circuit (seulement une commande OK)
 */

let client: Redis | null | undefined;
let loggedMissing = false;
let loggedReady = false;
let halfOpenProbeInFlight = false;
let consecutiveFailures = 0;
let lastError: string | null = null;

let circuitOpenUntil = 0;
const CIRCUIT_COOLDOWN_MS = 120_000;
const CIRCUIT_COOLDOWN_HARD_MS = 600_000;
/** Commandes cache (GET/SET) : court, ne pas freiner le hot path. */
const CMD_TIMEOUT_MS = 150;
/** Connect initial Serverless → Redis public : un peu plus large. */
const CONNECT_TIMEOUT_MS = 3_000;
const DIAG_PING_TIMEOUT_MS = 1_500;

export function isValkeyConfigured(): boolean {
  if (process.env.VALKEY_DISABLED === "1") return false;
  return Boolean(resolveValkeyUrl());
}

function resolveValkeyUrl(): string | null {
  if (process.env.VALKEY_DISABLED === "1") return null;
  const url =
    process.env.VALKEY_URL?.trim() ||
    process.env.REDIS_URL?.trim() ||
    "";
  return url || null;
}

function rememberError(reason: string): void {
  lastError = reason.slice(0, 240);
}

function tripCircuit(reason: string): void {
  consecutiveFailures += 1;
  rememberError(reason);
  const cool =
    consecutiveFailures >= 3 ? CIRCUIT_COOLDOWN_HARD_MS : CIRCUIT_COOLDOWN_MS;
  circuitOpenUntil = Math.max(circuitOpenUntil, Date.now() + cool);
  console.warn(
    `[valkey] coupe-circuit ${Math.round(cool / 1000)}s (échec #${consecutiveFailures}) — ${reason}`,
  );
}

function resetCircuit(): void {
  circuitOpenUntil = 0;
  consecutiveFailures = 0;
  lastError = null;
}

function circuitOpen(): boolean {
  return Date.now() < circuitOpenUntil;
}

function discardClient(instance?: Redis | null): void {
  const target = instance ?? client;
  if (!target) {
    client = undefined;
    return;
  }
  try {
    target.disconnect();
  } catch {
    /* ignore */
  }
  if (client === target) client = undefined;
  loggedReady = false;
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

function describeValkeyUrl(url: string): {
  scheme: string;
  host: string;
  port: string;
  hasUser: boolean;
  hasPassword: boolean;
  urlParseOk: boolean;
} {
  try {
    const u = new URL(url);
    return {
      scheme: u.protocol.replace(/:$/, ""),
      host: u.hostname,
      port: u.port || "6379",
      hasUser: Boolean(u.username),
      hasPassword: Boolean(u.password),
      urlParseOk: true,
    };
  } catch {
    return {
      scheme: "invalid",
      host: "",
      port: "",
      hasUser: false,
      hasPassword: false,
      urlParseOk: false,
    };
  }
}

function createRedisClient(url: string): Redis {
  const useTls = url.startsWith("rediss://");
  const redis = new Redis(url, {
    maxRetriesPerRequest: 0,
    enableReadyCheck: true,
    lazyConnect: true,
    connectTimeout: CONNECT_TIMEOUT_MS,
    // Ne PAS poser commandTimeout ici : ioredis l’applique aussi au handshake
    // (AUTH / ready check). 150 ms vers un Redis public Scaleway = "Connection is closed."
    enableOfflineQueue: false,
    keepAlive: 10_000,
    family: 4,
    ...(useTls
      ? {
          tls: {
            rejectUnauthorized:
              process.env.VALKEY_TLS_REJECT_UNAUTHORIZED !== "0",
          },
        }
      : {}),
    retryStrategy(times) {
      if (times > 1) {
        tripCircuit("retry épuisé");
        return null;
      }
      return 200;
    },
    reconnectOnError() {
      return false;
    },
  });

  redis.on("error", (err) => {
    const msg = err instanceof Error ? err.message : String(err);
    console.error("[valkey]", msg);
    rememberError(msg);
    if (/closed|ECONNRESET|ETIMEDOUT|ECONNREFUSED|timeout|NOAUTH|WRONGPASS|invalid|CERT/i.test(msg)) {
      tripCircuit(msg);
      discardClient(redis);
    }
  });
  redis.on("ready", () => {
    if (!loggedReady) {
      loggedReady = true;
      console.info("[valkey] TCP prêt (circuit inchangé jusqu’à commande OK)");
    }
  });
  redis.on("end", () => {
    // Évite de traiter un disconnect volontaire (discardClient) comme une panne.
    if (client !== redis) return;
    rememberError("connexion fermée");
    tripCircuit("connexion fermée");
    client = undefined;
    loggedReady = false;
  });

  return redis;
}

export function getValkey(): Redis | null {
  if (!isValkeyConfigured()) {
    if (!loggedMissing) {
      loggedMissing = true;
      console.info(
        "[valkey] non configuré / désactivé — caches partagés désactivés",
      );
    }
    return null;
  }
  if (circuitOpen()) return null;
  // Important : `client === null` (ancien bug) ne doit PAS bloquer les retries.
  if (client) return client;

  const url = resolveValkeyUrl();
  if (!url) {
    return null;
  }

  try {
    const redis = createRedisClient(url);
    client = redis;
    void redis.connect().catch((err) => {
      const msg = err instanceof Error ? err.message : "connect failed";
      tripCircuit(msg);
      console.error(
        "[valkey] connexion échouée — repli sans cache partagé",
        msg,
      );
      discardClient(redis);
    });
    return client;
  } catch (error) {
    const msg = error instanceof Error ? error.message : "init";
    console.error("[valkey] init", error);
    tripCircuit(msg);
    client = undefined;
    return null;
  }
}

async function runCommand<T>(fn: (v: Redis) => Promise<T>): Promise<T | null> {
  if (circuitOpen()) return null;
  const v = getValkey();
  if (!v) return null;
  if (v.status !== "ready") return null;

  const halfOpen = consecutiveFailures > 0;
  if (halfOpen) {
    if (halfOpenProbeInFlight) return null;
    halfOpenProbeInFlight = true;
  }

  try {
    const result = await withTimeout(fn(v), CMD_TIMEOUT_MS);
    resetCircuit();
    return result;
  } catch (error) {
    tripCircuit(error instanceof Error ? error.message : "cmd");
    discardClient(v);
    return null;
  } finally {
    if (halfOpen) halfOpenProbeInFlight = false;
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

/** Conservé pour usage optionnel — Better-Auth n’utilise plus secondaryStorage. */
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
      return n ?? 1;
    },
  };
}

export async function valkeyDeleteByPrefix(prefix: string): Promise<number> {
  if (!prefix || circuitOpen()) return 0;
  const v = getValkey();
  if (!v || v.status !== "ready") return 0;
  let deleted = 0;
  try {
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

/** Statut léger pour diagnostic UI (pas de secret dans la réponse). */
export async function getValkeyRuntimeStatus(): Promise<{
  configured: boolean;
  ready: boolean;
  pingOk: boolean;
  status: string;
  circuitOpen: boolean;
  lastError: string | null;
  url: {
    scheme: string;
    host: string;
    port: string;
    hasUser: boolean;
    hasPassword: boolean;
    urlParseOk: boolean;
  } | null;
}> {
  const configured = isValkeyConfigured();
  const url = resolveValkeyUrl();
  const urlInfo = url ? describeValkeyUrl(url) : null;

  if (!configured || !url) {
    return {
      configured: false,
      ready: false,
      pingOk: false,
      status: "absent",
      circuitOpen: false,
      lastError: null,
      url: null,
    };
  }

  // Diagnostic : on force une tentative même si le coupe-circuit est ouvert.
  const wasCircuitOpen = circuitOpen();
  if (wasCircuitOpen) {
    circuitOpenUntil = 0;
  }
  discardClient();

  let v: Redis | null = null;
  try {
    v = createRedisClient(url);
    client = v;
    await withTimeout(v.connect(), CONNECT_TIMEOUT_MS);
  } catch (error) {
    const msg = error instanceof Error ? error.message : "connect failed";
    rememberError(msg);
    discardClient(v);
    return {
      configured: true,
      ready: false,
      pingOk: false,
      status: "connect_failed",
      circuitOpen: wasCircuitOpen,
      lastError: lastError,
      url: urlInfo,
    };
  }

  const status = v.status;
  const ready = status === "ready";
  if (!ready) {
    rememberError(`status=${status}`);
    discardClient(v);
    return {
      configured: true,
      ready: false,
      pingOk: false,
      status,
      circuitOpen: wasCircuitOpen,
      lastError: lastError,
      url: urlInfo,
    };
  }

  try {
    const pong = await withTimeout(v.ping(), DIAG_PING_TIMEOUT_MS);
    const pingOk = pong === "PONG";
    if (pingOk) resetCircuit();
    else rememberError(`ping=${String(pong)}`);
    return {
      configured: true,
      ready: true,
      pingOk,
      status,
      circuitOpen: false,
      lastError: pingOk ? null : lastError,
      url: urlInfo,
    };
  } catch (error) {
    const msg = error instanceof Error ? error.message : "ping failed";
    rememberError(msg);
    tripCircuit(msg);
    discardClient(v);
    return {
      configured: true,
      ready: true,
      pingOk: false,
      status,
      circuitOpen: true,
      lastError: lastError,
      url: urlInfo,
    };
  }
}

export async function closeValkey(): Promise<void> {
  discardClient();
}
