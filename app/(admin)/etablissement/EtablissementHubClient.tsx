"use client";

import dynamic from "next/dynamic";
import { useMemo } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useUser } from "@clerk/nextjs";
import ModuleHubShell from "@/app/components/module-hub/ModuleHubShell";
import { useData } from "@/app/contexts/data";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { hasRole } from "@/app/lib/absences-types";
import { hasGlobalAdminRole, intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

const OrganigrammePage = dynamic(() => import("@/app/(admin)/organigramme/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement de l&apos;organigramme…</p>,
});
const RgpdPage = dynamic(() => import("@/app/(admin)/conformite-rgpd/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement RGPD…</p>,
});
const BrainPage = dynamic(() => import("@/app/(admin)/chatbot-knowledge/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement Brain AI…</p>,
});
const DomainPlanningPage = dynamic(() => import("@/app/(admin)/domain-planning/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement des enseignements…</p>,
});

const TAB_DEFS = [
  { id: "organigramme", label: "Organigramme", moduleId: "organigramme" },
  { id: "rgpd", label: "Conformité RGPD", moduleId: "conformite-rgpd" },
  { id: "brain", label: "Brain AI", moduleId: "chatbot-knowledge" },
  { id: "transversal", label: "Enseignements transversaux", moduleId: "domain-planning" },
] as const;

type TabId = (typeof TAB_DEFS)[number]["id"];

export default function EtablissementHubClient() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { isLoaded, user } = useUser();
  const isOrgAdmin = useIsOrgAdmin();
  const data = useData();

  const accessible = useMemo(() => {
    if (!isLoaded || !user || !data?.categories) return new Set<string>();
    const roles = intranetRolesFromMetadata(user.publicMetadata);
    const ids = new Set<string>();
    for (const category of data.categories) {
      if (category.orgAdminOnly && !isOrgAdmin) continue;
      if (
        hasGlobalAdminRole(roles) ||
        (category.allowedRoles ?? []).some((r) => hasRole(roles, r))
      ) {
        ids.add(category.moduleId);
      }
    }
    return ids;
  }, [isLoaded, user, data, isOrgAdmin]);

  const tabs = TAB_DEFS.filter((t) => accessible.has(t.moduleId));
  const raw = searchParams.get("tab") as TabId | null;
  const active: TabId =
    raw && tabs.some((t) => t.id === raw) ? raw : (tabs[0]?.id ?? "organigramme");

  const setTab = (id: string) => router.push(`/etablissement?tab=${id}`);

  if (!isLoaded) {
    return <p className="p-10 text-center text-slate-500">Chargement du module Établissement…</p>;
  }

  if (tabs.length === 0) {
    return (
      <ModuleHubShell
        title="Établissement"
        description="Aucun sous-module accessible pour votre profil."
        tabs={[]}
        active=""
        onChange={() => {}}
      >
        <p className="p-6 text-sm text-slate-500">Contactez un administrateur si besoin.</p>
      </ModuleHubShell>
    );
  }

  return (
    <ModuleHubShell
      title="Établissement"
      description="Portail établissement — organigramme, RGPD, Brain AI et enseignements transversaux."
      tabs={tabs.map((t) => ({ id: t.id, label: t.label }))}
      active={active}
      onChange={setTab}
    >
      {active === "organigramme" ? <OrganigrammePage /> : null}
      {active === "rgpd" ? <RgpdPage /> : null}
      {active === "brain" ? <BrainPage /> : null}
      {active === "transversal" ? <DomainPlanningPage /> : null}
    </ModuleHubShell>
  );
}
