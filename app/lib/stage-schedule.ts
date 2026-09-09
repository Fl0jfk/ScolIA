import type { StageDaySlot, StageSchedule, StageScheduleMode, StageWeekday } from "@/app/lib/stage-types";

/** Lundi → samedi (les dimanches restent exclus). */
export const STAGE_WEEKDAYS: StageWeekday[] = [1, 2, 3, 4, 5, 6];

/** Jours proposés par défaut (lun–ven) ; le samedi reste sélectionnable. */
export const STAGE_DEFAULT_WEEKDAYS: StageWeekday[] = [1, 2, 3, 4, 5];

export const STAGE_WEEKDAY_LABELS: Record<StageWeekday, string> = {
  1: "Lundi",
  2: "Mardi",
  3: "Mercredi",
  4: "Jeudi",
  5: "Vendredi",
  6: "Samedi",
};

export function defaultDayHoursTemplate(): StageDaySlot {
  return {
    hasLunchBreak: true,
    morningStart: "08:00",
    morningEnd: "12:00",
    afternoonStart: "13:00",
    afternoonEnd: "17:00",
  };
}

export function defaultStageSchedule(mode: StageScheduleMode = "uniform_week"): StageSchedule {
  const today = new Date();
  const start = new Date(today);
  start.setDate(start.getDate() + 14);
  const end = new Date(start);
  end.setDate(end.getDate() + 4);
  const template = defaultDayHoursTemplate();

  if (mode === "uniform_week") {
    return {
      mode,
      periodStart: start.toISOString().slice(0, 10),
      periodEnd: end.toISOString().slice(0, 10),
      days: STAGE_DEFAULT_WEEKDAYS.map((weekday) => ({ ...template, weekday })),
    };
  }

  return {
    mode,
    periodStart: start.toISOString().slice(0, 10),
    periodEnd: end.toISOString().slice(0, 10),
    days: [
      {
        ...template,
        date: start.toISOString().slice(0, 10),
      },
    ],
  };
}

/** Applique le modèle horaire aux jours de semaine sélectionnés. */
export function buildUniformWeekDays(
  template: StageDaySlot,
  weekdays: StageWeekday[] = STAGE_DEFAULT_WEEKDAYS,
): StageDaySlot[] {
  const base = { ...template };
  delete base.date;
  const unique = [...new Set(weekdays)].filter((w) => w >= 1 && w <= 6).sort((a, b) => a - b);
  const list = unique.length > 0 ? unique : STAGE_DEFAULT_WEEKDAYS;
  return list.map((weekday) => ({ ...base, weekday }));
}

export function normalizeStageSchedule(raw: unknown): StageSchedule {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const mode: StageScheduleMode = o.mode === "per_day" ? "per_day" : "uniform_week";
  const periodStart = String(o.periodStart ?? "").slice(0, 10);
  const periodEnd = String(o.periodEnd ?? "").slice(0, 10);
  const daysRaw = Array.isArray(o.days) ? o.days : [];

  let days: StageDaySlot[] = daysRaw.map((d) => normalizeDaySlot(d));

  if (mode === "uniform_week" && days.length === 1 && days[0] && !days[0].weekday) {
    days = buildUniformWeekDays(days[0], STAGE_DEFAULT_WEEKDAYS);
  }

  if (mode === "uniform_week" && days.length === 1 && days[0]?.weekday) {
    // Ancien format « 1 template » : on ne force plus lun–ven si un seul weekday
    // est déjà présent — on laisse tel quel.
  }

  if (days.length === 0) {
    return defaultStageSchedule(mode);
  }

  return {
    mode,
    periodStart: periodStart || defaultStageSchedule(mode).periodStart,
    periodEnd: periodEnd || defaultStageSchedule(mode).periodEnd,
    days,
  };
}

