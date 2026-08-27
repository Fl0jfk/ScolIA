import "server-only";

import { matchInternatStudent } from "@/app/lib/eleve-dossier-synthese";
import type { InternatStudent } from "@/app/lib/internat-types";
import { listAbsencesForDate } from "@/app/lib/vs-absences-db";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";

export type InternatCourseAbsenceHint = {
  absenceId: string;
  eleveId: string;
  type: "absence" | "retard";
  justifie: boolean;
  statut: string;
  motif: string | null;
  label: string;
};

/**
 * Croise absences de cours (VS) avec le roster internat du jour.
 * Sert de signal sur l'appel du soir (même fiche élève).
 */
export async function buildInternatCourseAbsenceHints(
  dateIso: string,
  students: InternatStudent[],
): Promise<Record<string, InternatCourseAbsenceHint>> {
  try {
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId || !students.length) return {};

    let absences: Awaited<ReturnType<typeof listAbsencesForDate>> = [];
    try {
      absences = await listAbsencesForDate(etabId, dateIso);
    } catch {
      return {};
    }
    if (!absences.length) return {};

    const out: Record<string, InternatCourseAbsenceHint> = {};
    for (const a of absences) {
      const match = matchInternatStudent(students, {
        nom: a.eleveNom || "",
        prenom: a.elevePrenom || "",
        ine: a.eleveIne,
        folderName: a.eleveFolderName,
      });
      if (!match || !match.actif) continue;

      const type = a.type === "retard" ? "retard" : "absence";
      // Priorité : absence > retard si plusieurs lignes
      const prev = out[match.id];
      if (prev && prev.type === "absence" && type === "retard") continue;

      out[match.id] = {
        absenceId: a.id,
        eleveId: a.eleveId,
        type,
        justifie: a.justifie,
        statut: a.statut,
        motif: a.motif,
        label:
          type === "retard"
            ? a.justifie
              ? "Retard cours (justifié)"
              : "Retard cours"
            : a.justifie
              ? "Absent cours (justifié)"
              : "Absent cours",
      };
    }
    return out;
  } catch (e) {
    console.warn("[internat] buildInternatCourseAbsenceHints", e);
    return {};
  }
}
