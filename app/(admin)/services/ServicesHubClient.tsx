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

const RequestsPage = dynamic(() => import("@/app/(admin)/requests/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement des demandes…</p>,
});
const ProfRoomPage = dynamic(() => import("@/app/(admin)/prof-room/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement des salles…</p>,
});
const PhotocopiesPage = dynamic(() => import("@/app/(admin)/photocopies-couleur/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement des photocopies…</p>,
});
const DocumentsPage = dynamic(() => import("@/app/(admin)/documents/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement du cloud…</p>,
});
const ToolboxPage = dynamic(() => import("@/app/(admin)/toolbox/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement de la boîte à outils…</p>,
});
const CovoituragePage = dynamic(() => import("@/app/(admin)/covoiturage/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement du covoiturage…</p>,
});
const ChannelsPage = dynamic(() => import("@/app/(admin)/channels/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement des salons…</p>,
});
const AssistancePage = dynamic(() => import("@/app/(admin)/assistance/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement de l&apos;assistance…</p>,
});

const TAB_DEFS = [
  { id: "demandes", label: "Demandes", moduleId: "requests-staff" },
  { id: "salles", label: "Salles", moduleId: "prof-room" },
  { id: "photocopies", label: "Photocopies", moduleId: "photocopies-couleur" },
  { id: "cloud", label: "Cloud", moduleId: "documents" },
  { id: "toolbox", label: "Boîte à outils", moduleId: "toolbox" },
  { id: "covoiturage", label: "Covoiturage", moduleId: "covoiturage" },
  { id: "salons", label: "Salons", moduleId: "channels" },
  { id: "assistance", label: "Assistance", moduleId: "assistance" },
] as const;

type TabId = (typeof TAB_DEFS)[number]["id"];

export default function ServicesHubClient() {
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
    raw && tabs.some((t) => t.id === raw) ? raw : (tabs[0]?.id ?? "demandes");

  const setTab = (id: string) => router.push(`/services?tab=${id}`);

  if (!isLoaded) {
    return <p className="p-10 text-center text-slate-500">Chargement du module Services…</p>;
  }

  if (tabs.length === 0) {
    return (
      <ModuleHubShell
        title="Services"
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
      title="Services"
      description="Portail services — demandes, salles, photocopies, cloud, outils et salons."
      tabs={tabs.map((t) => ({ id: t.id, label: t.label }))}
      active={active}
      onChange={setTab}
    >
      {active === "demandes" ? <RequestsPage /> : null}
      {active === "salles" ? <ProfRoomPage /> : null}
      {active === "photocopies" ? <PhotocopiesPage /> : null}
      {active === "cloud" ? <DocumentsPage /> : null}
      {active === "toolbox" ? <ToolboxPage /> : null}
      {active === "covoiturage" ? <CovoituragePage /> : null}
      {active === "salons" ? <ChannelsPage /> : null}
      {active === "assistance" ? <AssistancePage /> : null}
    </ModuleHubShell>
  );
}
