import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { getActiveEstablishments } from "@/app/lib/app-config-establishments";
import type { ClerkLikeUser } from "@/app/lib/clerk-user-types";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import {
  DEFAULT_ONEDRIVE_BASE_BY_SECTEUR,
  type OneDriveUserProfile,
} from "@/app/lib/onedrive-user-profiles";

function defaultBaseBySecteur(
  establishments: Awaited<ReturnType<typeof loadAppConfig>>["establishments"],
): Record<Secteur, { basePath: string; label: string }> {
  const fallback: Record<Secteur, { basePath: string; label: string }> = {
    ecole: { ...DEFAULT_ONEDRIVE_BASE_BY_SECTEUR.ecole },
    college: { ...DEFAULT_ONEDRIVE_BASE_BY_SECTEUR.college },
    lycee: { ...DEFAULT_ONEDRIVE_BASE_BY_SECTEUR.lycee },
  };
  for (const e of getActiveEstablishments(establishments)) {
    const kind = inferEstablishmentKind(e);
    if (kind === "ecole" || kind === "college" || kind === "lycee") {
      fallback[kind] = { basePath: `Dossier élèves/${e.label}`, label: e.label };
    }
  }
  return fallback;
}

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
 *  1. le mapping utilisateur → cycle configurable (Paramètres → Intégrations) ;
 *  2. la surcharge des dossiers racine par cycle (config tenant).
 */
export async function resolveOneDriveProfileForClerkUserServer(
  user: ClerkLikeUser,
): Promise<OneDriveUserProfile | null> {
  let profile: OneDriveUserProfile | null = null;

  let od: Awaited<ReturnType<typeof loadAppConfig>>["integrations"]["microsoftOneDrive"];
  let establishments: Awaited<ReturnType<typeof loadAppConfig>>["establishments"] = [];
  try {
    const config = await loadAppConfig();
    od = config.integrations.microsoftOneDrive;
    establishments = config.establishments;
  } catch {
    od = undefined;
  }

  const defaults = defaultBaseBySecteur(establishments);

  if (!profile && od?.userSecteurs?.length) {
    const clerkId = user.id?.trim();
    if (clerkId) {
      const byId = od.userSecteurs.find((m) => m.clerkUserId?.trim() === clerkId);
      if (byId) {
        const def = defaults[byId.secteur];
        profile = { key: byId.secteur, secteur: byId.secteur, basePath: def.basePath, label: def.label };
      }
    }
  }

  if (!profile && od?.userSecteurs?.length) {
    const identifiers = collectUserIdentifiers(user);
    const hit = od.userSecteurs.find((m) => {
      const target = normalizeMatch(m.match);
      return identifiers.some((id) => id === target || id.includes(target) || target.includes(id));
    });
    if (hit) {
      const def = defaults[hit.secteur];
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
