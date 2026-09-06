import { NextResponse } from "next/server";
import { processPortesOuvertesFollowUps } from "@/app/lib/portes-ouvertes-mail";
import { requireAdmin } from "@/app/lib/intranet-auth";

export async function POST(req: Request) {
  const body = (await req.json().catch(() => ({}))) as { cronSecret?: string };
  const cronSecret = process.env.PORTES_OUVERTES_CRON_SECRET || process.env.TRAVELS_CRON_SECRET;
  const isCron = Boolean(cronSecret && body.cronSecret === cronSecret);

  if (!isCron) {
    const gate = await requireAdmin();
    if (!gate.ok) return gate.response;
  }

  try {
    const result = await processPortesOuvertesFollowUps();
    return NextResponse.json({ success: true, ...result });
  } catch (e) {
    console.error("[portes-ouvertes follow-up]", e);
    return NextResponse.json({ error: "Traitement impossible." }, { status: 500 });
  }
}
