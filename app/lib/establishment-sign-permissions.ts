import type { Establishment } from "@/app/lib/app-config-schemas";
import type { ClerkLikeUser } from "@/app/lib/clerk-user-types";
import { normRole } from "@/app/lib/intranet-role-utils";

/** Rôles bruts Clerk (y compris libellés historiques hors catalogue). */
export function userRoleSlugs(user: ClerkLikeUser | null | undefined): string[] {
  if (!user?.publicMetadata) return [];
  const raw = user.publicMetadata.role;
  return Array.isArray(raw) ? raw.map(String) : raw ? [String(raw)] : [];
}

/** Signature direction : rôles Clerk configurés sur l’établissement (sync, client + serveur). */
export function canSignForEstablishmentLabel(
  user: ClerkLikeUser | null | undefined,
  establishments: Establishment[],
  etablissementLabel: string | null | undefined,
): boolean {
  if (!user || !etablissementLabel) return false;
  const est = establishments.find((e) => e.label === etablissementLabel && e.active !== false);
  if (!est) return false;
  if (est.directorClerkUserId && user.id && est.directorClerkUserId === user.id) return true;
  const roles = userRoleSlugs(user).map(normRole);
  const slugs = (est.clerkRoleSlugs || []).map(normRole);
  return slugs.some((s) => roles.some((r) => r === s || r.includes(s) || s.includes(r)));
}
