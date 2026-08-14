"use client";

import { Suspense, useEffect, useState } from "react";
import dynamic from "next/dynamic";
import { useUser } from "@clerk/nextjs";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { canAccessDomainPlanningSettingsFromRoles } from "@/app/lib/intranet-role-utils";
import TransversalSessionsTab from "@/app/components/domain-planning/TransversalSessionsTab";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";

const DomainPlanningSettingsTab = dynamic(
  () => import("@/app/components/domain-planning/DomainPlanningSettingsTab"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

type DomainPlanningTab = "positionnements" | "settings";

function DomainPlanningPageContent() {
  const { user, isLoaded } = useUser();
  const isOrgAdmin = useIsOrgAdmin();
  const intranetRoles = intranetRolesFromMetadata(user?.publicMetadata);
  const [domains, setDomains] = useState<{ id: string; coordinatorClerkUserIds?: string[] }[]>([]);
  const [activeTab, setActiveTab] = useState<DomainPlanningTab>("positionnements");

  const isEvarsCoordinator = Boolean(
    user?.id && domains.find((d) => d.id === "evars")?.coordinatorClerkUserIds?.includes(user.id),
  );

  const canAccessSettings =
    isOrgAdmin ||
    canAccessDomainPlanningSettingsFromRoles(intranetRoles) ||
    Boolean(user?.id && domains.some((d) => d.coordinatorClerkUserIds?.includes(user.id)));

  useEffect(() => {
    fetch("/api/domain-planning/domains", { cache: "no-store" })
      .then((r) => r.json())
      .then((j) => setDomains(j.domains || []))
      .catch(() => setDomains([]));
  }, []);

  if (!isLoaded || !user) {
    return <div className="p-20 text-center font-bold">Initialisation…</div>;
  }

  return (
    <ModulePageShell maxWidthClass="max-w-6xl" tourModuleId="domain-planning">
      <ModulePageHeader
        eyebrow="Services"
        title="Enseignements transversaux — EVARS"
      />

      <ModuleTabNav
        className="mb-4"
        tabs={[
          {
            id: "positionnements",
            label: "Positionnements",
            dataAttrs: { "data-domain-planning-tab": "reservation" },
          },
          { id: "settings", label: "Paramétrage", hidden: !canAccessSettings },
        ]}
        active={activeTab}
        onChange={setActiveTab}
      />

      {activeTab === "settings" && canAccessSettings ? (
        <DomainPlanningSettingsTab />
      ) : (
        <>
          {canAccessSettings && domains.some((d) => d.id === "evars" && !d.coordinatorClerkUserIds?.length) && (
            <div className="mb-4 rounded-2xl bg-amber-50 border border-amber-200 py-3 px-4 text-sm text-amber-900">
              <span className="font-black">Première configuration :</span> ouvrez l&apos;onglet{" "}
              <button type="button" className="font-black underline" onClick={() => setActiveTab("settings")}>
                Paramétrage
              </button>{" "}
              pour désigner la responsable EVARS.
            </div>
          )}
          <TransversalSessionsTab isCoordinator={isEvarsCoordinator} />
        </>
      )}
    </ModulePageShell>
  );
}

export default function DomainPlanningPage() {
  return (
    <Suspense fallback={<div className="p-8 text-slate-500 text-sm">Chargement…</div>}>
      <DomainPlanningPageContent />
    </Suspense>
  );
}
