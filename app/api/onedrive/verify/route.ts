import { NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";

/**
 * Vérifie un token Graph délégué (accès OneDrive) sans exposer l'appel Graph
 * dans la console navigateur. Route neutre (hors module OCR) pour stages / OCR / pilotage.
 */
export async function POST(req: Request) {
  const session = await resolveSession();
  if (!session?.userId) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }

  let body: { accessToken?: string };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const accessToken = typeof body.accessToken === "string" ? body.accessToken.trim() : "";
  if (!accessToken) {
    return NextResponse.json({ error: "accessToken requis." }, { status: 400 });
  }

  try {
    const graphRes = await fetch("https://graph.microsoft.com/v1.0/me/drive?$select=id", {
      headers: { Authorization: `Bearer ${accessToken}` },
      cache: "no-store",
    });
    if (graphRes.ok) {
      return NextResponse.json({ ok: true });
    }
    return NextResponse.json({ ok: false, status: graphRes.status });
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    return NextResponse.json({ ok: false, error: message }, { status: 502 });
  }
}
