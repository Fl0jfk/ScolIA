import "server-only";

import { getJson, putJson } from "@/app/lib/s3-storage";
import { stageCycleKindFromStudent } from "@/app/lib/stage-config";
import {
  defaultStageConstraintsConfig,
  normalizeStageConstraintsConfig,
  resolveCycleConstraints,
  type StageConstraintsConfig,
  type StageConstraintsPublicContext,
} from "@/app/lib/stage-constraints";
import { STAGE_S3, currentStageSchoolYear } from "@/app/lib/stage-types";

/** Réexport serveur pour les routes API — les composants client doivent importer `@/app/lib/stage-constraints`. */
export * from "@/app/lib/stage-constraints";

export async function getStageConstraintsConfig(
  schoolYear?: string,
): Promise<StageConstraintsConfig> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const hit = await getJson<StageConstraintsConfig>(STAGE_S3.constraintsConfig(year));
  if (!hit?.data) return defaultStageConstraintsConfig(year);
  return normalizeStageConstraintsConfig(hit.data, year);
}

export async function saveStageConstraintsConfig(
  config: StageConstraintsConfig,
): Promise<StageConstraintsConfig> {
  const next = normalizeStageConstraintsConfig(
    {
      ...config,
      updatedAt: new Date().toISOString(),
      updatedBy: config.updatedBy,
    },
    config.schoolYear,
  );
  await putJson(STAGE_S3.constraintsConfig(next.schoolYear), next);
  return next;
}

export async function getStageConstraintsPublicContext(params: {
  level: string;
  className?: string;
  schoolYear?: string;
}): Promise<StageConstraintsPublicContext> {
  const config = await getStageConstraintsConfig(params.schoolYear);
  const cycle = stageCycleKindFromStudent(params.level, params.className);
  return {
    cycle,
    rules: resolveCycleConstraints(config, cycle),
    blockedPeriods: config.blockedPeriods,
  };
}
