import "server-only";

import postgres from "postgres";
import { isWithinSendWindow } from "@/app/lib/travels-v2-reminders-job";

/**
 * Relances parents (J-2) et point final (veille 12 h) des autorisations de sortie : un passage toutes les
 * 30 minutes entre 8 h et 18 h (Europe/Paris). Verrou advisory Postgres + verrou en table par établissement ;
 * l'idempotence est portée par les horodatages de chaque sortie.
 */
const INTERVAL_MS = 30 * 60_000;
const FIRST_RUN_MS = 3 * 60_000;
const LOCK_KEY = 7_204_117_011;

export function isSortiesClasseJobEnabled(): boolean {
  return Boolean(process.env.DATABASE_URL?.trim()) && process.env.SORTIES_CLASSE_JOB_DISABLED !== "1";
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
      for (const tenant of await loadAllTenants()) {
        try {
          await runWithTenant(tenant, async () => {
            const { withNamedLock } = await import("@/app/lib/ent-json-postgres");
            await withNamedLock("jobs", "sorties-classe", 25 * 60, async () => {
              const { listerSorties } = await import("@/app/lib/sorties-classe-store");
              const { traiterEcheances } = await import("@/app/lib/sorties-classe-mail");
              const { sortieTerminee } = await import("@/app/lib/sorties-classe");
              for (const sortie of await listerSorties()) {
                if (sortie.cancelledAt || sortieTerminee(sortie)) continue;
                try {
                  const r = await traiterEcheances(sortie);
                  if (r.relances || r.points) {
                    console.info(`[sorties-classe] ${tenant.slug}: ${r.relances} relance(s), ${r.points} point(s)`);
                  }
                } catch (err) {
                  console.error("[sorties-classe] sortie ignorée", err instanceof Error ? err.name : "error");
                }
              }
            });
          });
        } catch (err) {
          console.error("[sorties-classe] établissement ignoré", err instanceof Error ? err.name : "error");
        }
      }
    } finally {
      await lockClient`select pg_advisory_unlock(${LOCK_KEY})`.catch(() => undefined);
    }
  } catch (err) {
    console.error("[sorties-classe] passage en échec", err instanceof Error ? err.name : "error");
  } finally {
    await lockClient.end({ timeout: 2 }).catch(() => undefined);
  }
}

let started = false;

export function startSortiesClasseJob(): void {
  if (started || !isSortiesClasseJobEnabled()) return;
  started = true;
  setTimeout(() => void runPass(), FIRST_RUN_MS).unref?.();
  setInterval(() => void runPass(), INTERVAL_MS).unref?.();
}
