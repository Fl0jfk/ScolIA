import {
  expandStagePresenceDates,
  mondayOfIsoWeek,
  STAGE_WEEKDAY_LABELS,
} from "@/app/lib/stage-schedule";
import {
  currentStageSchoolYear,
  type StageDaySlot,
  type StageSchedule,
  type StageWeekday,
} from "@/app/lib/stage-types";

/** Cycle élève pour les règles de stage (sans dépendre de stage-config serveur). */
export type StageCycleKind = "ecole" | "college" | "lycee";

/** Période où aucun stage n'est autorisé (ex. 2ᵉ semaine de vacances). */
export type StageBlockedPeriod = {
  id: string;
  label: string;
  periodStart: string;
  periodEnd: string;
};

/**
 * Règles paramétrables par cycle (collège / lycée).
 * Les valeurs par défaut suivent le Code du travail mineurs + usages établissement.
 */
export type StageCycleConstraints = {
  /** Début de journée autorisé (HH:mm), ex. 06:00. */
  earliestStart: string;
  /** Fin de journée autorisée (HH:mm), ex. 20:00. */
  latestEnd: string;
  /**
   * Fin plus stricte pour les moins de 16 ans (ex. lycée 22:00 global, 20:00 si &lt;16).
   * Si absent, `latestEnd` s'applique à tous.
   */
  latestEndUnder16?: string;
  /** Jours autorisés (1=lun … 6=sam). Dimanche toujours exclu. */
  allowedWeekdays: StageWeekday[];
  /** Plafond hebdomadaire si âge &lt; 15 ans à la date de début du stage. */
  maxWeeklyHoursUnder15: number;
  /** Plafond hebdomadaire si âge ≥ 15 ans. */
  maxWeeklyHoursFrom15: number;
  /** Durée max d'une journée (heures). Défaut légal : 7. */
  maxDailyHours: number;
};

export type StageConstraintsConfig = {
  schoolYear: string;
  updatedAt: string;
  updatedBy?: string;
  byCycle: {
    college: StageCycleConstraints;
    lycee: StageCycleConstraints;
    ecole: StageCycleConstraints;
  };
  blockedPeriods: StageBlockedPeriod[];
};

/** Contexte public exposé au formulaire (sans métadonnées admin). */
export type StageConstraintsPublicContext = {
  cycle: StageCycleKind;
  rules: StageCycleConstraints;
  blockedPeriods: StageBlockedPeriod[];
};

export function defaultCollegeConstraints(): StageCycleConstraints {
  return {
    earliestStart: "06:00",
    latestEnd: "20:00",
    allowedWeekdays: [1, 2, 3, 4, 5],
    maxWeeklyHoursUnder15: 30,
    maxWeeklyHoursFrom15: 35,
    maxDailyHours: 7,
  };
}

export function defaultLyceeConstraints(): StageCycleConstraints {
  return {
    earliestStart: "06:00",
    latestEnd: "22:00",
    latestEndUnder16: "20:00",
    allowedWeekdays: [1, 2, 3, 4, 5, 6],
    maxWeeklyHoursUnder15: 30,
    maxWeeklyHoursFrom15: 35,
    maxDailyHours: 7,
  };
}

export function defaultStageConstraintsConfig(schoolYear?: string): StageConstraintsConfig {
  const college = defaultCollegeConstraints();
  return {
    schoolYear: schoolYear?.trim() || currentStageSchoolYear(),
    updatedAt: new Date().toISOString(),
    byCycle: {
      college,
      lycee: defaultLyceeConstraints(),
      ecole: { ...college },
    },
    blockedPeriods: [],
  };
}

function normalizeTimeHm(raw: unknown, fallback: string): string {
  const s = typeof raw === "string" ? raw.trim() : "";
  if (/^\d{2}:\d{2}$/.test(s)) return s;
  if (/^\d{1,2}:\d{2}$/.test(s)) {
    const [h, m] = s.split(":");
    return `${h!.padStart(2, "0")}:${m}`;
  }
  return fallback;
}

