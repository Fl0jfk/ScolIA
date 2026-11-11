import { safeSecretEqual } from "@/app/lib/safe-secret-equal";
import { NextResponse } from "next/server";
import { requirePlatformMaster } from "@/app/lib/intranet-auth";
import { processDunningForAllTenants } from "@/app/lib/tenant-billing";

/** Traite relances et suspensions automatiques (cron Master ou planifié). */
export async function POST(req: Request) {
  const gate = await requirePlatformMaster();
  if (!gate.ok) return gate.response;

  const cronSecret = process.env.BILLING_CRON_SECRET?.trim();
  if (cronSecret) {
    const header = req.headers.get("x-billing-cron-secret") || req.headers.get("authorization");
    if (!safeSecretEqual(header, cronSecret) && !safeSecretEqual(header, `Bearer ${cronSecret}`)) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
    }
  }

  try {
    const result = await processDunningForAllTenants();
    return NextResponse.json({ ok: true, ...result });
  } catch (e) {
    console.error("[billing/dunning/process]", e);
    return NextResponse.json({ error: "Traitement impossible." }, { status: 500 });
  }
}
