"use client";

import { useEffect, useMemo, useState, type MouseEvent, type ReactNode } from "react";
import {
  PLANNING_WEEKDAY_LABELS,
  PLANNING_WEEKDAYS,
  SURVEILLANT_LOCATION_SUGGESTIONS,
  type PlanningWeekday,
  type StaffFixedSlot,
  type StaffMissionSlot,
  type TeacherPlanningCatalog,
  type TeacherPlanningDoc,
  type TeacherPlanningSlot,
} from "@/app/lib/rh/planning-types";
import { planningDayHeaderClass } from "@/app/lib/rh/planning-slot-colors";

export function newId(prefix: string) {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;
}

function emptyTeacherSlot(day: PlanningWeekday = 1): TeacherPlanningSlot {
  return {
    id: newId("slot"),
    day,
    start: "08:00",
    end: "09:00",
    subject: "",
    classes: [],
    room: "",
  };
}

export type TeacherSlotWeekMode = "A" | "B" | "both";

export function teacherSlotFingerprint(slot: TeacherPlanningSlot): string {
  return [
    slot.day,
    slot.start,
    slot.end,
    slot.subject.trim().toLowerCase(),
    [...(slot.classes || [])].map((c) => c.trim()).filter(Boolean).sort().join(","),
    (slot.room || "").trim().toLowerCase(),
  ].join("|");
}

export function detectTeacherSlotWeekMode(
  doc: TeacherPlanningDoc,
  slot: TeacherPlanningSlot,
  viewingWeek: "A" | "B",
): TeacherSlotWeekMode {
  const fp = teacherSlotFingerprint(slot);
  const inA = doc.weekA.some((s) => s.id === slot.id || teacherSlotFingerprint(s) === fp);
  const inB = doc.weekB.some((s) => s.id === slot.id || teacherSlotFingerprint(s) === fp);
  if (inA && inB) return "both";
  if (inA) return "A";
  if (inB) return "B";
  return viewingWeek;
}

/** Applique une édition de créneau (y compris déplacement semaine A / B / les deux). */
export function applyTeacherSlotQuickEdit(
  doc: TeacherPlanningDoc,
  draft: TeacherPlanningSlot,
  weekMode: TeacherSlotWeekMode,
  previousId: string | null,
): TeacherPlanningDoc {
  const removeIds = new Set<string>();
  if (previousId) {
    removeIds.add(previousId);
    const prev =
      doc.weekA.find((s) => s.id === previousId) ||
      doc.weekB.find((s) => s.id === previousId) ||
      null;
    if (prev) {
      const fp = teacherSlotFingerprint(prev);
      for (const s of doc.weekA) {
        if (teacherSlotFingerprint(s) === fp) removeIds.add(s.id);
      }
      for (const s of doc.weekB) {
        if (teacherSlotFingerprint(s) === fp) removeIds.add(s.id);
      }
    }
  }

  let weekA = doc.weekA.filter((s) => !removeIds.has(s.id));
  let weekB = doc.weekB.filter((s) => !removeIds.has(s.id));

  const base: TeacherPlanningSlot = {
    ...draft,
    id: newId("slot"),
    subject: draft.subject.trim(),
    classes: [...new Set((draft.classes || []).map((c) => c.trim()).filter(Boolean))],
    room: draft.room?.trim() || undefined,
  };

  if (weekMode === "A" || weekMode === "both") {
    weekA = [...weekA, { ...base, id: newId("slot") }];
  }
  if (weekMode === "B" || weekMode === "both") {
    weekB = [...weekB, { ...base, id: newId("slot") }];
  }

  return { ...doc, weekA, weekB };
}

export function removeTeacherSlotEverywhere(
  doc: TeacherPlanningDoc,
  slotId: string,
): TeacherPlanningDoc {
  const prev =
    doc.weekA.find((s) => s.id === slotId) || doc.weekB.find((s) => s.id === slotId) || null;
  if (!prev) {
    return {
      ...doc,
      weekA: doc.weekA.filter((s) => s.id !== slotId),
      weekB: doc.weekB.filter((s) => s.id !== slotId),
    };
  }
  const fp = teacherSlotFingerprint(prev);
  return {
    ...doc,
    weekA: doc.weekA.filter((s) => s.id !== slotId && teacherSlotFingerprint(s) !== fp),
    weekB: doc.weekB.filter((s) => s.id !== slotId && teacherSlotFingerprint(s) !== fp),
  };
}

