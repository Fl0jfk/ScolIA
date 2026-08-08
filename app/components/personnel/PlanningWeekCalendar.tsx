"use client";

import { useMemo, useState } from "react";
import {
  addDays,
  blocksForPlanningWeek,
  schoolWeekParity,
  startOfWeekMonday,
  toIsoDateLocal,
  weekDayContexts,
  type CalendarBlock,
  type DayContext,
  type LeaveSpan,
} from "@/app/lib/rh/planning-calendar";
import type { SchoolHolidayZone } from "@/app/lib/fr-school-holidays";
import {
  PLANNING_WEEKDAY_LABELS,
  PLANNING_WEEKDAYS,
  type RhPlanningDoc,
} from "@/app/lib/rh/planning-types";

const DAY_START = 7 * 60;
const DAY_END = 19 * 60;
const PX_PER_MIN = 1.1;

const WEEKEND_LABELS = ["Samedi", "Dimanche"] as const;

function formatWeekRange(weekStart: Date) {
  const end = addDays(weekStart, 6);
  const opts: Intl.DateTimeFormatOptions = { day: "numeric", month: "short" };
  return `${weekStart.toLocaleDateString("fr-FR", opts)} – ${end.toLocaleDateString("fr-FR", {
    ...opts,
    year: "numeric",
  })}`;
}

function blockTopHeight(b: CalendarBlock) {
  const start = Math.max(
    DAY_START,
    Number(b.start.split(":")[0]) * 60 + Number(b.start.split(":")[1]),
  );
  const end = Math.min(
    DAY_END,
    Number(b.end.split(":")[0]) * 60 + Number(b.end.split(":")[1]),
  );
  const top = (start - DAY_START) * PX_PER_MIN;
  const height = Math.max(22, (end - start) * PX_PER_MIN);
  return { top, height };
}

const KIND_CLASS: Record<CalendarBlock["kind"], string> = {
  type: "border-indigo-200 bg-indigo-50 text-indigo-950",
  replacement: "border-violet-300 bg-violet-100 text-violet-950",
  exception: "border-amber-300 bg-amber-100 text-amber-950",
  mission: "border-emerald-200 bg-emerald-50 text-emerald-950",
  leave: "border-rose-200 bg-rose-50 text-rose-950",
};

function dayBanner(ctx: DayContext, audience: "teacher" | "staff") {
  if (ctx.kind === "work") return null;
  if (ctx.kind === "weekend") {
    return { text: "Week-end", className: "bg-slate-200/80 text-slate-600" };
  }
  if (ctx.kind === "ferie") {
    return { text: "Jour férié", className: "bg-rose-100 text-rose-800" };
  }
  if (ctx.kind === "leave") {
    return { text: ctx.label, className: "bg-rose-100 text-rose-900" };
  }
  if (ctx.kind === "school_holiday") {
    if (audience === "teacher") {
      return { text: ctx.label, className: "bg-sky-100 text-sky-900" };
    }
    return {
      text: `${ctx.label} · personnel souvent en poste`,
      className: "bg-sky-50 text-sky-800",
    };
  }
  return null;
}

