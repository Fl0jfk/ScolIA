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
            className={`px-3 py-1.5 rounded-lg text-xs font-bold ${
              weekView === id
                ? "bg-slate-900 text-white"
                : "bg-white border border-slate-200 text-slate-600"
            }`}
          >
            {label}
          </button>
        ))}

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
          return (
            <div className="rounded-lg bg-white border border-indigo-100 px-2 py-1.5 text-[11px] leading-snug group relative">
              <p className="font-bold text-indigo-900">
                {full.start}–{full.end}
              </p>
              <p className="text-slate-800 font-semibold">{full.subject || "—"}</p>
              <p className="text-slate-500">
                {(full.classes || []).join(", ") || "Classe ?"}
                {full.room ? ` · ${full.room}` : ""}
              </p>
              {editMode ? (
                <button
                  type="button"
                  onClick={() => duplicateSlot(full)}
                  className="mt-1 text-[10px] font-bold text-indigo-600 hover:underline"
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
