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

const TravelsPage = dynamic(() => import("@/app/(admin)/travels/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement des sorties…</p>,
});
const InternatPage = dynamic(() => import("@/app/(admin)/gestion-internat/GestionInternatClient"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement de l&apos;internat…</p>,
});
const StagesPage = dynamic(() => import("@/app/(admin)/stages/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement des stages…</p>,
});
const OcrPage = dynamic(() => import("@/app/(admin)/agentIAOCR/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement OCR…</p>,
});
const CertificatesPage = dynamic(() => import("@/app/(admin)/certificates/page"), {
  loading: () => <p className="p-8 text-sm text-slate-500">Chargement des certificats…</p>,
});

const TAB_DEFS = [
  { id: "travels", label: "Sorties scolaires", moduleId: "travels" },
  { id: "internat", label: "Internat", moduleId: "internat" },
  { id: "stages", label: "Stages & conventions", moduleId: "stages" },
  { id: "ocr", label: "Documents IA", moduleId: "agent-ia-ocr" },
  { id: "certificates", label: "Certificats", moduleId: "certificates" },
] as const;

type TabId = (typeof TAB_DEFS)[number]["id"];

export default function ElevesHubClient() {
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
    raw && tabs.some((t) => t.id === raw) ? raw : (tabs[0]?.id ?? "travels");

  const setTab = (id: string) => router.push(`/eleves?tab=${id}`);

  if (!isLoaded) {
    return <p className="p-10 text-center text-slate-500">Chargement du module Élèves…</p>;
  }

  if (tabs.length === 0) {
    return (
      <ModuleHubShell
        title="Élèves"
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
      title="Élèves"
      description="Portail élève unifié — sorties, internat, stages, OCR et certificats."
      tabs={tabs.map((t) => ({ id: t.id, label: t.label }))}
      active={active}
      onChange={setTab}
    >
      {active === "travels" ? <TravelsPage /> : null}
      {active === "internat" ? <InternatPage /> : null}
      {active === "stages" ? <StagesPage /> : null}
      {active === "ocr" ? <OcrPage /> : null}
      {active === "certificates" ? <CertificatesPage /> : null}
    </ModuleHubShell>
  );
}
