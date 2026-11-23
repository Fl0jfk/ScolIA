import "server-only";

import { schoolClassesMatch } from "@/app/lib/school-classes-catalog";
import { inferStudentLevelFromClass } from "@/app/lib/stage-level";
import { getStagePeriodsForClass } from "@/app/lib/stage-periods-config";
import { stageStudentNameMatchesEleve, normalizeStagePersonName } from "@/app/lib/stage-person-name";
import { ensureConventionReferent } from "@/app/lib/stage-referents-config";
import { getConventionsIndex, getStageConvention, saveStageConvention } from "@/app/lib/stage-storage";
import { currentStageSchoolYear, type StageConvention } from "@/app/lib/stage-types";

/** Conventions « vivantes » : on ne touche ni aux annulées/archivées ni aux signées (document figé). */
const SYNCABLE_STATUSES = new Set<StageConvention["status"]>([
  "draft",
  "preconvention_submitted",
  "admin_review",
  "admin_rejected",
  "convention_deposited",
  "convention_ready",
  "signatures_pending",
]);

/**
 * Quand la classe d'un élève change dans son dossier, ses conventions de stage de
 * l'année en cours suivent (classe, niveau, période équivalente, référent si absent).
 * Sans cela la convention reste sur l'ancienne classe : introuvable dans le suivi,
 * professeur principal introuvable à la validation.
 *
 * Ne lève jamais : un échec ne doit pas bloquer le changement de classe.
 */
export async function syncStageConventionsToEleveClasse(opts: {
  nom: string;
  prenom: string;
  newClasse: string;
  actorName?: string;
}): Promise<{ updated: string[] }> {
  const updated: string[] = [];
  const newClasse = opts.newClasse.trim();
  if (!newClasse || !opts.nom.trim() || !opts.prenom.trim()) return { updated };

  try {
    const year = currentStageSchoolYear();
    const index = await getConventionsIndex();
    const candidates = index.filter(
      (e) =>
        SYNCABLE_STATUSES.has(e.status) &&
        e.schoolYear === year &&
        !schoolClassesMatch(String(e.className ?? ""), newClasse) &&
        stageStudentNameMatchesEleve(e.studentName, { nom: opts.nom, prenom: opts.prenom }),
    );

    for (const entry of candidates) {
      try {
        const convention = await getStageConvention(entry.id);
        if (!convention) continue;
        const oldClasse = convention.student.className;

        let stagePeriodId = convention.stagePeriodId;
        if (stagePeriodId) {
          const [oldPeriods, newPeriods] = await Promise.all([
            getStagePeriodsForClass(oldClasse, convention.schoolYear),
            getStagePeriodsForClass(newClasse, convention.schoolYear),
          ]);
          const old = oldPeriods.find((p) => p.id === stagePeriodId);
          const sameInNew =
            newPeriods.find((p) => p.id === stagePeriodId) ??
            (old
              ? newPeriods.find(
                  (p) =>
                    normalizeStagePersonName(p.label) === normalizeStagePersonName(old.label) ||
                    (p.periodStart === old.periodStart && p.periodEnd === old.periodEnd),
                )
              : undefined);
          stagePeriodId = sameInNew?.id;
        }

        const now = new Date().toISOString();
        let next: StageConvention = {
          ...convention,
          student: {
            ...convention.student,
            className: newClasse,
            level: inferStudentLevelFromClass(newClasse) || convention.student.level,
          },
          stagePeriodId,
          updatedAt: now,
          history: [
            ...convention.history,
            {
              at: now,
              by: opts.actorName || "Système",
              action: "CLASSE_MODIFIEE",
              note: `${oldClasse || "—"} → ${newClasse}`,
            },
          ],
        };
        next = await ensureConventionReferent(next);
        await saveStageConvention(next);
        updated.push(next.id);
      } catch (error) {
        console.error("[stages] sync classe convention", entry.id, error);
      }
    }
  } catch (error) {
    console.error("[stages] sync classe conventions", error);
  }
  return { updated };
}
