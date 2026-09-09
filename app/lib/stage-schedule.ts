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
  const presenceWeekdays = [...STAGE_DEFAULT_WEEKDAYS];

  if (mode === "uniform_week") {
    return {
      mode,
      periodStart: start.toISOString().slice(0, 10),
      periodEnd: end.toISOString().slice(0, 10),
      presenceWeekdays,
      days: STAGE_DEFAULT_WEEKDAYS.map((weekday) => ({ ...template, weekday })),
    };
  }

  return {
    mode,
    periodStart: start.toISOString().slice(0, 10),
    periodEnd: end.toISOString().slice(0, 10),
    presenceWeekdays,
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

  const presenceFromRaw = Array.isArray(o.presenceWeekdays)
    ? o.presenceWeekdays
        .filter((w): w is StageWeekday => typeof w === "number" && w >= 1 && w <= 6)
        .sort((a, b) => a - b)
    : [];

  const presenceFromDays: StageWeekday[] =
    mode === "uniform_week"
      ? [
          ...new Set(
            days
              .map((d) => d.weekday)
              .filter((w): w is StageWeekday => typeof w === "number" && w >= 1 && w <= 6),
          ),
        ].sort((a, b) => a - b)
      : [
          ...new Set(
            days
              .map((d) => {
                if (!d.date) return null;
                const dow = new Date(`${d.date}T12:00:00`).getDay();
                return dow >= 1 && dow <= 6 ? (dow as StageWeekday) : null;
              })
              .filter((w): w is StageWeekday => w !== null),
          ),
        ].sort((a, b) => a - b);

  const presenceWeekdays = Array.isArray(o.presenceWeekdays)
    ? presenceFromRaw
    : presenceFromDays.length > 0
      ? presenceFromDays
      : [...STAGE_DEFAULT_WEEKDAYS];

  if (days.length === 0) {
    const fallback = defaultStageSchedule(mode);
    return { ...fallback, presenceWeekdays };
  }

  return {
    mode,
    periodStart: periodStart || defaultStageSchedule(mode).periodStart,
    periodEnd: periodEnd || defaultStageSchedule(mode).periodEnd,
    presenceWeekdays,
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

/** Génère les dates lun–sam (ou filtre) entre deux bornes ISO. */
export function expandWeekdayDates(
  periodStart: string,
  periodEnd: string,
  weekdays: StageWeekday[] = STAGE_WEEKDAYS,
): string[] {
  const allowed = new Set(weekdays);
  const out: string[] = [];
  const start = new Date(`${periodStart}T12:00:00`);
  const end = new Date(`${periodEnd}T12:00:00`);
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) return out;
  const cur = new Date(start);
  while (cur <= end) {
    const dow = cur.getDay(); // 0=dim … 6=sam
    const stageDow = (dow === 0 ? null : (dow as StageWeekday));
    if (stageDow && allowed.has(stageDow)) {
      out.push(cur.toISOString().slice(0, 10));
    }
    cur.setDate(cur.getDate() + 1);
  }
  return out;
}

export function buildPerDaySlotsFromTemplate(
  periodStart: string,
  periodEnd: string,
  template: StageDaySlot,
  weekdays: StageWeekday[] = STAGE_DEFAULT_WEEKDAYS,
): StageDaySlot[] {
  const base = { ...template };
  delete base.weekday;
  return expandWeekdayDates(periodStart, periodEnd, weekdays).map((date) => ({
    ...base,
    date,
    weekday: undefined,
  }));
}

/**
 * Régénère les jours calendaires en conservant les horaires déjà saisis pour une date.
 * Utile quand on change la période ou les jours de la semaine cochés.
 */
export function rebuildPerDaySlots(params: {
  periodStart: string;
  periodEnd: string;
  weekdays: StageWeekday[];
  existing: StageDaySlot[];
  template?: StageDaySlot;
}): StageDaySlot[] {
  const template = params.template || defaultDayHoursTemplate();
  const byDate = new Map(
    params.existing
      .filter((d) => d.date)
      .map((d) => [d.date!, d] as const),
  );
  return expandWeekdayDates(params.periodStart, params.periodEnd, params.weekdays).map((date) => {
    const prev = byDate.get(date);
    if (prev) {
      return { ...prev, date, weekday: undefined };
    }
    const base = { ...template };
    delete base.weekday;
    return { ...base, date, weekday: undefined };
  });
}

/** Jours de la semaine (lun–sam) réellement présents entre deux dates. */
export function weekdaysPresentInPeriod(
  periodStart: string,
  periodEnd: string,
): StageWeekday[] {
  return [
    ...new Set(
      expandWeekdayDates(periodStart, periodEnd, STAGE_WEEKDAYS).map((date) => {
        const dow = new Date(`${date}T12:00:00`).getDay();
        return dow as StageWeekday;
      }),
    ),
  ].sort((a, b) => a - b);
}

/**
 * Préselection lun–ven selon la période.
 * Le samedi reste selectable manuellement, jamais précoché.
 */
export function suggestPresenceWeekdays(
  periodStart: string,
  periodEnd: string,
): StageWeekday[] {
  return weekdaysPresentInPeriod(periodStart, periodEnd).filter((w) => w <= 5);
}

export function formatWeekdayListFr(weekdays: StageWeekday[]): string {
  const labels = [...new Set(weekdays)]
    .filter((w) => w >= 1 && w <= 6)
    .sort((a, b) => a - b)
    .map((w) => STAGE_WEEKDAY_LABELS[w].toLowerCase());
  if (labels.length === 0) return "";
  if (labels.length === 1) return labels[0];
  if (labels.length === 2) return `${labels[0]} et ${labels[1]}`;
  return `${labels.slice(0, -1).join(", ")} et ${labels[labels.length - 1]}`;
}

export function formatPeriodRangeFr(periodStart: string, periodEnd: string): string {
  const fmt = (iso: string) =>
    new Date(`${iso}T12:00:00`).toLocaleDateString("fr-FR", {
      weekday: "long",
      day: "numeric",
      month: "long",
    });
  if (!periodStart || !periodEnd) return "";
  return `du ${fmt(periodStart)} au ${fmt(periodEnd)}`;
}

/** Jours cochés qui ne tombent dans aucune date de la période. */
export function weekdaysOutsidePeriod(
  weekdays: StageWeekday[],
  periodStart: string,
  periodEnd: string,
): StageWeekday[] {
  const present = new Set(weekdaysPresentInPeriod(periodStart, periodEnd));
  return [...new Set(weekdays)]
    .filter((w) => w >= 1 && w <= 6 && !present.has(w))
    .sort((a, b) => a - b);
}

/**
 * Message d’aide si les jours cochés ne correspondent pas à la période.
 * Ex. : « Vous cochez le lundi, mais la période ne contient que mercredi à samedi. »
 */
export function presencePeriodMismatchMessage(params: {
  selectedWeekdays: StageWeekday[];
  periodStart: string;
  periodEnd: string;
  attemptedWeekday?: StageWeekday;
}): string | null {
  const { periodStart, periodEnd, selectedWeekdays, attemptedWeekday } = params;
  if (!periodStart || !periodEnd || periodEnd < periodStart) return null;

  const present = weekdaysPresentInPeriod(periodStart, periodEnd);
  const presentSet = new Set(present);
  const range = formatPeriodRangeFr(periodStart, periodEnd);

  if (attemptedWeekday && !presentSet.has(attemptedWeekday)) {
    const dayLabel = STAGE_WEEKDAY_LABELS[attemptedWeekday].toLowerCase();
    if (present.length === 0) {
      return `Vous cochez le ${dayLabel}, mais aucune journée ouvrable (lundi–samedi) ne tombe ${range}. Modifiez la date de début.`;
    }
    return `Vous cochez le ${dayLabel}, mais il n’y a aucun ${dayLabel} ${range} — la période ne couvre que ${formatWeekdayListFr(present)} (${present.length} jour${present.length > 1 ? "s" : ""}). Modifiez la date de début (ou de fin).`;
  }

  const outside = weekdaysOutsidePeriod(selectedWeekdays, periodStart, periodEnd);
  if (outside.length === 0) return null;
  if (present.length === 0) {
    return `Les jours cochés (${formatWeekdayListFr(selectedWeekdays)}) ne tombent pas ${range}. Modifiez la date de début.`;
  }
  return `Vous avez coché ${formatWeekdayListFr(selectedWeekdays)}, mais ${range} ne contient que ${formatWeekdayListFr(present)} (${present.length} jour${present.length > 1 ? "s" : ""}). Ajustez la date de début (ou de fin), ou décochez les jours absents.`;
}

export function scheduleSummary(schedule: StageSchedule): string {
  const labels = schedule.days.map(formatDaySlotLabel);
  return `${schedule.periodStart} → ${schedule.periodEnd} · ${labels.join(", ")}`;
}

/** Dates ISO où l'élève est en stage (absent établissement / repas). */
export function expandStagePresenceDates(schedule: StageSchedule): string[] {
  if (schedule.mode === "per_day") {
    return [
      ...new Set(
        schedule.days
          .map((d) => d.date?.slice(0, 10))
          .filter((d): d is string => Boolean(d)),
      ),
    ].sort();
  }
  const weekdays =
    schedule.presenceWeekdays && schedule.presenceWeekdays.length > 0
      ? schedule.presenceWeekdays
      : schedule.days
          .map((d) => d.weekday)
          .filter((w): w is StageWeekday => typeof w === "number" && w >= 1 && w <= 6);
  const list = weekdays.length > 0 ? weekdays : STAGE_DEFAULT_WEEKDAYS;
  return expandWeekdayDates(schedule.periodStart, schedule.periodEnd, list);
}

export function validateStageSchedule(schedule: StageSchedule): string | null {
  if (!schedule.periodStart || !schedule.periodEnd) return "Période de stage obligatoire.";
  if (schedule.periodEnd < schedule.periodStart) return "La date de fin doit être après le début.";
  if (schedule.days.length === 0) {
    return schedule.mode === "per_day"
      ? "Aucun jour de présence dans la période — cochez au moins un jour (lundi à samedi)."
      : "Sélectionnez au moins un jour de stage (lundi à samedi).";
  }

  const selected =
    schedule.presenceWeekdays && schedule.presenceWeekdays.length > 0
      ? schedule.presenceWeekdays
      : schedule.mode === "uniform_week"
        ? schedule.days
            .map((d) => d.weekday)
            .filter((w): w is StageWeekday => typeof w === "number" && w >= 1 && w <= 6)
        : [];
  const mismatch = presencePeriodMismatchMessage({
    selectedWeekdays: selected,
    periodStart: schedule.periodStart,
    periodEnd: schedule.periodEnd,
  });
  if (mismatch) return mismatch;

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
