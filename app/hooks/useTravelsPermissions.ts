"use client";

import { useUser } from "@clerk/nextjs";
import { useMemo } from "react";
import { useAppContext } from "@/app/hooks/useAppContext";
import {
  canSignTravelsDirectionForEtab,
  isTripOwnerOrCreator,
} from "@/app/lib/travels-direction-permissions";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import type { TravelsTrip } from "@/app/lib/travels-types";

export function useTravelsPermissions(trip: TravelsTrip | null) {
  const { user } = useUser();
  const { data: appCtx } = useAppContext();

  const roles = useMemo(() => {
    const fromContext = appCtx?.session?.intranetRoles;
    if (Array.isArray(fromContext) && fromContext.length > 0) return fromContext;
    return intranetRolesFromMetadata(user?.publicMetadata);
  }, [appCtx?.session?.intranetRoles, user?.publicMetadata]);

  return useMemo(() => {
    const etabForSign = trip?.data?.etablissement || "";
    const isDirection = canSignTravelsDirectionForEtab(user, etabForSign);
    const isCompta =
      roles.includes("comptabilité") || hasRole(roles, "comptabilite");
    const isAdministratif = hasRole(roles, "administratif");
    const isGlobalAdmin = appCtx?.session?.isGlobalAdmin === true || hasGlobalAdminRole(roles);
    const canReassignTripOwner = isAdministratif || isGlobalAdmin;
    const isOwner = trip ? isTripOwnerOrCreator(trip, user) : false;
    const canManageFiles = isOwner || isDirection || isCompta;
    const canAddDocuments = canManageFiles || isAdministratif;
    const canUseInternalThread = isOwner || isDirection || isCompta;
    const canSeeTravelDocHoverActions = isDirection || isAdministratif || isCompta;
    const canEditEffectif =
      (isOwner || isDirection || isAdministratif || isGlobalAdmin) &&
      trip != null &&
      !["SEANCE_ANNULEE", "REJETE", "ANNULE"].includes(trip.status);
    const canAccessComptaTab = isCompta || isAdministratif || isDirection;

    return {
      user,
      isOwner,
      isDirection,
      canSign: isDirection,
      isCompta,
      canAccessComptaTab,
      isAdministratif,
      isGlobalAdmin,
      canReassignTripOwner,
      canManageFiles,
      canAddDocuments,
      canUseInternalThread,
      canSeeTravelDocHoverActions,
      canEditEffectif,
    };
  }, [trip, user, roles, appCtx?.session?.isGlobalAdmin]);
}
