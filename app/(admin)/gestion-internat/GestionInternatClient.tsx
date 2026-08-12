"use client";

import { useUser } from "@clerk/nextjs";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";
import InternatActivitiesPanel from "@/app/components/internat/InternatActivitiesPanel";
import InternatAlertsPanel from "@/app/components/internat/InternatAlertsPanel";
import InternatDashboardPanel from "@/app/components/internat/InternatDashboardPanel";
import InternatHubNav, { type InternatTab } from "@/app/components/internat/InternatHubNav";
import InternatOutingsPanel from "@/app/components/internat/InternatOutingsPanel";
import InternatRollCallPanel from "@/app/components/internat/InternatRollCallPanel";
import InternatRoomsPanel from "@/app/components/internat/InternatRoomsPanel";
import InternatRollCallHistoryPanel from "@/app/components/internat/InternatRollCallHistoryPanel";
import InternatStudyPanel from "@/app/components/internat/InternatStudyPanel";
import InternatSupervisorsPanel from "@/app/components/internat/InternatSupervisorsPanel";
import InternatEducationalPanel from "@/app/components/internat/InternatEducationalPanel";
import InternatCommunicationPanel from "@/app/components/internat/InternatCommunicationPanel";
import InternatInstallationPanel from "@/app/components/internat/InternatInstallationPanel";
import InternatStudentsPanel from "@/app/components/internat/InternatStudentsPanel";
import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import {
  canAccessInternatFromMetadata,
  canManageInternatConfig,
  rolesFromMetadata,
} from "@/app/lib/internat-rbac";
import type { InternatDashboardStats } from "@/app/lib/internat-stats";
import type { InternatBuilding, InternatIncident, InternatRoom, InternatStudent } from "@/app/lib/internat-types";

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
      <div className="max-w-6xl mx-auto px-4 py-10">
        <p className="text-slate-500 text-sm">Chargement…</p>
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto px-4 py-8 sm:py-10">
      <header className="mb-6">
        <h1 className="text-3xl font-black text-slate-900 tracking-tight">Gestion internat</h1>
        <p className="text-slate-500 mt-1 text-sm">
          Chambres, internes, appel du soir, activités et alertes — équipe éducation / direction.
        </p>
      </header>

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
      <ReplayModuleTourButton moduleId="internat" />
    </div>
  );
}
