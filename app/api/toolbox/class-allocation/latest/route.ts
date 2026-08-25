import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { loadCampaignConfig, loadLatestRun } from "@/app/lib/class-allocation-storage";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const campaign = await loadCampaignConfig();
  const run = await loadLatestRun(campaign.id);
  return NextResponse.json({ ok: true, campaignId: campaign.id, run });
}
