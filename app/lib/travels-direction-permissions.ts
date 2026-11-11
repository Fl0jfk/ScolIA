/** Aligné sur la logique « canSign » de la fiche voyage (direction par établissement). */

import type { SessionLikeUser } from "@/app/lib/app-actor-types";
import { directionRolesMatchEstablishmentRef } from "@/app/lib/establishment-catalog";
import { userRoleSlugs } from "@/app/lib/establishment-sign-permissions";

export function canSignTravelsDirectionForEtab(
  user: SessionLikeUser | null | undefined,
  etablissement: string | null | undefined,
): boolean {
  if (!user) return false;
  return directionRolesMatchEstablishmentRef(
    userRoleSlugs(user),
    etablissement,
    [],
    user.id,
  );
}

export function isTripOwner(tripOwnerId: string | null | undefined, externalUserId: string | null | undefined): boolean {
  return Boolean(tripOwnerId && externalUserId && tripOwnerId === externalUserId);
}

/** ownerId en priorité, repli sur ownerName pour dossiers anciens. */
export function isTripOwnerOrCreator(
  trip: { ownerId?: string | null; ownerName?: string | null },
  user: SessionLikeUser | null | undefined,
): boolean {
  if (!user) return false;
  if (isTripOwner(trip.ownerId, user.id)) return true;
  if (trip.ownerName && user.fullName && trip.ownerName.trim() === user.fullName.trim()) return true;
  return false;
}

/** Statuts workflow autorisés lors d'une réouverture depuis « Finalisé » (VALIDE). */
const TRAVELS_REOPEN_FROM_VALIDE_STATUSES = [
  "EN_ATTENTE_DIR_INITIAL",
  "PROF_LOGISTICS",
  "EN_ATTENTE_BUS_SIGNATURE",
  "EN_ATTENTE_COMPTA",
  "EN_ATTENTE_DIR_FINAL",
] as const;

export function isValidTravelsReopenFromValideStatus(status: string): boolean {
  return (TRAVELS_REOPEN_FROM_VALIDE_STATUSES as readonly string[]).includes(status);
}

/**
 * NJ-07 (D-053) : qui peut réécrire un dossier voyage existant, et qui peut le passer en « finalisé ».
 * Porteur du dossier, direction, comptabilité, administratif, admin. Un autre rôle du module (professeur,
 * surveillant, CPE, infirmerie, accueil…) ne modifie pas le dossier d'un autre.
 */
export function canEditExistingTravelTrip(opts: {
  roles: string[];
  orgAdmin?: boolean;
  isOwner: boolean;
}): boolean {
  return opts.isOwner || canFinalizeTravelTrip(opts);
}

const TRIP_FINAL_STATUSES = new Set(["VALIDE", "FINALISE_DIR_ATTENTE_ELEVES"]);

export function canFinalizeTravelTrip(opts: { roles: string[]; orgAdmin?: boolean }): boolean {
  if (opts.orgAdmin) return true;
  return opts.roles.some(
    (r) =>
      r === "admin" ||
      r === "master" ||
      r === "administratif" ||
      r === "comptabilite" ||
      r === "direction" ||
      r.startsWith("direction_"),
  );
}

/** Passer un dossier à un statut final sans y être habilité est refusé. */
export function isForbiddenTravelStatusJump(
  existingStatus: unknown,
  requestedStatus: unknown,
  canFinalize: boolean,
): boolean {
  if (canFinalize) return false;
  const next = String(requestedStatus ?? "");
  return TRIP_FINAL_STATUSES.has(next) && String(existingStatus ?? "") !== next;
}
