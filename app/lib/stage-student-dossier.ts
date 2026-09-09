import "server-only";

import { listConventionsForDossier } from "@/app/lib/stage-storage";
import {
  getStagePeriodsForClass,
  type StageClassPeriod,
} from "@/app/lib/stage-periods-config";
import {
  isActiveConventionStatus,
  toConventionCard,
  type StageConventionCard,
} from "@/app/lib/stage-signature-summary";
import { currentStageSchoolYear, type StageConvention } from "@/app/lib/stage-types";

export type StagePeriodAvailability = StageClassPeriod & {
  used: boolean;
  conventionId?: string;
};

export type StudentStageDossier = {
  schoolYear: string;
  conventions: StageConventionCard[];
  rawConventions: StageConvention[];
  availablePeriods: StagePeriodAvailability[];
  canCreateNew: boolean;
};

export async function buildStudentStageDossier(params: {
  firstName: string;
  lastName: string;
  className: string;
  schoolYear?: string;
}): Promise<StudentStageDossier> {
  const schoolYear = params.schoolYear?.trim() || currentStageSchoolYear();
  const rawConventions = await listConventionsForDossier({
    firstName: params.firstName,
    lastName: params.lastName,
    className: params.className,
  });

  const yearConventions = rawConventions.filter((c) => c.schoolYear === schoolYear);
  const active = yearConventions.filter((c) => isActiveConventionStatus(c.status));
  const configuredPeriods = await getStagePeriodsForClass(params.className, schoolYear);

  const usedPeriodIds = new Set(
    active.map((c) => c.stagePeriodId).filter((id): id is string => Boolean(id)),
  );

  const availablePeriods: StagePeriodAvailability[] = configuredPeriods.map((p) => {
    const linked = active.find((c) => c.stagePeriodId === p.id);
    return {
      ...p,
      used: usedPeriodIds.has(p.id),
      conventionId: linked?.id,
    };
  });

  // Toujours autoriser une nouvelle demande : les périodes officielles sont
  // indicatives ; l'établissement accepte ou refuse ensuite.
  const canCreateNew = true;

  return {
    schoolYear,
    conventions: yearConventions.map(toConventionCard),
    rawConventions: yearConventions,
    availablePeriods,
    canCreateNew,
  };
}
