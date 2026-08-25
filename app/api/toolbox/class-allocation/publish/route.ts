import { NextResponse } from "next/server";
import { appendClassAllocationAudit } from "@/app/lib/class-allocation-audit";
import { publishClassAllocationRun } from "@/app/lib/class-allocation-publish";
import { loadCampaignConfig, loadLatestRun } from "@/app/lib/class-allocation-storage";
import { requireAdmin } from "@/app/lib/intranet-auth";

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  let runId: string | undefined;
  try {
    const body = (await req.json()) as { runId?: unknown };
    if (typeof body.runId === "string" && body.runId.trim()) {
      runId = body.runId.trim();
    }
  } catch {
    /* body optionnel */
  }

  const campaign = await loadCampaignConfig();
  const latest = await loadLatestRun(campaign.id);
  if (!latest) {
    return NextResponse.json(
      { error: "Aucune proposition à publier. Générez d’abord une répartition." },
      { status: 400 },
    );
  }
  if (runId && latest.id !== runId) {
    return NextResponse.json(
      {
        error:
          "La proposition affichée n’est plus la dernière. Rechargez la page puis republiez.",
      },
      { status: 409 },
    );
  }

  const result = await publishClassAllocationRun(latest);
  await appendClassAllocationAudit({
    at: new Date().toISOString(),
    action: "run_published",
    actor: gate.ctx.userId,
    details: {
      campaignId: campaign.id,
      runId: latest.id,
      updated: result.updated,
      unchanged: result.unchanged,
      missingCount: result.missingInes.length,
    },
  });

  return NextResponse.json({
    ok: true,
    runId: latest.id,
    ...result,
    message:
      result.updated > 0
        ? `${result.updated} élève(s) mis à jour dans le registre (classes cible).`
        : result.unchanged > 0
          ? "Aucune modification : les classes étaient déjà à jour."
          : "Aucun élève affecté dans cette proposition.",
  });
}
