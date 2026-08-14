import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import type { ClerkLikeUser } from "@/app/lib/clerk-user-types";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import {
  getOneDriveProfileForClerkUser,
  type OneDriveUserProfile,
} from "@/app/lib/onedrive-user-profiles";

/** Dossiers racine par défaut (repli si la config tenant ne surcharge pas). */
const DEFAULT_BASE_BY_SECTEUR: Record<Secteur, { basePath: string; label: string }> = {
  ecole: { basePath: "Dossier élèves/École", label: "École" },
  college: { basePath: "Dossier élèves/Collège", label: "Collège" },
  lycee: { basePath: "Dossier élèves/Lycée", label: "Lycée" },
};

function normalizeMatch(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

function collectUserIdentifiers(user: ClerkLikeUser): string[] {
  const out: string[] = [];
  if (user.lastName?.trim()) out.push(normalizeMatch(user.lastName));
  if (user.primaryEmailAddress?.emailAddress) {
    out.push(normalizeMatch(user.primaryEmailAddress.emailAddress));
  }
  for (const e of user.emailAddresses ?? []) {
    if (e.emailAddress) out.push(normalizeMatch(e.emailAddress));
  }
  return out;
}

/**
 * Résout le profil OneDrive (dossier racine + cycle) d'un utilisateur en combinant :
 *  1. le mapping en dur historique (rétro-compat) ;
 *  2. le mapping utilisateur → cycle configurable par établissement ;
 *  3. la surcharge des dossiers racine par cycle (config tenant).
 */
export async function resolveOneDriveProfileForClerkUserServer(
  user: ClerkLikeUser,
): Promise<OneDriveUserProfile | null> {
  let profile = getOneDriveProfileForClerkUser(user);

  let od: Awaited<ReturnType<typeof loadAppConfig>>["integrations"]["microsoftOneDrive"];
  try {
    const config = await loadAppConfig();
    od = config.integrations.microsoftOneDrive;
  } catch {
    od = undefined;
  }

  if (!profile && od?.userSecteurs?.length) {
    const identifiers = collectUserIdentifiers(user);
    const hit = od.userSecteurs.find((m) => {
      const target = normalizeMatch(m.match);
      return identifiers.some((id) => id === target || id.includes(target) || target.includes(id));
    });
    if (hit) {
      const def = DEFAULT_BASE_BY_SECTEUR[hit.secteur];
      profile = { key: hit.secteur, secteur: hit.secteur, basePath: def.basePath, label: def.label };
    }
  }

  if (!profile) return null;

  const override = od?.basesBySecteur?.[profile.secteur];
  if (override?.basePath?.trim()) {
    profile = {
      ...profile,
      basePath: override.basePath.trim(),
      label: override.label?.trim() || profile.label,
    };
  }

  return profile;
}
