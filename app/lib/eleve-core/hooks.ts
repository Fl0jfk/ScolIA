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

  if (!event.eleveId) return;

  const syncRegime =
    event.type === METIER_EVENT_TYPES.ELEVE_REGIME_CHANGED ||
    event.type === METIER_EVENT_TYPES.SCOLARITE_OPENED;
  const syncClasse =
    event.type === METIER_EVENT_TYPES.SCOLARITE_CLASSE_CHANGED ||
    event.type === METIER_EVENT_TYPES.SCOLARITE_OPENED;

  if (!syncRegime && !syncClasse) return;

  try {
    const { listElevesFromDb } = await import("@/app/lib/ent-core-db");
    const eleves = await listElevesFromDb(event.etablissementId);
    const one = eleves.find((e) => e.id === event.eleveId);
    if (!one) return;
    const {
      syncOneEleveInternatRegime,
      syncOneEleveInternatClasse,
    } = await import("@/app/lib/internat-import");
    // Sync ciblé : ne jamais passer un roster d’un seul élève à applyInternatRoster
    // (sinon tous les autres internes seraient sortis).
    if (syncRegime) {
      await syncOneEleveInternatRegime(one, `eleve-core:${event.type}`);
    } else if (syncClasse) {
      await syncOneEleveInternatClasse(one, `eleve-core:${event.type}`);
    }
  } catch (error) {
    console.error("[eleve-core] hook internat dossier", error);
  }
}
