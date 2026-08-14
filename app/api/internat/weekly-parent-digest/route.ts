import { NextResponse } from "next/server";
import { requireInternatManage } from "@/app/api/internat/_auth";
import { runInternatWeeklyParentDigest } from "@/app/lib/internat-weekly-digest";

export async function POST(req: Request) {
  const body = await req.json().catch(() => ({}));
  const cronSecret = process.env.INTERNAT_CRON_SECRET || process.env.TRAVELS_CRON_SECRET;
  const isCron = Boolean(cronSecret && body.cronSecret === cronSecret);

  if (!isCron) {
    const access = await requireInternatManage();
    if (!access.ok) return access.response;
  }

  try {
    const result = await runInternatWeeklyParentDigest({ force: Boolean(body.force) || isCron });
    return NextResponse.json(result);
  } catch (e) {
    console.error("[internat weekly digest]", e);
    return NextResponse.json({ error: "Envoi impossible." }, { status: 500 });
  }
}
