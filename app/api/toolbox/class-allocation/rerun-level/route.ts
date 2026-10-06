import { NextResponse } from "next/server";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  listParentWishes,
  listScores,
  listStaffWishes,
  loadCampaignConfig,
  loadLatestRun,
  saveRun,
} from "@/app/lib/class-allocation-storage";
import { solveClassAllocation } from "@/app/lib/class-allocation-solver";
import type { ClassLevel } from "@/app/lib/class-allocation-types";
import { appendClassAllocationAudit } from "@/app/lib/class-allocation-audit";

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const body = (await req.json()) as { level?: ClassLevel };
  const level = body.level;
  if (!level) return NextResponse.json({ error: "Niveau requis." }, { status: 400 });
  const campaign = await loadCampaignConfig();
  const latest = await loadLatestRun(campaign.id);
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
    onlyLevel: level,
  });
  const merged = latest
    ? {
        levelResults: { ...latest.levelResults, ...solved.levelResults },
        diagnostics: [...latest.diagnostics, ...solved.diagnostics],
        score: solved.score,
      }
    : solved;
  const run = await saveRun(campaign.id, merged);
  await appendClassAllocationAudit({
    at: new Date().toISOString(),
    action: "run_rerun_level",
    actor: gate.ctx.userId,
    details: { campaignId: campaign.id, runId: run.id, level },
  });
  return NextResponse.json({ ok: true, run });
}
