"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import dynamic from "next/dynamic";
import { useUser } from "@clerk/nextjs";
import PlanningWeekCalendar from "@/app/components/personnel/PlanningWeekCalendar";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import { dash } from "@/app/lib/dashboard-brand";
import { hasGlobalAdminRole, intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import {
  canManagePersonnel,
  PERSONNEL_LEAVE_TYPE_LABELS,
  type PersonnelLeaveRequest,
} from "@/app/lib/personnel-types";
import {
  findCurrentActivity,
  schoolWeekParity,
  type LeaveSpan,
} from "@/app/lib/rh/planning-calendar";
import type { SchoolHolidayZone } from "@/app/lib/fr-school-holidays";
import type { RhPlanningDoc, RhPlanningKind } from "@/app/lib/rh/planning-types";

const RhPlanningPanel = dynamic(() => import("@/app/components/personnel/RhPlanningPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});

type Tab = "mine" | "gestion";

export default function MonPlanningClient() {
  const { isLoaded, user } = useUser();
  const [tab, setTab] = useState<Tab>("mine");
  const [planning, setPlanning] = useState<RhPlanningDoc | null>(null);
  const [kind, setKind] = useState<RhPlanningKind>("teacher");
  const [displayName, setDisplayName] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [canManage, setCanManage] = useState(false);
  const [leaves, setLeaves] = useState<LeaveSpan[]>([]);
  const [schoolHolidayZone, setSchoolHolidayZone] = useState<SchoolHolidayZone | null>(null);

  const roles = useMemo(() => {
    if (!user) return [];
    return intranetRolesFromMetadata(user.publicMetadata);
  }, [user]);

  const showGestion = canManage || hasGlobalAdminRole(roles) || canManagePersonnel(roles);

  const loadMine = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [planRes, leaveRes] = await Promise.all([
        fetch("/api/rh/planning", { cache: "no-store" }),
        fetch("/api/personnel/leaves", { cache: "no-store" }),
      ]);
      const j = await planRes.json();
      if (!planRes.ok) throw new Error(j.error || "Chargement impossible");
      setPlanning(j.planning);
      setKind(j.kind);
      setDisplayName(j.displayName || "");
      setCanManage(!!j.canManage);
      const z = j.schoolHolidayZone;
      setSchoolHolidayZone(z === "A" || z === "B" || z === "C" ? z : null);
      const pid = j.personnelId as string | undefined;

      if (leaveRes.ok) {
        const lj = await leaveRes.json();
        const requests = (lj.requests || []) as PersonnelLeaveRequest[];
        const validated = requests.filter((r) => r.status === "validee");
        // Staff : dossier RH. Non-managers : l’API leaves renvoie déjà « les miens ».
        const scoped =
          j.kind === "staff" && pid
            ? validated.filter((r) => r.personnelId === pid)
            : j.canManage
              ? []
              : validated;
        setLeaves(
          scoped.map((r) => ({
            startDate: r.startDate,
            endDate: r.endDate,
            type: r.type,
            label: PERSONNEL_LEAVE_TYPE_LABELS[r.type] || r.type,
          })),
        );
      } else {
        setLeaves([]);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
      setPlanning(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!isLoaded) return;
    void loadMine();
  }, [isLoaded, loadMine]);

  const nowActivity = useMemo(() => {
    if (!planning) return null;
    return findCurrentActivity(
      planning,
      new Date(),
      schoolWeekParity(new Date()),
      leaves,
      schoolHolidayZone,
    );
  }, [planning, leaves, schoolHolidayZone]);

  if (!isLoaded || loading) {
    return (
      <ModulePageShell maxWidthClass="max-w-[1400px]">
        <p className={`text-center text-sm ${dash.textMid}`}>Chargement du planning…</p>
      </ModulePageShell>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-[1400px]">
      <ModulePageHeader
        eyebrow="RH"
        title="Mon planning"
        description={
          <>
            Vue semaine{displayName ? ` — ${displayName}` : ""}.
            {kind === "teacher" ? " Semaines types A/B pour l’année." : ""}
          </>
        }
      />
      {showGestion ? (
        <ModuleTabNav
          className="mb-5"
          tabs={[
            { id: "mine", label: "Mon planning" },
            { id: "gestion", label: "Gestion des plannings" },
          ]}
          active={tab}
          onChange={setTab}
        />
      ) : null}

      {tab === "gestion" && showGestion ? (
        <RhPlanningPanel />
      ) : (
        <div className="space-y-4">
          {nowActivity ? (
            <ModuleCard bodyClassName={`px-4 py-3 ${dash.bgSoft}`}>
              <p className={dash.fieldLabel}>En ce moment</p>
              <p className={`mt-0.5 text-lg font-semibold ${dash.ink}`}>{nowActivity.title}</p>
              <p className={`text-sm ${dash.textMid}`}>
                {nowActivity.detail}
                {nowActivity.start !== "—" ? ` · ${nowActivity.start}–${nowActivity.end}` : ""}
              </p>
            </ModuleCard>
          ) : null}

          {error ? (
            <div className="rounded-2xl border border-rose-200 bg-rose-50 p-4 text-sm text-rose-700">
              {error}
              <p className="mt-2 text-xs text-rose-600/80">
                Si vous n’avez pas encore de planning, un admin RH peut l’importer depuis l’onglet
                Gestion.
              </p>
            </div>
          ) : planning ? (
            <ModuleCard bodyClassName="p-4">
              <PlanningWeekCalendar
                planning={planning}
                leaves={leaves}
                schoolHolidayZone={schoolHolidayZone}
              />
            </ModuleCard>
          ) : (
            <p className="text-sm text-slate-500 italic">Aucun planning enregistré.</p>
          )}
        </div>
      )}
    </ModulePageShell>
  );
}
