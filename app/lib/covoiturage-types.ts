export const COVOITURAGE_S3 = {
  profiles: "covoiturage/profiles.json",
  matches: "covoiturage/matches.json",
} as const;

export type CovoiturageStatus = "active" | "complete" | "unregistered";

export type CovoiturageDirection = "morning" | "evening" | "both";

export type CovoiturageProfile = {
  externalUserId: string;
  displayName: string;
  email: string;
  status: CovoiturageStatus;
  establishments: string[];
  zones: string[];
  direction: CovoiturageDirection;
  note?: string;
  schoolYear: string;
  registeredAt: string;
  updatedAt: string;
  consentAt: string;
};

export type CovoiturageMatchStatus = "pending" | "revealed" | "declined" | "cancelled";

export type CovoiturageMatch = {
  id: string;
  profileA: string;
  profileB: string;
  matchedZone: string;
  matchedEstablishments: string[];
  status: CovoiturageMatchStatus;
  acceptedA: boolean;
  acceptedB: boolean;
  declinedBy?: string;
  createdAt: string;
  revealedAt?: string;
};

export function currentSchoolYear(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth();
  if (m >= 8) return `${y}-${y + 1}`;
  return `${y - 1}-${y}`;
}

export function normalizePostalCode(raw: string): string | null {
  const v = raw.trim().replace(/\s+/g, "");
  if (!/^\d{5}$/.test(v)) return null;
  return v;
}

export function directionLabel(d: CovoiturageDirection): string {
  if (d === "morning") return "Matin";
  if (d === "evening") return "Soir";
  return "Matin et soir";
}