export function TeacherSlotQuickModal({
  open,
  title,
  slot,
  weekMode,
  catalog,
  onWeekModeChange,
  onChange,
  onClose,
  onSave,
  onDelete,
}: {
  open: boolean;
  title: string;
  slot: TeacherPlanningSlot;
  weekMode: TeacherSlotWeekMode;
  catalog?: TeacherPlanningCatalog | null;
  onWeekModeChange: (mode: TeacherSlotWeekMode) => void;
  onChange: (slot: TeacherPlanningSlot) => void;
  onClose: () => void;
  onSave: () => void;
  onDelete?: () => void;
}) {
  const subjects = catalog?.subjects ?? [];
  const rooms = catalog?.rooms ?? [];
  const classOptions = useMemo(() => {
    if (!catalog) return [];
    const assigned = new Set(catalog.assignedClasses);
    const mine = catalog.assignedClasses;
    const rest = catalog.classes.filter((c) => !assigned.has(c));
    return [...mine, ...rest];
  }, [catalog]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  const subjectListId = "teacher-slot-quick-subjects";
  const roomListId = "teacher-slot-quick-rooms";

  return (
    <div
      className="fixed inset-0 z-50 flex items-end sm:items-center justify-center bg-slate-900/40 p-3"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      onClick={onClose}
    >
      <div
        className="w-full max-w-lg rounded-2xl border border-slate-200 bg-white shadow-xl max-h-[90vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-start justify-between gap-3 border-b border-slate-100 px-4 py-3">
          <div>
            <p className="text-sm font-black text-slate-900">{title}</p>
            <p className="text-xs text-slate-500 mt-0.5">
              Modifiez la classe, les horaires ou la semaine A/B.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="text-xs font-bold text-slate-500 hover:text-slate-800 px-2 py-1"
          >
            Fermer
          </button>
        </div>

        <div className="px-4 py-3 space-y-3">
          <div>
            <p className="text-[11px] font-black uppercase tracking-wide text-slate-500 mb-1.5">
              Semaine type
            </p>
            <div className="flex flex-wrap gap-1.5">
              {(
                [
                  ["A", "Semaine A seulement"],
                  ["B", "Semaine B seulement"],
                  ["both", "A et B (toutes semaines)"],
                ] as const
              ).map(([id, label]) => (
                <button
                  key={id}
                  type="button"
                  onClick={() => onWeekModeChange(id)}
                  className={`px-3 py-1.5 rounded-xl text-xs font-bold border ${
                    weekMode === id
                      ? "bg-slate-900 text-white border-slate-900"
                      : "bg-white text-slate-600 border-slate-200 hover:bg-slate-50"
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-3 gap-2">
            <Field label="Jour">
              <select
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                value={slot.day}
                onChange={(e) =>
                  onChange({ ...slot, day: Number(e.target.value) as PlanningWeekday })
                }
              >
                {PLANNING_WEEKDAYS.map((d) => (
                  <option key={d} value={d}>
                    {PLANNING_WEEKDAY_LABELS[d]}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Début">
              <input
                type="time"
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                value={slot.start}
                onChange={(e) => onChange({ ...slot, start: e.target.value })}
              />
            </Field>
            <Field label="Fin">
              <input
                type="time"
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                value={slot.end}
                onChange={(e) => onChange({ ...slot, end: e.target.value })}
              />
            </Field>
            <Field label="Matière">
              <input
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                list={subjects.length ? subjectListId : undefined}
                placeholder="Matière"
                value={slot.subject}
                onChange={(e) => onChange({ ...slot, subject: e.target.value })}
              />
            </Field>
            <Field label="Salle">
              <input
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                list={rooms.length ? roomListId : undefined}
                placeholder="Salle…"
                value={slot.room || ""}
                onChange={(e) => onChange({ ...slot, room: e.target.value })}
              />
            </Field>
          </div>

          {subjects.length > 0 ? (
            <datalist id={subjectListId}>
              {subjects.map((s) => (
                <option key={s} value={s} />
              ))}
            </datalist>
          ) : null}
          {rooms.length > 0 ? (
            <datalist id={roomListId}>
              {rooms.map((r) => (
                <option key={r} value={r} />
              ))}
            </datalist>
          ) : null}

          <Field label="Classe(s)">
            {classOptions.length > 0 ? (
              <div className="flex flex-wrap gap-1.5 mt-1 max-h-40 overflow-y-auto">
                {classOptions.map((className) => {
                  const checked = (slot.classes || []).includes(className);
                  return (
                    <label
                      key={className}
                      className={`inline-flex items-center gap-1 rounded-lg border px-2 py-1 text-[11px] font-semibold cursor-pointer ${
                        checked
                          ? "border-indigo-300 bg-indigo-100 text-indigo-900"
                          : "border-slate-200 bg-white text-slate-600 hover:bg-slate-50"
                      }`}
                    >
                      <input
                        type="checkbox"
                        className="sr-only"
                        checked={checked}
                        onChange={() => {
                          const current = new Set(slot.classes || []);
                          if (checked) current.delete(className);
                          else current.add(className);
                          onChange({ ...slot, classes: [...current] });
                        }}
                      />
                      {className}
                    </label>
                  );
                })}
              </div>
            ) : (
              <input
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white mt-0.5"
                placeholder="6A, 5B…"
                value={(slot.classes || []).join(", ")}
                onChange={(e) =>
                  onChange({
                    ...slot,
                    classes: e.target.value
                      .split(",")
                      .map((c) => c.trim())
                      .filter(Boolean),
                  })
                }
              />
            )}
          </Field>
        </div>

        <div className="flex flex-wrap items-center gap-2 border-t border-slate-100 px-4 py-3">
          {onDelete ? (
            <button
              type="button"
              onClick={onDelete}
              className="px-3 py-1.5 rounded-xl text-xs font-bold text-rose-600 border border-rose-200 hover:bg-rose-50"
            >
              Supprimer
            </button>
          ) : null}
          <div className="ml-auto flex flex-wrap gap-2">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 rounded-xl text-xs font-bold border border-slate-200 text-slate-600"
            >
              Annuler
            </button>
            <button
              type="button"
              onClick={onSave}
              className="px-3 py-1.5 rounded-xl text-xs font-bold bg-emerald-600 text-white hover:bg-emerald-700"
            >
              Appliquer
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function emptyFixedSlot(day: PlanningWeekday = 1): StaffFixedSlot {
  return {
    id: newId("slot"),
    day,
    start: "08:00",
    end: "12:00",
    label: "",
  };
}

function emptyMissionSlot(day: PlanningWeekday = 1): StaffMissionSlot {
  return {
    id: newId("slot"),
    day,
    start: "08:00",
    end: "09:00",
    mission: "",
    location: "",
  };
}

export function WeekGrid({
  slots,
  renderCard,
  editable = false,
  selectedSlotId = null,
  onSlotClick,
  onEmptyClick,
}: {
  slots: { id: string; day: PlanningWeekday; start: string; end: string }[];
  renderCard: (slot: (typeof slots)[number]) => ReactNode;
  /** Mode édition : clic créneau / clic vide pour ajouter. */
  editable?: boolean;
  selectedSlotId?: string | null;
  onSlotClick?: (slotId: string) => void;
  /** Clic sur une zone vide de la journée (heure arrondie). */
  onEmptyClick?: (day: PlanningWeekday, start: string, end: string) => void;
}) {
  const DAY_START_MIN = 7 * 60;
  const DAY_END_MIN = 19 * 60;
  const PX_PER_MIN = 1.15;
  const totalHeight = (DAY_END_MIN - DAY_START_MIN) * PX_PER_MIN;

  const hours = useMemo(() => {
    const list: number[] = [];
    for (let m = DAY_START_MIN; m <= DAY_END_MIN; m += 60) list.push(m);
    return list;
  }, []);

  const byDay = useMemo(() => {
    const map = new Map<PlanningWeekday, typeof slots>();
    for (const d of PLANNING_WEEKDAYS) map.set(d, []);
    for (const s of slots) {
      map.get(s.day)?.push(s);
    }
    for (const d of PLANNING_WEEKDAYS) {
      map.get(d)?.sort((a, b) => a.start.localeCompare(b.start));
    }
    return map;
  }, [slots]);

  const toMin = (hhmm: string) => {
    const [h, m] = hhmm.split(":").map(Number);
    return (h || 0) * 60 + (m || 0);
  };

  const minToHhmm = (total: number) => {
    const clamped = Math.max(DAY_START_MIN, Math.min(DAY_END_MIN - 30, total));
    const h = Math.floor(clamped / 60);
    const m = clamped % 60;
    return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
  };

  const blockStyle = (start: string, end: string) => {
    const s = Math.max(DAY_START_MIN, toMin(start));
    const e = Math.min(DAY_END_MIN, toMin(end));
    const top = (s - DAY_START_MIN) * PX_PER_MIN;
    const height = Math.max(28, (e - s) * PX_PER_MIN);
    return { top, height };
  };

  const handleDayBackgroundClick = (
    day: PlanningWeekday,
    e: MouseEvent<HTMLDivElement>,
  ) => {
    if (!editable || !onEmptyClick) return;
    if ((e.target as HTMLElement).closest("[data-planning-slot]")) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const y = e.clientY - rect.top;
    const rawMin = DAY_START_MIN + y / PX_PER_MIN;
    const snapped = Math.round(rawMin / 30) * 30;
    const start = minToHhmm(snapped);
    const end = minToHhmm(snapped + 60);
    if (start >= end) return;
    onEmptyClick(day, start, end);
  };

  return (
    <div className="overflow-x-auto rounded-2xl border border-slate-200 bg-gradient-to-br from-white via-slate-50/30 to-indigo-50/20 shadow-sm">
      <div
        className="grid min-w-[820px]"
        style={{ gridTemplateColumns: "48px repeat(5, minmax(0, 1fr))" }}
      >
        <div className="border-b border-slate-100 bg-slate-50/80 px-1 py-2" />
        {PLANNING_WEEKDAYS.map((day) => (
          <div key={`h-${day}`} className={`border-b ${planningDayHeaderClass(day)}`}>
            <p className="text-[10px] font-black uppercase tracking-wide text-slate-600">
              {PLANNING_WEEKDAY_LABELS[day]}
            </p>
          </div>
        ))}

        <div className="relative border-r border-slate-100 bg-slate-50/40" style={{ height: totalHeight }}>
          {hours.map((m) => (
            <div
              key={m}
              className="absolute left-0 right-0 pr-1 text-right text-[10px] font-bold text-slate-400"
              style={{ top: (m - DAY_START_MIN) * PX_PER_MIN - 6 }}
            >
              {String(Math.floor(m / 60)).padStart(2, "0")}:00
            </div>
          ))}
        </div>

        {PLANNING_WEEKDAYS.map((day) => (
          <div
            key={day}
            className={`relative border-l border-slate-100/90 bg-white/60 ${
              editable && onEmptyClick ? "cursor-crosshair" : ""
            }`}
            style={{ height: totalHeight }}
            onClick={(e) => handleDayBackgroundClick(day, e)}
            title={editable ? "Cliquez dans le vide pour ajouter un créneau" : undefined}
          >
            {hours.map((m) => (
              <div
                key={m}
                className={`absolute left-0 right-0 border-t ${
                  m === 12 * 60 ? "border-amber-200/90" : "border-slate-100/90"
                }`}
                style={{ top: (m - DAY_START_MIN) * PX_PER_MIN }}
              />
            ))}
            <div
              className="pointer-events-none absolute left-0 right-0 bg-amber-50/40"
              style={{
                top: (12 * 60 - DAY_START_MIN) * PX_PER_MIN,
                height: 45 * PX_PER_MIN,
              }}
              title="Pause méridienne (indicatif)"
            />
            {(byDay.get(day) || []).map((slot) => {
              const { top, height } = blockStyle(slot.start, slot.end);
              const selected = selectedSlotId === slot.id;
              return (
                <div
                  key={slot.id}
                  data-planning-slot={slot.id}
                  className={`absolute left-1 right-1 z-[1] overflow-hidden ${
                    editable ? "cursor-pointer" : ""
                  } ${selected ? "ring-2 ring-indigo-500 ring-offset-1 rounded-lg z-[2]" : ""}`}
                  style={{ top, height }}
                  onClick={(e) => {
                    if (!editable || !onSlotClick) return;
                    e.stopPropagation();
                    onSlotClick(slot.id);
                  }}
                >
                  <div className="h-full min-h-0 overflow-hidden [&_>_*]:h-full [&_>_*]:min-h-0 [&_>_*]:overflow-hidden">
                    {renderCard(slot)}
                  </div>
                </div>
              );
            })}
          </div>
        ))}
      </div>
      <p className="border-t border-slate-100 px-3 py-1.5 text-[10px] text-slate-400">
        {editable
          ? "Cliquez un créneau pour le modifier · cliquez dans le vide pour en ajouter un (7h–19h)."
          : "Grille horaire réelle (7h–19h) — les trous et la pause midi restent visibles."}
      </p>
    </div>
  );
}

export function TeacherSlotEditor({
  slots,
  onChange,
  catalog,
}: {
  slots: TeacherPlanningSlot[];
  onChange: (slots: TeacherPlanningSlot[]) => void;
  catalog?: TeacherPlanningCatalog | null;
}) {
  const subjects = catalog?.subjects ?? [];
  const rooms = catalog?.rooms ?? [];
  const classOptions = useMemo(() => {
    if (!catalog) return [];
    const assigned = new Set(catalog.assignedClasses);
    const mine = catalog.assignedClasses;
    const rest = catalog.classes.filter((c) => !assigned.has(c));
    return [...mine, ...rest];
  }, [catalog]);

  const subjectListId = "teacher-planning-subjects";
  const roomListId = "teacher-planning-rooms";

  return (
    <div className="space-y-3 border-t border-slate-100 pt-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-xs font-black uppercase tracking-wide text-slate-500">Créneaux</p>
        <div className="flex flex-wrap gap-2">
          {(
            [
              ["08:00", "09:00"],
              ["09:00", "10:00"],
              ["10:00", "11:00"],
              ["11:00", "12:00"],
              ["13:00", "14:00"],
              ["14:00", "15:00"],
              ["15:00", "16:00"],
              ["16:00", "17:00"],
            ] as const
          ).map(([start, end]) => (
            <button
              key={`${start}-${end}`}
              type="button"
              onClick={() =>
                onChange([
                  ...slots,
                  { ...emptyTeacherSlot(), start, end, subject: "", classes: [], room: "" },
                ])
              }
              className="px-2 py-1 rounded-lg text-[10px] font-bold border border-slate-200 text-slate-600 hover:bg-slate-100"
            >
              + {start}–{end}
            </button>
          ))}
          <button
            type="button"
            onClick={() => onChange([...slots, emptyTeacherSlot()])}
            className="text-xs font-bold text-indigo-600 hover:underline px-1"
          >
            + Créneau libre
          </button>
        </div>
      </div>

      {subjects.length > 0 ? (
        <datalist id={subjectListId}>
          {subjects.map((s) => (
            <option key={s} value={s} />
          ))}
        </datalist>
      ) : null}
      {rooms.length > 0 ? (
        <datalist id={roomListId}>
          {rooms.map((r) => (
            <option key={r} value={r} />
          ))}
        </datalist>
      ) : null}

      {slots.map((slot, idx) => (
        <div key={slot.id} className="rounded-xl border border-slate-200 bg-slate-50/80 p-3 space-y-3">
          <div className="grid grid-cols-2 sm:grid-cols-6 gap-2 items-end">
            <Field label="Jour">
              <select
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                value={slot.day}
                onChange={(e) => {
                  const next = [...slots];
                  next[idx] = { ...slot, day: Number(e.target.value) as PlanningWeekday };
                  onChange(next);
                }}
              >
                {PLANNING_WEEKDAYS.map((d) => (
                  <option key={d} value={d}>
                    {PLANNING_WEEKDAY_LABELS[d]}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Début">
              <input
                type="time"
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                value={slot.start}
                onChange={(e) => {
                  const next = [...slots];
                  next[idx] = { ...slot, start: e.target.value };
                  onChange(next);
                }}
              />
            </Field>
            <Field label="Fin">
              <input
                type="time"
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                value={slot.end}
                onChange={(e) => {
                  const next = [...slots];
                  next[idx] = { ...slot, end: e.target.value };
                  onChange(next);
                }}
              />
            </Field>
            <Field label="Matière">
              <input
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                list={subjects.length ? subjectListId : undefined}
                placeholder={subjects.length ? "Choisir ou saisir…" : "Matière"}
                value={slot.subject}
                onChange={(e) => {
                  const next = [...slots];
                  next[idx] = { ...slot, subject: e.target.value };
                  onChange(next);
                }}
              />
            </Field>
            <Field label="Salle">
              <input
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                list={rooms.length ? roomListId : undefined}
                placeholder="Salle 12…"
                value={slot.room || ""}
                onChange={(e) => {
                  const next = [...slots];
                  next[idx] = { ...slot, room: e.target.value };
                  onChange(next);
                }}
              />
            </Field>
            <div className="flex items-end justify-end">
              <button
                type="button"
                className="text-rose-500 text-xs font-bold px-2 py-1.5"
                onClick={() => onChange(slots.filter((s) => s.id !== slot.id))}
              >
                Supprimer
              </button>
            </div>
          </div>

          <Field label="Classe(s)">
            {classOptions.length > 0 ? (
              <div className="flex flex-wrap gap-1.5 mt-1">
                {classOptions.map((className) => {
                  const checked = (slot.classes || []).includes(className);
                  const isAssigned = catalog?.assignedClasses.includes(className);
                  return (
                    <label
                      key={className}
                      className={`inline-flex items-center gap-1 rounded-lg border px-2 py-1 text-[11px] font-semibold cursor-pointer ${
                        checked
                          ? "border-indigo-300 bg-indigo-100 text-indigo-900"
                          : "border-slate-200 bg-white text-slate-600 hover:bg-slate-50"
                      }`}
                    >
                      <input
                        type="checkbox"
                        className="sr-only"
                        checked={checked}
                        onChange={() => {
                          const next = [...slots];
                          const current = new Set(slot.classes || []);
                          if (checked) current.delete(className);
                          else current.add(className);
                          next[idx] = { ...slot, classes: [...current] };
                          onChange(next);
                        }}
                      />
                      {className}
                      {isAssigned ? (
                        <span className="text-[9px] font-black uppercase text-indigo-500/80">
                          moi
                        </span>
                      ) : null}
                    </label>
                  );
                })}
              </div>
            ) : (
              <input
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white mt-0.5"
                placeholder="6A, 6B"
                value={(slot.classes || []).join(", ")}
                onChange={(e) => {
                  const next = [...slots];
                  next[idx] = {
                    ...slot,
                    classes: e.target.value
                      .split(",")
                      .map((c) => c.trim())
                      .filter(Boolean),
                  };
                  onChange(next);
                }}
              />
            )}
          </Field>
        </div>
      ))}
    </div>
  );
}

export function FixedSlotEditor({
  slots,
  onChange,
}: {
  slots: StaffFixedSlot[];
  onChange: (slots: StaffFixedSlot[]) => void;
}) {
  return (
    <div className="space-y-3 border-t border-slate-100 pt-4">
      <div className="flex items-center justify-between">
        <p className="text-xs font-black uppercase tracking-wide text-slate-500">Créneaux</p>
        <button
          type="button"
          onClick={() => onChange([...slots, emptyFixedSlot()])}
          className="text-xs font-bold text-amber-700 hover:underline"
        >
          + Ajouter
        </button>
      </div>
      {slots.map((slot, idx) => (
        <div key={slot.id} className="grid grid-cols-2 sm:grid-cols-5 gap-2 items-end bg-slate-50 rounded-xl p-3">
          <Field label="Jour">
            <select
              className="w-full border rounded-lg px-2 py-1.5 text-sm"
              value={slot.day}
              onChange={(e) => {
                const next = [...slots];
                next[idx] = { ...slot, day: Number(e.target.value) as PlanningWeekday };
                onChange(next);
              }}
            >
              {PLANNING_WEEKDAYS.map((d) => (
                <option key={d} value={d}>
                  {PLANNING_WEEKDAY_LABELS[d]}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Début">
            <input
              type="time"
              className="w-full border rounded-lg px-2 py-1.5 text-sm"
              value={slot.start}
              onChange={(e) => {
                const next = [...slots];
                next[idx] = { ...slot, start: e.target.value };
                onChange(next);
              }}
            />
          </Field>
          <Field label="Fin">
            <input
              type="time"
              className="w-full border rounded-lg px-2 py-1.5 text-sm"
              value={slot.end}
              onChange={(e) => {
                const next = [...slots];
                next[idx] = { ...slot, end: e.target.value };
                onChange(next);
              }}
            />
          </Field>
          <Field label="Poste / libellé">
            <input
              className="w-full border rounded-lg px-2 py-1.5 text-sm"
              value={slot.label}
              onChange={(e) => {
                const next = [...slots];
                next[idx] = { ...slot, label: e.target.value };
                onChange(next);
              }}
            />
          </Field>
          <button
            type="button"
            className="text-rose-500 text-xs font-bold px-2 py-1.5 justify-self-start"
            onClick={() => onChange(slots.filter((s) => s.id !== slot.id))}
          >
            Supprimer
          </button>
        </div>
      ))}
    </div>
  );
}

export function MissionSlotEditor({
  slots,
  onChange,
  onAddRotation,
  rotationLabel,
  onRenameRotation,
}: {
  slots: StaffMissionSlot[];
  onChange: (slots: StaffMissionSlot[]) => void;
  onAddRotation: () => void;
  rotationLabel: string;
  onRenameRotation: (label: string) => void;
}) {
  return (
    <div className="space-y-3 border-t border-slate-100 pt-4">
      <div className="flex flex-wrap items-end gap-3">
        <Field label="Nom de la variante">
          <input
            className="border rounded-lg px-2 py-1.5 text-sm min-w-[160px]"
            value={rotationLabel}
            onChange={(e) => onRenameRotation(e.target.value)}
          />
        </Field>
        <button
          type="button"
          onClick={onAddRotation}
          className="text-xs font-bold text-emerald-700 hover:underline pb-2"
        >
          + Variante de semaine
        </button>
        <button
          type="button"
          onClick={() => onChange([...slots, emptyMissionSlot()])}
          className="ml-auto text-xs font-bold text-emerald-700 hover:underline pb-2"
        >
          + Créneau
        </button>
      </div>
      <datalist id="surveillant-locations">
        {SURVEILLANT_LOCATION_SUGGESTIONS.map((loc) => (
          <option key={loc} value={loc} />
        ))}
      </datalist>
      {slots.map((slot, idx) => (
        <div key={slot.id} className="grid grid-cols-2 sm:grid-cols-6 gap-2 items-end bg-slate-50 rounded-xl p-3">
          <Field label="Jour">
            <select
              className="w-full border rounded-lg px-2 py-1.5 text-sm"
              value={slot.day}
              onChange={(e) => {
                const next = [...slots];
                next[idx] = { ...slot, day: Number(e.target.value) as PlanningWeekday };
                onChange(next);
              }}
            >
              {PLANNING_WEEKDAYS.map((d) => (
                <option key={d} value={d}>
                  {PLANNING_WEEKDAY_LABELS[d]}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Début">
            <input
              type="time"
              className="w-full border rounded-lg px-2 py-1.5 text-sm"
              value={slot.start}
              onChange={(e) => {
                const next = [...slots];
                next[idx] = { ...slot, start: e.target.value };
                onChange(next);
              }}
            />
          </Field>
          <Field label="Fin">
            <input
              type="time"
              className="w-full border rounded-lg px-2 py-1.5 text-sm"
              value={slot.end}
              onChange={(e) => {
                const next = [...slots];
                next[idx] = { ...slot, end: e.target.value };
                onChange(next);
              }}
            />
          </Field>
          <Field label="Mission">
            <input
              className="w-full border rounded-lg px-2 py-1.5 text-sm"
              placeholder="Surveillance, étude…"
              value={slot.mission}
              onChange={(e) => {
                const next = [...slots];
                next[idx] = { ...slot, mission: e.target.value };
                onChange(next);
              }}
            />
          </Field>
          <Field label="Lieu">
            <input
              className="w-full border rounded-lg px-2 py-1.5 text-sm"
              list="surveillant-locations"
              placeholder="Entrée, cour, étude…"
              value={slot.location || ""}
              onChange={(e) => {
                const next = [...slots];
                next[idx] = { ...slot, location: e.target.value };
                onChange(next);
              }}
            />
          </Field>
          <button
            type="button"
            className="text-rose-500 text-xs font-bold px-2 py-1.5 justify-self-start"
            onClick={() => onChange(slots.filter((s) => s.id !== slot.id))}
          >
            Supprimer
          </button>
        </div>
      ))}
    </div>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="block text-[11px] font-bold text-slate-500">
      {label}
      <div className="mt-0.5">{children}</div>
    </label>
  );
}
