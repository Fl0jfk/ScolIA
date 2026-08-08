"use client";

import { Suspense, useCallback, useEffect, useMemo, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useUser } from "@clerk/nextjs";
import AbsencesPageClient from "@/app/(admin)/absences/AbsencesPageClient";
import DemandesHsePanel from "@/app/components/demandes-hse/DemandesHsePanel";
import PersonnelDashboard from "@/app/components/personnel/PersonnelDashboard";
import PersonnelStaffCard from "@/app/components/personnel/PersonnelStaffCard";
import RhAdminOverviewPanel from "@/app/components/personnel/RhAdminOverviewPanel";
import RhHubNav, { type RhHubTab } from "@/app/components/personnel/RhHubNav";
import RhNewStaffModal from "@/app/components/personnel/RhNewStaffModal";
import RhDemandePanel from "@/app/components/personnel/RhDemandePanel";
import RhMoodPulseAdminPanel from "@/app/components/personnel/RhMoodPulseAdminPanel";
import RhOnboardingPanel from "@/app/components/personnel/RhOnboardingPanel";
import RhBulkDepositPanel from "@/app/components/personnel/RhBulkDepositPanel";
import RhOrganigramPanel from "@/app/components/personnel/RhOrganigramPanel";
import RhPersonnelHome from "@/app/components/personnel/RhPersonnelHome";
import RhPlanningPanel from "@/app/components/personnel/RhPlanningPanel";
import RhRegistrePanel from "@/app/components/personnel/RhRegistrePanel";

import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";
import { canAccessHseModule } from "@/app/lib/demandes-hse-access";
import type { PersonnelDashboardData } from "@/app/lib/personnel-dashboard";
import { type PersonnelIndexEntry, type SharedPersonnelDocument } from "@/app/lib/personnel-types";

const TAB_IDS: RhHubTab[] = [
  "dashboard",
  "annuaire",
  "admin",
  "onboarding",
  "registre",
  "absences",
  "hse",
  "demande",
  "planning",
  "organigramme",
  "deposit",
];

function parseTab(raw: string | null): RhHubTab {
  if (raw === "temps") return "absences";
  if (raw && TAB_IDS.includes(raw as RhHubTab)) return raw as RhHubTab;
  return "dashboard";
}

export default function RhModuleClient() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const activeTab = parseTab(searchParams.get("tab"));
  const { isLoaded, user } = useUser();

  const [dashboard, setDashboard] = useState<PersonnelDashboardData | null>(null);
  const [index, setIndex] = useState<PersonnelIndexEntry[]>([]);
  const [sharedDocs, setSharedDocs] = useState<SharedPersonnelDocument[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showNew, setShowNew] = useState(false);
  const [canManage, setCanManage] = useState(false);

  const roles = useMemo(() => {
    const rolesRaw = user?.publicMetadata?.role;
    return Array.isArray(rolesRaw) ? rolesRaw.map(String) : rolesRaw ? [String(rolesRaw)] : [];
  }, [user]);

  const canAccessHse = canAccessHseModule(roles);

  const setTab = (tab: RhHubTab) => {
    if (tab === "absences") {
      router.push("/rh?tab=absences&view=se-declarer");
      return;
    }
    router.push(`/rh?tab=${tab}`);
  };

  const load = useCallback(async () => {
    setError(null);
    try {
      const [dRes, lRes] = await Promise.all([
        fetch("/api/personnel/dashboard", { cache: "no-store" }),
        fetch("/api/personnel", { cache: "no-store" }),
      ]);
      const dJson = await dRes.json();
      const lJson = await lRes.json();
      if (!dRes.ok) throw new Error(dJson.error || "Dashboard indisponible");
      if (!lRes.ok) throw new Error(lJson.error || "Liste indisponible");
      setDashboard(dJson);
      setIndex(lJson.index || []);
      setSharedDocs(lJson.sharedDocs || []);
      setCanManage(!!lJson.canManage);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!isLoaded) return;
    void load();
  }, [isLoaded, load]);

  useEffect(() => {
    if (!isLoaded) return;
    if (activeTab === "hse" && !canAccessHse) {
      router.replace("/rh?tab=dashboard");
    }
  }, [activeTab, canAccessHse, isLoaded, router]);

  if (!isLoaded || (loading && !dashboard)) {
    return <p className="p-10 text-center text-slate-500">Chargement du module RH…</p>;
  }
  if (error) return <p className="p-10 text-center text-rose-600">{error}</p>;
  if (!dashboard) return null;

  return (
    <div className="max-w-7xl mx-auto p-4 sm:p-6 space-y-6">
      <header>
        <h1 className="text-3xl font-black text-slate-900 tracking-tight">Module RH</h1>
        <p className="text-slate-500 text-sm mt-1">
          Portail RH unifié — personnel, absences, HSE (sans paie ni coffre bulletins).
        </p>
      </header>

      <RhHubNav
        active={activeTab}
        onChange={setTab}
        canManage={canManage}
        canAccessHse={canAccessHse}
      />

      {activeTab === "deposit" && canManage ? (
        <RhBulkDepositPanel />
      ) : activeTab === "admin" && canManage ? (
        <RhAdminOverviewPanel index={index} />
      ) : activeTab === "onboarding" && canManage ? (
        <RhOnboardingPanel />
      ) : activeTab === "registre" && canManage ? (
        <RhRegistrePanel />
      ) : activeTab === "absences" ? (
        <Suspense fallback={<p className="text-sm text-slate-500">Chargement des absences…</p>}>
          <AbsencesPageClient embeddedInRh />
        </Suspense>
      ) : activeTab === "hse" && canAccessHse ? (
        <DemandesHsePanel embeddedInRh />
      ) : activeTab === "demande" ? (
        <RhDemandePanel />
      ) : activeTab === "planning" ? (
        <RhPlanningPanel />
      ) : activeTab === "organigramme" ? (
        <RhOrganigramPanel index={index} />
      ) : activeTab === "annuaire" ? (
        <div className="space-y-6">
          <div className="bg-white rounded-2xl border border-slate-100 p-5 shadow-sm">
            <h2 className="font-black text-slate-800 mb-1">Annuaire RH</h2>
            {canManage && (
              <p className="text-xs text-indigo-600 font-medium mb-4">
                Glissez un fichier sur une fiche pour le déposer via l&apos;IA.
              </p>
            )}
            {index.length === 0 ? (
              <p className="text-sm text-slate-400 italic">Aucun dossier.</p>
            ) : (
              <div data-tour="rh-list" className="grid sm:grid-cols-2 lg:grid-cols-3 gap-3">
                {index.map((p) => (
                  <PersonnelStaffCard key={p.id} person={p} canDrop={canManage} onUploaded={() => void load()} />
                ))}
              </div>
            )}
          </div>
          <SharedDocsBlock sharedDocs={sharedDocs} canManage={canManage} onRefresh={load} />
        </div>
      ) : (
        <>
          <RhPersonnelHome canManage={canManage} />
          {canManage && (
            <div className="pt-2 border-t border-slate-100 space-y-4">
              <h2 className="text-lg font-black text-slate-800">Pilotage RH</h2>
              <RhMoodPulseAdminPanel />
              <PersonnelDashboard data={dashboard} onNewStaff={() => setShowNew(true)} />
              <SharedDocsBlock sharedDocs={sharedDocs} canManage={canManage} onRefresh={load} />
            </div>
          )}
        </>
      )}

      <RhNewStaffModal
        open={showNew}
        onClose={() => setShowNew(false)}
        onCreated={async (recordId) => {
          setShowNew(false);
          await load();
          router.push(`/rh/${recordId}`);
        }}
      />
      <ReplayModuleTourButton moduleId="rh" />
    </div>
  );
}

