/** Types partagés — Invitations cérémonies (Événements). */

export const INVITATION_THEMES = ["remise_diplome", "neutre"] as const;
export type InvitationTheme = (typeof INVITATION_THEMES)[number];

export const INVITATION_DIPLOMA_MODES = ["none", "bac", "brevet", "both"] as const;
export type InvitationDiplomaMode = (typeof INVITATION_DIPLOMA_MODES)[number];

export const INVITATION_DIPLOMAS = ["bac", "brevet"] as const;
export type InvitationDiploma = (typeof INVITATION_DIPLOMAS)[number];

export const INVITATION_RESPONSES = ["oui", "non"] as const;
export type InvitationResponse = (typeof INVITATION_RESPONSES)[number];

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
  location: string;
  diplomaMode: InvitationDiplomaMode;
  maxTotalPersons: number;
  maxPersonsPerEleve: number;
  notifyEmail: string | null;
  createdAt: string;
  updatedAt: string;
};

export type InvitationRsvpRecord = {
  id: string;
  etablissementId: string;
  pageId: string;
  eleveFirstName: string;
  eleveLastName: string;
  eleveNameNorm: string;
  response: InvitationResponse;
  presentCount: number;
  parentEmail: string;
  diploma: InvitationDiploma | null;
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
  location: string;
  diplomaMode: InvitationDiplomaMode;
  maxPersonsPerEleve: number;
  placesRemaining: number | null;
  schoolName: string;
};

export type InvitationDashboardStats = {
  ouiCount: number;
  nonCount: number;
  totalPersons: number;
  placesRemaining: number | null;
  maxTotalPersons: number;
};

export type InvitationDuplicateSuspect = {
  eleveNameNorm: string;
  eleveLabel: string;
  rsvpIds: string[];
};

/** Normalise prénom+nom pour détection de doublons. */
export function normalizeEleveName(firstName: string, lastName: string): string {
  const fold = (s: string) =>
    s
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9\s]/g, " ")
      .replace(/\s+/g, " ")
      .trim();
  return `${fold(firstName)}|${fold(lastName)}`;
}

/** Slug URL sûr (a-z0-9-). */
export function slugifyInvitation(input: string): string {
  return input
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 80) || "invitation";
}

export function diplomaLabel(d: InvitationDiploma | null | undefined): string {
  if (d === "bac") return "Baccalauréat";
  if (d === "brevet") return "Brevet";
  return "";
}
