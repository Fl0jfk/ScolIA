"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import ChannelsPanel from "@/app/components/messaging/ChannelsPanel";
import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { isEleveOnlyRoleSet } from "@/app/lib/intranet-role-utils";

function canAccessStaffMessaging(roles: string[]): boolean {
  if (isEleveOnlyRoleSet(roles)) return false;
  const visible = roles.filter((r) => r !== "master");
  if (visible.length === 0) return roles.includes("master");
  return visible.some((r) => r !== "parent" && r !== "eleve");
}

/**
 * Page /channels — les élèves (sans messagerie privée) restent ici.
 * Le personnel est redirigé vers l’onglet Salons de la messagerie unifiée.
 */
export default function ChannelsPage() {
  const router = useRouter();
  const { user, isLoaded } = useSessionUser();
  const roles = rolesFromUserLike(user);
  const staffMessaging = canAccessStaffMessaging(roles);

  useEffect(() => {
    if (!isLoaded) return;
    if (staffMessaging) {
      router.replace("/messagerie?section=salons");
    }
  }, [isLoaded, staffMessaging, router]);

  if (!isLoaded || staffMessaging) {
    return (
      <DashboardThemeRoot>
        <p className="p-8 text-sm text-neutral-500">Redirection…</p>
      </DashboardThemeRoot>
    );
  }

  return (
    <DashboardThemeRoot>
      <div className="mx-auto flex h-[calc(100dvh-5.5rem)] w-full max-w-[90rem] flex-col gap-3 p-3 sm:p-4 lg:p-5">
        <header className="flex items-center justify-between gap-3 px-1">
          <div>
            <h1 className="text-lg font-semibold tracking-tight text-[var(--dash-ink)] sm:text-xl">
              Salons
            </h1>
            <p className="text-[11px] font-medium text-[var(--dash-mid)]">
              Discussions d’équipe
            </p>
          </div>
          <ReplayModuleTourButton moduleId="channels" />
        </header>
        <ChannelsPanel className="min-h-0 flex-1" />
      </div>
    </DashboardThemeRoot>
  );
}
