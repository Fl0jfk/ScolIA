import "server-only";

import postgres from "postgres";

/**
 * Relances automatiques du circuit voyages v2 (J-3 / J-1 avant la date limite, pièces « avant départ »,
 * alerte organisateur à J-7). Même mécanisme que le job « nouveautés » (instrumentation.ts, runtime Node) :
 * un passage toutes les heures, verrou advisory Postgres pour qu'un seul serveur travaille à la fois, et
 * idempotence portée par les données (clés `reminders` de chaque invitation + `ops/state` du voyage).
 * Aucun envoi la nuit : fenêtre 8 h – 18 h (heure de Paris).
 */

const INTERVAL_MS = 60 * 60_000;
const FIRST_RUN_MS = 3 * 60_000;
const LOCK_KEY = 7_204_117_002; // clé constante du verrou advisory Postgres (distincte du job nouveautés)
const FIRST_HOUR = 8;
const LAST_HOUR = 18;

export function isRemindersJobEnabled(): boolean {
  return Boolean(process.env.DATABASE_URL?.trim()) && process.env.VOYAGES_V2_REMINDERS_DISABLED !== "1";
}

/** Heure de Paris (0-23). */
export function parisHour(now: Date = new Date()): number {
  return Number(new Intl.DateTimeFormat("fr-FR", { timeZone: "Europe/Paris", hour: "2-digit", hourCycle: "h23" }).format(now));
}

export function isWithinSendWindow(now: Date = new Date()): boolean {
  const h = parisHour(now);
  return h >= FIRST_HOUR && h < LAST_HOUR;
}

async function runPass(): Promise<void> {
  if (!isWithinSendWindow()) return;
  const url = process.env.DATABASE_URL?.trim();
  if (!url) return;
  // Connexion dédiée : le verrou advisory est lié à la session ; fermée explicitement à la fin.
  const lockClient = postgres(url, { max: 1, prepare: false, connect_timeout: 5, idle_timeout: 0 });
  try {
    const [lock] = await lockClient`select pg_try_advisory_lock(${LOCK_KEY}) as ok`;
    if (!lock?.ok) return;
    try {
      const { loadAllTenants } = await import("@/app/lib/tenant-registry");
      const { runWithTenant, getTenantAppUrl } = await import("@/app/lib/tenant-context");
      const { runAutoRemindersForTenant } = await import("@/app/lib/travels-v2-lot3-store");
      const tenants = await loadAllTenants();
      for (const tenant of tenants) {
        try {
          await runWithTenant(tenant, async () => {
            const r = await runAutoRemindersForTenant(await getTenantAppUrl());
            if (r.sent || r.failed) console.info(`[voyages-v2-relances] ${tenant.slug}: ${r.sent} envoyée(s), ${r.failed} en échec`);
          });
        } catch (err) {
          console.error("[voyages-v2-relances] établissement ignoré", err instanceof Error ? err.message : "error");
        }
      }
    } finally {
      await lockClient`select pg_advisory_unlock(${LOCK_KEY})`.catch(() => undefined);
    }
  } catch (err) {
    console.error("[voyages-v2-relances] passage en échec", err instanceof Error ? err.name : "error");
  } finally {
    await lockClient.end({ timeout: 2 }).catch(() => undefined);
  }
}

const g = globalThis as { __scoliaVoyagesRemindersJob?: boolean };

/** Démarre le job périodique (idempotent). Appelé depuis instrumentation.ts (runtime nodejs). */
export function startRemindersJob(): void {
  if (g.__scoliaVoyagesRemindersJob || !isRemindersJobEnabled()) return;
  g.__scoliaVoyagesRemindersJob = true;
  let running = false;
  const tick = async () => {
    if (running) return;
    running = true;
    try {
      await runPass();
    } catch {
      /* jamais de crash */
    } finally {
      running = false;
    }
  };
  setTimeout(() => {
    void tick();
    setInterval(() => void tick(), INTERVAL_MS).unref();
  }, FIRST_RUN_MS).unref();
}
