"use client";

import { useCallback, useMemo, useState } from "react";
import {
  TeacherSlotEditor,
  WeekGrid,
  newId,
} from "@/app/components/personnel/RhPlanningEditors";
import {
  estimateTeacherWeeklyHours,
  findOverlappingTeacherSlots,
  type TeacherPlanningCatalog,
  type TeacherPlanningDoc,
  type TeacherPlanningSlot,
  type TeacherWeeklyHoursSummary,
} from "@/app/lib/rh/planning-types";
import { downloadTeacherPlanningPdf } from "@/app/lib/rh/planning-export-pdf";
import {
  planningSlotCardClass,
  planningSlotMetaTextClass,
  planningSlotTimeClass,
  planningSlotTitleTextClass,
  planningWeekTabClass,
} from "@/app/lib/rh/planning-slot-colors";
import { slotAudienceLabel } from "@/app/lib/rh/teaching-groups";

type Props = {
  initialPlanning: TeacherPlanningDoc;
  personnelId: string;
  catalog: TeacherPlanningCatalog | null;
  teacherWeeklyHours: TeacherWeeklyHoursSummary | null;
  onSaved: (planning: TeacherPlanningDoc) => void;
};

export default function TeacherPlanningSelfEditor({
  initialPlanning,
  personnelId,
  catalog,
  teacherWeeklyHours,
  onSaved,
}: Props) {
  const [teacher, setTeacher] = useState(initialPlanning);
  const [weekView, setWeekView] = useState<"A" | "B">("A");
  const [editMode, setEditMode] = useState(false);
  const [saving, setSaving] = useState(false);
  const [exportingPdf, setExportingPdf] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const teacherSlots = weekView === "A" ? teacher.weekA : teacher.weekB;

  const hours = useMemo(() => estimateTeacherWeeklyHours(teacher), [teacher]);
  const overlapWarnings = useMemo(
    () => findOverlappingTeacherSlots(teacherSlots),
    [teacherSlots],
  );

  const resetFromServer = useCallback((planning: TeacherPlanningDoc) => {
    setTeacher(planning);
    setEditMode(false);
    setError(null);
  }, []);

  const save = async () => {
    setSaving(true);
    setError(null);
    setMsg(null);
    try {
      const res = await fetch("/api/rh/planning", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ personnelId, planning: teacher }),
      });
      const j = (await res.json()) as {
        error?: string;
        planning?: TeacherPlanningDoc;
        conflicts?: Array<{ message: string }>;
      };
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      if (j.planning?.kind === "teacher") {
        setTeacher(j.planning);
        onSaved(j.planning);
      }
      const n = j.conflicts?.length ?? 0;
      setMsg(
        n > 0
          ? `Planning enregistré — ${n} conflit${n !== 1 ? "s" : ""} inter-profs (direction).`
          : "Planning enregistré.",
      );
      setEditMode(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const copyWeekAToB = () => {
    setTeacher((prev) => ({
      ...prev,
      weekB: prev.weekA.map((slot) => ({ ...slot, id: newId("slot") })),
    }));
    setEditMode(true);
    setWeekView("B");
    setMsg("Semaine A copiée vers B — vérifiez puis enregistrez.");
  };

  const duplicateSlot = (slot: TeacherPlanningSlot) => {
    const copy: TeacherPlanningSlot = { ...slot, id: newId("slot") };
    setTeacher((prev) =>
      weekView === "A"
        ? { ...prev, weekA: [...prev.weekA, copy] }
        : { ...prev, weekB: [...prev.weekB, copy] },
    );
    setEditMode(true);
  };

  const exportPdf = async () => {
    setExportingPdf(true);
    setError(null);
    try {
      await downloadTeacherPlanningPdf({
        displayName: "Mon planning",
        week: weekView,
        slots: teacherSlots,
      });
      setMsg("PDF téléchargé.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Export PDF impossible");
    } finally {
      setExportingPdf(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-indigo-100 bg-indigo-50/60 px-4 py-3 text-sm text-indigo-950">
        <p className="font-bold">Volume hebdomadaire (semaines types)</p>
        <p className="mt-1 text-indigo-900/90">
          Semaine A : <span className="font-semibold">{hours.weekA} h</span>
          {" · "}
          Semaine B : <span className="font-semibold">{hours.weekB} h</span>
          {" · "}
          Moyenne : <span className="font-semibold">{hours.averageWeekly} h</span>
        </p>
        {teacherWeeklyHours &&
        (teacherWeeklyHours.weekA !== hours.weekA || teacherWeeklyHours.weekB !== hours.weekB) ? (
          <p className="mt-1 text-xs text-indigo-700/80">Modifications non enregistrées.</p>
        ) : null}
      </div>

      <div className="flex flex-wrap items-center gap-2">
        {(
          [
            ["A", "Semaine type A"],
            ["B", "Semaine type B"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            onClick={() => setWeekView(id)}
            className={`px-3 py-1.5 rounded-lg text-xs font-bold ${planningWeekTabClass(weekView === id, id)}`}
          >
            {label}
          </button>
        ))}

        {teacherSlots.length > 0 ? (
          <button
            type="button"
            disabled={exportingPdf}
            onClick={() => void exportPdf()}
            className="px-3 py-1.5 rounded-lg text-xs font-bold border border-slate-200 bg-white text-slate-700 hover:bg-slate-50 disabled:opacity-60"
          >
            {exportingPdf ? "Export…" : "Exporter PDF"}
          </button>
        ) : null}

        <div className="ml-auto flex flex-wrap gap-2">
          {editMode ? (
            <>
              <button
                type="button"
                onClick={copyWeekAToB}
                className="px-3 py-1.5 rounded-xl text-xs font-bold border border-indigo-200 text-indigo-800 bg-white hover:bg-indigo-50"
              >
                Copier A → B
              </button>
              <button
                type="button"
                onClick={() => resetFromServer(initialPlanning)}
                className="px-3 py-1.5 rounded-xl text-xs font-bold border border-slate-200 text-slate-600"
              >
                Annuler
              </button>
              <button
                type="button"
                disabled={saving || overlapWarnings.length > 0}
                onClick={() => void save()}
                className="px-3 py-1.5 rounded-xl text-xs font-bold bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-60"
              >
                {saving ? "Enregistrement…" : "Enregistrer"}
              </button>
            </>
          ) : (
            <button
              type="button"
              onClick={() => setEditMode(true)}
              className="px-3 py-1.5 rounded-xl text-xs font-bold bg-indigo-600 text-white hover:bg-indigo-700"
            >
              Éditer mon emploi du temps
            </button>
          )}
        </div>
      </div>

      {overlapWarnings.length > 0 ? (
        <ul className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900 list-disc pl-5 space-y-0.5">
          {overlapWarnings.map((w) => (
            <li key={w}>{w}</li>
          ))}
        </ul>
      ) : null}

      {error ? <p className="text-sm text-rose-600">{error}</p> : null}
      {msg ? <p className="text-sm text-emerald-700">{msg}</p> : null}

      <WeekGrid
        slots={teacherSlots}
        renderCard={(slot) => {
          const full = teacherSlots.find((s) => s.id === slot.id)!;
          const colorKey = full.subject || "cours";
          return (
            <div className={`${planningSlotCardClass(colorKey)} group relative`}>
              <p className={planningSlotTimeClass(colorKey)}>
                {full.start}–{full.end}
              </p>
              <p className={planningSlotTitleTextClass(colorKey)}>{full.subject || "—"}</p>
              <p className={planningSlotMetaTextClass(colorKey)}>
                {slotAudienceLabel(full)}
                {full.room ? ` · ${full.room}` : ""}
              </p>
              {editMode ? (
                <button
                  type="button"
                  onClick={() => duplicateSlot(full)}
                  className="mt-0.5 text-[10px] font-bold text-indigo-600 hover:underline"
                >
                  Dupliquer
                </button>
              ) : null}
            </div>
          );
        }}
      />

      {editMode ? (
        <TeacherSlotEditor
          slots={teacherSlots}
          catalog={catalog}
          onChange={(slots) => {
            setTeacher((prev) =>
              weekView === "A" ? { ...prev, weekA: slots } : { ...prev, weekB: slots },
            );
          }}
        />
      ) : null}
    </div>
  );
}
