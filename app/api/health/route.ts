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
  } catch (err) {
    const message = err instanceof Error ? err.message : "health_error";
    return NextResponse.json({ ok: false, error: message }, { status: 503 });
  }
}
