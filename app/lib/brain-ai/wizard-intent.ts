/**
 * Détecte une intention d'action guidée pour lancer le wizard
 * sans laisser le LLM poser des questions en texte libre.
 */

function normalize(text: string): string {
  return String(text || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/['’]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** Questions « comment faire » / info → laisser le LLM / knowledge. */
function isMetaHowTo(t: string): boolean {
  return (
    /\b(comment|pourquoi|c est quoi|qu est ce|expliquer|aide pour comprendre)\b/.test(t) ||
    /\b(procedure|reglement|consigne)\b/.test(t)
  );
}

/**
 * Retourne le nom d'outil wizard à démarrer immédiatement, ou null.
 */
export function detectWizardStartTool(message: string): string | null {
  const t = normalize(message);
  if (!t || t.length > 280) return null;
  if (isMetaHowTo(t)) return null;

  // Réservation de salle
  if (
    (/\b(reserv|reserver|reservation|book)\b/.test(t) &&
      /\b(salle|salles|local|locaux|amphi)\b/.test(t)) ||
    /\b(je (veux|voudrais|souhaite)|besoin de|faire)\b.{0,40}\b(reserv|salle)\b/.test(t) ||
    /\breserv(er)?\b.{0,20}\b(une |la )?(salle|local)\b/.test(t)
  ) {
    return "create_reservation";
  }

  // Sortie / voyage / séjour
  if (
    (/\b(creer|organiser|planifier|faire|demarrer|lancer|nouvelle?)\b/.test(t) &&
      /\b(sortie|voyage|sejour|trip)\b/.test(t)) ||
    /\b(sortie scolaire|voyage scolaire|sejour scolaire)\b/.test(t)
  ) {
    return "create_trip";
  }

  // Absence
  if (
    (/\b(declar|declarer|poser|signaler|annoncer|autorisation)\b/.test(t) && /\babsence\b/.test(t)) ||
    /\b(demande d['']autorisation d['']absence)\b/.test(t) ||
    /\b(je (suis|serai) absent|mon absence)\b/.test(t)
  ) {
    return "create_absence";
  }

  // Demande interne (évite HSE / photocopies)
  if (
    (/\b(creer|faire|ouvrir|nouvelle?)\b/.test(t) &&
      /\bdemande\b/.test(t) &&
      !/\bhse\b/.test(t) &&
      !/\bphotocop/.test(t)) ||
    /\bdemande interne\b/.test(t)
  ) {
    return "create_request";
  }

  // Photocopies
  if (/\bphotocop/.test(t) && /\b(demande|couleur|faire|creer|besoin)\b/.test(t)) {
    return "create_photocopie_demand";
  }

  // HSE
  if (/\bhse\b/.test(t) && /\b(demande|creer|faire|besoin)\b/.test(t)) {
    return "create_hse_demand";
  }

  return null;
}
