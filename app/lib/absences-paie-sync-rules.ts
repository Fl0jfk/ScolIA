/**
 * Règles pures du sync absence → paie (sans server-only / DB).
 * Conservées ici pour les tests unitaires ; la façade serveur réexporte.
 */

import { isRattrapageTreatment } from "@/app/lib/absence-hours-treatment";

/** Traitements qui doivent remonter en paie light (pas le rattrapage interne). */
export function absenceShouldCreatePaieElement(
  hoursTreatment?: string | null,
): boolean {
  if (!hoursTreatment) return true;
  if (isRattrapageTreatment(hoursTreatment)) return false;
  return true;
}
