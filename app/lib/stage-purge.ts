import {
  getConventionsIndex,
  getOffersIndex,
  getStageConvention,
  getStageOffer,
  saveStageConvention,
  saveStageOffer,
} from "@/app/lib/stage-storage";
import type { StageConventionStatus, StageOfferStatus } from "@/app/lib/stage-types";

type StagePurgeResult = {
  schoolYear: string;
  offersArchived: number;
  conventionsArchived: number;
  dryRun: boolean;
};

export async function purgeStageSchoolYear(
  schoolYear: string,
  opts?: { dryRun?: boolean },
): Promise<StagePurgeResult> {
  const dryRun = opts?.dryRun === true;
  let offersArchived = 0;
  let conventionsArchived = 0;

  const offerIndex = await getOffersIndex();
  for (const entry of offerIndex) {
    if (entry.schoolYear !== schoolYear) continue;
    const offer = await getStageOffer(entry.id);
    if (!offer || offer.status === "archived") continue;
    offersArchived += 1;
    if (!dryRun) {
      await saveStageOffer({
        ...offer,
        status: "archived" as StageOfferStatus,
        updatedAt: new Date().toISOString(),
      });
    }
  }

  const convIndex = await getConventionsIndex();
  for (const entry of convIndex) {
    if (entry.schoolYear !== schoolYear) continue;
    const convention = await getStageConvention(entry.id);
    if (!convention || convention.status === "archived") continue;
    conventionsArchived += 1;
    if (!dryRun) {
      const now = new Date().toISOString();
      await saveStageConvention({
        ...convention,
        status: "archived" as StageConventionStatus,
        updatedAt: now,
        history: [
          ...convention.history,
          { at: now, by: "Système", action: "ARCHIVE_FIN_ANNEE", note: schoolYear },
        ],
      });
    }
  }

  return { schoolYear, offersArchived, conventionsArchived, dryRun };
}
