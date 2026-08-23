"use client";

import { useEffect, useMemo } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import PillarModuleDashboard from "@/app/components/module-hub/PillarModuleDashboard";
import { useData } from "@/app/contexts/data";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { hasRole } from "@/app/lib/absences-types";
import { hasGlobalAdminRole, intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { resolveLegacyPillarTab } from "@/app/lib/pillar-module-routes";

export default function ElevesHubClient() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { isLoaded, user } = useSessionUser();
  const isOrgAdmin = useIsOrgAdmin();
  const data = useData();

  useEffect(() => {
    const target = resolveLegacyPillarTab("eleves", searchParams.get("tab"));
    if (target) router.replace(target);
  }, [searchParams, router]);

  const accessible = useMemo(() => {
    if (!isLoaded || !user || !data?.categories) return new Set<string>();
    const roles = intranetRolesFromMetadata(user.publicMetadata);
    const ids = new Set<string>();
    for (const category of data.categories) {
      if (category.orgAdminOnly) {
        if (isOrgAdmin) ids.add(category.moduleId);
        continue;
      }
      if (
        hasGlobalAdminRole(roles) ||
        (category.allowedRoles ?? []).some((r) => hasRole(roles, r))
      ) {
        ids.add(category.moduleId);
      }
    }
    return ids;
  }, [isLoaded, user, data, isOrgAdmin]);

  if (!isLoaded) {
    return <p className="p-10 text-center text-slate-500">Chargement du module Élèves…</p>;
  }

  if (searchParams.get("tab") && resolveLegacyPillarTab("eleves", searchParams.get("tab"))) {
    return <p className="p-10 text-center text-slate-500">Redirection…</p>;
  }

  return (
    <PillarModuleDashboard
      pillarId="eleves"
      categories={data?.categories ?? []}
      accessibleModuleIds={accessible}
    />
  );
}
