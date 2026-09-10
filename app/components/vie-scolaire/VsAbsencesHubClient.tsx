"use client";

import { useMemo } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import AccueilAbsencesClient from "@/app/components/accueil/AccueilAbsencesClient";
import AccueilAbsencesConsultationClient from "@/app/components/vie-scolaire/AccueilAbsencesConsultationClient";
import VsAppelsClient from "@/app/components/vie-scolaire/VsAppelsClient";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabNav, { type ModuleTabItem } from "@/app/components/module-chrome/ModuleTabNav";
import { useData } from "@/app/contexts/data";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { hasRole } from "@/app/lib/intranet-role-utils";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";

export type AbsencesHubTab = "declarer" | "consulter" | "appels";

function resolveDefaultTab(
  available: AbsencesHubTab[],
  roles: string[],
): AbsencesHubTab {
  if (available.length === 0) return "declarer";
  if (available.length === 1) return available[0]!;
  // Accueil : priorité déclaration. CPE / VS : priorité consultation si dispo.
  if (hasRole(roles, "accueil") && available.includes("declarer")) return "declarer";
  if (
    (hasRole(roles, "cpe") || hasRole(roles, "surveillant")) &&
    available.includes("consulter")
  ) {
    return "consulter";
  }
  return available[0]!;
}

export default function VsAbsencesHubClient() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const pathname = usePathname();
  const { user } = useSessionUser();
  const isOrgAdmin = useIsOrgAdmin();
  const { accessibleModuleIds } = useData();
  const roles = rolesFromUserLike(user);

  const canDeclarer =
    isOrgAdmin ||
    Boolean(accessibleModuleIds?.has("accueil-absences")) ||
    (accessibleModuleIds === null &&
      (hasRole(roles, "accueil") ||
        hasRole(roles, "cpe") ||
        hasRole(roles, "administratif") ||
        hasRole(roles, "surveillant") ||
        hasRole(roles, "comptabilite")));

  const canConsulter =
    isOrgAdmin ||
    Boolean(accessibleModuleIds?.has("absences-accueil-consultation")) ||
    (accessibleModuleIds === null &&
      (hasRole(roles, "cpe") ||
        hasRole(roles, "surveillant") ||
        hasRole(roles, "administratif")));

  // Appels : uniquement si le module est réellement accessible (sinon WIP masqué).
  const canAppels = isOrgAdmin || Boolean(accessibleModuleIds?.has("vs-appels"));

  const available = useMemo(() => {
    const tabs: AbsencesHubTab[] = [];
    if (canDeclarer) tabs.push("declarer");
    if (canConsulter) tabs.push("consulter");
    if (canAppels) tabs.push("appels");
    return tabs;
  }, [canDeclarer, canConsulter, canAppels]);

  const tab = useMemo((): AbsencesHubTab => {
    const raw = searchParams.get("tab");
    if (raw === "declarer" || raw === "declare" || raw === "accueil") {
      return available.includes("declarer") ? "declarer" : resolveDefaultTab(available, roles);
    }
    if (raw === "consulter" || raw === "consultation" || raw === "declarees") {
      return available.includes("consulter") ? "consulter" : resolveDefaultTab(available, roles);
    }
    if (raw === "appels" || raw === "appel" || raw === "presence") {
      return available.includes("appels") ? "appels" : resolveDefaultTab(available, roles);
    }
    return resolveDefaultTab(available, roles);
  }, [searchParams, available, roles]);

  const setTab = (next: AbsencesHubTab) => {
    const params = new URLSearchParams(searchParams.toString());
    params.set("tab", next);
    const q = params.toString();
    router.replace(q ? `${pathname}?${q}` : pathname, { scroll: false });
  };

  const tabs: ModuleTabItem<AbsencesHubTab>[] = [
    { id: "declarer", label: "Absence accueil", icon: "☎️", hidden: !canDeclarer },
    {
      id: "consulter",
      label: "Absences déclarées à l'accueil",
      icon: "📋",
      hidden: !canConsulter,
    },
    { id: "appels", label: "Appel en classe", icon: "✅", hidden: !canAppels },
  ];

  if (available.length === 0) {
    return (
      <ModulePageShell maxWidthClass="max-w-3xl">
        <ModulePageHeader
          eyebrow="Vie scolaire"
          title="Absences"
          description="Aucun accès absence disponible pour votre profil."
        />
      </ModulePageShell>
    );
  }

  const showTabs = available.length > 1;

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Vie scolaire"
        title="Absences"
        description={
          showTabs
            ? "Déclaration au standard, consultation des saisies et appel de classe — selon vos droits."
            : tab === "declarer"
              ? "Déclarer une absence au standard."
              : tab === "consulter"
                ? "Consulter les absences saisies à l’accueil."
                : "Appel de présence en classe."
        }
      />
      {showTabs ? (
        <ModuleTabNav tabs={tabs} active={tab} onChange={setTab} className="mb-4" scroll />
      ) : null}
      {tab === "declarer" && canDeclarer ? <AccueilAbsencesClient embedded /> : null}
      {tab === "consulter" && canConsulter ? (
        <AccueilAbsencesConsultationClient embedded />
      ) : null}
      {tab === "appels" && canAppels ? <VsAppelsClient embedded /> : null}
    </ModulePageShell>
  );
}
