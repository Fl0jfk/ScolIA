"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import dynamic from "next/dynamic";
import Link from "next/link";
import { useSessionUser } from "@/app/hooks/useAppUser";
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
import {
  estimateTeacherWeeklyHours,
  type RhPlanningDoc,
  type RhPlanningKind,
  type TeacherPlanningCatalog,
  type TeacherPlanningDoc,
  type TeacherWeeklyHoursSummary,
} from "@/app/lib/rh/planning-types";

const RhPlanningPanel = dynamic(() => import("@/app/components/personnel/RhPlanningPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});

const TeacherPlanningSelfEditor = dynamic(
  () => import("@/app/components/personnel/TeacherPlanningSelfEditor"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

type Tab = "mine" | "edit" | "classes" | "gestion";

const ClassPlanningPanel = dynamic(
  () => import("@/app/components/personnel/ClassPlanningPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

export default function MonPlanningClient() {
  const { isLoaded, user } = useSessionUser();
  const [tab, setTab] = useState<Tab>("mine");
  const [planning, setPlanning] = useState<RhPlanningDoc | null>(null);
  const [kind, setKind] = useState<RhPlanningKind>("teacher");
  const [displayName, setDisplayName] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [canManage, setCanManage] = useState(false);
  const [canEdit, setCanEdit] = useState(false);
  const [personnelId, setPersonnelId] = useState("");
  const [catalog, setCatalog] = useState<TeacherPlanningCatalog | null>(null);
  const [teacherWeeklyHours, setTeacherWeeklyHours] = useState<TeacherWeeklyHoursSummary | null>(
    null,
  );
  const [leaves, setLeaves] = useState<LeaveSpan[]>([]);
  const [schoolHolidayZone, setSchoolHolidayZone] = useState<SchoolHolidayZone | null>(null);

  const roles = useMemo(() => {
    if (!user) return [];
    return intranetRolesFromMetadata(user.publicMetadata);
  }, [user]);

  const showGestion = canManage || hasGlobalAdminRole(roles) || canManagePersonnel(roles);
  const showTeacherEdit =
    kind === "teacher" && canEdit && planning?.kind === "teacher" && personnelId;
  const showClassEdt =
    kind === "teacher" || showGestion || hasGlobalAdminRole(roles);

  const tabs: { id: Tab; label: string }[] = [{ id: "mine", label: "Vue semaine" }];
  if (showClassEdt) {
    tabs.push({ id: "classes", label: "EDT de mes classes" });
  }
  if (showTeacherEdit) {
    tabs.push({ id: "edit", label: "Éditer mon EDT" });
  }
  if (showGestion) {
    tabs.push({ id: "gestion", label: "Gestion des plannings" });
  }

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
      setCanEdit(!!j.canEdit);
      setPersonnelId(typeof j.personnelId === "string" ? j.personnelId : "");
      setCatalog((j.catalog as TeacherPlanningCatalog | null) ?? null);
      setTeacherWeeklyHours((j.teacherWeeklyHours as TeacherWeeklyHoursSummary | null) ?? null);
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
        actions={
          showGestion ? (
            <Link
              href="/edt-etablissement"
              className="text-sm font-bold text-indigo-600 hover:underline"
            >
              EDT établissement
            </Link>
          ) : undefined
        }
      />
      {tabs.length > 1 ? (
        <ModuleTabNav
          className="mb-5"
          tabs={tabs}
          active={tab}
          onChange={setTab}
        />
      ) : null}

      {tab === "gestion" && showGestion ? (
        <RhPlanningPanel />
      ) : tab === "classes" && showClassEdt ? (
        <ModuleCard bodyClassName="p-4">
          <ClassPlanningPanel compact />
        </ModuleCard>
      ) : tab === "edit" && showTeacherEdit ? (
        <ModuleCard bodyClassName="p-4">
          <TeacherPlanningSelfEditor
            key={planning.updatedAt}
            initialPlanning={planning as TeacherPlanningDoc}
            personnelId={personnelId}
            catalog={catalog}
            teacherWeeklyHours={teacherWeeklyHours}
            onSaved={(next) => {
              setPlanning(next);
              setTeacherWeeklyHours(estimateTeacherWeeklyHours(next));
            }}
          />
        </ModuleCard>
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