function normalizeDaySlot(raw: unknown): StageDaySlot {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const str = (k: string) => {
    const v = o[k];
    return typeof v === "string" && v.trim() ? v.trim() : null;
  };
  const weekday =
    typeof o.weekday === "number" && o.weekday >= 1 && o.weekday <= 6
      ? (o.weekday as StageWeekday)
      : undefined;
  const date = typeof o.date === "string" ? o.date.slice(0, 10) : undefined;

  return {
    date,
    weekday,
    hasLunchBreak: o.hasLunchBreak !== false,
    morningStart: str("morningStart"),
    morningEnd: str("morningEnd"),
    afternoonStart: str("afternoonStart"),
    afternoonEnd: str("afternoonEnd"),
    fullDayStart: str("fullDayStart"),
    fullDayEnd: str("fullDayEnd"),
  };
}

export function formatDaySlotLabel(slot: StageDaySlot): string {
  const weekdayNames = ["", "Lun", "Mar", "Mer", "Jeu", "Ven", "Sam"];
  const head = slot.date
    ? new Date(slot.date).toLocaleDateString("fr-FR", {
        weekday: "short",
        day: "numeric",
        month: "short",
      })
    : slot.weekday
      ? weekdayNames[slot.weekday] ?? `J${slot.weekday}`
      : "Jour";

  if (!slot.hasLunchBreak && slot.fullDayStart && slot.fullDayEnd) {
    return `${head} ${slot.fullDayStart}–${slot.fullDayEnd}`;
  }
  const parts: string[] = [];
  if (slot.morningStart && slot.morningEnd) parts.push(`${slot.morningStart}–${slot.morningEnd}`);
  if (slot.afternoonStart && slot.afternoonEnd) {
    parts.push(`${slot.afternoonStart}–${slot.afternoonEnd}`);
  }
  return `${head} ${parts.join(" / ") || "—"}`;
}

/** Génère une entrée par jour (lun–sam) entre deux dates ISO (mode per_day). */
function expandWeekdayDates(periodStart: string, periodEnd: string): string[] {
  const out: string[] = [];
  const start = new Date(`${periodStart}T12:00:00`);
  const end = new Date(`${periodEnd}T12:00:00`);
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) return out;
  const cur = new Date(start);
  while (cur <= end) {
    const dow = cur.getDay(); // 0=dim … 6=sam
    if (dow >= 1 && dow <= 6) out.push(cur.toISOString().slice(0, 10));
    cur.setDate(cur.getDate() + 1);
  }
  return out;
}

export function buildPerDaySlotsFromTemplate(
  periodStart: string,
  periodEnd: string,
  template: StageDaySlot,
): StageDaySlot[] {
  return expandWeekdayDates(periodStart, periodEnd).map((date) => ({
    ...template,
    date,
    weekday: undefined,
  }));
}

export function scheduleSummary(schedule: StageSchedule): string {
  const labels = schedule.days.map(formatDaySlotLabel);
  return `${schedule.periodStart} → ${schedule.periodEnd} · ${labels.join(", ")}`;
}

export function validateStageSchedule(schedule: StageSchedule): string | null {
  if (!schedule.periodStart || !schedule.periodEnd) return "Période de stage obligatoire.";
  if (schedule.periodEnd < schedule.periodStart) return "La date de fin doit être après le début.";
  if (schedule.days.length === 0) {
    return "Sélectionnez au moins un jour de stage (lundi à samedi).";
  }

  for (const day of schedule.days) {
    if (schedule.mode === "per_day" && !day.date) return "Chaque jour doit avoir une date.";
    if (schedule.mode === "uniform_week" && !day.weekday) {
      return "Chaque jour doit avoir un jour de semaine.";
    }
    if (!day.hasLunchBreak) {
      if (!day.fullDayStart && !(day.morningStart && day.morningEnd)) {
        return "Horaires journée continue incomplets.";
      }
    } else if (!day.morningStart || !day.morningEnd) {
      return "Horaires matin incomplets.";
    }
  }
  return null;
}
