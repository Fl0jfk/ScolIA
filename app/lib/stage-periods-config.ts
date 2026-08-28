import { getJson, putJson } from "@/app/lib/s3-storage";
import { classKey } from "@/app/lib/stage-referents-config";
import { STAGE_S3, currentStageSchoolYear } from "@/app/lib/stage-types";

export type StagePeriodReminder = {
  id: string;
  /** Titre court (ex. « Dates 2de — PFMP 1 »). */
  label: string;
  /** Texte affiché aux familles (ex. « Attention : votre stage doit se situer entre… »). */
  message: string;
  /** Période indicative (optionnelle, pour rappel visuel). */
  periodStart?: string;
  periodEnd?: string;
};

export type StageClassPeriod = {
  id: string;
  label: string;
  periodStart: string;
  periodEnd: string;
};

export type StageClassStageConfig = {
  className: string;
  /** Si false, la classe n'apparaît pas dans les référents ni le formulaire public. */
  enabled: boolean;
  /** Une ou plusieurs périodes de stage pour l'année (ex. PFMP 1, PFMP 2). */
  periods: StageClassPeriod[];
  /** Rappels affichés sur le formulaire public pour cette classe. */
  reminders: StagePeriodReminder[];
};

export type StagePeriodsConfig = {
  schoolYear: string;
  updatedAt: string;
  updatedBy?: string;
  classes: StageClassStageConfig[];
};

function normalizeClassName(className: string): string {
  return className.trim();
}

function normalizeReminder(raw: unknown): StagePeriodReminder | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = String(o.id ?? "").trim();
  const label = String(o.label ?? "").trim();
  const message = String(o.message ?? "").trim();
  if (!id || !label || !message) return null;
  const periodStart = typeof o.periodStart === "string" ? o.periodStart.slice(0, 10) : undefined;
  const periodEnd = typeof o.periodEnd === "string" ? o.periodEnd.slice(0, 10) : undefined;
  return { id, label, message, periodStart, periodEnd };
}

function normalizePeriod(raw: unknown): StageClassPeriod | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = String(o.id ?? "").trim();
  const label = String(o.label ?? "").trim();
  const periodStart = typeof o.periodStart === "string" ? o.periodStart.slice(0, 10) : "";
  const periodEnd = typeof o.periodEnd === "string" ? o.periodEnd.slice(0, 10) : "";
  if (!id || !label || !periodStart || !periodEnd) return null;
  return { id, label, periodStart, periodEnd };
}

function normalizeClassConfig(raw: unknown): StageClassStageConfig | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const className = normalizeClassName(String(o.className ?? ""));
  if (!className) return null;
  const enabled = o.enabled !== false;
  const periods = Array.isArray(o.periods)
    ? o.periods.map(normalizePeriod).filter((p): p is StageClassPeriod => p !== null)
    : [];
  const reminders = Array.isArray(o.reminders)
    ? o.reminders.map(normalizeReminder).filter((r): r is StagePeriodReminder => r !== null)
    : [];
  return { className, enabled, periods, reminders };
}

export async function getStagePeriodsConfig(schoolYear: string): Promise<StagePeriodsConfig | null> {
  const hit = await getJson<StagePeriodsConfig>(STAGE_S3.periodsConfig(schoolYear));
  if (!hit?.data?.schoolYear) return null;
  const classes = Array.isArray(hit.data.classes)
    ? hit.data.classes.map(normalizeClassConfig).filter((c): c is StageClassStageConfig => c !== null)
    : [];
  return {
    schoolYear: hit.data.schoolYear,
    updatedAt: hit.data.updatedAt,
    updatedBy: hit.data.updatedBy,
    classes,
  };
}

export async function saveStagePeriodsConfig(config: StagePeriodsConfig): Promise<StagePeriodsConfig> {
  const next: StagePeriodsConfig = {
    schoolYear: config.schoolYear,
    updatedAt: new Date().toISOString(),
    updatedBy: config.updatedBy,
    classes: config.classes
      .map(normalizeClassConfig)
      .filter((c): c is StageClassStageConfig => c !== null),
  };
  await putJson(STAGE_S3.periodsConfig(next.schoolYear), next);
  return next;
}

function findClassConfig(
  config: StagePeriodsConfig | null | undefined,
  className: string,
): StageClassStageConfig | null {
  if (!config || !className.trim()) return null;
  const key = classKey(className);
  return config.classes.find((c) => classKey(c.className) === key) ?? null;
}

/** Classes activées pour les stages (référents, formulaire public). */
export async function listStageEnabledClassNames(schoolYear?: string): Promise<string[]> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStagePeriodsConfig(year);
  if (!config?.classes.length) return [];
  return config.classes
    .filter((c) => c.enabled)
    .map((c) => c.className)
    .sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

/** True si la config périodes existe et contient au moins une classe activée. */
export async function hasStagePeriodsConfig(schoolYear?: string): Promise<boolean> {
  const names = await listStageEnabledClassNames(schoolYear);
  return names.length > 0;
}

/** Vérifie si la classe de l'élève peut déposer une préconvention. */
export async function isClassEligibleForStage(
  className: string,
  schoolYear?: string,
): Promise<{ ok: true } | { ok: false; reason: string }> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStagePeriodsConfig(year);
  if (!config?.classes.length) {
    return { ok: true };
  }
  const entry = findClassConfig(config, className);
  if (!entry) {
    return {
      ok: false,
      reason:
        "Votre classe n'est pas concernée par les stages cette année. Contactez le secrétariat si vous pensez qu'il s'agit d'une erreur.",
    };
  }
  if (!entry.enabled) {
    return {
      ok: false,
      reason:
        "Les stages ne sont pas ouverts pour votre classe pour le moment. Contactez le secrétariat.",
    };
  }
  return { ok: true };
}

export async function getStageRemindersForClass(
  className: string,
  schoolYear?: string,
): Promise<StagePeriodReminder[]> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStagePeriodsConfig(year);
  const entry = findClassConfig(config, className);
  return entry?.reminders ?? [];
}

export async function getStagePeriodsForClass(
  className: string,
  schoolYear?: string,
): Promise<StageClassPeriod[]> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStagePeriodsConfig(year);
  const entry = findClassConfig(config, className);
  return entry?.periods ?? [];
}
