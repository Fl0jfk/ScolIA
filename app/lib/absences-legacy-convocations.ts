import "server-only";

/**
 * Compat absences : plus de lecture/écriture JSON S3 ni EAV « convocations ».
 * Source unique = table Postgres `absence`.
 */
import { absencesDbReady, getAbsenceFromDb } from "@/app/lib/absence-db";
import type { AbsenceRecord } from "@/app/lib/absences-types";

/** Ancien merge S3/EAV — no-op (tout est déjà en table `absence`). */
export async function mergeLegacyConvocationsForCalendar(
  index: AbsenceRecord[],
): Promise<AbsenceRecord[]> {
  return index;
}

export async function getAbsenceOrLegacyRecord(id: string): Promise<AbsenceRecord | null> {
  const etabId = await absencesDbReady();
  if (!etabId) return null;
  return getAbsenceFromDb(etabId, id);
}

/** Plus de fichiers convocations à purger. */
export async function deleteLegacyConvocation(_id: string): Promise<void> {
  /* no-op */
}

/**
 * Plus d’index convocations : les pièces jointes ne sont référencées
 * que via la table `absence` (voir isDocumentKeyReferenced côté appelant).
 */
export async function isDocumentKeyReferencedInLegacy(_key: string): Promise<boolean> {
  return false;
}
