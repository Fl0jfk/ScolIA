import "server-only";

import {
  assessScheduleAgainstOfficialPeriods,
  type StagePeriodAlignment,
} from "@/app/lib/stage-period-alignment";
import { getStagePeriodsForClass } from "@/app/lib/stage-periods-config";
import type { StageConvention } from "@/app/lib/stage-types";

export async function assessConventionPeriodAlignment(
  convention: Pick<
    StageConvention,
    "student" | "schedule" | "stagePeriodId" | "stageLabel" | "schoolYear"
  >,
): Promise<StagePeriodAlignment> {
  const officialPeriods = await getStagePeriodsForClass(
    convention.student.className,
    convention.schoolYear,
  );
  return assessScheduleAgainstOfficialPeriods({
    className: convention.student.className,
    schedule: convention.schedule,
    officialPeriods,
    stagePeriodId: convention.stagePeriodId,
    stageLabel: convention.stageLabel,
  });
}
