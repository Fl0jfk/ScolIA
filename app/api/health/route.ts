import { NextResponse } from "next/server";
import { getDeployHealthPayload } from "@/app/lib/deploy-health";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

/** Santé déploiement : SHA image + dernière migration (sans donnée métier). */
export async function GET() {
  try {
    const body = await getDeployHealthPayload();
    return NextResponse.json(body, {
      headers: { "Cache-Control": "no-store" },
    });
  } catch {
    return NextResponse.json({ ok: false, error: "service_unavailable" }, { status: 503 });
  }
}
