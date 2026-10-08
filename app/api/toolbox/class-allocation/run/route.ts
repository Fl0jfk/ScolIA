import { NextResponse } from "next/server";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  listParentWishes,
  listScores,
  listStaffWishes,
  loadCampaignConfig,
  saveRun,
} from "@/app/lib/class-allocation-storage";
import { solveClassAllocation } from "@/app/lib/class-allocation-solver";
import { appendClassAllocationAudit } from "@/app/lib/class-allocation-audit";

export async function POST() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const campaign = await loadCampaignConfig();
  const [elevesHit, parentWishes, staffWishes, scores] = await Promise.all([
    loadElevesActifsRegistry(),
    listParentWishes(campaign.id),
    listStaffWishes(campaign.id),
    listScores(campaign.id),
  ]);
  const students = elevesHit;
  const solved = solveClassAllocation({
    campaign,
    students,
    parentWishes,
    staffWishes,
    scores,
  });
  const run = await saveRun(campaign.id, solved);
  await appendClassAllocationAudit({
    at: new Date().toISOString(),
    action: "run_created",
    actor: gate.ctx.userId,
    details: { campaignId: campaign.id, runId: run.id },
  });
  return NextResponse.json({ ok: true, run });
}
