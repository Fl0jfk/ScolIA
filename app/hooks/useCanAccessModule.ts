"use client";

import { useMemo } from "react";
import { useData } from "@/app/contexts/data";
import { useAppUser } from "@/app/hooks/useAppUser";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { hasRole } from "@/app/lib/absences-types";
import { getIntranetModuleById } from "@/app/lib/intranet-modules";
import { hasGlobalAdminRole } from "@/app/lib/intranet-roles";

/**
 * Accès client à un module intranet (matrice tenant via `/api/me/module-access`,
 * repli sur `allowedRoles` du catalogue si la matrice n’est pas encore chargée).
 */
export function useCanAccessModule(moduleId: string): {
  isLoaded: boolean;
  canAccess: boolean;
} {
  const { isLoaded, user } = useAppUser();
  const isOrgAdmin = useIsOrgAdmin();
  const data = useData();

  return useMemo(() => {
    if (!isLoaded) return { isLoaded: false, canAccess: false };
    if (!user) return { isLoaded: true, canAccess: false };
    if (isOrgAdmin || user.orgAdmin || hasGlobalAdminRole(user.roles)) {
      return { isLoaded: true, canAccess: true };
    }
    if (data.accessibleModuleIds) {
      return {
        isLoaded: true,
        canAccess: data.accessibleModuleIds.has(moduleId),
      };
    }
    const module = getIntranetModuleById(moduleId);
    if (!module) return { isLoaded: true, canAccess: false };
    if (module.orgAdminOnly) return { isLoaded: true, canAccess: false };
    const roles = user.roles ?? [];
    return {
      isLoaded: true,
      canAccess: (module.allowedRoles ?? []).some((r) => hasRole(roles, r)),
    };
  }, [isLoaded, user, isOrgAdmin, data.accessibleModuleIds, moduleId]);
}
