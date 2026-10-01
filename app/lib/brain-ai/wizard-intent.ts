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

const TRIP_NOUN = /\b(sortie|voyage|sejour|trip)s?\b/;
const TRIP_PHRASE = /\b(sortie|voyage|sejour)s?\s+scolaire(s)?\b/;
const OPEN_VERB =
  /\b(ouvre|ouvrir|montre|montrer|affiche|afficher|va sur|accede|acceder|consulte|consulter|voir|vois|regarde|regarder|liste|lister|trouve|trouver)\b/;
const CREATE_VERB =
  /\b(creer|cree|organise|organiser|planifie|planifier|demarre|demarrer|lance|lancer|nouvelle|nouveau)\b/;

/**
 * Retourne le nom d'outil wizard / navigation à démarrer immédiatement, ou null.
 */
export function detectWizardStartTool(message: string): string | null {
  const t = normalize(message);
  if (!t || t.length > 280) return null;
  if (isMetaHowTo(t)) return null;

  const mentionsTrip = TRIP_NOUN.test(t) || TRIP_PHRASE.test(t);

  // Sorties : OUVRIR avant CRÉER
  // (avant : « sortie scolaire » seul forçait create_trip → « ouvre une sortie » créait).
  if (mentionsTrip && OPEN_VERB.test(t) && !CREATE_VERB.test(t)) {
    return "open_trip";
  }
  if (mentionsTrip && CREATE_VERB.test(t)) {
    return "create_trip";
  }
  // « faire une sortie » = créer ; pas « ouvrir »
  if (mentionsTrip && /\bfaire\b/.test(t) && !OPEN_VERB.test(t)) {
    return "create_trip";
  }
  if (
    mentionsTrip &&
    /\b(mes|les|des)\s+(sorties|voyages|sejours)\b/.test(t) &&
    !CREATE_VERB.test(t)
  ) {
    return "open_trip";
  }

  // Réservation de salle
  if (
    (/\b(reserv|reserver|reservation|book)\b/.test(t) &&
      /\b(salle|salles|local|locaux|amphi)\b/.test(t)) ||
    /\b(je (veux|voudrais|souhaite)|besoin de|faire)\b.{0,40}\b(reserv|salle)\b/.test(t) ||
    /\breserv(er)?\b.{0,20}\b(une |la )?(salle|local)\b/.test(t)
  ) {
    return "create_reservation";
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
