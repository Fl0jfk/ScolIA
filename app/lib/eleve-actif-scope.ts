import "server-only";

import { and, eq, isNull, ne, or, type SQL, sql } from "drizzle-orm";
import { eleve } from "@/db/schema";
import { parisDateKey } from "@/app/lib/paris-time";

export {
  formatDateSortieFromRow,
  isEleveActifPourListes,
  isEleveSortantEtablissement,
  type EleveActifListesFields,
} from "@/app/lib/eleve-actif-shared";

/**
 * Fragment SQL Drizzle à combiner avec `etablissement_id` (et autres filtres).
 */
export function drizzleEleveActifPourListes(now: Date = new Date()) {
  const today = parisDateKey(now);
  return and(
    eq(eleve.status, "inscrit"),
    or(isNull(eleve.dateSortie), sql`${eleve.dateSortie} >= ${today}::date`),
  );
}

/** Élève scolarisé sur une période de notes / bulletins (date début de période ou repli rentrée). */
export function drizzleEleveVisiblePourPeriodeNotes(periodeDateDebut: string) {
  const debut = periodeDateDebut.trim();
  const dateClause = or(
    isNull(eleve.dateSortie),
    sql`${eleve.dateSortie} >= ${debut}::date`,
  );
  return and(ne(eleve.status, "preinscrit"), dateClause);
}

/** Même règle en SQL brut (scripts / migrations). */
export function sqlEleveActifPourListesClause(now: Date = new Date()): SQL {
  const today = parisDateKey(now);
  return sql`status = 'inscrit' AND (date_sortie IS NULL OR date_sortie >= ${today}::date)`;
}
