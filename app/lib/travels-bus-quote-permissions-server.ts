import { canSignTravelsDirectionForEtab } from "@/app/lib/establishments";
import { isOrgAdminFromPublicMetadata } from "@/app/lib/intranet-auth-metadata";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { hasGlobalAdminRole } from "@/app/lib/intranet-role-utils";
import { isTripOwnerOrCreator } from "@/app/lib/travels-direction-permissions";
import type { TravelsBusQuoteSelectionCtx } from "@/app/lib/travels-bus-quote-permissions";
import {
  userHasAdministratifRole,
  userHasComptaRoleFromMetadata,
} from "@/app/lib/travels-roles";
import type { TravelsTrip } from "@/app/lib/travels-types";

export async function buildTravelsBusQuoteSelectionCtx(
  user: {
    id: string;
    publicMetadata?: Record<string, unknown> | null;
  },
  trip: TravelsTrip,
): Promise<TravelsBusQuoteSelectionCtx> {
  const meta = user.publicMetadata;
  const roles = intranetRolesFromMetadata(meta);
  const etab = trip.data?.etablissement;
  return {
    isOwner: isTripOwnerOrCreator(trip, user),
    canSign: await canSignTravelsDirectionForEtab(user, etab),
    isAdministratif: userHasAdministratifRole(user),
    isCompta: userHasComptaRoleFromMetadata(meta),
    isGlobalAdmin: isOrgAdminFromPublicMetadata(meta) || hasGlobalAdminRole(roles),
  };
}