function normalizeWeekdays(raw: unknown, fallback: StageWeekday[]): StageWeekday[] {
  if (!Array.isArray(raw)) return [...fallback];
  const list = raw
    .map((w) => (typeof w === "number" ? w : Number(w)))
    .filter((w): w is StageWeekday => w === 1 || w === 2 || w === 3 || w === 4 || w === 5 || w === 6);
  const unique = [...new Set(list)].sort((a, b) => a - b) as StageWeekday[];
  return unique.length > 0 ? unique : [...fallback];
}

function normalizePositiveNumber(raw: unknown, fallback: number): number {
  const n = typeof raw === "number" ? raw : Number(raw);
  if (!Number.isFinite(n) || n <= 0) return fallback;
  return Math.round(n * 100) / 100;
}

export function normalizeCycleConstraints(
  raw: unknown,
  fallback: StageCycleConstraints,
): StageCycleConstraints {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const latestEndUnder16Raw =
    typeof o.latestEndUnder16 === "string" && o.latestEndUnder16.trim()
      ? normalizeTimeHm(o.latestEndUnder16, fallback.latestEndUnder16 || fallback.latestEnd)
      : fallback.latestEndUnder16;
  return {
    earliestStart: normalizeTimeHm(o.earliestStart, fallback.earliestStart),
    latestEnd: normalizeTimeHm(o.latestEnd, fallback.latestEnd),
    ...(latestEndUnder16Raw ? { latestEndUnder16: latestEndUnder16Raw } : {}),
    allowedWeekdays: normalizeWeekdays(o.allowedWeekdays, fallback.allowedWeekdays),
    maxWeeklyHoursUnder15: normalizePositiveNumber(
      o.maxWeeklyHoursUnder15,
      fallback.maxWeeklyHoursUnder15,
    ),
    maxWeeklyHoursFrom15: normalizePositiveNumber(
      o.maxWeeklyHoursFrom15,
      fallback.maxWeeklyHoursFrom15,
    ),
    maxDailyHours: normalizePositiveNumber(o.maxDailyHours, fallback.maxDailyHours),
  };
}

export function normalizeBlockedPeriod(raw: unknown): StageBlockedPeriod | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = String(o.id ?? "").trim();
  const label = String(o.label ?? "").trim();
  const periodStart = typeof o.periodStart === "string" ? o.periodStart.slice(0, 10) : "";
  const periodEnd = typeof o.periodEnd === "string" ? o.periodEnd.slice(0, 10) : "";
  if (!id || !label || !periodStart || !periodEnd) return null;
  if (periodEnd < periodStart) return null;
  return { id, label, periodStart, periodEnd };
}

export function normalizeStageConstraintsConfig(
  raw: unknown,
  schoolYear?: string,
): StageConstraintsConfig {
  const defaults = defaultStageConstraintsConfig(schoolYear);
  if (!raw || typeof raw !== "object") return defaults;
  const o = raw as Record<string, unknown>;
  const byCycleRaw =
    o.byCycle && typeof o.byCycle === "object"
      ? (o.byCycle as Record<string, unknown>)
      : {};
  const blocked = Array.isArray(o.blockedPeriods)
    ? o.blockedPeriods
        .map(normalizeBlockedPeriod)
        .filter((p): p is StageBlockedPeriod => p !== null)
    : [];
  return {
    schoolYear: String(o.schoolYear ?? schoolYear ?? defaults.schoolYear).trim() || defaults.schoolYear,
    updatedAt: typeof o.updatedAt === "string" ? o.updatedAt : defaults.updatedAt,
    updatedBy: typeof o.updatedBy === "string" ? o.updatedBy : undefined,
    byCycle: {
      college: normalizeCycleConstraints(byCycleRaw.college, defaults.byCycle.college),
      lycee: normalizeCycleConstraints(byCycleRaw.lycee, defaults.byCycle.lycee),
      ecole: normalizeCycleConstraints(byCycleRaw.ecole, defaults.byCycle.ecole),
    },
    blockedPeriods: blocked,
  };
}

export function resolveCycleConstraints(
  config: StageConstraintsConfig,
  cycle: StageCycleKind,
): StageCycleConstraints {
  return config.byCycle[cycle] ?? config.byCycle.college;
}

