"use client";

import { useEffect, useMemo } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import PillarModuleDashboard from "@/app/components/module-hub/PillarModuleDashboard";
import { useData } from "@/app/contexts/data";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { hasRole } from "@/app/lib/absences-types";
import {
  DASHBOARD_PILLARS,
  pillarAllowedForRoles,
  type DashboardPillarId,
} from "@/app/lib/dashboard-pillars";
import { hasGlobalAdminRole, intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { resolveLegacyPillarTab } from "@/app/lib/pillar-module-routes";

type Props = {
  pillarId: DashboardPillarId;
  loadingLabel: string;
};

export default function PillarHubClient({ pillarId, loadingLabel }: Props) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { isLoaded, user } = useSessionUser();
  const isOrgAdmin = useIsOrgAdmin();
  const data = useData();

  useEffect(() => {
    const target = resolveLegacyPillarTab(pillarId, searchParams.get("tab"));
    if (target) router.replace(target);
  }, [searchParams, router, pillarId]);

  const roles = useMemo(() => {
    if (!user) return [];
    return intranetRolesFromMetadata(user.publicMetadata);
  }, [user]);

  const accessible = useMemo(() => {
    if (!isLoaded || !user || !data?.categories) return new Set<string>();
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
  }, [isLoaded, user, data, isOrgAdmin, roles]);

  const pillar = DASHBOARD_PILLARS.find((p) => p.id === pillarId);

  if (!isLoaded) {
    return <p className="p-10 text-center text-slate-500">{loadingLabel}</p>;
  }

  if (searchParams.get("tab") && resolveLegacyPillarTab(pillarId, searchParams.get("tab"))) {
    return <p className="p-10 text-center text-slate-500">Redirection…</p>;
  }

  if (
    !pillar ||
    !pillarAllowedForRoles(pillar, roles, { orgAdmin: isOrgAdmin })
  ) {
    return (
      <p className="p-10 text-center text-slate-500">
        Cet espace n’est pas accessible avec votre profil.
      </p>
    );
  }

  return (
    <PillarModuleDashboard
      pillarId={pillarId}
      categories={data?.categories ?? []}
      accessibleModuleIds={accessible}
      roles={roles}
      orgAdmin={isOrgAdmin}
    />
  );
}
