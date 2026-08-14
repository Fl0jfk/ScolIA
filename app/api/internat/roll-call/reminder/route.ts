import { NextResponse } from "next/server";
import { requireInternatManage } from "@/app/api/internat/_auth";
import { runInternatRollCallReminder } from "@/app/lib/internat-roll-call-reminder";

export async function POST(req: Request) {
  const body = await req.json().catch(() => ({}));
  const cronSecret = process.env.INTERNAT_CRON_SECRET || process.env.TRAVELS_CRON_SECRET;
  const isCron = Boolean(cronSecret && body.cronSecret === cronSecret);

  if (!isCron) {
    const access = await requireInternatManage();
    if (!access.ok) return access.response;
  }

  try {
    const result = await runInternatRollCallReminder({ force: Boolean(body.force) || isCron });
    return NextResponse.json(result);
  } catch (e) {
    console.error("[internat roll-call reminder]", e);
    return NextResponse.json({ error: "Rappel impossible." }, { status: 500 });
  }
}
