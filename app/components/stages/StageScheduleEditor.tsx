"use client";

import { useRef, useState } from "react";
import type { StageDaySlot, StageSchedule, StageScheduleMode, StageWeekday } from "@/app/lib/stage-types";
import {
  STAGE_DEFAULT_WEEKDAYS,
  STAGE_WEEKDAY_LABELS,
  STAGE_WEEKDAYS,
  buildUniformWeekDays,
  defaultDayHoursTemplate,
  formatDaySlotLabel,
  presencePeriodMismatchMessage,
  rebuildPerDaySlots,
  weekdaysPresentInPeriod,
} from "@/app/lib/stage-schedule";

const fieldInputClass =
  "mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm text-stone-900";

function TimeField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <label className="block text-xs font-semibold text-[#1F3D2B]">
      {label}
      <input
        type="time"
        className="mt-1 w-full rounded-lg border-2 border-[#2F6B4A]/40 bg-white px-3 py-2.5 text-sm font-semibold text-[#1F3D2B] shadow-sm focus:border-[#2F6B4A] focus:outline-none focus:ring-2 focus:ring-[#2F6B4A]/30"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
    </label>
  );
}

function DayHoursEditor({
  day,
  onPatch,
}: {
  day: StageDaySlot;
  onPatch: (patch: Partial<StageDaySlot>) => void;
}) {
  return (
    <div className="space-y-3 rounded-xl border border-[#2F6B4A]/25 bg-[#f3faf6] p-3">
      <label className="flex items-center gap-2 text-xs font-semibold text-[#1F3D2B]">
        <input
          type="checkbox"
          checked={day.hasLunchBreak !== false}
          onChange={(e) => onPatch({ hasLunchBreak: e.target.checked })}
        />
        Pause le midi
      </label>
      {day.hasLunchBreak !== false ? (
        <div className="grid grid-cols-2 gap-3">
          <TimeField
            label="Matin — début"
            value={day.morningStart || ""}
            onChange={(v) => onPatch({ morningStart: v })}
          />
          <TimeField
            label="Matin — fin"
            value={day.morningEnd || ""}
            onChange={(v) => onPatch({ morningEnd: v })}
          />
          <TimeField
            label="Après-midi — début"
            value={day.afternoonStart || ""}
            onChange={(v) => onPatch({ afternoonStart: v })}
          />
          <TimeField
            label="Après-midi — fin"
            value={day.afternoonEnd || ""}
            onChange={(v) => onPatch({ afternoonEnd: v })}
          />
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-3">
          <TimeField
            label="Journée — début"
            value={day.fullDayStart || day.morningStart || ""}
            onChange={(v) => onPatch({ fullDayStart: v, morningStart: v })}
          />
          <TimeField
            label="Journée — fin"
            value={day.fullDayEnd || day.morningEnd || ""}
            onChange={(v) => onPatch({ fullDayEnd: v, morningEnd: v })}
          />
        </div>
      )}
    </div>
  );
}

/**
 * Éditeur période / jours / horaires (préconvention, tuteur, admin).
 */
