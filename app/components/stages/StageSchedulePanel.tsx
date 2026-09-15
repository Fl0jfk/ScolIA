"use client";

import type { StageConvention, StageDaySlot } from "@/app/lib/stage-types";
import {
  formatDaySlotTimeParts,
  groupStageDaysByCalendarWeek,
  STAGE_WEEKDAY_LABELS,
} from "@/app/lib/stage-schedule";

type Props = {
  periodLabel: string;
  days: StageDaySlot[];
  mode: StageConvention["schedule"]["mode"];
};

function dayCardKey(day: StageDaySlot, index: number) {
  return day.date || (day.weekday != null ? `w-${day.weekday}` : `d-${index}`);
}

function dayCardTitle(day: StageDaySlot) {
  if (day.date) {
    return new Date(`${day.date}T12:00:00`).toLocaleDateString("fr-FR", {
      weekday: "long",
      day: "numeric",
      month: "short",
    });
  }
  if (day.weekday != null && STAGE_WEEKDAY_LABELS[day.weekday]) {
    return STAGE_WEEKDAY_LABELS[day.weekday];
  }
  return "Jour";
}

function TimeChip({
  label,
  start,
  end,
  accent,
}: {
  label: string;
  start: string;
  end: string;
  accent: "morning" | "afternoon" | "full";
}) {
  if (!start && !end) {
    return (
      <div className="flex items-center justify-between gap-2 rounded-md bg-stone-50 px-2 py-1.5 text-[11px] text-stone-400">
        <span>{label}</span>
        <span>—</span>
      </div>
    );
  }

  const tone =
    accent === "morning"
      ? "bg-amber-50 text-amber-950 ring-amber-200/80"
      : accent === "afternoon"
        ? "bg-sky-50 text-sky-950 ring-sky-200/80"
        : "bg-emerald-50 text-emerald-950 ring-emerald-200/80";

  return (
    <div className={`flex items-center justify-between gap-2 rounded-md px-2 py-1.5 text-[11px] ring-1 ${tone}`}>
      <span className="font-semibold opacity-80">{label}</span>
      <span className="font-mono text-xs font-bold tracking-tight">
        {start || "—"}
        <span className="mx-1 opacity-40">→</span>
        {end || "—"}
      </span>
    </div>
  );
}

function DayHoursCard({ day }: { day: StageDaySlot }) {
  const times = formatDaySlotTimeParts(day);

  return (
    <div className="rounded-lg border border-white/80 bg-white/90 p-3 shadow-sm shadow-[#1F3D2B]/5 ring-1 ring-stone-200/70">
      <p className="text-xs font-bold capitalize text-[#1F3D2B]">{dayCardTitle(day)}</p>
      <div className="mt-2 space-y-1.5">
        {times.continuous ? (
          <TimeChip
            label="Journée"
            start={(day.fullDayStart || day.morningStart || "").toString()}
            end={(day.fullDayEnd || day.morningEnd || "").toString()}
            accent="full"
          />
        ) : (
          <>
            <TimeChip
              label="Matin"
              start={day.morningStart || ""}
              end={day.morningEnd || ""}
              accent="morning"
            />
            <TimeChip
              label="Après-midi"
              start={day.afternoonStart || ""}
              end={day.afternoonEnd || ""}
              accent="afternoon"
            />
          </>
        )}
      </div>
    </div>
  );
}

/** Horaires du stage — même visu admin / élève (semaines séparées, cartes jour). */
export default function StageSchedulePanel({ periodLabel, days, mode }: Props) {
  const weekGroups = groupStageDaysByCalendarWeek(days);

  return (
    <div className="overflow-hidden rounded-xl border border-[#2F6B4A]/20 bg-gradient-to-br from-[#f3f8f5] via-white to-[#eef5f1]">
      <div className="flex flex-wrap items-end justify-between gap-2 border-b border-[#2F6B4A]/15 px-4 py-3">
        <div>
          <p className="text-[11px] font-bold uppercase tracking-[0.08em] text-[#2F6B4A]">
            Horaires du stage
          </p>
          <p className="mt-0.5 text-sm font-semibold capitalize text-[#1F3D2B]">{periodLabel}</p>
        </div>
        <span className="rounded-full bg-white/80 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wide text-[#2F6B4A] ring-1 ring-[#2F6B4A]/20">
          {mode === "per_day" ? "Jour par jour" : "Semaine type"}
        </span>
      </div>

      {days.length === 0 ? (
        <p className="px-4 py-4 text-xs text-stone-500">Aucun horaire renseigné.</p>
      ) : (
        <div className="space-y-4 p-3">
          {weekGroups.map((group) => (
            <div key={group.weekKey} className="space-y-2">
              {weekGroups.length > 1 && (
                <div className="flex flex-wrap items-center gap-2 px-1">
                  <span className="rounded-md bg-[#2F6B4A] px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-white">
                    Semaine {group.weekIndex + 1}
                  </span>
                  <span className="text-xs font-medium text-[#1F3D2B]/80">
                    {group.rangeLabel}
                  </span>
                  {group.weekIndex > 0 && (
                    <span className="ml-auto hidden h-px flex-1 bg-[#2F6B4A]/15 sm:block" aria-hidden />
                  )}
                </div>
              )}
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
                {group.days.map((day, index) => (
                  <DayHoursCard key={dayCardKey(day, index)} day={day} />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
