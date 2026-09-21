import "server-only";

import type { MetierEventRecord } from "@/app/lib/eleve-core/events";
import { METIER_EVENT_TYPES } from "@/app/lib/eleve-core/events";

/**
 * Hooks in-process après journal.
 * Import de masse : skipHooks (le registry fait déjà le fan-out internat en lot).
 */
export async function runEleveCoreHooks(
  event: MetierEventRecord,
  opts?: { skipHooks?: boolean },
): Promise<void> {
  if (opts?.skipHooks) return;

  try {
    const { invalidateElevesRegistryCache } = await import("@/app/lib/eleves-registry");
    await invalidateElevesRegistryCache(event.etablissementId);
  } catch (error) {
    console.error("[eleve-core] invalidate registry", error);
  }

  if (event.type !== METIER_EVENT_TYPES.ELEVE_REGIME_CHANGED || !event.eleveId) return;

  try {
    const { listElevesFromDb } = await import("@/app/lib/ent-core-db");
    const eleves = await listElevesFromDb(event.etablissementId);
    const one = eleves.filter((e) => e.id === event.eleveId);
    if (!one.length) return;
    const { syncInternatFromElevesRegime } = await import("@/app/lib/internat-import");
    await syncInternatFromElevesRegime(one, "eleve-core:regime");
  } catch (error) {
    console.error("[eleve-core] hook internat régime", error);
  }
}
