"use client";

import { useMemo } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabNav, { type ModuleTabItem } from "@/app/components/module-chrome/ModuleTabNav";
import VsAbsencesClient from "@/app/components/vie-scolaire/VsAbsencesClient";
import VsAppelsClient from "@/app/components/vie-scolaire/VsAppelsClient";
import FeuilleDuJourClient from "@/app/components/vie-scolaire/FeuilleDuJourClient";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

type PresenceTab = "appel" | "feuille" | "absences";

function canFollowAbsences(roles: string[], orgAdmin: boolean): boolean {
  if (orgAdmin || hasGlobalAdminRole(roles)) return true;
  if (INTRANET_DIRECTION_SLUGS.some((slug) => roles.includes(slug))) return true;
  return (
    hasRole(roles, "cpe") ||
    hasRole(roles, "administratif") ||
    roles.includes("admin")
  );
}

export default function VsPresenceClient() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const pathname = usePathname();
  const { user } = useSessionUser();
  const isOrgAdmin = useIsOrgAdmin();
  const roles = user?.publicMetadata?.role ?? [];
  const showAbsences = canFollowAbsences(roles, isOrgAdmin);

  const tab = useMemo((): PresenceTab => {
    const raw = searchParams.get("tab");
    if (raw === "absences" && showAbsences) return "absences";
    if (raw === "feuille" || raw === "jour" || raw === "presence") return "feuille";
    if (raw === "appel" || raw === "appels") return "appel";
    if (searchParams.get("filtre") && showAbsences) return "absences";
    return "appel";
  }, [searchParams, showAbsences]);

  const setTab = (next: PresenceTab) => {
    const params = new URLSearchParams(searchParams.toString());
    if (next === "appel") {
      params.set("tab", "appel");
      params.delete("filtre");
    } else if (next === "feuille") {
      params.set("tab", "feuille");
      params.delete("filtre");
    } else {
      params.set("tab", "absences");
    }
    const q = params.toString();
    router.replace(q ? `${pathname}?${q}` : pathname, { scroll: false });
  };

  const tabs: ModuleTabItem<PresenceTab>[] = [
    { id: "appel", label: "Appel", icon: "✅" },
    { id: "feuille", label: "Feuille du jour", icon: "📍" },
    { id: "absences", label: "Absences", icon: "⏱️", hidden: !showAbsences },
  ];

  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Vie scolaire"
        title="Appels & absences"
        description="Présence en classe (appel), feuille du jour (où est X — occupancy), puis suivi CPE des absents."
      />
      <ModuleTabNav tabs={tabs} active={tab} onChange={setTab} className="mb-4" />
      {tab === "appel" ? <VsAppelsClient embedded /> : null}
      {tab === "feuille" ? <FeuilleDuJourClient /> : null}
      {tab === "absences" && showAbsences ? <VsAbsencesClient embedded /> : null}
    </ModulePageShell>
  );
}
