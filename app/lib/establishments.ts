import type { AppUser } from "@/app/lib/app-session";
import type { SessionLikeUser } from "@/app/lib/app-actor-types";
import { getEstablishmentByLabel, loadAppConfig } from "@/app/lib/app-config";
import {
  canSignForEstablishmentLabel,
  userRoleSlugs,
} from "@/app/lib/establishment-sign-permissions";
import { matchEstablishment } from "@/app/lib/establishment-catalog";

export async function canSignTravelsDirectionForEtab(
  user: SessionLikeUser | null | undefined,
  etablissement: string | null | undefined,
  opts?: { rolesOverride?: string[]; extraUserIds?: string[] },
): Promise<boolean> {
  const bundle = await loadAppConfig();
  return canSignForEstablishmentLabel(user, bundle.establishments, etablissement, opts);
}

export async function canSignTravelsDirectionForEstablishmentId(
  user: SessionLikeUser | null | undefined,
  establishmentId: string | null | undefined,
  opts?: { rolesOverride?: string[]; extraUserIds?: string[] },
): Promise<boolean> {
  const bundle = await loadAppConfig();
  const est = matchEstablishment(bundle.establishments, establishmentId);
  if (!est) return false;
  return canSignForEstablishmentLabel(user, bundle.establishments, est.label, opts);
}

export function appUserToSessionLike(user: AppUser): SessionLikeUser {
  return {
    id: user.businessUserId || user.id,
    firstName: user.firstName ?? null,
    lastName: user.lastName ?? null,
    publicMetadata: { role: user.roles },
    primaryEmailAddress: user.email ? { emailAddress: user.email } : null,
  };
}

export function appUserIdCandidates(user: AppUser): string[] {
  return [user.id, user.businessUserId, user.externalUserId].filter(
    (v): v is string => Boolean(v?.trim()),
  );
}

export async function canSignTravelsDirectionFromAppUser(
  user: AppUser,
  etablissement: string | null | undefined,
): Promise<boolean> {
  return canSignTravelsDirectionForEtab(appUserToSessionLike(user), etablissement, {
    rolesOverride: user.roles,
    extraUserIds: appUserIdCandidates(user),
  });
}

export async function canSignTravelsDirectionFromAppUserByEstablishmentId(
  user: AppUser,
  establishmentId: string | null | undefined,
): Promise<boolean> {
  const bundle = await loadAppConfig();
  const est = matchEstablishment(bundle.establishments, establishmentId);
  if (!est) return false;
  return canSignTravelsDirectionFromAppUser(user, est.label);
}

export async function resolveDirectorForEstablishment(
  etablissementLabel: string | null | undefined,
): Promise<{ label: string; directrice: string; email: string }> {
  const bundle = await loadAppConfig();
  const est = getEstablishmentByLabel(bundle, etablissementLabel || "");
  if (est) {
    return {
      label: est.label,
      directrice: est.directorName || est.label,
      email: est.directorEmail || "",
    };
  }
  return {
    label: bundle.identity.shortName || bundle.identity.name,
    directrice: bundle.identity.name,
    email: "",
  };
}
