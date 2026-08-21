import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { getActiveEstablishments } from "@/app/lib/app-config-establishments";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import {
  hasMasterRole,
  normRole,
} from "@/app/lib/intranet-role-utils";
import { elevesSecteursFromCapabilities } from "@/app/lib/ocr-flux";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import { resolveOcrCapabilitiesForClerkUserServer } from "@/app/lib/onedrive-user-profiles.server";
import { safeCurrentUser } from "@/app/lib/intranet-session";

const DIRECTION_SECTEUR_ROLES: Array<{ role: string; secteur: Secteur }> = [
  { role: "direction_ecole", secteur: "ecole" },
  { role: "direction_college", secteur: "college" },
  { role: "direction_lycee", secteur: "lycee" },
];

function hasExactRole(roles: string[], slug: string): boolean {
  const want = normRole(slug);
  return roles.some((r) => normRole(r) === want);
}

export function directionSecteursFromRoles(roles: string[]): Secteur[] {
  return DIRECTION_SECTEUR_ROLES.filter((row) => hasExactRole(roles, row.role)).map((row) => row.secteur);
}

export function isPilotageDirectionRole(roles: string[]): boolean {
  return directionSecteursFromRoles(roles).length > 0;
}

export function isPilotageAdministratif(roles: string[]): boolean {
  return hasExactRole(roles, "administratif");
}

/** Tuile + routes : uniquement direction d’un cycle ou secrétariat. Pas prof, CPE, admin nu, « direction » générique. */
export function canAccessPilotageModule(roles: string[]): boolean {
  if (hasMasterRole(roles)) return true;
  return isPilotageAdministratif(roles) || isPilotageDirectionRole(roles);
}

export async function resolvePilotageSecteursForRoles(
  roles: string[],
  userId: string,
): Promise<Secteur[]> {
  void userId;
  const fromDirection = directionSecteursFromRoles(roles);

  const user = await safeCurrentUser();
  if (isPilotageAdministratif(roles) && user) {
    const caps = await resolveOcrCapabilitiesForClerkUserServer(user);
    const fromOcr = elevesSecteursFromCapabilities(caps);
    if (fromOcr.length) return fromOcr;
    if (!fromDirection.length) return [];
  }

  if (fromDirection.length > 0) return fromDirection;

  if (user?.id) {
    const assigned = getActiveEstablishments((await loadAppConfig()).establishments)
      .filter((e) => e.directorClerkUserId === user.id)
      .map((e) => inferEstablishmentKind(e))
      .filter((k): k is Secteur => k === "ecole" || k === "college" || k === "lycee");
    if (assigned.length) return [...new Set(assigned)];
  }

  return [];
}

/** Notes de classeur : direction du cycle uniquement — jamais le secrétariat. */
export function canSeePilotageNotes(roles: string[]): boolean {
  if (isPilotageAdministratif(roles)) return false;
  return isPilotageDirectionRole(roles);
}

export function canWritePilotageNotes(roles: string[]): boolean {
  return canSeePilotageNotes(roles);
}

export function canIndexPilotage(roles: string[]): boolean {
  return isPilotageAdministratif(roles);
}