function SharedDocsBlock({
  sharedDocs,
  canManage,
  onRefresh,
}: {
  sharedDocs: SharedPersonnelDocument[];
  canManage: boolean;
  onRefresh: () => Promise<void>;
}) {
  return (
    <div className="bg-white rounded-2xl border border-slate-100 p-5 shadow-sm">
      <h2 className="font-black text-slate-800 mb-2">Documents utiles (tous les collaborateurs)</h2>
      <p className="text-xs text-slate-500 mb-4">Règlement intérieur, convention collective, notes de service…</p>
      {sharedDocs.length > 0 && (
        <ul className="space-y-2 mb-4">
          {sharedDocs.map((d) => (
            <li key={d.id} className="flex items-center justify-between gap-2 text-sm">
              <a href={d.fileUrl} target="_blank" rel="noopener noreferrer" className="font-bold text-indigo-600 underline">
                {d.name}
              </a>
              {canManage && (
                <button
                  type="button"
                  className="text-xs text-rose-600 font-bold underline"
                  onClick={async () => {
                    if (!confirm("Supprimer ce document partagé ?")) return;
                    await fetch("/api/personnel", {
                      method: "PATCH",
                      headers: { "Content-Type": "application/json" },
                      body: JSON.stringify({ action: "delete-shared-doc", docId: d.id }),
                    });
                    void onRefresh();
                  }}
                >
                  Supprimer
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      {canManage && (
        <label className="inline-block cursor-pointer">
          <input
            type="file"
            className="hidden"
            onChange={async (e) => {
              const file = e.target.files?.[0];
              if (!file) return;
              try {
                const prep = await fetch("/api/personnel/upload", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({ fileName: file.name, fileType: file.type }),
                });
                const pj = await prep.json();
                if (!prep.ok) throw new Error(pj.error);
                await fetch(pj.uploadUrl, { method: "PUT", body: file, headers: { "Content-Type": file.type } });
                await fetch("/api/personnel", {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({
                    action: "shared-doc",
                    document: { name: file.name, fileUrl: pj.fileUrl },
                  }),
                });
                void onRefresh();
              } catch (err) {
                alert(err instanceof Error ? err.message : "Erreur");
              }
              e.target.value = "";
            }}
          />
          <span className="px-4 py-2 rounded-xl bg-indigo-600 text-white text-xs font-bold">+ Ajouter un document utile</span>
        </label>
      )}
    </div>
  );
}
