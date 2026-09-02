import type { Establishment } from "@/app/lib/app-config-schemas";
import type { SessionLikeUser } from "@/app/lib/app-actor-types";
import { userCanActAsDirectionFor } from "@/app/lib/establishment-catalog";

/** Rôles bruts (y compris libellés historiques hors catalogue). */
export function userRoleSlugs(user: SessionLikeUser | null | undefined): string[] {
  if (!user?.publicMetadata) return [];
  const raw = user.publicMetadata.role;
  return Array.isArray(raw) ? raw.map(String) : raw ? [String(raw)] : [];
}

/** Signature direction : responsable nommé ou rôle intranet de la fiche (id / libellé / kind). */
export function canSignForEstablishmentLabel(
  user: SessionLikeUser | null | undefined,
  establishments: Establishment[],
  etablissementLabel: string | null | undefined,
  opts?: { rolesOverride?: string[]; extraUserIds?: string[] },
): boolean {
  if (!user || !etablissementLabel) return false;
  return userCanActAsDirectionFor(
    user,
    establishments,
    etablissementLabel,
    opts?.rolesOverride,
    opts?.extraUserIds,
  );
}
