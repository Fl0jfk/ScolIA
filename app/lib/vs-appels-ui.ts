/**
 * Textes UI appels — partagés Brain (confirm) et feuille d’appel.
 */

export function vsAppelCloseConfirmMessageFr(appelId?: string | null): string {
  const id = (appelId || "").trim();
  if (id) {
    return `Clôturer l’appel ${id} et transmettre les absences au suivi CPE ?`;
  }
  return "Clôturer cet appel et transmettre les absences au suivi CPE ?";
}
