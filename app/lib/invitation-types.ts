/** Types partagés — Invitations cérémonies (Événements). */

export const INVITATION_THEMES = ["remise_diplome", "neutre"] as const;
export type InvitationTheme = (typeof INVITATION_THEMES)[number];

export const INVITATION_DIPLOMA_MODES = ["none", "bac", "brevet", "both"] as const;
export type InvitationDiplomaMode = (typeof INVITATION_DIPLOMA_MODES)[number];

export const INVITATION_DIPLOMAS = ["bac", "brevet"] as const;
export type InvitationDiploma = (typeof INVITATION_DIPLOMAS)[number];

export const INVITATION_RESPONSES = ["oui", "non"] as const;
export type InvitationResponse = (typeof INVITATION_RESPONSES)[number];

export const INVITATION_ASK_SITUATION = ["off", "always", "bac_only"] as const;
export type InvitationAskSituation = (typeof INVITATION_ASK_SITUATION)[number];

export const INVITATION_SITUATION_STATUSES = ["etudes", "emploi", "autre"] as const;
export type InvitationSituationStatus = (typeof INVITATION_SITUATION_STATUSES)[number];

export function isInvitationTheme(v: string): v is InvitationTheme {
  return (INVITATION_THEMES as readonly string[]).includes(v);
}

export function isInvitationDiplomaMode(v: string): v is InvitationDiplomaMode {
  return (INVITATION_DIPLOMA_MODES as readonly string[]).includes(v);
}

export function isInvitationDiploma(v: string): v is InvitationDiploma {
  return (INVITATION_DIPLOMAS as readonly string[]).includes(v);
}

export function isInvitationResponse(v: string): v is InvitationResponse {
  return (INVITATION_RESPONSES as readonly string[]).includes(v);
}

export function isInvitationAskSituation(v: string): v is InvitationAskSituation {
  return (INVITATION_ASK_SITUATION as readonly string[]).includes(v);
}

export function isInvitationSituationStatus(v: string): v is InvitationSituationStatus {
  return (INVITATION_SITUATION_STATUSES as readonly string[]).includes(v);
}

export type InvitationPageRecord = {
  id: string;
  etablissementId: string;
  slug: string;
  title: string;
  intro: string;
  theme: InvitationTheme;
  enabled: boolean;
  startsAt: string | null;
  endsAt: string | null;
  rsvpClosesAt: string | null;
  location: string;
  diplomaMode: InvitationDiplomaMode;
  maxTotalPersons: number;
  maxPersonsPerEleve: number;
  notifyEmail: string | null;
  requireEligible: boolean;
  askSituation: InvitationAskSituation;
  createdAt: string;
  updatedAt: string;
};

export type InvitationEligibleRecord = {
  id: string;
  etablissementId: string;
  pageId: string;
  eleveFirstName: string;
  eleveLastName: string;
  eleveNameNorm: string;
  /** YYYY-MM-DD */
  birthDate: string | null;
  diploma: InvitationDiploma | null;
  createdAt: string;
};

