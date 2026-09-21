/** Types métier — RDV inscriptions (Google Agenda). */

/** Config globale du module (activation + état Google) — le reste est par direction. */
export type RdvInscriptionConfigPublic = {
  enabled: boolean;
  googleLinked: boolean;
  googleLinkedEmail: string | null;
  googleLinkedAt: string | null;
};

export type RdvInscriptionDirectionRow = {
  id: string;
  slug: string;
  label: string;
  googleCalendarId: string;
  directriceDisplayName: string | null;
  title: string;
  intro: string;
  eventTitlePattern: string;
  notifyEmail: string | null;
  location: string;
  consentLabel: string;
  horizonDays: number;
  active: boolean;
  sortOrder: number;
};

/** Sous-ensemble utilisé pour les mails / page publique d’une direction. */
export type RdvInscriptionDirectionPageSettings = {
  title: string;
  intro: string;
  eventTitlePattern: string;
  notifyEmail: string | null;
  location: string;
  consentLabel: string;
  horizonDays: number;
};

export type RdvInscriptionSlot = {
  eventId: string;
  calendarId: string;
  title: string;
  startAt: string;
  endAt: string;
  htmlLink: string | null;
};

export type RdvInscriptionBookingStatus =
  | "pending"
  | "confirmed"
  | "cancelled"
  | "expired";

export type RdvInscriptionMatchStatus = "confirmed" | "created";

export type RdvInscriptionReconfirmStatus = "pending" | "ok" | "cancelled";

export type RdvInscriptionBookingRow = {
  id: string;
  directionId: string;
  directionSlug: string;
  googleEventId: string;
  googleCalendarId: string;
  googleHtmlLink: string | null;
  startAt: string;
  endAt: string;
  studentFirstName: string;
  studentLastName: string;
  parentEmail: string;
  parentPhone: string;
  /** Prénom du parent / responsable. */
  parentFirstName: string | null;
  /** Nom du parent / responsable (peut différer de l’élève). */
  parentLastName: string | null;
  /** madame | monsieur | les_deux */
  rdvAttendee: "madame" | "monsieur" | "les_deux" | null;
  niveauId: string | null;
  niveauLabel: string | null;
  eleveId: string | null;
  matchStatus: RdvInscriptionMatchStatus | null;
  createNew: boolean;
  hasPap: "yes" | "no" | null;
  papS3Key: string | null;
  papFileName: string | null;
  papMimeType: string | null;
  papBringToRdv: boolean;
  etablissementOrigineRne: string | null;
  etablissementOrigineLabel: string | null;
  etablissementOrigineAdresse: string | null;
  status: RdvInscriptionBookingStatus;
  confirmExpiresAt: string | null;
  confirmedAt: string | null;
  reconfirmStatus: RdvInscriptionReconfirmStatus | null;
  reconfirmMailSentAt: string | null;
  reconfirmedAt: string | null;
  createdAt: string;
};

export type RdvInscriptionBookInput = {
  eventId: string;
  studentFirstName: string;
  studentLastName: string;
  parentEmail: string;
  parentPhone: string;
  parentFirstName?: string;
  parentLastName?: string;
  /** madame | monsieur | les_deux */
  rdvAttendee?: "madame" | "monsieur" | "les_deux";
  niveauId: string;
  /** Élève confirmé par le parent (matching interactif). */
  eleveId?: string | null;
  /** Création d’un nouveau préinscrit à la validation e-mail. */
  createNew?: boolean;
  /** Date de naissance (AAAA-MM-JJ) — obligatoire si matching identité. */
  studentDateNaissance?: string | null;
  hasPap?: "yes" | "no";
  papS3Key?: string | null;
  papFileName?: string | null;
  papMimeType?: string | null;
  papBringToRdv?: boolean;
  etablissementOrigineRne?: string | null;
  etablissementOrigineLabel?: string | null;
  etablissementOrigineAdresse?: string | null;
  /**
   * Anti-clic trop rapide : le parent doit retaper « CONFIRME »
   * (insensible à la casse / accents).
   */
  confirmTyped?: string;
};

/** Phrase à retaper pour valider une réservation publique. */
export const RDV_BOOK_CONFIRM_PHRASE = "CONFIRME";

export const DEFAULT_RDV_INSCRIPTION_TITLE = "Rendez-vous d’inscription";
export const DEFAULT_RDV_INSCRIPTION_PATTERN = "rendez-vous inscription";
export const DEFAULT_RDV_INSCRIPTION_CONSENT =
  "J’accepte que mes coordonnées soient utilisées pour organiser ce rendez-vous d’inscription.";

export const DEFAULT_RDV_DIRECTIONS: Array<{
  slug: string;
  label: string;
  sortOrder: number;
}> = [
  { slug: "ecole", label: "École", sortOrder: 1 },
  { slug: "college", label: "Collège", sortOrder: 2 },
  { slug: "lycee", label: "Lycée", sortOrder: 3 },
];

/** Normalise pour comparer titres d’événements (casse / accents). */
export function normalizeRdvEventTitle(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/['’`]/g, "'")
    .replace(/[-_/\u2010-\u2015\u2212]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/**
 * Découpe le champ paramétrage en plusieurs motifs exacts.
 * Séparateurs acceptés : `|`, `;`, retours ligne.
 */
export function splitRdvTitlePatterns(patternField: string): string[] {
  return patternField
    .split(/[|;\n\r]+/)
    .map((p) => p.trim())
    .filter(Boolean);
}

/**
 * Le titre Google doit contenir le motif (ou l’un des motifs) tel que saisi
 * dans le paramétrage. Aucune synonymie : casse et accents seulement ignorés.
 */
export function eventTitleMatchesPattern(eventTitle: string, pattern: string): boolean {
  const t = normalizeRdvEventTitle(eventTitle);
  if (!t) return false;
  for (const raw of splitRdvTitlePatterns(pattern)) {
    const p = normalizeRdvEventTitle(raw);
    if (p && t.includes(p)) return true;
  }
  return false;
}

export function isValidDirectionSlug(slug: string): boolean {
  return /^[a-z0-9][a-z0-9-]{0,62}$/.test(slug);
}
