"use client";

import { useAppUser, useSessionUser } from "@/app/hooks/useAppUser";
import { useMemo } from "react";
import { useAppContext } from "@/app/hooks/useAppContext";
import { canSignForEstablishmentLabel } from "@/app/lib/establishment-sign-permissions";
import { userIsAnyDirection } from "@/app/lib/establishment-catalog";
import { isTripOwnerOrCreator } from "@/app/lib/travels-direction-permissions";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import type { TravelsTrip } from "@/app/lib/travels-types";

export function useTravelsPermissions(trip: TravelsTrip | null) {
  const { user } = useSessionUser();
  const { user: appUser } = useAppUser();
  const { data: appCtx } = useAppContext();
  const roles = useMemo(() => {
    const fromContext = appCtx?.session?.intranetRoles;
    if (Array.isArray(fromContext) && fromContext.length > 0) return fromContext;
    if (appUser?.roles?.length) return appUser.roles;
    return intranetRolesFromMetadata(user?.publicMetadata);
  }, [appCtx?.session?.intranetRoles, appUser?.roles, user?.publicMetadata]);
  const extraUserIds = useMemo(
    () =>
      appUser
        ? [appUser.id, appUser.businessUserId, appUser.externalUserId].filter(
            (v): v is string => Boolean(v?.trim()),
          )
        : [],
    [appUser],
  );
  return useMemo(() => {
    const etabForSign = trip?.data?.etablissement || "";
    const establishments = appCtx?.establishments ?? [];
    const isDirection = userIsAnyDirection(user ?? null, establishments, roles, extraUserIds);
    const canSign = canSignForEstablishmentLabel(user ?? null, establishments, etabForSign, {
      rolesOverride: roles,
      extraUserIds,
    });
    const isCompta = roles.includes("comptabilité") || hasRole(roles, "comptabilite");
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
      canSign,
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
  }, [trip, user, roles, extraUserIds, appCtx?.session?.isGlobalAdmin, appCtx?.establishments]);
}