export default function PlanningWeekCalendar({
  planning,
  initialWeekAB,
  leaves = [],
  schoolHolidayZone = null,
}: {
  planning: RhPlanningDoc;
  initialWeekAB?: "A" | "B";
  leaves?: LeaveSpan[];
  schoolHolidayZone?: SchoolHolidayZone | null;
}) {
  const audience = planning.kind === "teacher" ? "teacher" : "staff";
  const [weekStart, setWeekStart] = useState(() => startOfWeekMonday(new Date()));
  const [weekAB, setWeekAB] = useState<"A" | "B">(
    () => initialWeekAB || schoolWeekParity(new Date()),
  );
  const [rotationId, setRotationId] = useState(
    () => (planning.kind === "staff" ? planning.rotations[0]?.id : undefined) || "",
  );

  const dayContexts = useMemo(
    () =>
      weekDayContexts({
        weekStart,
        audience,
        zone: schoolHolidayZone,
        leaves,
        includeWeekend: true,
      }),
    [weekStart, audience, leaves, schoolHolidayZone],
  );

  const blocks = useMemo(
    () =>
      blocksForPlanningWeek({
        planning,
        weekAB,
        rotationId: rotationId || undefined,
        weekStart,
        zone: schoolHolidayZone,
        leaves,
      }),
    [planning, weekAB, rotationId, weekStart, leaves, schoolHolidayZone],
  );

  const hours = useMemo(() => {
    const list: number[] = [];
    for (let m = DAY_START; m <= DAY_END; m += 60) list.push(m);
    return list;
  }, []);

  const totalHeight = (DAY_END - DAY_START) * PX_PER_MIN;
  const holidayHint = dayContexts.find((c) => c.kind === "school_holiday" || c.kind === "leave");

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          className="rounded-xl border border-slate-200 bg-white px-3 py-1.5 text-xs font-bold text-slate-700 hover:bg-slate-50"
          onClick={() => setWeekStart((w) => addDays(w, -7))}
        >
          ← Semaine préc.
        </button>
        <button
          type="button"
          className="rounded-xl border border-slate-200 bg-white px-3 py-1.5 text-xs font-bold text-slate-700 hover:bg-slate-50"
          onClick={() => setWeekStart(startOfWeekMonday(new Date()))}
        >
          Aujourd’hui
        </button>
        <button
          type="button"
          className="rounded-xl border border-slate-200 bg-white px-3 py-1.5 text-xs font-bold text-slate-700 hover:bg-slate-50"
          onClick={() => setWeekStart((w) => addDays(w, 7))}
        >
          Semaine suiv. →
        </button>
        <p className="text-sm font-bold text-slate-800 ml-1">{formatWeekRange(weekStart)}</p>

        {planning.kind === "teacher" ? (
          <div className="ml-auto flex gap-1">
            {(["A", "B"] as const).map((id) => (
              <button
                key={id}
                type="button"
                onClick={() => setWeekAB(id)}
                className={`px-3 py-1.5 rounded-lg text-xs font-bold ${
                  weekAB === id
                    ? "bg-slate-900 text-white"
                    : "bg-white border border-slate-200 text-slate-600"
                }`}
              >
                Type {id}
              </button>
            ))}
          </div>
        ) : null}

        {planning.kind === "staff" &&
        planning.mode === "rotation" &&
        planning.rotations.length > 1 ? (
          <select
            className="ml-auto border rounded-lg px-2 py-1.5 text-xs font-bold"
            value={rotationId}
            onChange={(e) => setRotationId(e.target.value)}
          >
            {planning.rotations.map((r) => (
              <option key={r.id} value={r.id}>
                {r.label}
              </option>
            ))}
          </select>
        ) : null}
      </div>

      <p className="text-[11px] text-slate-500">
        {schoolHolidayZone
          ? `Zone ${schoolHolidayZone} · week-end & jours fériés détectés`
          : "Zone vacances non configurée (Paramètres → Identité) · week-end & jours fériés détectés"}
        {audience === "teacher"
          ? " · vacances scolaires = pas de cours type"
          : " · vacances scolaires = le personnel peut travailler ; CP/RTT validés = congé"}
        . Créneaux identiques adjacents fusionnés.
      </p>

      {!schoolHolidayZone ? (
        <p className="text-xs font-semibold text-amber-900 bg-amber-50 border border-amber-100 rounded-xl px-3 py-2">
          Zone de vacances scolaires non configurée — Paramètres généraux → Identité.
        </p>
      ) : null}

      {holidayHint && holidayHint.kind !== "work" ? (
        <p className="text-xs font-semibold text-sky-900 bg-sky-50 border border-sky-100 rounded-xl px-3 py-2">
          Cette semaine contient : {holidayHint.label}
          {audience === "staff" && holidayHint.kind === "school_holiday"
            ? " — le personnel OGEC est souvent en poste (sauf CP/RTT)."
            : null}
        </p>
      ) : null}

      <div className="overflow-x-auto rounded-2xl border border-slate-200 bg-white">
        <div
          className="grid min-w-[900px]"
          style={{ gridTemplateColumns: "52px repeat(7, minmax(0, 1fr))" }}
        >
          <div className="border-b border-slate-100" />
          {[0, 1, 2, 3, 4, 5, 6].map((i) => {
            const d = addDays(weekStart, i);
            const ctx = dayContexts[i]!;
            const label =
              i < 5 ? PLANNING_WEEKDAY_LABELS[PLANNING_WEEKDAYS[i]!] : WEEKEND_LABELS[i - 5]!;
            const banner = dayBanner(ctx, audience);
            return (
              <div
                key={i}
                className={`border-b border-l border-slate-100 px-1.5 py-2 text-center ${
                  ctx.kind === "weekend" ? "bg-slate-100/80" : ""
                }`}
              >
                <p className="text-[10px] font-black uppercase tracking-wide text-slate-400">
                  {label}
                </p>
                <p className="text-sm font-bold text-slate-800">{toIsoDateLocal(d).slice(8)}</p>
                {banner ? (
                  <p
                    className={`mt-1 rounded-md px-1 py-0.5 text-[9px] font-bold leading-tight ${banner.className}`}
                  >
                    {banner.text}
                  </p>
                ) : null}
              </div>
            );
          })}

          <div className="relative border-r border-slate-100" style={{ height: totalHeight }}>
            {hours.map((m) => (
              <div
                key={m}
                className="absolute left-0 right-0 text-[10px] font-bold text-slate-400 pr-1 text-right"
                style={{ top: (m - DAY_START) * PX_PER_MIN - 6 }}
              >
                {String(Math.floor(m / 60)).padStart(2, "0")}:00
              </div>
            ))}
          </div>

          {[0, 1, 2, 3, 4, 5, 6].map((i) => {
            const ctx = dayContexts[i]!;
            const isWeekend = i >= 5;
            const dayBlocks = isWeekend
              ? []
              : blocks.filter((b) => {
                  if (b.day !== i + 1) return false;
                  if (ctx.suppressTypeSlots && b.kind !== "replacement") return false;
                  return true;
                });

            return (
              <div
                key={i}
                className={`relative border-l border-slate-100 ${
                  isWeekend || ctx.kind === "ferie" || ctx.kind === "leave"
                    ? "bg-slate-100/70"
                    : ctx.kind === "school_holiday"
                      ? audience === "teacher"
                        ? "bg-sky-50/60"
                        : "bg-slate-50/40"
                      : "bg-slate-50/40"
                }`}
                style={{ height: totalHeight }}
              >
                {hours.map((m) => (
                  <div
                    key={m}
                    className="absolute left-0 right-0 border-t border-slate-100/80"
                    style={{ top: (m - DAY_START) * PX_PER_MIN }}
                  />
                ))}
                {dayBlocks.map((b) => {
                  const { top, height } = blockTopHeight(b);
                  return (
                    <div
                      key={b.id}
                      className={`absolute left-1 right-1 overflow-hidden rounded-lg border px-1.5 py-1 shadow-sm ${KIND_CLASS[b.kind]}`}
                      style={{ top, height }}
                      title={`${b.start}–${b.end} ${b.title}`}
                    >
                      <p className="text-[10px] font-black leading-tight">
                        {b.start}–{b.end}
                      </p>
                      <p className="text-[11px] font-bold leading-snug line-clamp-2">{b.title}</p>
                      {b.subtitle ? (
                        <p className="text-[10px] opacity-80 line-clamp-2">{b.subtitle}</p>
                      ) : null}
                    </div>
                  );
                })}
                {dayBlocks.length === 0 &&
                (isWeekend ||
                  ctx.kind === "leave" ||
                  ctx.kind === "ferie" ||
                  (ctx.kind === "school_holiday" && audience === "teacher")) ? (
                  <div className="absolute inset-2 flex items-center justify-center">
                    <p className="text-[10px] font-bold text-slate-400 text-center px-1">
                      {ctx.label || "—"}
                    </p>
                  </div>
                ) : null}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