/** Minutes depuis minuit pour HH:mm ; null si invalide. */
export function parseTimeToMinutes(value: string | null | undefined): number | null {
  if (!value || typeof value !== "string") return null;
  const m = value.trim().match(/^(\d{1,2}):(\d{2})$/);
  if (!m) return null;
  const h = Number(m[1]);
  const min = Number(m[2]);
  if (!Number.isFinite(h) || !Number.isFinite(min) || h < 0 || h > 23 || min < 0 || min > 59) {
    return null;
  }
  return h * 60 + min;
}

export function formatMinutesAsHours(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = Math.round(minutes % 60);
  if (m === 0) return `${h} h`;
  return `${h} h ${String(m).padStart(2, "0")}`;
}

/** Âge révolu à une date ISO (YYYY-MM-DD). */
export function ageInYearsAt(dateNaissance: string, atIsoDate: string): number | null {
  const dob = dateNaissance.slice(0, 10);
  const at = atIsoDate.slice(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(dob) || !/^\d{4}-\d{2}-\d{2}$/.test(at)) return null;
  const [y, mo, d] = dob.split("-").map(Number);
  const [ay, amo, ad] = at.split("-").map(Number);
  if (!y || !mo || !d || !ay || !amo || !ad) return null;
  let age = ay - y;
  if (amo < mo || (amo === mo && ad < d)) age -= 1;
  return age >= 0 && age < 120 ? age : null;
}

type TimeRange = { start: number; end: number };

function rangesFromDaySlot(slot: StageDaySlot): TimeRange[] {
  if (!slot.hasLunchBreak) {
    const start =
      parseTimeToMinutes(slot.fullDayStart) ?? parseTimeToMinutes(slot.morningStart);
    const end = parseTimeToMinutes(slot.fullDayEnd) ?? parseTimeToMinutes(slot.morningEnd);
    if (start == null || end == null) return [];
    return [{ start, end }];
  }
  const out: TimeRange[] = [];
  const mStart = parseTimeToMinutes(slot.morningStart);
  const mEnd = parseTimeToMinutes(slot.morningEnd);
  if (mStart != null && mEnd != null) out.push({ start: mStart, end: mEnd });
  const aStart = parseTimeToMinutes(slot.afternoonStart);
  const aEnd = parseTimeToMinutes(slot.afternoonEnd);
  if (aStart != null && aEnd != null) out.push({ start: aStart, end: aEnd });
  return out;
}

/** Durée en minutes d'un créneau journalier. */
export function daySlotDurationMinutes(slot: StageDaySlot): number {
  return rangesFromDaySlot(slot).reduce((sum, r) => {
    const delta = r.end - r.start;
    return sum + (delta > 0 ? delta : 0);
  }, 0);
}

export function daySlotDurationHours(slot: StageDaySlot): number {
  return Math.round((daySlotDurationMinutes(slot) / 60) * 100) / 100;
}

function effectiveLatestEndMinutes(rules: StageCycleConstraints, age: number | null): number {
  if (age != null && age < 16 && rules.latestEndUnder16) {
    const under16 = parseTimeToMinutes(rules.latestEndUnder16);
    if (under16 != null) return under16;
  }
  return parseTimeToMinutes(rules.latestEnd) ?? 20 * 60;
}

function weeklyCapHours(rules: StageCycleConstraints, age: number | null): number {
  if (age == null || age < 15) return rules.maxWeeklyHoursUnder15;
  return rules.maxWeeklyHoursFrom15;
}

function datesOverlap(
  aStart: string,
  aEnd: string,
  bStart: string,
  bEnd: string,
): boolean {
  return aStart <= bEnd && bStart <= aEnd;
}

function formatWeekdayList(weekdays: StageWeekday[]): string {
  const labels = [...weekdays]
    .sort((a, b) => a - b)
    .map((w) => STAGE_WEEKDAY_LABELS[w].toLowerCase());
  if (labels.length === 0) return "aucun";
  if (labels.length === 1) return labels[0]!;
  if (labels.length === 2) return `${labels[0]} et ${labels[1]}`;
  return `${labels.slice(0, -1).join(", ")} et ${labels[labels.length - 1]}`;
}

