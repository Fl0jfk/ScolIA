import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { getActiveEstablishments } from "@/app/lib/app-config-establishments";
import { isAnyDirectionRole } from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import {
  hasGlobalAdminRole,
  hasMasterRole,
  hasRole,
} from "@/app/lib/intranet-role-utils";
import { elevesSecteursFromCapabilities } from "@/app/lib/ocr-flux";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import { resolveOcrCapabilitiesForClerkUserServer } from "@/app/lib/onedrive-user-profiles.server";
import { safeCurrentUser } from "@/app/lib/intranet-session";

const ALL_SECTEURS: Secteur[] = ["ecole", "college", "lycee"];

export function isPilotageDirectionRole(roles: string[]): boolean {
  return (
    hasRole(roles, "direction_ecole") ||
    hasRole(roles, "direction_college") ||
    hasRole(roles, "direction_lycee") ||
    isAnyDirectionRole(roles)
  );
}

export function canAccessPilotageModule(roles: string[]): boolean {
  if (hasGlobalAdminRole(roles) || hasMasterRole(roles)) return true;
  if (hasRole(roles, "administratif")) return true;
  return isPilotageDirectionRole(roles);
}

function secteursFromDirectionRoles(roles: string[]): Secteur[] {
  const out: Secteur[] = [];
  if (hasRole(roles, "direction_ecole")) out.push("ecole");
  if (hasRole(roles, "direction_college")) out.push("college");
  if (hasRole(roles, "direction_lycee")) out.push("lycee");
  return out;
}

export async function resolvePilotageSecteursForRoles(
  roles: string[],
  userId: string,
): Promise<Secteur[]> {
  if (hasGlobalAdminRole(roles) || hasMasterRole(roles)) return [...ALL_SECTEURS];

  const fromDirection = secteursFromDirectionRoles(roles);
  if (fromDirection.length > 0) return fromDirection;

  const user = await safeCurrentUser();
  if (user?.id) {
    const assigned = getActiveEstablishments((await loadAppConfig()).establishments)
      .filter((e) => e.directorClerkUserId === user.id)
      .map((e) => inferEstablishmentKind(e))
      .filter((k): k is Secteur => k === "ecole" || k === "college" || k === "lycee");
    if (assigned.length) return [...new Set(assigned)];
  }

  if (hasRole(roles, "administratif") || isAnyDirectionRole(roles)) {
    const clerkUser = await safeCurrentUser();
    if (clerkUser) {
      const caps = await resolveOcrCapabilitiesForClerkUserServer(clerkUser);
      const fromOcr = elevesSecteursFromCapabilities(caps);
      if (fromOcr.length) return fromOcr;
    }
    if (hasRole(roles, "administratif")) return [...ALL_SECTEURS];
  }

  void userId;
  return fromDirection;
}

export function canSeePilotageNotes(roles: string[]): boolean {
  return isPilotageDirectionRole(roles);
}

/** Notes : direction seulement (pas le secrétariat seul). */
export function canWritePilotageNotes(roles: string[]): boolean {
  return isPilotageDirectionRole(roles);
}

export function canIndexPilotage(roles: string[]): boolean {
  if (hasGlobalAdminRole(roles) || hasMasterRole(roles)) return true;
  return hasRole(roles, "administratif");
}
