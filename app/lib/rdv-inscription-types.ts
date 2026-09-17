/** Types métier — RDV inscriptions (Google Agenda). */

export type RdvInscriptionConfigPublic = {
  enabled: boolean;
  title: string;
  intro: string;
  eventTitlePattern: string;
  notifyEmail: string | null;
  location: string;
  consentLabel: string;
  horizonDays: number;
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
  active: boolean;
  sortOrder: number;
};

export type RdvInscriptionSlot = {
  eventId: string;
  calendarId: string;
  title: string;
  startAt: string;
  endAt: string;
  htmlLink: string | null;
};

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
  status: "confirmed" | "cancelled";
  createdAt: string;
};

export type RdvInscriptionBookInput = {
  eventId: string;
  studentFirstName: string;
  studentLastName: string;
  parentEmail: string;
  parentPhone: string;
};

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

export function eventTitleMatchesPattern(eventTitle: string, pattern: string): boolean {
  const t = normalizeRdvEventTitle(eventTitle);
  const p = normalizeRdvEventTitle(pattern);
  if (!p) return false;
  return t.includes(p);
}

export function isValidDirectionSlug(slug: string): boolean {
  return /^[a-z0-9][a-z0-9-]{0,62}$/.test(slug);
}
