import "server-only";

import postgres from "postgres";
import { isWithinSendWindow } from "@/app/lib/travels-v2-reminders-job";

/**
 * Cycle de vie des voyages v2 : clôture automatique des inscriptions à la date limite (au plus tard J-14),
 * alerte si un désaccord bloque la clôture, proposition de clôture anticipée, reprise des étapes en échec et
 * purge du lendemain du retour. Même mécanisme que les relances : un passage par heure (8 h – 18 h, Europe/Paris),
 * verrou advisory Postgres (un seul serveur à la fois), idempotence portée par les données.
 */

/**
 * ATTENTION — verrou de session : `pg_try_advisory_lock` est lié à UNE connexion Postgres. Il exige une connexion DIRECTE
 * (`DATABASE_URL` sans PgBouncer en mode « transaction »). Défense en profondeur, valable même derrière PgBouncer :
 * un verrou EN TABLE par établissement pour le passage (`withNamedLock`, durée limitée) et des verrous EN TABLE par voyage
 * autour de la clôture, de ses effets (liste transporteur, e-mail compta) et de la purge (voir travels-v2-lot4-store.ts).
 */
const INTERVAL_MS = 60 * 60_000;
const FIRST_RUN_MS = 5 * 60_000;
const LOCK_KEY = 7_204_117_003; // distinct des relances (…002) et des nouveautés

export function isLifecycleJobEnabled(): boolean {
  return Boolean(process.env.DATABASE_URL?.trim()) && process.env.VOYAGES_V2_LIFECYCLE_DISABLED !== "1";
}

async function runPass(): Promise<void> {
  if (!isWithinSendWindow()) return;
  const url = process.env.DATABASE_URL?.trim();
  if (!url) return;
  const lockClient = postgres(url, { max: 1, prepare: false, connect_timeout: 5, idle_timeout: 0 });
  try {
    const [lock] = await lockClient`select pg_try_advisory_lock(${LOCK_KEY}) as ok`;
    if (!lock?.ok) return;
    try {
      const { loadAllTenants } = await import("@/app/lib/tenant-registry");
      const { runWithTenant } = await import("@/app/lib/tenant-context");
      const { runLifecycleForTenant } = await import("@/app/lib/travels-v2-lot4-store");
      const tenants = await loadAllTenants();
      for (const tenant of tenants) {
        try {
          await runWithTenant(tenant, async () => {
            const { withNamedLock } = await import("@/app/lib/ent-json-postgres");
            const locked = await withNamedLock("jobs", "voyages-v2-cycle", 55 * 60, () => runLifecycleForTenant());
            if (locked.busy) return;
            const r = locked.value;
            if (r.closed || r.proposed || r.blocked || r.purged || r.retried || r.invited) {
              console.info(
                `[voyages-v2-cycle] ${tenant.slug}: ${r.closed} clôturé(s), ${r.proposed} proposition(s), ${r.blocked} blocage(s), ${r.purged} purgé(s), ${r.retried} reprise(s), ${r.invited} invitation(s) envoyée(s)`,
              );
            }
          });
        } catch (err) {
          console.error("[voyages-v2-cycle] établissement ignoré", err instanceof Error ? err.message : "error");
        }
      }
    } finally {
      await lockClient`select pg_advisory_unlock(${LOCK_KEY})`.catch(() => undefined);
    }
  } catch (err) {
    console.error("[voyages-v2-cycle] passage en échec", err instanceof Error ? err.name : "error");
  } finally {
    await lockClient.end({ timeout: 2 }).catch(() => undefined);
  }
}

const g = globalThis as { __scoliaVoyagesLifecycleJob?: boolean };

/** Démarre le job périodique (idempotent). Appelé depuis instrumentation.ts (runtime nodejs). */
export function startLifecycleJob(): void {
  if (g.__scoliaVoyagesLifecycleJob || !isLifecycleJobEnabled()) return;
  g.__scoliaVoyagesLifecycleJob = true;
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
