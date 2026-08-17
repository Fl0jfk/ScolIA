"use client";

import dynamic from "next/dynamic";
import { useUser } from "@clerk/nextjs";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";
import InternatDashboardPanel from "@/app/components/internat/InternatDashboardPanel";
import InternatHubNav, { type InternatTab } from "@/app/components/internat/InternatHubNav";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import {
  canAccessInternatFromMetadata,
  canManageInternatConfig,
  rolesFromMetadata,
} from "@/app/lib/internat-rbac";
import type { InternatDashboardStats } from "@/app/lib/internat-stats";
import type { InternatBuilding, InternatIncident, InternatRoom, InternatStudent } from "@/app/lib/internat-types";

const InternatRoomsPanel = dynamic(() => import("@/app/components/internat/InternatRoomsPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const InternatStudentsPanel = dynamic(() => import("@/app/components/internat/InternatStudentsPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const InternatOutingsPanel = dynamic(() => import("@/app/components/internat/InternatOutingsPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const InternatRollCallPanel = dynamic(() => import("@/app/components/internat/InternatRollCallPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const InternatRollCallHistoryPanel = dynamic(
  () => import("@/app/components/internat/InternatRollCallHistoryPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const InternatStudyPanel = dynamic(() => import("@/app/components/internat/InternatStudyPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const InternatSupervisorsPanel = dynamic(() => import("@/app/components/internat/InternatSupervisorsPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const InternatEducationalPanel = dynamic(() => import("@/app/components/internat/InternatEducationalPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const InternatCommunicationPanel = dynamic(
  () => import("@/app/components/internat/InternatCommunicationPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);
const InternatActivitiesPanel = dynamic(() => import("@/app/components/internat/InternatActivitiesPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const InternatAlertsPanel = dynamic(() => import("@/app/components/internat/InternatAlertsPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const InternatInstallationPanel = dynamic(
  () => import("@/app/components/internat/InternatInstallationPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

const TAB_IDS: InternatTab[] = [
  "dashboard",
  "chambres",
  "internes",
  "sorties",
  "appel",
  "historique",
  "etudes",
  "surveillants",
  "suivi",
  "communication",
  "activites",
  "alertes",
  "installation",
];

function parseTab(raw: string | null): InternatTab {
  if (raw && TAB_IDS.includes(raw as InternatTab)) return raw as InternatTab;
  return "dashboard";
}

export default function GestionInternatClient() {
  const { user, isLoaded } = useUser();
  const router = useRouter();
  const searchParams = useSearchParams();
  const activeTab = parseTab(searchParams.get("tab"));

  const isOrgAdmin = useIsOrgAdmin();
  const roles = useMemo(() => rolesFromMetadata(user?.publicMetadata), [user]);
  const allowed =
    isLoaded && (canAccessInternatFromMetadata(user?.publicMetadata) || isOrgAdmin);
  const canManage = isOrgAdmin || canManageInternatConfig(roles);

  const [rooms, setRooms] = useState<InternatRoom[]>([]);
  const [buildings, setBuildings] = useState<InternatBuilding[]>([]);
  const [students, setStudents] = useState<InternatStudent[]>([]);
  const [incidents, setIncidents] = useState<InternatIncident[]>([]);
  const [stats, setStats] = useState<InternatDashboardStats | null>(null);
  const [loading, setLoading] = useState(true);

  const setTab = (tab: InternatTab) => {
    router.push(`/gestion-internat?tab=${tab}`);
  };

  const refresh = useCallback(async () => {
    const [roomsRes, studentsRes, statsRes, incidentsRes] = await Promise.all([
      fetch("/api/internat/rooms", { cache: "no-store" }),
      fetch("/api/internat/students", { cache: "no-store" }),
      fetch("/api/internat/stats", { cache: "no-store" }),
      fetch("/api/internat/incidents", { cache: "no-store" }),
    ]);
    const roomsData = await roomsRes.json();
    const studentsData = await studentsRes.json();
    const statsData = await statsRes.json();
    const incidentsData = await incidentsRes.json();
    if (roomsRes.ok) {
      setRooms(roomsData.rooms || []);
      setBuildings(roomsData.buildings || []);
    }
    if (studentsRes.ok) setStudents(studentsData.students || []);
    if (statsRes.ok) setStats(statsData.stats || null);
    if (incidentsRes.ok) setIncidents(incidentsData.incidents || []);
  }, []);

  useEffect(() => {
    if (!isLoaded) return;
    if (!allowed) {
      router.replace("/dashboard");
      return;
    }
    setLoading(true);
    void refresh().finally(() => setLoading(false));
  }, [isLoaded, allowed, router, refresh]);

  if (!isLoaded || !allowed) {
    return (
      <ModulePageShell maxWidthClass="max-w-[1400px]">
        <p className="text-slate-500 text-sm">Chargement…</p>
      </ModulePageShell>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-[1400px]" tourModuleId="internat">
      <ModulePageHeader
        eyebrow="Élèves"
        title="Gestion internat"
        description="Chambres, internes, appel du soir, activités et alertes — équipe éducation / direction."
      />

      <InternatHubNav active={activeTab} onChange={setTab} />

      {loading && activeTab === "dashboard" ? (
        <p className="text-slate-500 text-sm">Chargement…</p>
      ) : (
        <>
          {activeTab === "dashboard" && <InternatDashboardPanel stats={stats} />}
          {activeTab === "chambres" && (
            <InternatRoomsPanel
              rooms={rooms}
              buildings={buildings}
              students={students}
              incidents={incidents}
              canManage={canManage}
              onRefresh={refresh}
            />
          )}
          {activeTab === "internes" && (
            <div data-tour="internat-roster">
              <InternatStudentsPanel
                students={students}
                rooms={rooms}
                buildings={buildings}
                canManage={canManage}
                onRefresh={refresh}
              />
            </div>
          )}
          {activeTab === "sorties" && (
            <div data-tour="internat-outings">
              <InternatOutingsPanel students={students} canManage={canManage} />
            </div>
          )}
          {activeTab === "appel" && <InternatRollCallPanel onRefresh={refresh} />}
          {activeTab === "historique" && <InternatRollCallHistoryPanel />}
          {activeTab === "etudes" && (
            <InternatStudyPanel students={students} canManage={canManage} />
          )}
          {activeTab === "surveillants" && <InternatSupervisorsPanel canManage={canManage} />}
          {activeTab === "suivi" && (
            <InternatEducationalPanel students={students} canManage={canManage} />
          )}
          {activeTab === "communication" && <InternatCommunicationPanel canManage={canManage} />}
          {activeTab === "activites" && <InternatActivitiesPanel />}
          {activeTab === "alertes" && <InternatAlertsPanel />}
          {activeTab === "installation" && (
            <InternatInstallationPanel canManage={canManage} />
          )}
        </>
      )}
    </ModulePageShell>
  );
}
