"use client";

import { Suspense, useCallback, useEffect, useMemo, useState } from "react";
import dynamic from "next/dynamic";
import { useRouter, useSearchParams } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import PersonnelDashboard from "@/app/components/personnel/PersonnelDashboard";
import PersonnelStaffCard from "@/app/components/personnel/PersonnelStaffCard";
import RhHubNav, { RhPilotageNav, type RhHubTab, type RhPilotageSection } from "@/app/components/personnel/RhHubNav";
import RhNewStaffModal from "@/app/components/personnel/RhNewStaffModal";
import RhMoodPulseAdminPanel from "@/app/components/personnel/RhMoodPulseAdminPanel";
import RhPersonnelHome from "@/app/components/personnel/RhPersonnelHome";
import RhValidationsPanel from "@/app/components/personnel/RhValidationsPanel";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import type { PersonnelDashboardData } from "@/app/lib/personnel-dashboard";
import { type PersonnelIndexEntry, type SharedPersonnelDocument } from "@/app/lib/personnel-types";
import {
  canAccessRhPilotageDashboard,
} from "@/app/lib/rh/rh-hub-access";

const RhAdminOverviewPanel = dynamic(() => import("@/app/components/personnel/RhAdminOverviewPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const RhOnboardingPanel = dynamic(() => import("@/app/components/personnel/RhOnboardingPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const RhRegistrePanel = dynamic(() => import("@/app/components/personnel/RhRegistrePanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});

const LEGACY_TAB_MAP: Record<string, RhHubTab> = {
  dashboard: "dashboard",
  temps: "dashboard",
  absences: "dashboard",
  hse: "dashboard",
  demande: "dashboard",
  planning: "dashboard",
  annuaire: "pilotage",
  admin: "pilotage",
  onboarding: "pilotage",
  registre: "pilotage",
  pilotage: "pilotage",
};

const LEGACY_PILOTAGE_MAP: Record<string, RhPilotageSection> = {
  annuaire: "annuaire",
  admin: "admin",
  onboarding: "onboarding",
  registre: "registre",
};

function parseHubTab(raw: string | null): RhHubTab {
  const mapped = raw ? LEGACY_TAB_MAP[raw] : undefined;
  return mapped ?? "dashboard";
}

function parsePilotageSection(raw: string | null, legacyTab: string | null): RhPilotageSection {
  if (raw && ["overview", "validations", "annuaire", "admin", "onboarding", "registre"].includes(raw)) {
    return raw as RhPilotageSection;
  }
  if (legacyTab && LEGACY_PILOTAGE_MAP[legacyTab]) {
    return LEGACY_PILOTAGE_MAP[legacyTab];
  }
  return "overview";
}

function parseDashboardSection(raw: string | null, legacyTab: string | null): string | null {
  if (raw) return raw;
  if (legacyTab && ["absences", "hse", "demande", "planning"].includes(legacyTab)) return legacyTab;
  return null;
}

export default function RhModuleClient() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const legacyTab = searchParams.get("tab");
  const activeTab = parseHubTab(legacyTab);
  const pilotageSection = parsePilotageSection(searchParams.get("section"), legacyTab);
  const dashboardSection = parseDashboardSection(searchParams.get("section"), legacyTab);
  const { isLoaded, user } = useSessionUser();

  const [dashboard, setDashboard] = useState<PersonnelDashboardData | null>(null);
  const [index, setIndex] = useState<PersonnelIndexEntry[]>([]);
  const [sharedDocs, setSharedDocs] = useState<SharedPersonnelDocument[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showNew, setShowNew] = useState(false);

  const roles = useMemo(() => rolesFromUserLike(user), [user]);
  const canPilotage = canAccessRhPilotageDashboard(roles);

  const setTab = (tab: RhHubTab) => {
    if (tab === "pilotage") {
      router.push("/rh?tab=pilotage&section=overview");
      return;
    }
    router.push("/rh?tab=dashboard");
  };

  const setPilotageSection = (section: RhPilotageSection) => {
    router.push(`/rh?tab=pilotage&section=${section}`);
  };

  const load = useCallback(async () => {
    setError(null);
    try {
      const [dRes, lRes] = await Promise.all([
        fetch("/api/personnel/dashboard", { cache: "no-store" }),
        fetch("/api/personnel", { cache: "no-store" }),
      ]);
      const dJson = await dRes.json().catch(() => ({}));
      const lJson = await lRes.json().catch(() => ({}));
      // 403 = pas le droit dashboard (ex. prof sur l’espace perso) — on continue avec la liste.
      if (dRes.ok) {
        setDashboard(dJson);
      } else if (dRes.status !== 403) {
        console.error("[rh] dashboard", dRes.status, dJson);
        setDashboard(null);
      } else {
        setDashboard(null);
      }
      if (lRes.ok) {
        setIndex(lJson.index || []);
        setSharedDocs(lJson.sharedDocs || []);
      } else if (lRes.status !== 403) {
        throw new Error(lJson.error || "Liste personnel indisponible");
      } else {
        setIndex([]);
        setSharedDocs([]);
      }
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
    if (activeTab === "pilotage" && !canPilotage) {
      router.replace("/rh?tab=dashboard");
    }
  }, [activeTab, canPilotage, isLoaded, router]);

  if (!isLoaded || (loading && !dashboard && index.length === 0)) {
    return <p className="p-10 text-center text-slate-500">Chargement du module RH…</p>;
  }
  if (error) return <p className="p-10 text-center text-rose-600">{error}</p>;

  return (
    <ModulePageShell maxWidthClass="max-w-[1500px]" tourModuleId="rh">
      <ModulePageHeader
        eyebrow="RH"
        title="Ressources humaines"
        description="Votre espace personnel — demandes, documents et suivi. Le pilotage RH est réservé aux gestionnaires."
      />

      <RhHubNav active={activeTab} onChange={setTab} canPilotage={canPilotage} />

      {activeTab === "pilotage" && canPilotage ? (
        <div className="space-y-4">
          <RhPilotageNav active={pilotageSection} onChange={setPilotageSection} />

          {pilotageSection === "validations" ? (
            <RhValidationsPanel />
          ) : pilotageSection === "admin" ? (
            <RhAdminOverviewPanel index={index} />
          ) : pilotageSection === "onboarding" ? (
            <RhOnboardingPanel />
          ) : pilotageSection === "registre" ? (
            <RhRegistrePanel />
          ) : pilotageSection === "annuaire" ? (
            <div className="space-y-6">
              <div className="bg-white rounded-2xl border border-slate-100 p-5 shadow-sm">
                <h2 className="font-black text-slate-800 mb-1">Annuaire RH</h2>
                <p className="text-xs text-indigo-600 font-medium mb-4">
                  Glissez un fichier sur une fiche pour le déposer via l&apos;IA.
                </p>
                {index.length === 0 ? (
                  <p className="text-sm text-slate-400 italic">Aucun dossier.</p>
                ) : (
                  <div data-tour="rh-list" className="grid sm:grid-cols-2 lg:grid-cols-3 gap-3">
                    {index.map((p) => (
                      <PersonnelStaffCard key={p.id} person={p} canDrop onUploaded={() => void load()} />
                    ))}
                  </div>
                )}
              </div>
              <SharedDocsBlock sharedDocs={sharedDocs} canManage onRefresh={load} />
            </div>
          ) : (
            <div className="space-y-4">
              <RhMoodPulseAdminPanel />
              {dashboard ? (
                <PersonnelDashboard data={dashboard} onNewStaff={() => setShowNew(true)} />
              ) : null}
              <SharedDocsBlock sharedDocs={sharedDocs} canManage onRefresh={load} />
            </div>
          )}
        </div>
      ) : (
        <RhPersonnelHome dashboardSection={dashboardSection} />
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
    </ModulePageShell>
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
