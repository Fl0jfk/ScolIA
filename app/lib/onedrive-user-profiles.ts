import type { ClerkLikeUser } from "@/app/lib/clerk-user-types";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";

export type OneDriveUserProfile = {
  key: string;
  basePath: string;
  secteur: Secteur;
  label: string;
};

/** Clé = nom de famille Clerk (majuscules, sans accents). */
export const ONEDRIVE_USER_BASES: Record<
  string,
  { basePath: string; secteur: Secteur; label: string }
> = {
  "HACQUEVILLE-MATHI": {
    basePath: "Dossier élèves/Lycée",
    secteur: "lycee",
    label: "Lycée",
  },
  VILLIER: { basePath: "Dossier élèves/Collège", secteur: "college", label: "Collège" },
  VILLIERS: { basePath: "Dossier élèves/Collège", secteur: "college", label: "Collège" },
  BUNO: { basePath: "Dossier élèves/Collège", secteur: "college", label: "Collège" },
  LEBLOND: { basePath: "Dossier élèves/École", secteur: "ecole", label: "École" },
};

const ONEDRIVE_EMAIL_TO_BASE_KEY: Record<string, keyof typeof ONEDRIVE_USER_BASES> = {
  "sarah.buno@ac-normandie.fr": "VILLIER",
  "sarah@laprovidence-nicolasbarre.fr": "VILLIER",
  "florian@h-me.fr": "HACQUEVILLE-MATHI",
  "florian.hacqueville-mathi@ac-normandie.fr": "HACQUEVILLE-MATHI",
  "m.leblond@laprovidence-nicolasbarre.fr": "LEBLOND",
  "pauline.leblond@ac-normandie.fr": "LEBLOND",
  "0762041f@ac-normandie.fr": "LEBLOND",
};

export function getOneDriveProfileForClerkLastName(lastName: string): OneDriveUserProfile | null {
  const key = lastName
    .toUpperCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/\s+/g, "-")
    .trim();
  if (ONEDRIVE_USER_BASES[key]) return { key, ...ONEDRIVE_USER_BASES[key] };
  for (const [k, v] of Object.entries(ONEDRIVE_USER_BASES)) {
    if (key.includes(k) || k.includes(key.replace(/-/g, ""))) return { key: k, ...v };
  }
  return null;
}

export function getOneDriveProfileForUser(input: {
  lastName?: string;
  emails?: string[];
}): OneDriveUserProfile | null {
  for (const raw of input.emails ?? []) {
    const email = raw.trim().toLowerCase();
    if (!email) continue;
    const baseKey = ONEDRIVE_EMAIL_TO_BASE_KEY[email];
    if (baseKey && ONEDRIVE_USER_BASES[baseKey]) {
      return { key: baseKey, ...ONEDRIVE_USER_BASES[baseKey] };
    }
  }
  if (input.lastName?.trim()) {
    return getOneDriveProfileForClerkLastName(input.lastName);
  }
  return null;
}

/** Premier profil configuré pour un secteur (Lycée, Collège, École). */
export function getOneDriveProfileForSecteur(secteur: Secteur): OneDriveUserProfile | null {
  for (const [key, v] of Object.entries(ONEDRIVE_USER_BASES)) {
    if (v.secteur === secteur) return { key, ...v };
  }
  return null;
}

export function getOneDriveProfileForClerkUser(user: ClerkLikeUser): OneDriveUserProfile | null {
  const emails = [
    user.primaryEmailAddress?.emailAddress,
    ...(user.emailAddresses?.map((e) => e.emailAddress) ?? []),
  ].filter((e): e is string => Boolean(e?.trim()));
  return getOneDriveProfileForUser({ lastName: user.lastName ?? undefined, emails });
}
