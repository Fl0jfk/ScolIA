import type { SessionLikeUser } from "@/app/lib/app-actor-types";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";

export type OneDriveUserProfile = {
  key: string;
  basePath: string;
  secteur: Secteur;
  label: string;
};

/** Repli générique (surchargé par les libellés d’établissements côté serveur). */
export const DEFAULT_ONEDRIVE_BASE_BY_SECTEUR: Record<
  Secteur,
  { basePath: string; label: string }
> = {
  ecole: { basePath: "Dossier élèves/École", label: "École" },
  college: { basePath: "Dossier élèves/Collège", label: "Collège" },
  lycee: { basePath: "Dossier élèves/Lycée", label: "Lycée" },
};

/** Mapping utilisateur → cycle : désormais uniquement via Paramètres → Intégrations (userSecteurs). */
export const ONEDRIVE_USER_BASES: Record<
  string,
  { basePath: string; secteur: Secteur; label: string }
> = {};

const ONEDRIVE_EMAIL_TO_BASE_KEY: Record<string, string> = {};

export function getOneDriveProfileForLastName(lastName: string): OneDriveUserProfile | null {
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
    return getOneDriveProfileForLastName(input.lastName);
  }
  return null;
}

/** Premier profil configuré pour un secteur (Lycée, Collège, École). */
export function getOneDriveProfileForSecteur(secteur: Secteur): OneDriveUserProfile {
  const def = DEFAULT_ONEDRIVE_BASE_BY_SECTEUR[secteur];
  return { key: secteur, secteur, basePath: def.basePath, label: def.label };
}

export function getOneDriveProfileForSessionUser(user: SessionLikeUser): OneDriveUserProfile | null {
  const emails = [
    user.primaryEmailAddress?.emailAddress,
    ...(user.emailAddresses?.map((e) => e.emailAddress) ?? []),
  ].filter((e): e is string => Boolean(e?.trim()));
  return getOneDriveProfileForUser({ lastName: user.lastName ?? undefined, emails });
}
