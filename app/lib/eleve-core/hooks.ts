import "server-only";

import type { MetierEventRecord } from "@/app/lib/eleve-core/events";
import { METIER_EVENT_TYPES } from "@/app/lib/eleve-core/events";

/** Répercute la classe de l'élève sur ses conventions de stage de l'année (ne lève jamais). */
export async function syncStageConventionsForEleve(opts: {
  etablissementId: string;
  eleveId: string;
  classe: string;
}): Promise<void> {
  try {
    const { getDb } = await import("@/db/index");
    const { eleve } = await import("@/db/schema");
    const { and, eq } = await import("drizzle-orm");
    const [row] = await getDb()
      .select({ nom: eleve.nom, prenom: eleve.prenom })
      .from(eleve)
      .where(and(eq(eleve.etablissementId, opts.etablissementId), eq(eleve.id, opts.eleveId)))
      .limit(1);
    if (!row) return;
    const { syncStageConventionsToEleveClasse } = await import("@/app/lib/stage-class-change-sync");
    await syncStageConventionsToEleveClasse({
      nom: row.nom,
      prenom: row.prenom,
      newClasse: opts.classe,
    });
  } catch (error) {
    console.error("[eleve-core] sync conventions stage", error);
  }
}

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

  if (syncClasse) {
    const payload = event.payload;
    const classe = String(payload?.after ?? payload?.classe ?? "").trim();
    if (classe) {
      await syncStageConventionsForEleve({
        etablissementId: event.etablissementId,
        eleveId: event.eleveId,
        classe,
      });
    }
  }

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