export function describeStageConstraintsRules(rules: StageCycleConstraints): string {
  const days = formatWeekdayList(rules.allowedWeekdays);
  const under16 = rules.latestEndUnder16
    ? ` (moins de 16 ans : jusqu'à ${rules.latestEndUnder16})`
    : "";
  return (
    `Horaires ${rules.earliestStart}–${rules.latestEnd}${under16} · ` +
    `jours : ${days} · ` +
    `max ${rules.maxWeeklyHoursUnder15} h/sem (<15 ans) / ${rules.maxWeeklyHoursFrom15} h/sem (≥15 ans) · ` +
    `max ${rules.maxDailyHours} h/jour`
  );
}

/**
 * Calcule le total hebdomadaire (minutes) selon le mode.
 * - uniform_week : somme des jours cochés (= une semaine type)
 * - per_day : map semaine ISO → minutes
 */
export function computeWeeklyMinutes(schedule: StageSchedule): {
  mode: "uniform" | "per_week";
  uniformMinutes?: number;
  byWeek?: Array<{ weekKey: string; minutes: number }>;
} {
  if (schedule.mode === "uniform_week") {
    const minutes = schedule.days.reduce((sum, d) => sum + daySlotDurationMinutes(d), 0);
    return { mode: "uniform", uniformMinutes: minutes };
  }
  const map = new Map<string, number>();
  for (const day of schedule.days) {
    if (!day.date) continue;
    const key = mondayOfIsoWeek(day.date);
    map.set(key, (map.get(key) ?? 0) + daySlotDurationMinutes(day));
  }
  return {
    mode: "per_week",
    byWeek: [...map.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([weekKey, minutes]) => ({ weekKey, minutes })),
  };
}

export type StageScheduleConstraintContext = {
  rules: StageCycleConstraints;
  blockedPeriods: StageBlockedPeriod[];
  /** Date de naissance ISO — si absente, plafond &lt;15 ans (plus strict). */
  dateNaissance?: string | null;
  cycleLabel?: string;
};

/**
 * Valide horaires / jours / plafonds / périodes bloquées.
 * À appeler après `validateStageSchedule` (structure de base).
 */
