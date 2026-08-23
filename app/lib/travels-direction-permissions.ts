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
