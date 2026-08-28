import "server-only";

import { getJson, putJson } from "@/app/lib/s3-storage";
import { purgeStageSchoolYear } from "@/app/lib/stage-purge";
import { currentStageSchoolYear } from "@/app/lib/stage-types";

const AUTO_PURGE_STATE_KEY = "stages/auto-purge-state.json";

type StageAutoPurgeState = {
  lastPurgedSchoolYear: string;
  updatedAt: string;
};

function previousSchoolYear(schoolYear: string): string | null {
  const parts = schoolYear.split("-").map(Number);
  if (parts.length !== 2 || parts.some((n) => Number.isNaN(n))) return null;
  return `${parts[0]! - 1}-${parts[1]! - 1}`;
}

async function readAutoPurgeState(): Promise<StageAutoPurgeState | null> {
  const hit = await getJson<StageAutoPurgeState>(AUTO_PURGE_STATE_KEY);
  if (!hit?.data) return null;
  const year = hit.data.lastPurgedSchoolYear?.trim();
  if (!year) return null;
  return {
    lastPurgedSchoolYear: year,
    updatedAt: hit.data.updatedAt || new Date().toISOString(),
  };
}

async function writeAutoPurgeState(state: StageAutoPurgeState): Promise<void> {
  await putJson(AUTO_PURGE_STATE_KEY, state);
}

/**
 * Archive automatiquement les offres et conventions de l'année scolaire précédente
 * au passage à la nouvelle année. Les conventions déjà rangées dans le dossier élève
 * (PostgreSQL) restent accessibles indépendamment de cette purge S3.
 */
export async function ensureStageYearAutoPurge(): Promise<void> {
  const currentYear = currentStageSchoolYear();
  const yearToPurge = previousSchoolYear(currentYear);
  if (!yearToPurge) return;

  const state = await readAutoPurgeState();
  if (state?.lastPurgedSchoolYear === yearToPurge) return;

  await purgeStageSchoolYear(yearToPurge);
  await writeAutoPurgeState({
    lastPurgedSchoolYear: yearToPurge,
    updatedAt: new Date().toISOString(),
  });
}