export type InvitationRsvpRecord = {
  id: string;
  etablissementId: string;
  pageId: string;
  eligibleId: string | null;
  eleveFirstName: string;
  eleveLastName: string;
  eleveNameNorm: string;
  /** YYYY-MM-DD */
  birthDate: string | null;
  response: InvitationResponse;
  presentCount: number;
  parentEmail: string;
  diploma: InvitationDiploma | null;
  situationStatus: InvitationSituationStatus | null;
  situationDetail: string;
  situationEstablishment: string;
  duplicateGroupId: string | null;
  duplicateDismissedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type InvitationPagePublic = {
  slug: string;
  title: string;
  intro: string;
  theme: InvitationTheme;
  startsAt: string | null;
  endsAt: string | null;
  rsvpClosesAt: string | null;
  location: string;
  diplomaMode: InvitationDiplomaMode;
  maxPersonsPerEleve: number;
  placesRemaining: number | null;
  schoolName: string;
  logoUrl: string | null;
  requireEligible: boolean;
  askSituation: InvitationAskSituation;
  rsvpOpen: boolean;
};

export type InvitationDashboardStats = {
  ouiCount: number;
  nonCount: number;
  totalPersons: number;
  placesRemaining: number | null;
  maxTotalPersons: number;
  eligibleCount: number;
  /** Invités de la liste (ou RSVP totaux s’il n’y a pas de liste) qui ont répondu. */
  respondedCount: number;
  /** Invités de la liste encore sans réponse (décompte). */
  pendingCount: number;
};

export type InvitationDuplicateSuspect = {
  eleveNameNorm: string;
  eleveLabel: string;
  rsvpIds: string[];
};

/** Normalise une partie de nom (prénom ou nom) : accents, tirets, casse. */
export function normalizeNamePart(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9\s]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** Normalise prénom+nom pour index / affichage. */
export function normalizeEleveName(firstName: string, lastName: string): string {
  return `${normalizeNamePart(firstName)}|${normalizeNamePart(lastName)}`;
}

/**
 * Parse une date saisie (JJ/MM/AAAA, AAAA-MM-JJ, JJ-MM-AAAA) → YYYY-MM-DD.
 * Retourne null si invalide.
 */
export function parseInvitationBirthDate(raw: string): string | null {
  const s = raw.trim();
  if (!s) return null;
  let y = 0;
  let m = 0;
  let d = 0;
  const iso = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s);
  const fr = /^(\d{1,2})[/.-](\d{1,2})[/.-](\d{4})$/.exec(s);
  if (iso) {
    y = Number(iso[1]);
    m = Number(iso[2]);
    d = Number(iso[3]);
  } else if (fr) {
    d = Number(fr[1]);
    m = Number(fr[2]);
    y = Number(fr[3]);
  } else {
    return null;
  }
  if (y < 1950 || y > 2020 || m < 1 || m > 12 || d < 1 || d > 31) return null;
  const dt = new Date(Date.UTC(y, m - 1, d));
  if (dt.getUTCFullYear() !== y || dt.getUTCMonth() !== m - 1 || dt.getUTCDate() !== d) {
    return null;
  }
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${y}-${pad(m)}-${pad(d)}`;
}

export type IdentityMatchInput = {
  firstName: string;
  lastName: string;
  birthDate: string | null;
};

export type IdentityMatchCandidate = {
  id: string;
  eleveFirstName: string;
  eleveLastName: string;
  birthDate: string | null;
};

/** Score 0–3 : prénom, nom, date de naissance (égalité après normalisation). */
export function scoreIdentityMatch(
  input: IdentityMatchInput,
  candidate: IdentityMatchCandidate,
): number {
  let score = 0;
  if (
    normalizeNamePart(input.firstName) &&
    normalizeNamePart(input.firstName) === normalizeNamePart(candidate.eleveFirstName)
  ) {
    score += 1;
  }
  if (
    normalizeNamePart(input.lastName) &&
    normalizeNamePart(input.lastName) === normalizeNamePart(candidate.eleveLastName)
  ) {
    score += 1;
  }
  if (input.birthDate && candidate.birthDate && input.birthDate === candidate.birthDate) {
    score += 1;
  }
  return score;
}

/**
 * Meilleur candidat avec au moins 2 critères sur 3.
 * Si ex æquo au même score, ambigu → null (ask human).
 */
export function pickBestIdentityMatch<T extends IdentityMatchCandidate>(
  input: IdentityMatchInput,
  candidates: T[],
  minScore = 2,
): { match: T; score: number } | null {
  let best: T | null = null;
  let bestScore = 0;
  let tie = false;
  for (const c of candidates) {
    const score = scoreIdentityMatch(input, c);
    if (score < minScore) continue;
    if (score > bestScore) {
      best = c;
      bestScore = score;
      tie = false;
    } else if (score === bestScore && best && c.id !== best.id) {
      tie = true;
    }
  }
  if (!best || tie) return null;
  return { match: best, score: bestScore };
}

/** Nom de famille : majuscules (comme les listes établissement). */
export function formatInvitationLastName(value: string): string {
  return value.trim().replace(/\s+/g, " ").toLocaleUpperCase("fr-FR");
}

/** Prénom : initiale majuscule, reste minuscule (tirets / espaces / apostrophes). */
export function formatInvitationFirstName(value: string): string {
  const trimmed = value.trim().replace(/\s+/g, " ");
  if (!trimmed) return "";
  return trimmed
    .split(/([ '-]+)/)
    .map((part) => {
      if (!part || /^[ '-]+$/.test(part)) return part;
      const lower = part.toLocaleLowerCase("fr-FR");
      const first = lower.charAt(0).toLocaleUpperCase("fr-FR");
      return `${first}${lower.slice(1)}`;
    })
    .join("");
}

export function formatInvitationEleveLabel(firstName: string, lastName: string): string {
  return `${formatInvitationFirstName(firstName)} ${formatInvitationLastName(lastName)}`.trim();
}

/** Slug URL sûr (a-z0-9-). */
export function slugifyInvitation(input: string): string {
  return (
    input
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 80) || "invitation"
  );
}

export function diplomaLabel(d: InvitationDiploma | null | undefined): string {
  if (d === "bac") return "Baccalauréat";
  if (d === "brevet") return "Brevet";
  return "";
}

export function situationStatusLabel(s: InvitationSituationStatus | null | undefined): string {
  if (s === "etudes") return "Études / poursuite d’études";
  if (s === "emploi") return "Emploi / stage";
  if (s === "autre") return "Autre";
  return "";
}

/** Affiche les champs situation selon le réglage page + diplôme choisi. */
export function shouldAskSituation(
  ask: InvitationAskSituation,
  diploma: InvitationDiploma | null,
  diplomaMode: InvitationDiplomaMode,
): boolean {
  if (ask === "off") return false;
  if (ask === "always") return true;
  // bac_only
  if (diplomaMode === "bac") return true;
  if (diplomaMode === "brevet" || diplomaMode === "none") return false;
  return diploma === "bac";
}

export function isInvitationRsvpOpen(rsvpClosesAt: string | null | undefined, now = new Date()): boolean {
  if (!rsvpClosesAt) return true;
  const t = new Date(rsvpClosesAt).getTime();
  if (Number.isNaN(t)) return true;
  return now.getTime() <= t;
}
