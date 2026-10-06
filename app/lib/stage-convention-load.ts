import "server-only";

import { getConventionsIndex, getStageConventionsByIds } from "@/app/lib/stage-storage";
import type { StageConvention, StageConventionStatus } from "@/app/lib/stage-types";

/**
 * Charge des conventions par id en lot (2 requêtes SQL), plus un GET N+1.
 */
export async function loadStageConventionsByIds(ids: string[]): Promise<StageConvention[]> {
  return getStageConventionsByIds(ids);
}

const SKIP_STATUSES: ReadonlySet<StageConventionStatus> = new Set([
  "archived",
  "draft",
  "cancelled",
]);

/** Index filtré puis chargement en lot — pour les listes admin. */
export async function loadActiveStageConventions(): Promise<StageConvention[]> {
  const index = await getConventionsIndex();
  const ids = index.filter((e) => !SKIP_STATUSES.has(e.status)).map((e) => e.id);
  return loadStageConventionsByIds(ids);
}

/** Statuts utiles au tableau de bord (pas les conventions déjà signées). */
const HUB_BOARD_STATUSES: ReadonlySet<StageConventionStatus> = new Set([
  "admin_review",
  "preconvention_submitted",
  "convention_deposited",
  "convention_ready",
  "signatures_pending",
]);

/**
 * Charge uniquement les conventions encore « actives » pour le hub
 * (file admin + signatures), pas tout l’historique signé.
 */
export async function loadHubBoardStageConventions(): Promise<StageConvention[]> {
  const index = await getConventionsIndex();
  const ids = index.filter((e) => HUB_BOARD_STATUSES.has(e.status)).map((e) => e.id);
  return loadStageConventionsByIds(ids);
}

/** Uniquement les conventions en signatures en cours (signaux dashboard). */
export async function loadSignaturesPendingStageConventions(): Promise<StageConvention[]> {
  const index = await getConventionsIndex();
  const ids = index.filter((e) => e.status === "signatures_pending").map((e) => e.id);
  return loadStageConventionsByIds(ids);
}