export function validateStageScheduleConstraints(
  schedule: StageSchedule,
  ctx: StageScheduleConstraintContext,
): string | null {
  const { rules, blockedPeriods } = ctx;
  const age = ctx.dateNaissance
    ? ageInYearsAt(ctx.dateNaissance, schedule.periodStart || schedule.periodEnd)
    : null;
  const earliest = parseTimeToMinutes(rules.earliestStart) ?? 6 * 60;
  const latest = effectiveLatestEndMinutes(rules, age);
  const allowed = new Set(rules.allowedWeekdays);
  const cycleHint = ctx.cycleLabel ? ` (${ctx.cycleLabel})` : "";

  const selectedWeekdays: StageWeekday[] =
    schedule.presenceWeekdays && schedule.presenceWeekdays.length > 0
      ? schedule.presenceWeekdays
      : schedule.days
          .map((d) => d.weekday)
          .filter((w): w is StageWeekday => typeof w === "number" && w >= 1 && w <= 6);

  for (const weekday of selectedWeekdays) {
    if (!allowed.has(weekday)) {
      return (
        `Le ${STAGE_WEEKDAY_LABELS[weekday].toLowerCase()} n'est pas autorisé pour les stages` +
        `${cycleHint}. Jours possibles : ${formatWeekdayList(rules.allowedWeekdays)}.`
      );
    }
  }

  if (schedule.mode === "per_day") {
    for (const day of schedule.days) {
      if (!day.date) continue;
      const dow = new Date(`${day.date}T12:00:00`).getDay();
      if (dow === 0) {
        return `Le dimanche ${day.date} n'est pas autorisé pour un stage.`;
      }
      const weekday = dow as StageWeekday;
      if (!allowed.has(weekday)) {
        return (
          `Le ${STAGE_WEEKDAY_LABELS[weekday].toLowerCase()} ${new Date(`${day.date}T12:00:00`).toLocaleDateString("fr-FR")}` +
          ` n'est pas autorisé pour les stages${cycleHint}.`
        );
      }
    }
  }

  for (const day of schedule.days) {
    const ranges = rangesFromDaySlot(day);
    for (const range of ranges) {
      if (range.end <= range.start) {
        return "Chaque créneau doit avoir une heure de fin après l'heure de début.";
      }
      if (range.start < earliest || range.end > latest) {
        const latestLabel =
          Math.floor(latest / 60)
            .toString()
            .padStart(2, "0") +
          ":" +
          String(latest % 60).padStart(2, "0");
        const ageNote =
          age != null && age < 16 && rules.latestEndUnder16
            ? ` (élève de moins de 16 ans)`
            : "";
        return (
          `Les horaires doivent être compris entre ${rules.earliestStart} et ${latestLabel}` +
          `${ageNote}${cycleHint}.`
        );
      }
    }

    const dailyHours = daySlotDurationHours(day);
    if (dailyHours > rules.maxDailyHours + 0.001) {
      return (
        `La durée quotidienne ne peut pas dépasser ${rules.maxDailyHours} h` +
        ` (créneau à ${formatMinutesAsHours(daySlotDurationMinutes(day))}).`
      );
    }
  }

  const capHours = weeklyCapHours(rules, age);
  const capMinutes = Math.round(capHours * 60);
  const weekly = computeWeeklyMinutes(schedule);
  if (weekly.mode === "uniform" && weekly.uniformMinutes != null) {
    if (weekly.uniformMinutes > capMinutes) {
      const ageLabel =
        age == null
          ? "moins de 15 ans (plafond le plus strict, date de naissance manquante)"
          : age < 15
            ? "moins de 15 ans"
            : "15 ans ou plus";
      return (
        `Le volume hebdomadaire (${formatMinutesAsHours(weekly.uniformMinutes)}) dépasse le maximum` +
        ` de ${capHours} h pour un élève de ${ageLabel}.`
      );
    }
  } else if (weekly.byWeek) {
    for (const week of weekly.byWeek) {
      if (week.minutes > capMinutes) {
        const ageLabel =
          age == null ? "moins de 15 ans" : age < 15 ? "moins de 15 ans" : "15 ans ou plus";
        return (
          `La semaine du ${new Date(`${week.weekKey}T12:00:00`).toLocaleDateString("fr-FR")}` +
          ` totalise ${formatMinutesAsHours(week.minutes)}, au-delà du maximum de ${capHours} h` +
          ` (${ageLabel}).`
        );
      }
    }
  }

  if (blockedPeriods.length > 0) {
    const presenceDates = expandStagePresenceDates(schedule);
    for (const blocked of blockedPeriods) {
      const hitDates = presenceDates.filter(
        (d) => d >= blocked.periodStart && d <= blocked.periodEnd,
      );
      if (hitDates.length > 0) {
        const first = hitDates[0]!;
        return (
          `La période « ${blocked.label} » (${new Date(`${blocked.periodStart}T12:00:00`).toLocaleDateString("fr-FR")}` +
          ` → ${new Date(`${blocked.periodEnd}T12:00:00`).toLocaleDateString("fr-FR")})` +
          ` est fermée aux stages — présence prévue le ${new Date(`${first}T12:00:00`).toLocaleDateString("fr-FR")}.`
        );
      }
      if (
        schedule.periodStart &&
        schedule.periodEnd &&
        datesOverlap(
          schedule.periodStart,
          schedule.periodEnd,
          blocked.periodStart,
          blocked.periodEnd,
        ) &&
        presenceDates.length === 0
      ) {
        // Période chevauche un blocage mais aucun jour coché dans le chevauchement : OK.
      }
    }
  }

  return null;
}

/** Résumé live pour l'UI (heures / semaine). */
export function stageScheduleHoursSummary(
  schedule: StageSchedule,
  ctx: Pick<StageScheduleConstraintContext, "rules" | "dateNaissance">,
): {
  weeklyHoursLabel: string;
  capHours: number;
  overCap: boolean;
  age: number | null;
} {
  const age = ctx.dateNaissance
    ? ageInYearsAt(ctx.dateNaissance, schedule.periodStart || schedule.periodEnd)
    : null;
  const capHours = weeklyCapHours(ctx.rules, age);
  const weekly = computeWeeklyMinutes(schedule);
  let minutes = 0;
  if (weekly.mode === "uniform") {
    minutes = weekly.uniformMinutes ?? 0;
  } else {
    minutes = Math.max(0, ...(weekly.byWeek?.map((w) => w.minutes) ?? [0]));
  }
  return {
    weeklyHoursLabel: formatMinutesAsHours(minutes),
    capHours,
    overCap: minutes > Math.round(capHours * 60),
    age,
  };
}
