/**
 * Absences `vs_absence_eleve` liées aux conventions de stage (legacy).
 * Motif historique : `Stage — {entreprise} [{suffixe id}]` (voir ancien stage-absences-sync).
 * La présence live lit désormais `en_stage` sur la convention — pas de workflow justificatif CPE.
 */

import { sql, type SQL } from "drizzle-orm";
import { vsAbsenceEleve } from "@/db/schema";
import { isStageAbsenceMotif } from "@/app/lib/occupancy/types";

export { isStageAbsenceMotif as isStageVsAbsenceMotif };

/** Motif PostgreSQL (opérateur ~*) aligné sur {@link isStageAbsenceMotif}. */
export const STAGE_VS_ABSENCE_MOTIF_PG_REGEX = "^Stage[[:space:]]*[—–\\-]";

/** Filtre SQL : exclut les absences stage du suivi CPE / signaux justificatifs. */
export function sqlExcludeStageVsAbsences(): SQL {
  return sql`coalesce(${vsAbsenceEleve.motif}, '') !~* ${STAGE_VS_ABSENCE_MOTIF_PG_REGEX}`;
}

export function isStageVsAbsenceRow(row: { motif?: string | null }): boolean {
  return isStageAbsenceMotif(row.motif);
}
