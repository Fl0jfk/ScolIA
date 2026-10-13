/**
 * Règles pures du sync absence → paie (sans server-only / DB).
 * Conservées ici pour les tests unitaires ; la façade serveur réexporte.
 */

import {
  isRattrapageTreatment,
  isSansIncidenceTreatment,
} from "@/app/lib/absence-hours-treatment";

/** Traitements qui doivent remonter en paie light (pas rattrapage ni prise d’acte sans suite). */
export function absenceShouldCreatePaieElement(
  hoursTreatment?: string | null,
): boolean {
  if (!hoursTreatment) return true;
  if (isRattrapageTreatment(hoursTreatment)) return false;
  if (isSansIncidenceTreatment(hoursTreatment)) return false;
  return true;
}
