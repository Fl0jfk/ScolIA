import "server-only";

import { and, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleveFoyerLink, foyerResponsable } from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import { loadMefSecteurMap } from "@/app/lib/mef-secteurs";
import { isClassEligibleForStage } from "@/app/lib/stage-periods-config";
import {
  filterStageSiblings,
  type StageSibling,
} from "@/app/lib/stage-siblings-filter";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { getElevePhotoUrl } from "@/app/lib/eleve-photos";
import { buildStudentStageDossier } from "@/app/lib/stage-student-dossier";
import {
  EMPTY_SIBLING_STAGE_SUMMARY,
  buildSiblingStageSummary,
  sortSiblingsCurrentFirst,
  type SiblingStageSummary,
} from "@/app/lib/stage-siblings-summary";

export type { StageSibling } from "@/app/lib/stage-siblings-filter";

/**
 * Plafond de fratrie : au-delà, l'adresse est probablement collective ou mal saisie ;
 * on n'expose aucun enfant plutôt que d'ouvrir l'accès à tout un groupe.
 */
export const STAGE_SIBLINGS_MAX = 6;

/** Ids d'élèves rattachés (établissement courant) à un foyer dont un responsable a l'une de ces adresses. */
async function listEleveIdsForFoyerEmails(emails: string[]): Promise<string[]> {
  if (emails.length === 0) return [];
  const etabId = await resolveCurrentEtablissementId().catch(() => null);
  if (!etabId) return [];
  const db = getDb();
  const foyers = await db
    .select({ foyerId: foyerResponsable.foyerId })
    .from(foyerResponsable)
    .where(
      and(
        eq(foyerResponsable.etablissementId, etabId),
        inArray(sql`lower(trim(${foyerResponsable.email}))`, emails),
      ),
    );
  const foyerIds = [...new Set(foyers.map((f) => f.foyerId))];
  if (foyerIds.length === 0) return [];
  const links = await db
    .select({ eleveId: eleveFoyerLink.eleveId })
    .from(eleveFoyerLink)
    .where(
      and(eq(eleveFoyerLink.etablissementId, etabId), inArray(eleveFoyerLink.foyerId, foyerIds)),
    );
  return [...new Set(links.map((l) => l.eleveId))];
}

/**
 * Enfants actifs (hors école) rattachés aux adresses vérifiées par OTP.
 * `eleve.email` n'est jamais un critère. Registre et foyers sont scopés à l'établissement courant.
 */
export async function listStageSiblingsForEmails(
  emails: string[] | undefined,
  opts: { currentEleveId?: string } = {},
): Promise<StageSibling[]> {
  const verified = [
    ...new Set((emails ?? []).map((e) => String(e ?? "").trim().toLowerCase()).filter(Boolean)),
  ];
  if (verified.length === 0) return [];

  const [eleves, foyerEleveIds, mefMap] = await Promise.all([
    loadElevesActifsRegistry() as Promise<EleveConfig[]>,
    listEleveIdsForFoyerEmails(verified),
    loadMefSecteurMap().catch(() => null),
  ]);

  // isClassEligibleForStage est asynchrone : on pré-calcule pour les classes distinctes.
  const classes = [...new Set(eleves.map((e) => String(e.classe ?? "").trim()))];
  const eligibility = new Map<string, boolean>();
  await Promise.all(
    classes.map(async (c) => {
      eligibility.set(c, (await isClassEligibleForStage(c)).ok);
    }),
  );

  const siblings = filterStageSiblings({
    eleves,
    verifiedEmails: verified,
    foyerEleveIds,
    mefMap,
    isClassEligible: (c) => eligibility.get(c.trim()) ?? true,
    currentEleveId: opts.currentEleveId,
  });
  if (siblings.length > STAGE_SIBLINGS_MAX) {
    console.warn(`[stage-siblings] fratrie ignorée : ${siblings.length} élèves rattachés`);
    return [];
  }
  return siblings;
}

export type StageSiblingCard = StageSibling & {
  /** URL de photo pré-signée (S3, ~1 h) ; jamais une URL publique permanente. */
  photoUrl: string | null;
  stageSummary: SiblingStageSummary;
};

/**
 * Enrichit la fratrie pour l'écran famille : photo + récapitulatif de stages.
 * Rien d'autre n'est exposé (pas d'e-mail, téléphone, INE, date de naissance, jeton).
 */
export async function enrichStageSiblings(
  siblings: StageSibling[],
  currentEleveId?: string,
): Promise<StageSiblingCard[]> {
  const capped = siblings.slice(0, STAGE_SIBLINGS_MAX);
  if (capped.length === 0) return [];
  const registry = await loadElevesActifsRegistry().catch(() => [] as EleveConfig[]);
  const byId = new Map(registry.map((e) => [e.id, e]));
  const cards = await Promise.all(
    capped.map(async (s): Promise<StageSiblingCard> => {
      const eleve = byId.get(s.id);
      const [photoUrl, stageSummary] = await Promise.all([
        eleve ? getElevePhotoUrl(eleve).catch(() => null) : Promise.resolve(null),
        buildStudentStageDossier({
          firstName: s.firstName,
          lastName: s.lastName,
          className: s.className,
        })
          .then(buildSiblingStageSummary)
          .catch(() => EMPTY_SIBLING_STAGE_SUMMARY),
      ]);
      return {
        ...s,
        current: currentEleveId ? s.id === currentEleveId : s.current,
        photoUrl,
        stageSummary,
      };
    }),
  );
  return sortSiblingsCurrentFirst(cards);
}
