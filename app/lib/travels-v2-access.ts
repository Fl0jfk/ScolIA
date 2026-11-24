import "server-only";

import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canSignTravelsDirectionForEtab, isTripOwnerOrCreator } from "@/app/lib/travels-direction-permissions";
import { isGlobalAdminUser, userHasAdministratifRole, userHasComptaRoleFromMetadata } from "@/app/lib/travels-roles";
import type { TravelsTrip } from "@/app/lib/travels-types";

/** Rôles de l'utilisateur courant vis-à-vis d'un voyage (vue Inscriptions, circuit v2). */
export type Access = {
  isOwner: boolean;
  isDirection: boolean;
  isAdmin: boolean;
  isAdministratif: boolean;
  isCompta: boolean;
  userName: string;
};

export async function resolveAccess(trip: TravelsTrip): Promise<Access | null> {
  const me = await safeCurrentUser();
  if (!me) return null;
  const meta = me.publicMetadata as Record<string, unknown> | undefined;
  return {
    isOwner: isTripOwnerOrCreator(trip, me),
    isDirection: Boolean(await canSignTravelsDirectionForEtab(me, trip.data?.etablissement)),
    isAdmin: isGlobalAdminUser({ publicMetadata: meta }),
    isAdministratif: userHasAdministratifRole({ publicMetadata: meta }),
    isCompta: userHasComptaRoleFromMetadata(meta),
    userName: me.fullName || "Utilisateur",
  };
}

export const canRead = (a: Access) => a.isOwner || a.isDirection || a.isAdmin || a.isAdministratif || a.isCompta;
export const canWrite = (a: Access) => a.isOwner || a.isDirection || a.isAdmin || a.isAdministratif;
export const canSeeAmount = (a: Access) => a.isCompta || a.isDirection || a.isAdmin;