export default function StageScheduleEditor({
  value,
  onChange,
  title = "Période et horaires",
}: {
  value: StageSchedule;
  onChange: (next: StageSchedule) => void;
  title?: string;
}) {
  const schedule = value;
  const hoursTemplate = schedule.days[0] || defaultDayHoursTemplate();
  const periodStartRef = useRef<HTMLInputElement>(null);
  const [periodHint, setPeriodHint] = useState<string | null>(null);

  const selectedWeekdaysList: StageWeekday[] = Array.isArray(schedule.presenceWeekdays)
    ? [...schedule.presenceWeekdays].filter((w) => w >= 1 && w <= 6).sort((a, b) => a - b)
    : schedule.mode === "uniform_week"
      ? (() => {
          const fromDays = schedule.days
            .map((d) => d.weekday)
            .filter((w): w is StageWeekday => typeof w === "number" && w >= 1 && w <= 6);
          return fromDays.length > 0
            ? [...new Set(fromDays)].sort((a, b) => a - b)
            : [...STAGE_DEFAULT_WEEKDAYS];
        })()
      : [...STAGE_DEFAULT_WEEKDAYS];
  const selectedWeekdays = new Set(selectedWeekdaysList);

  const periodMismatch =
    periodHint ||
    presencePeriodMismatchMessage({
      selectedWeekdays: selectedWeekdaysList,
      periodStart: schedule.periodStart,
      periodEnd: schedule.periodEnd,
    });

  function focusPeriodStart(message: string) {
    setPeriodHint(message);
    requestAnimationFrame(() => {
      const el = periodStartRef.current;
      if (!el) return;
      el.scrollIntoView({ behavior: "smooth", block: "center" });
      el.focus();
    });
  }

  function clearPeriodHint() {
    setPeriodHint(null);
  }

  function updateSchedule(patch: Partial<StageSchedule>) {
    onChange({ ...schedule, ...patch });
  }

  function applyUniformHours(patch: Partial<StageDaySlot>) {
    const template = { ...hoursTemplate, ...patch };
    delete template.date;
    delete template.weekday;
    updateSchedule({
      mode: "uniform_week",
      presenceWeekdays: selectedWeekdaysList,
      days:
        selectedWeekdaysList.length === 0
          ? []
          : buildUniformWeekDays(template, selectedWeekdaysList),
    });
  }

  function updateDay(index: number, patch: Partial<StageDaySlot>) {
    const days = [...(schedule.days || [])];
    days[index] = { ...(days[index] || defaultDayHoursTemplate()), ...patch };
    updateSchedule({ days });
  }

  function applyPresenceWeekdays(
    weekdays: StageWeekday[],
    bounds?: { periodStart: string; periodEnd: string },
  ) {
    const periodStart = bounds?.periodStart ?? schedule.periodStart;
    const periodEnd = bounds?.periodEnd ?? schedule.periodEnd;
    const sorted = [...new Set(weekdays)].filter((w) => w >= 1 && w <= 6).sort((a, b) => a - b);
    const template = { ...hoursTemplate };
    delete template.date;
    delete template.weekday;
    if (schedule.mode === "per_day") {
      updateSchedule({
        periodStart,
        periodEnd,
        presenceWeekdays: sorted,
        days:
          sorted.length === 0
            ? []
            : rebuildPerDaySlots({
                periodStart,
                periodEnd,
                weekdays: sorted,
                existing: schedule.days,
                template,
              }),
      });
      return;
    }
    updateSchedule({
      mode: "uniform_week",
      periodStart,
      periodEnd,
      presenceWeekdays: sorted,
      days: sorted.length === 0 ? [] : buildUniformWeekDays(template, sorted),
    });
  }

  function setPresenceWeekdays(weekdays: StageWeekday[]) {
    applyPresenceWeekdays(weekdays);
  }

  function toggleWeekday(weekday: StageWeekday, enabled: boolean) {
    if (enabled) {
      const present = weekdaysPresentInPeriod(schedule.periodStart, schedule.periodEnd);
      if (!present.includes(weekday)) {
        const message = presencePeriodMismatchMessage({
          selectedWeekdays: selectedWeekdaysList,
          periodStart: schedule.periodStart,
          periodEnd: schedule.periodEnd,
          attemptedWeekday: weekday,
        });
        focusPeriodStart(
          message ||
            `Ce jour ne tombe pas dans la période indiquée. Modifiez la date de début.`,
        );
        return;
      }
      clearPeriodHint();
      const weekdays = selectedWeekdaysList.includes(weekday)
        ? selectedWeekdaysList
        : [...selectedWeekdaysList, weekday];
      setPresenceWeekdays(weekdays);
      return;
    }
    clearPeriodHint();
    setPresenceWeekdays(selectedWeekdaysList.filter((w) => w !== weekday));
  }

  function setScheduleMode(nextMode: StageScheduleMode) {
    const template = { ...hoursTemplate };
    delete template.date;
    delete template.weekday;
    if (nextMode === "per_day") {
      updateSchedule({
        mode: "per_day",
        presenceWeekdays: selectedWeekdaysList,
        days:
          selectedWeekdaysList.length === 0
            ? []
            : rebuildPerDaySlots({
                periodStart: schedule.periodStart,
                periodEnd: schedule.periodEnd,
                weekdays: selectedWeekdaysList,
                existing: schedule.days,
                template,
              }),
      });
      return;
    }
    updateSchedule({
      mode: "uniform_week",
      presenceWeekdays: selectedWeekdaysList,
      days:
        selectedWeekdaysList.length === 0
          ? []
          : buildUniformWeekDays(template, selectedWeekdaysList),
    });
  }

  function setPeriodBounds(patch: { periodStart?: string; periodEnd?: string }) {
    clearPeriodHint();
    const periodStart = patch.periodStart ?? schedule.periodStart;
    const periodEnd = patch.periodEnd ?? schedule.periodEnd;
    applyPresenceWeekdays(selectedWeekdaysList, { periodStart, periodEnd });
  }

  return (
    <section className="space-y-4 rounded-xl border border-stone-200 bg-stone-50/80 p-4">
      <h3 className="text-sm font-bold text-[#1F3D2B]">{title}</h3>

      <div className="grid gap-3 sm:grid-cols-2">
        <label className="block text-xs font-semibold text-stone-600">
          Date de début *
          <input
            ref={periodStartRef}
            type="date"
            className={fieldInputClass}
            value={schedule.periodStart}
            onChange={(e) => setPeriodBounds({ periodStart: e.target.value })}
          />
        </label>
        <label className="block text-xs font-semibold text-stone-600">
          Date de fin *
          <input
            type="date"
            className={fieldInputClass}
            value={schedule.periodEnd}
            onChange={(e) => setPeriodBounds({ periodEnd: e.target.value })}
          />
        </label>
      </div>

      {periodMismatch ? (
        <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-800 leading-relaxed">
          {periodMismatch}
        </p>
      ) : null}

      <div className="space-y-2">
        <p className="text-xs font-bold text-[#1F3D2B]">Comment sont vos horaires ?</p>
        <div className="grid gap-2 sm:grid-cols-2">
          <button
            type="button"
            onClick={() => setScheduleMode("uniform_week")}
            className={`rounded-xl border-2 px-4 py-3 text-left transition ${
              schedule.mode === "uniform_week"
                ? "border-[#2F6B4A] bg-[#e8f5ee] shadow-sm"
                : "border-stone-200 bg-white hover:border-stone-300"
            }`}
          >
            <span className="block text-sm font-bold text-[#1F3D2B]">
              Mêmes horaires tous les jours
            </span>
            <span className="mt-1 block text-xs text-stone-600 leading-snug">
              Une seule configuration — appliquée à chaque jour coché.
            </span>
          </button>
          <button
            type="button"
            onClick={() => setScheduleMode("per_day")}
            className={`rounded-xl border-2 px-4 py-3 text-left transition ${
              schedule.mode === "per_day"
                ? "border-[#2F6B4A] bg-[#e8f5ee] shadow-sm"
                : "border-stone-200 bg-white hover:border-stone-300"
            }`}
          >
            <span className="block text-sm font-bold text-[#1F3D2B]">
              Horaires différents chaque jour
            </span>
            <span className="mt-1 block text-xs text-stone-600 leading-snug">
              Chaque date de la période se règle à part.
            </span>
          </button>
        </div>
      </div>

      <div className="rounded-xl border border-stone-200 bg-white p-4 space-y-3">
        <div>
          <p className="text-xs font-bold text-[#1F3D2B]">Jours de présence dans la semaine</p>
          <p className="mt-1 text-xs text-stone-500">
            Cochez les jours où l&apos;élève est présent en entreprise.
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
          {STAGE_WEEKDAYS.map((weekday) => {
            const checked = selectedWeekdays.has(weekday);
            return (
              <label
                key={weekday}
                className={`flex cursor-pointer items-center gap-2 rounded-lg border px-3 py-2 text-sm font-semibold transition ${
                  checked
                    ? "border-[#2F6B4A] bg-[#e8f5ee] text-[#1F3D2B]"
                    : "border-stone-200 bg-stone-50 text-stone-500"
                }`}
              >
                <input
                  type="checkbox"
                  className="accent-[#2F6B4A]"
                  checked={checked}
                  onChange={(e) => toggleWeekday(weekday, e.target.checked)}
                />
                {STAGE_WEEKDAY_LABELS[weekday]}
              </label>
            );
          })}
        </div>
        {selectedWeekdaysList.length === 0 && (
          <p className="text-xs text-rose-700">Sélectionnez au moins un jour.</p>
        )}
      </div>

      {schedule.mode === "uniform_week" ? (
        <div className="space-y-2">
          <p className="text-xs font-bold text-[#1F3D2B]">Horaires (une seule configuration)</p>
          <DayHoursEditor day={hoursTemplate} onPatch={(patch) => applyUniformHours(patch)} />
        </div>
      ) : (
        <div className="space-y-3">
          <p className="text-xs font-bold text-[#1F3D2B]">Horaires jour par jour</p>
          {schedule.days.length === 0 ? (
            <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-800">
              Aucun jour dans la période avec les cases cochées. Élargissez les dates ou cochez
              d&apos;autres jours.
            </p>
          ) : (
            <ul className="max-h-96 space-y-3 overflow-y-auto pr-1">
              {schedule.days.map((day, i) => (
                <li key={day.date || i} className="rounded-xl border border-stone-200 bg-white p-3">
                  <p className="mb-2 text-sm font-bold text-[#1F3D2B]">
                    {day.date
                      ? new Date(`${day.date}T12:00:00`).toLocaleDateString("fr-FR", {
                          weekday: "long",
                          day: "numeric",
                          month: "long",
                        })
                      : formatDaySlotLabel(day)}
                  </p>
                  <DayHoursEditor day={day} onPatch={(patch) => updateDay(i, patch)} />
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}
