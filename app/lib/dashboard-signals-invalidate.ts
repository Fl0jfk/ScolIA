import "server-only";

import { bumpDashboardSignalsBundleGeneration } from "@/app/lib/dashboard-signals-bundle-generation-server";
import { valkeyDeleteByPrefix } from "@/app/lib/valkey";
import {
  valkeyPrefixDashboardSignalsCreneaux,
  valkeyPrefixDashboardSignalsEtab,
} from "@/app/lib/valkey-keys";

const NS = "scola";

/** Purge caches dashboard (agrégat + sous-caches) pour un établissement. */
export async function invalidateDashboardSignalsForEstablishment(
  etablissementId: string,
): Promise<void> {
  const etabId = etablissementId.trim();
  if (!etabId) return;

  await bumpDashboardSignalsBundleGeneration(etabId);

  await Promise.all([
    valkeyDeleteByPrefix(valkeyPrefixDashboardSignalsEtab(etabId)),
    valkeyDeleteByPrefix(valkeyPrefixDashboardSignalsCreneaux(etabId)),
    valkeyDeleteByPrefix(`${NS}:dash:sig-src:vs:${etabId}:`),
    valkeyDeleteByPrefix(`${NS}:dash:sig-src:rh-now:${etabId}:`),
    valkeyDeleteByPrefix(`${NS}:dash:sig-src:stages-pend:${etabId}:`),
    valkeyDeleteByPrefix(`${NS}:dash:sig-src:req-board:${etabId}:`),
    // On garde volontairement `dash:sig-bundle:*` (dernière valeur connue par source) :
    // l'effacer à chaque saisie d'absence faisait afficher « Compteur indisponible »
    // dès qu'une source dépassait le délai. Le bump de génération ci-dessus empêche
    // déjà d'y réécrire une valeur calculée avant l'invalidation.
    valkeyDeleteByPrefix(`${NS}:dash:sig-src:abs-index:${etabId}:`),
  ]);
}

/**
 * Purge ciblée après une annulation d’absence à l’accueil.
 * Ne touche ni stages, ni demandes : inutile de faire tout recalculer à tout le monde.
 * - élève : compteurs vie scolaire + agrégat ;
 * - personnel : en plus index absences RH, « en ce moment » du planning et créneaux vides.
 */
export async function invalidateDashboardSignalsAfterAccueilAbsence(
  etablissementId: string,
  kind: "eleve" | "personnel",
): Promise<void> {
  const etabId = etablissementId.trim();
  if (!etabId) return;

  await bumpDashboardSignalsBundleGeneration(etabId);

  const prefixes = [
    valkeyPrefixDashboardSignalsEtab(etabId),
    `${NS}:dash:sig-src:vs:${etabId}:`,
  ];
  if (kind === "personnel") {
    prefixes.push(
      `${NS}:dash:sig-src:abs-index:${etabId}:`,
      `${NS}:dash:sig-src:rh-now:${etabId}:`,
      valkeyPrefixDashboardSignalsCreneaux(etabId),
    );
  }
  await Promise.all(prefixes.map((p) => valkeyDeleteByPrefix(p)));
}
