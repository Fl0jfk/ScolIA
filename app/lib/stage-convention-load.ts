import "server-only";

import { getConventionsFromDb } from "@/app/lib/stage-db";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getConventionsIndex, getStageConvention } from "@/app/lib/stage-storage";
import type { StageConvention, StageConventionStatus } from "@/app/lib/stage-types";
import { sanitizeElevePersonalEmail } from "@/app/lib/eleve-direction-email";

function withSanitizedStudentEmail(convention: StageConvention): StageConvention {
  const raw = convention.student.email;
  const cleaned = sanitizeElevePersonalEmail(raw);
  const previous = raw?.trim() ? raw.trim().toLowerCase() : undefined;
  if (cleaned === previous) return convention;
  return {
    ...convention,
    student: {
      ...convention.student,
      email: cleaned,
    },
  };
}

/**
 * Charge des conventions par id en 2 requêtes SQL par lot (ids chunkés).
 * Repli unitaire `getStageConvention` pour les ids encore uniquement en legacy.
 */
export async function loadStageConventionsByIds(ids: string[]): Promise<StageConvention[]> {
  const unique = [...new Set(ids.map((id) => id.trim()).filter(Boolean))];
  if (unique.length === 0) return [];

  const etabId = await resolveCurrentEtablissementId().catch(() => null);
  const out: StageConvention[] = [];

  if (etabId) {
    const loaded = await getConventionsFromDb(etabId, unique);
    for (const c of loaded) {
      out.push(withSanitizedStudentEmail(c));
    }
    const found = new Set(loaded.map((c) => c.id));
    const missing = unique.filter((id) => !found.has(id));
    for (const id of missing) {
      const convention = await getStageConvention(id);
      if (convention) out.push(convention);
    }
    return out;
  }

  for (const id of unique) {
    const convention = await getStageConvention(id);
    if (convention) out.push(convention);
  }
  return out;
}

const SKIP_STATUSES: ReadonlySet<StageConventionStatus> = new Set([
  "archived",
  "draft",
  "cancelled",
]);

/** Index filtré puis chargement par lot — pour le hub stages. */
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
