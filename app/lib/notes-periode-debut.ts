import "server-only";

import { and, asc, eq } from "drizzle-orm";
import { getCurrentAnneeScolaire } from "@/app/lib/annees-scolaires-db";
import { currentSchoolYearLabel } from "@/app/lib/ent-core-db";
import {
  defaultSchoolYearStartIsoFromLabel,
  formatSqlDateToIso,
  isUuidV4Like,
  pickCalendrierAnneeDebutIso,
} from "@/app/lib/notes-periode-debut-logic";
import { getDb } from "@/db/index";
import { anneeScolaire, calendrierScolaire, notePeriode } from "@/db/schema";

export {
  defaultSchoolYearStartIsoFromLabel,
  formatSqlDateToIso,
  isUuidV4Like,
  pickCalendrierAnneeDebutIso,
} from "@/app/lib/notes-periode-debut-logic";

export class InvalidPeriodeIdError extends Error {
  constructor() {
    super("Identifiant de période invalide.");
    this.name = "InvalidPeriodeIdError";
  }
}

/**
 * Date pivot pour filtrer les élèves en notes / bulletins :
 * date_debut de la période si renseignée, sinon début d’année (calendrier ou 1er sept.).
 */
export async function resolveNotesPeriodeDateDebutIso(
  etablissementId: string,
  periodeId: string,
): Promise<string> {
  const pid = periodeId.trim();
  if (!pid) {
    return await resolveDebutAnneeScolaireCouranteIso(etablissementId);
  }
  if (!isUuidV4Like(pid)) {
    throw new InvalidPeriodeIdError();
  }
  const db = getDb();

  const [periode] = await db
    .select({
      dateDebut: notePeriode.dateDebut,
      anneeScolaireId: notePeriode.anneeScolaireId,
    })
    .from(notePeriode)
    .where(and(eq(notePeriode.etablissementId, etablissementId), eq(notePeriode.id, pid)))
    .limit(1);

  const fromPeriode = formatSqlDateToIso(periode?.dateDebut);
  if (fromPeriode) return fromPeriode;

  return await resolveDebutAnneeScolaireCouranteIso(
    etablissementId,
    periode?.anneeScolaireId ?? null,
  );
}

export async function resolveDebutAnneeScolaireCouranteIso(
  etablissementId: string,
  anneeScolaireId?: string | null,
): Promise<string> {
  const db = getDb();
  let anneeId = anneeScolaireId?.trim() || null;
  let label = currentSchoolYearLabel();

  if (anneeId) {
    const [row] = await db
      .select({ label: anneeScolaire.label, startsOn: anneeScolaire.startsOn })
      .from(anneeScolaire)
      .where(and(eq(anneeScolaire.etablissementId, etablissementId), eq(anneeScolaire.id, anneeId)))
      .limit(1);
    if (row?.startsOn) return formatSqlDateToIso(row.startsOn) ?? defaultSchoolYearStartIsoFromLabel(row.label);
    if (row?.label) label = row.label;
  } else {
    const current = await getCurrentAnneeScolaire(etablissementId);
    if (current?.startsOn) {
      return formatSqlDateToIso(current.startsOn) ?? defaultSchoolYearStartIsoFromLabel(current.label);
    }
    if (current?.label) {
      label = current.label;
      anneeId = current.id;
    }
  }

  if (anneeId) {
    const calRows = await db
      .select({
        dateDebut: calendrierScolaire.dateDebut,
        type: calendrierScolaire.type,
      })
      .from(calendrierScolaire)
      .where(
        and(
          eq(calendrierScolaire.etablissementId, etablissementId),
          eq(calendrierScolaire.anneeScolaireId, anneeId),
        ),
      )
      .orderBy(asc(calendrierScolaire.dateDebut));
    const fromCal = pickCalendrierAnneeDebutIso(calRows);
    if (fromCal) return fromCal;
  }

  return defaultSchoolYearStartIsoFromLabel(label);
}
